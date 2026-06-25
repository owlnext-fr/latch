# Phase 5 — Endpoint MCP + panneau Settings (design)

> Spec validée en brainstorming le 2026-06-25. Le **contrat fait loi** : ce document
> détaille l'implémentation de la Phase 5 du ROADMAP et **étend** le contrat §5/§9
> (endpoint Settings exposant le `deploy_token`, env `LATCH_PUBLIC_BASE_URL`). Le
> contrat sera mis à jour en conséquence pendant l'implémentation (doc d'abord).

## 1. Objectif & périmètre

Livrer la troisième surface du binaire : l'**endpoint MCP** `/mcp` appelé par Claude
pour déployer un prototype, **plus** un **panneau Settings admin minimal** qui affiche
les informations de branchement MCP (token + URL `/mcp`) pour que la surface soit
utilisable **de bout en bout sans accès shell**.

**Dans le périmètre :**
- Adaptateur entrant MCP (`backend/src/mcp/`) : tools `deploy_prototype` + `list_projects`.
- `rmcp ≥ 1.4.0`, transport Streamable HTTP, `allowed_hosts` (CVE Host-header).
- Env `LATCH_PUBLIC_BASE_URL` (source de vérité de l'hôte public, runtime, fail-secure).
- `DEPLOY_TOKEN` fail-secure au boot (refus de démarrer en prod si absent).
- `GET /api/settings` (sous `AdminAuth`) → `{ deploy_token, mcp_url, public_base_url }`.
- Panneau Settings React minimal (infos MCP seulement), accessible depuis la topbar.

**Hors périmètre (reste Phase 7) :** locale/thème dans le menu Settings, logo, titres
de page dynamiques, i18n centralisé.

**Décisions de brainstorming actées :**
1. **Slug inconnu → erreur** : `deploy_prototype` exige que le projet **préexiste**
   (créé via /admin). MCP ne crée jamais de projet, ne touche jamais code/PIN.
2. **`activate` par défaut `true`** : sans précision, la version déployée devient
   immédiatement l'active servie sur `/c/<slug>`.
3. **Réponse `deploy_prototype`** : `{ url, version, code_protected }` — jamais le PIN.
4. **Hôte public via env `LATCH_PUBLIC_BASE_URL`** (runtime, backend uniquement),
   `allowed_hosts` dérivé de son composant hôte (source unique).
5. **SPA admin garde `window.location.origin`** pour l'URL publique par projet
   (runtime-correct, aucun rebuild par déploiement — image distribuée buildée une fois).
6. **Phase 5 élargie** : endpoint Settings + UI minimale livrés avec le MCP.

## 2. Architecture

Adaptateur entrant **fin**, conforme au contrat §1. Le **cœur reste inchangé** et déjà
testé (`ProjectsService`, `DeployService`, `services::security::secure_compare`).

```
backend/src/
  mcp/
    mod.rs          # serveur rmcp : struct LatchMcp { ctx }, ServerHandler,
                    #   2 tools. Valide deploy_token PUIS appelle un service.
                    #   Mappe CoreError → tool-error (mapping local, comme controllers/error.rs).
  controllers/
    settings.rs     # GET /api/settings (AdminAuth) → SettingsResponse
  web/mod.rs        # + deploy_token(ctx) et public_base_url(ctx) (fail-secure, dev fallback)
  dto/mod.rs        # + SettingsResponse, + DeployResult/ProjectSummary (formes MCP si besoin de ToSchema)
  app.rs            # after_routes : nest_service("/mcp", StreamableHttpService(LatchMcp))
```

**L'auth vit dans l'adaptateur** (invariant §9.4) : chaque tool MCP valide le
`deploy_token` avant tout appel au cœur. Le service `deploy()` ne voit jamais de secret
de transport. La garde d'architecture `backend/tests/architecture.rs` (pas de `use axum`/
`use loco_rs` dans `src/services/`) reste verte — le module `mcp/` est un adaptateur, pas
le cœur.

## 3. Surface MCP (contrat §5)

### 3.1 `deploy_prototype(slug, html, deploy_token, activate?)`

1. **Gate token** : `secure_compare(deploy_token, deploy_token_attendu)` → sinon tool-error
   `unauthorized` (même message générique que list, pas d'oracle).
2. **Résolution slug** : `ProjectsService::get_by_slug(slug)` → si `CoreError::NotFound`,
   tool-error « projet inconnu : `<slug>` » (pas d'auto-création).
3. **`activate`** : `Option<bool>`, défaut **`true`**.
4. **Déploiement** : `DeployService::deploy(project.id, &html, activate)` — **même cœur que
   l'admin**, ordre fichier→DB imposé (contrat §8).
5. **Réponse structurée** :
   ```jsonc
   {
     "url": "https://latch.owlnext.fr/c/mon-projet-k7Qp2maZ",  // <public_base_url>/c/<slug>
     "version": 3,             // n de la version créée
     "code_protected": true    // project.code_enabled — Claude prévient « un PIN sera demandé »
   }
   ```
   **Jamais le PIN** (§9.2), jamais de hash (§9.1).

### 3.2 `list_projects(deploy_token)`

1. **Gate token** identique (lecture comprise, §9.3 — gater évite de fuiter la liste).
2. `ProjectsService::list_with_versions()`.
3. **Réponse** : liste de
   ```jsonc
   { "slug": "mon-projet-k7Qp2maZ", "name": "Mon Projet",
     "code_protected": true, "active_version": 3 }   // active_version: Option<i32> (null si jamais déployé)
   ```
   **Pas de PIN, pas de hash, pas d'id interne** — juste de quoi choisir un slug.

### 3.3 Erreurs

Mapping `CoreError` → tool-error **dans l'adaptateur** (le cœur ne connaît pas rmcp) :
- `NotFound` → « projet inconnu ».
- `Validation(msg)` → message renvoyé tel quel (déjà sans secret).
- `Db` / `Io` → message interne **générique** (« erreur interne »), pas de fuite de détail.
- Token invalide → `unauthorized` (avant tout appel au cœur).

## 4. Secrets, hôte public, fail-secure

Réutilise le pattern `resolve_*` de `backend/src/web/mod.rs` (déjà éprouvé pour
`SESSION_SECRET`/`UNLOCK_COOKIE_SECRET`).

- **`DEPLOY_TOKEN`** : validé sur **tous** les tools (§5, §9.3). Helper
  `web::deploy_token(ctx) -> Result<String>` : hors Dev/Test, refus de boot si absent ;
  Dev → fallback déterministe (`dev-deploy-token-...`). Pas de contrainte de longueur 64
  (ce n'est pas une clé HMAC mais un secret partagé ; longueur libre, conseillée longue
  dans `.env.example`).
- **`LATCH_PUBLIC_BASE_URL`** (nouveau) : ex. `https://latch.owlnext.fr`. Helper
  `web::public_base_url(ctx) -> Result<String>` : hors Dev/Test, refus de boot si absent ;
  Dev → `http://localhost:<PORT>` (PORT lu de la config/env, défaut 5150). Normalisée
  (sans `/` final) pour la concaténation `<base>/c/<slug>` et `<base>/mcp`.
- **`allowed_hosts`** (rmcp ≥ 1.4) : **dérivé du composant hôte** de
  `LATCH_PUBLIC_BASE_URL` (parse URL → host). Source unique : pas de seconde liste à
  garder synchrone. Caddy revalide le Host en amont (défense en profondeur).
- **Fail-fast au boot** : `after_routes` (ou `before_run`) appelle `deploy_token(ctx)?` et
  `public_base_url(ctx)?` pour casser le démarrage tôt, comme `unlock_secret(ctx)?`
  aujourd'hui — pas un 500 à la première requête MCP.

## 5. Panneau Settings admin (Phase 5 élargie)

### 5.1 Backend — `GET /api/settings`

- Contrôleur `controllers/settings.rs`, handler sous `AdminAuth` (401 sans session).
- **Pas de garde `require_same_origin`** : c'est une lecture (GET), pas une mutation.
- DTO `SettingsResponse { deploy_token: String, mcp_url: String, public_base_url: String }` :
  - `public_base_url` = `web::public_base_url(ctx)`.
  - `mcp_url` = `<public_base_url>/mcp`.
  - `deploy_token` = `web::deploy_token(ctx)`.
- Annoté `#[utoipa::path]`, enregistré dans `openapi::ApiDoc` → `openapi.json` régénéré +
  test de drift (`backend/tests/openapi_drift.rs`) vert + `schema.d.ts` régénéré.
- **Sécurité** : expose un secret applicatif à un **admin déjà authentifié** — acceptable,
  même logique que le PIN au détail (§9.2 ne vise que PIN/hash ; un admin a le contrôle
  total). À acter au contrat §5/§9 (note explicite). Les invariants §9.1 (pas de hash) et
  §9.2 (PIN absent des listes / du MCP) restent vrais : le `deploy_token` n'est ni un hash
  ni un PIN, et n'apparaît **que** sur cet endpoint admin, jamais via MCP ni dans la liste
  de projets.

### 5.2 Frontend — panneau Settings minimal

- Accès depuis la **topbar** (`components/topbar.tsx`) : icône/lien « Settings » → route
  `/admin/settings` (TanStack Router, code-based) **ou** `<Sheet>` (à trancher au plan ;
  une route est plus naturelle pour une future page Settings Phase 7).
- Contenu (lecture seule, infos de branchement MCP) :
  - **`deploy_token`** : champ masqué `••••••` + œil révéler/masquer + bouton copier —
    **réutilise le composant `PinField`** (déjà : masque, reveal, CopyButton).
  - **`mcp_url`** : texte + `CopyButton`.
  - **`public_base_url`** : texte (lecture seule, informatif).
  - Court texte d'aide i18n : « Connecteur MCP Claude — coller cette URL et ce token ».
- Hook TanStack Query `useSettings()` (`hooks/use-settings.ts`), GET `/api/settings`.
- i18n : nouvelles clés `settings.*` (FR + EN) dans les catalogues admin.

## 6. Montage (app.rs `after_routes`)

```
// fail-fast secrets/config MCP (comme unlock_secret aujourd'hui)
crate::web::deploy_token(ctx)?;
crate::web::public_base_url(ctx)?;

// MCP : nest sous /mcp, allowed_hosts dérivé de public_base_url
let mcp = crate::mcp::service(ctx.clone())?;   // StreamableHttpService(LatchMcp)
let router = router.nest_service("/mcp", mcp);
```

API exacte de `StreamableHttpService` / `ServerHandler` / macros `#[tool]` / `allowed_hosts`
à **résoudre via Context7** (`rmcp`) à l'implémentation — versions pré-1.0, l'API a sauté en
1.x. Le `/api/settings` passe par `AppRoutes` (`controllers::settings::routes()`).

## 7. Tests (critères de sortie ROADMAP Phase 5)

- **Gate token sur les DEUX tools** (lecture comprise, §9.3) : token absent/faux →
  erreur, sur `deploy_prototype` **et** `list_projects`. Token valide → succès.
- **`deploy_prototype` crée une version** + flippe le pointeur si `activate` (et ne flippe
  pas si `activate=false`).
- **Slug inconnu → erreur** (pas d'auto-création).
- **Invariant MCP** : la réponse `list_projects` ne contient **ni PIN ni hash** ;
  `deploy_prototype` ne renvoie pas le PIN.
- **`GET /api/settings`** : 401 sans session ; avec session, renvoie `deploy_token` +
  `mcp_url` (= base + `/mcp`) + `public_base_url`.
- **Frontend** : Vitest + MSW sur le panneau Settings (token masqué par défaut, reveal
  affiche, copie OK) — réutilise le harness existant.
- **Approche MCP** : tester la **logique des tools** au niveau handler (appel des fns de
  tool avec un `AppContext` de test SQLite in-memory) plutôt que monter le transport HTTP
  complet — léger, déterministe, cohérent avec les tests existants. Le harness précis
  (appel direct vs client rmcp in-process) sera tranché au plan via Context7.

## 8. Dépendances & versions

- **`rmcp ≥ 1.4.0`** ajouté à `backend/Cargo.toml`, features
  `["transport-streamable-http-server", "server", ...]` (exact à résoudre via Context7),
  **épinglé**. Compat **axum 0.8 / loco** à vérifier (transport-streamable-http-server
  expose un service tower montable via `nest_service`).
- `cargo deny` / `cargo audit` doivent rester verts (nouvelles deps rmcp).

## 9. Risques & points à lever (Context7 obligatoire)

- **API rmcp 1.4** : définition d'un serveur de tools (`ServerHandler`, macros `#[tool]` /
  `#[tool_router]`), signature de `StreamableHttpService`, configuration `allowed_hosts`,
  forme exacte des réponses de tool (texte vs JSON structuré). À résoudre **avant de coder**.
- **Compat axum 0.8** du transport HTTP rmcp — si friction, fallback documenté à évaluer.
- **Harness de test** des tools (handler direct vs client in-process) — confirmer au plan.

## 10. Mémoire à mettre à jour en clôture (définition de « terminé »)

- **`docs/contrat-deploy.md`** : §5 (réponses des tools, `LATCH_PUBLIC_BASE_URL`,
  `allowed_hosts`) + note §9 (endpoint `/api/settings` exposant `deploy_token` à l'admin
  authentifié ; invariants §9.1/§9.2 toujours vrais).
- **`docs/ROADMAP.md`** : Phase 5 ✅ LIVRÉE.
- **`docs/INDEX.md`** : livrables backend MCP + settings + frontend panneau.
- **`docs/HANDOFF.md`** : entrée datée (état, suspens, prochaine chose).
- **`docs/ENVIRONMENT.md`** : `LATCH_PUBLIC_BASE_URL` (+ `DEPLOY_TOKEN` déjà listé, préciser
  fail-secure), path `/mcp`, `allowed_hosts` dérivé.
- **`docs/QUIRKS.md`** : pièges rmcp 1.4 rencontrés (API, compat axum, allowed_hosts).
- **`docs/CONVENTIONS.md`** : skeleton adaptateur MCP (gate token → service → map error),
  hook `useSettings`.
- **`.env.example`** : `LATCH_PUBLIC_BASE_URL` ajouté (+ commentaire fail-secure).
```
