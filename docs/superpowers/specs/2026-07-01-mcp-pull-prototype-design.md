# Spec — MCP `pull_prototype` (protocole de pull)

> Issue #2 — « MCP : protocole de pull ». Nouveau tool MCP qui récupère le HTML d'une
> version d'un prototype **et** tous ses fils de commentaires, pour permettre un
> `/latch-pull …` côté Claude : rapatrier la maquette + le feedback et itérer dessus
> sans refournir le contexte ni le fichier à la main.

## 1. Objectif & périmètre

Ajouter un **troisième tool** à l'adaptateur MCP (`backend/src/mcp/mod.rs`), à côté de
`deploy_prototype` et `list_projects` :

```
pull_prototype(slug, version?, deploy_token) -> Json<PullResult>
```

Il renvoie, en un seul appel :
- le **HTML brut** de la version demandée (cible d'édition) ;
- **tous les fils de commentaires non supprimés** de cette version (visiteurs + admin) ;
- des métadonnées de contexte (slug, n° de version, URL publique, `comments_enabled`,
  `release_notes`).

**Hors périmètre (YAGNI, v1) :** écriture depuis MCP (répondre/résoudre un commentaire),
diff inter-versions, pagination, filtrage par auteur. Le tool est **lecture seule**.

## 2. Décisions de cadrage (tranchées en brainstorming)

| Décision | Choix | Raison |
|---|---|---|
| Identifiant projet | **`slug`** (pas l'`id` DB) | Cohérent avec `deploy_prototype`/`list_projects` ; respecte l'invariant §5.1 « jamais d'`id` DB via MCP ». Le flux naturel : `list_projects` → slug → `pull_prototype`. |
| Version | **optionnelle**, défaut = **version active** | Ergonomie `/latch-pull mon-projet` ; on peut cibler un `n` précis. |
| Portée des commentaires | **tous** les fils non supprimés (visiteurs + admin) | Le `deploy_token` = secret du propriétaire ; il rapatrie le feedback de son propre projet, comme la vue Review admin. |
| Ancrage | **descripteur brut**, passé tel quel | Zéro parsing serveur (l'ancre reste 100 % opaque, contrat §3) ; Claude dérive lui-même un libellé depuis le descripteur. |

## 3. Signature & séquence (adaptateur)

Auth **d'abord**, cœur **ensuite** (contrat §1). Même pattern `Parameters<Args>` →
`Json<Result>` que les tools existants.

**Arguments** (`PullArgs`, `Deserialize + JsonSchema`) :
- `slug: String` — slug public du projet (doit exister).
- `version: Option<i32>` — n° de version ; omis → version active.
- `deploy_token: String` — secret, validé contre `DEPLOY_TOKEN`.

**Séquence :**
1. `check_token(&args.deploy_token)?` — **premier geste**, `secure_compare`, avant tout
   accès DB. Échec → `invalid_params "deploy_token invalide"` (identique aux autres tools,
   anti-timing §9.3).
2. `ProjectsService::get_by_slug(slug)` → `NotFound` mappé en `invalid_params "projet inconnu"`.
3. **Résolution de version :**
   - `version = Some(n)` → charge la version `(project_id, n)` ; absente → erreur `invalid_params`
     (« version inconnue »).
   - `version = None` → `project.active_version_id` ; `null` (jamais déployé) → erreur
     `invalid_params` (« aucune version active »).
4. `storage.read(version.html_path)` → HTML brut ; échec I/O → `internal_error "erreur interne"`
   (aucune fuite de chemin).
5. `CommentsService::list_for_version(version.id)` → `Vec<PinWithMessages>` (existant : pins
   vivants + messages vivants, une requête groupée, pas de N+1).
6. Mapping → `PullResult` (§4) et retour `Json`.

Le pull **ne gate pas** sur `comments_enabled` : le propriétaire voit les fils existants même
si les écritures visiteur sont désactivées (parité avec la Review admin). `comments_enabled`
est purement informatif dans la réponse.

## 4. Forme de la réponse

Nouveaux types dans `mcp/mod.rs` (`Serialize + JsonSchema`, **racine objet** — rmcp panique
au boot si le schéma racine est un `array`, cf. QUIRKS) :

```rust
pub struct PullResult {
    pub slug: String,
    pub version: i32,                 // n de la version renvoyée
    pub url: String,                  // URL publique stable (comme DeployResult)
    pub comments_enabled: bool,       // informatif
    pub release_notes: Option<String>,// markdown brut de la version (contexte d'itération)
    pub html: String,                 // HTML brut du proto (cible d'édition)
    pub threads: Vec<PullThread>,     // vide si aucun fil
}

pub struct PullThread {
    pub anchor: String,               // descripteur d'ancrage JSON brut (passé tel quel)
    pub messages: Vec<PullMessage>,
}

pub struct PullMessage {
    pub author_name: String,          // nom auto-déclaré ; côté admin = "admin" (brut)
    pub is_admin: bool,               // dérivé : owner_token == ADMIN_OWNER_TOKEN
    pub body: String,                 // texte brut
    pub created_at: String,           // ISO 8601
}
```

**Mapping (adaptateur) :**
- `PinWithMessages` → `PullThread { anchor: pin.anchor (brut), messages: … }`.
- `comments::Model` → `PullMessage { author_name, is_admin: owner_token == ADMIN_OWNER_TOKEN,
  body, created_at }`.
- Le champ `owner_token` (pin **et** message) n'est **jamais** copié dans les DTO.
- Ordre : pins par `id` asc, messages par `id` asc (déjà garanti par `list_for_version`).
- `url = format!("{}/c/{}", public_base_url, slug)` — `public_base_url` est déjà un champ de
  `LatchMcp` (source `LATCH_PUBLIC_BASE_URL`), identique à `DeployResult.url`.

## 5. Cœur réutilisé / à ajouter

| Besoin | Source |
|---|---|
| Projet par slug | `ProjectsService::get_by_slug` — **existant** |
| Version par `(project_id, n)` et via pointeur actif | **à ajouter** : méthode(s) de lecture dans le cœur (ex. `ProjectsService::get_version(project_id, n)` + résolution de l'active via `active_version_id`), rendant `versions::Model` / `CoreError::NotFound`. Aucun `use axum`/`loco_rs`. |
| Liste des fils d'une version | `CommentsService::list_for_version(version_id)` — **existant** |
| Lecture HTML | `Storage::read(html_path)` — **existant** |
| Sentinelle admin | `ADMIN_OWNER_TOKEN` (`services::comments`) — **existant** |

L'adaptateur MCP orchestre ces appels ; il ne porte aucune logique métier au-delà du mapping
DTO et du gating token.

## 6. Erreurs (via `map_core_err`, zéro fuite)

| Cas | Retour MCP |
|---|---|
| `deploy_token` invalide | `invalid_params "deploy_token invalide"` (avant tout accès DB) |
| slug inconnu | `invalid_params "projet inconnu"` |
| `version = n` inconnue | `invalid_params` (« version inconnue ») |
| aucune version active (défaut) | `invalid_params` (« aucune version active ») |
| échec lecture storage / DB | `internal_error "erreur interne"` |

## 7. Invariants de sécurité (contrat §5/§9)

1. **Token gate en premier** (§9.3) — `secure_compare` avant toute lecture ; le rejet ne doit
   jamais dépendre d'un chemin DB (test asserte le message exact).
2. **`owner_token` jamais sérialisé** (§9.7) — garanti structurellement (les DTO `PullThread`/
   `PullMessage` n'ont pas de champ `owner_token`) + test qui vérifie l'absence dans le JSON.
3. **Pas de PIN, pas de hash, pas d'`id` DB** (§5.1/§9.1/§9.2) — les types ne portent aucun de
   ces champs ; test de non-présence.
4. **Ancre non interprétée** (§3) — le descripteur est passé brut, aucun parsing serveur.

## 8. Tests

Niveau handler (comme les tests existants de `mcp/mod.rs`, appels directs `.await`) + **1 e2e
HTTP** dans `backend/tests/mcp_http.rs` (transport Streamable HTTP réel) :

- **Gate token** : `deploy_token` faux → erreur, message exact, **avant** tout accès DB.
- **Slug inconnu** → erreur.
- **Version** : défaut = active (bon `n`, bon HTML) ; `version=n` explicite ; projet sans
  version active → erreur ; `version` inexistante → erreur.
- **Threads** : mélange d'un fil visiteur et d'un fil admin (sentinelle) → `is_admin` correct
  sur chaque message ; `anchor` brut présent ; ordre déterministe.
- **Invariant `owner_token`** : le JSON sérialisé de `PullResult` ne contient **jamais**
  `owner_token` ni la valeur d'un token visiteur/admin (test qui casse le build).
- **Pas de PIN/hash** : projet à code activé (PIN connu) → le PIN n'apparaît pas dans la réponse.
- **Sans commentaires** : `comments_enabled=false` ou zéro fil → `threads` vide, `html` présent.

## 9. Documentation à mettre à jour

- `docs/contrat-deploy.md` §5 : mentionner le 3ᵉ tool `pull_prototype` (lecture, gardé par token).
- `docs/contrat-deploy.md` §5.1 : forme de la réponse `PullResult` (jamais de hash/PIN/`id`/`owner_token`).
- `docs/INDEX.md`, `docs/HANDOFF.md` en fin d'implémentation.
- `public_docs/` (Fumadocs) : section « publish-from-claude » — documenter le tool et l'usage
  `/latch-pull` (EN). **Pas** de régénération `openapi.json` (les schémas MCP sont auto-générés
  par rmcp, hors OpenAPI REST).

## 10. Découpage d'implémentation (indicatif — détaillé dans le plan)

1. Cœur : méthode(s) de résolution de version (`get_version` + active) + tests unitaires.
2. Adaptateur MCP : `PullArgs`/`PullResult`/`PullThread`/`PullMessage`, tool `pull_prototype`,
   mapping, gating, `map_core_err` (cas version) + tests handler.
3. e2e HTTP `mcp_http.rs` + test invariant `owner_token`.
4. Doc : contrat §5/§5.1, `public_docs`, mémoire (INDEX/HANDOFF).
