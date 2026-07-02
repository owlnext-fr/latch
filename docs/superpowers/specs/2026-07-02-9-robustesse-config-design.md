# Spec — #9 Robustesse config prod : garde-fou chemins relatif/absolu

> Issue : [#9](https://github.com/owlnext-fr/latch/issues/9) (milestone « 🆓 Version non-commercialisable »).
> Branche : `feat/9-robustesse-config`. Date : 2026-07-02.
>
> Ticket issu du triage du parapluie Phase 9 : ne conserve que le **traitement lourd**
> (robustesse de la configuration de production). Items redispatchés : login
> `LanguageSelect` → #21 ; zoom images + relecture doc → #35.

## 1. Problème

Incident prod **2026-06-29** (`docs/QUIRKS.md`) : `LATCH_STORAGE_ROOT=./data` (chemin
**relatif**) résolvait vers `/app/data` — la **couche d'écriture éphémère** du conteneur
(WORKDIR `/app`), et non le volume monté `./data:/data`. Résultat : les HTML de versions
étaient écrits hors du volume et **perdus à chaque recréation du conteneur** (`docker compose
up -d`), pendant que la base SQLite (chemin **absolu** `sqlite:///data/...`) survivait →
404/500 sur toutes les versions, base intacte mais fichiers disparus. Invisible jusqu'au
premier redéploiement.

Le correctif immédiat a été appliqué (`LATCH_STORAGE_ROOT=/data` dans `.env.example`), mais
**rien n'empêche structurellement** de reproduire la mauvaise config : `storage_from_ctx`
(`web/mod.rs:28`) fait `std::env::var("LATCH_STORAGE_ROOT").unwrap_or_else(|_| "data")` — un
défaut **relatif**, aucune validation.

## 2. Objectif

Rendre cette classe d'erreur **impossible en prod par construction** (fail-secure), à l'image
des secrets de cookie : le boot **refuse de démarrer** si une variable de chemin filesystem
est relative hors `Development`/`Test`. Plus documenter le couplage `.env` ↔ volume ↔
`DATABASE_URL` pour la distribution.

## 3. Périmètre

**Couvert par le garde-fou de boot** (2 chemins filesystem passant par `web/mod.rs`) :
- `LATCH_STORAGE_ROOT` — racine des HTML de versions (le coupable direct de l'incident).
- `LATCH_SPA_DIST` — racine des assets buildés de la SPA.

**Audit + doc seulement** (décision explicite, pas un oubli) :
- `DATABASE_URL` — c'est une **URI sqlite** (chemin embarqué, cas `sqlite::memory:` et
  non-sqlite à gérer), consommée **directement par Loco** via `get_env` dans les YAML de
  config, pas via `web/mod.rs`. Son défaut prod (`sqlite:///data/latch.sqlite`) est **déjà
  absolu** et l'idiome URI (`sqlite:///...`) rend le piège « `./data` par réflexe » moins
  probable. On documente l'exigence (chemin absolu obligatoire) plutôt que d'ajouter un
  parseur d'URI + un code-path de validation distinct pour un gain marginal.

**Hors périmètre** (redispatché) : login `LanguageSelect` (#21), zoom images + relecture
`public_docs/` (#35).

## 4. Design

### 4.1 Helper de validation (`backend/src/web/mod.rs`)

Défauts remontés en **constantes partagées** (source unique de vérité — évite le piège
duplication Sonar de `docs/QUIRKS.md`, et la dérive entre le résolveur et le validateur) :

```rust
const STORAGE_ROOT_DEFAULT: &str = "data";
const SPA_DIST_DEFAULT: &str = "../frontend/dist";
```

Nouveau helper privé, calqué sur `resolve_required` (mêmes paramètres, même style d'erreur) :

```rust
/// Valide qu'un chemin de configuration est ABSOLU en production (fail-secure).
/// Un chemin relatif hors Dev/Test résout depuis le WORKDIR `/app` → couche éphémère
/// → perte de données au redéploiement (incident 2026-06-29, cf. QUIRKS).
fn resolve_abs_path(
    env_value: Option<String>,
    is_prod: bool,
    default: &str,
    label: &str,
) -> Result<PathBuf> {
    let raw = env_value
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string());
    let path = PathBuf::from(&raw);
    if is_prod && !path.is_absolute() {
        return Err(loco_rs::Error::Message(format!(
            "{label} doit être un chemin ABSOLU en production (reçu : {raw:?}). \
             Un chemin relatif résout vers la couche éphémère /app/… du conteneur \
             et perd les données au redéploiement (cf. incident 2026-06-29)."
        )));
    }
    Ok(path)
}
```

Note comportementale (voulue) : si la var est **unset en prod**, la valeur effective est le
**défaut relatif** → le garde-fou échoue. C'est fail-secure : mieux vaut refuser le boot que
démarrer sur un chemin éphémère. L'image Docker pose toujours `LATCH_STORAGE_ROOT=/data` et
`LATCH_SPA_DIST=/app/frontend/dist` ; un déploiement prod hors-image devra donc définir ces
deux vars — défense-en-profondeur assumée.

### 4.2 Point d'ancrage boot (`backend/src/app.rs`)

Fonction `validate_path_config(ctx) -> Result<()>` (dans `web/mod.rs`) qui appelle
`resolve_abs_path` pour les deux vars et ne propage que l'`Err` (le `PathBuf` est jeté) :

```rust
pub fn validate_path_config(ctx: &AppContext) -> Result<()> {
    let is_prod = cookie_secure(ctx); // = !matches!(env, Development | Test)
    resolve_abs_path(std::env::var("LATCH_STORAGE_ROOT").ok(), is_prod, STORAGE_ROOT_DEFAULT, "LATCH_STORAGE_ROOT")?;
    resolve_abs_path(std::env::var("LATCH_SPA_DIST").ok(), is_prod, SPA_DIST_DEFAULT, "LATCH_SPA_DIST")?;
    Ok(())
}
```

Appelée **en tête de `after_routes`** (avant tout montage), à côté des fail-fast existants
`unlock_secret(ctx)?` / `deploy_token(ctx)?` :

```rust
async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
    crate::web::validate_path_config(ctx)?; // fail-fast : chemins relatifs interdits en prod
    let store = crate::web::build_session_store(ctx).await?;
    // ...
}
```

`storage_from_ctx` et `spa_dist_dir` **restent infaillibles** (appelés par requête) : ils
consomment désormais les constantes `*_DEFAULT` mais ne changent pas de signature. Le garde-fou
est une vérif de boot **dédiée**, pas un changement de contrat qui rippleraît sur tous les
call-sites per-requête.

### 4.3 Tests (`backend/src/web/mod.rs`, module `#[cfg(test)]`)

Table-driven, calqués sur les tests `resolve_cookie_secret` existants (lignes ~233-315) :

| env_value | is_prod | attendu |
|---|---|---|
| `Some("/data")` | `true` | `Ok` |
| `Some("data")` / `Some("./data")` | `true` | `Err` |
| `None` (défaut relatif) | `true` | `Err` |
| `Some("data")` | `false` (dev) | `Ok` |
| `Some("/abs")` | `false` | `Ok` |

Couvre les deux labels (`LATCH_STORAGE_ROOT`, `LATCH_SPA_DIST`) via le helper commun. La gate
Sonar new-code ≥ 80 % est satisfaite par ces cas (branches Err + Ok, prod + dev).

### 4.4 Documentation

- `docs/ENVIRONMENT.md` : documenter le **nouveau comportement de boot** (« latch refuse de
  démarrer en prod si `LATCH_STORAGE_ROOT` ou `LATCH_SPA_DIST` est relatif ou absent ») +
  expliciter le couplage `.env` ↔ volume `docker-compose.yml` ↔ `DATABASE_URL` (les trois
  doivent pointer la même persistance `/data`).
- `docs/QUIRKS.md` : compléter l'entrée incident existante avec « désormais garde-fou de boot ».
- `public_docs/` (page deploy/configuration) : mentionner l'exigence chemin absolu prod.
- `.env.example` : déjà refondu (sections, requis flaggés, hints `openssl`, avertissement
  storage root) → **vérif de cohérence uniquement**, pas de re-refonte. Ajouter une ligne sur
  `LATCH_SPA_DIST` (actuellement vide) rappelant l'exigence absolue en prod.
- `docs/CONVENTIONS.md` : ajouter le pattern « garde-fou de chemin fail-secure au boot »
  à côté du pattern secrets (si jugé réutilisable).

## 5. Invariant / definition of done

- `cargo fmt` + `cargo clippy -D warnings` verts ; `cargo nextest run` vert (dont les nouveaux
  tests table-driven).
- Un boot simulé en env prod avec `LATCH_STORAGE_ROOT` relatif **échoue** avec un message clair.
- Un boot dev avec chemin relatif **démarre** (comportement dev inchangé).
- Doc à jour (ENVIRONMENT, QUIRKS, public_docs, `.env.example`).
- Gate Sonar new_coverage ≥ 80 % sur le code neuf.
- Mémoire projet mise à jour (INDEX, HANDOFF, éventuellement CONVENTIONS/QUIRKS).

## 6. Risques / notes

- **Faux positif possible** : un déploiement prod hors-image Docker qui omettait `LATCH_SPA_DIST`
  démarrera désormais en erreur. C'est voulu (fail-secure), mais à **signaler clairement** dans
  la doc de migration/déploiement pour éviter la surprise.
- **`is_absolute()` est OS-dépendant** : sur la cible Linux (conteneur + dev), `/data` est absolu,
  `data`/`./data`/`../frontend/dist` sont relatifs — comportement attendu. Pas de cas Windows en
  prod (image Linux).
- Le garde-fou ne valide **pas l'existence** du chemin (un `/data` absolu mais non monté passe) —
  hors périmètre : on cible la classe d'erreur « relatif → éphémère », pas « volume mal monté »
  (qui échouerait de toute façon visiblement à la première écriture).
