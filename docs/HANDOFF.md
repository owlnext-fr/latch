# Handoff — état courant

> Notes informelles pour la prochaine session (humaine ou Claude). Format libre,
> chronologique inverse (le plus récent en haut). À mettre à jour en fin de session
> significative — l'idée est de se resituer en 30 secondes.

## 2026-06-24 — Test live de la SPA (Playwright) : 3 bugs corrigés + punch-list UX

### Dernière chose faite
- Test manuel de la SPA avec l'humain via Playwright. **3 bugs corrigés ce jour**
  (invisibles aux reviews SDD/smoke curl car ils n'exercent pas le wasm rendu) :
  1. **Routing 404** — `BrowserRouter basename="/admin"` cassait tout (bug
     `strip_basename` de yew-router 0.18 sur l'URL racine → `//admin`). Fix : **pas
     de basename**, `#[at("/admin/...")]` absolus (`routes.rs`, `main.rs`).
  2. **CSS de layout absente** — seule la CSS des composants shadcn était vendorisée.
     Fix : `frontend/styles/app.css` (classes `.admin-page`/`.topbar`/`.kv`/… + liée
     dans `index.html`, copiée par Trunk).
  3. **Animation Sheet buggée** — `slide-in-*` laisse un transform résiduel qui pousse
     le drawer hors écran (contenu invisible). Fix : override `.sheet-content` dans
     `app.css` (animation/transform none, flex column, footer en bas).
- Parcours re-validé au navigateur : login centré, liste, **side-panel de création OK**,
  création d'un projet, page détail (cards Accès public / Configuration / Versions,
  actions Éditer/Déployer/Supprimer).
- **Punch-list des retours UX rangée dans** `docs/superpowers/specs/2026-06-24-phase-3-punchlist-ux.md`
  (source de vérité prochaine session). BACKLOG + QUIRKS + contrat §4 mis à jour
  (note `basename` erronée corrigée).

### Trucs en suspens (patchs prochaine session — voir la punch-list)
- Login : espace manquant entre mot de passe et bouton.
- Liste : badge code activé → vert, libre → orange.
- Form : **le toggle `Switch` ne bascule pas visuellement** (quirk shadcn) ; PIN à
  passer en `disabled` (pas masqué) quand code off ; **slug à passer en `disabled`**
  en édition (éditable aujourd'hui).
- Déploiement : **dropzone** (input file moche) + même bug de toggle.
- Général : **snackbars/toasts** succès/échec.
- Chantier plus large (après patchs) : explications champs + pages, **UI en anglais (EN)**,
  revue UX distribution, self-review produit.

### Prochaine chose à creuser
- Prochaine session : appliquer les patchs de la punch-list → **tout valider avec
  Playwright** → self-review produit (i18n EN, explications, distribution). Puis
  reprendre le choix merge/PR de la branche `feat/phase-3-spa-yew-admin`.

### Notes pour future Claude
- Dev : `cd frontend && trunk build` puis backend depuis `backend/` avec env
  (`LATCH_SPA_DIST=../frontend/dist`, `ADMIN_USER`/`ADMIN_PASS`/`SESSION_SECRET`/`DATABASE_URL`).
  SPA sur `http://127.0.0.1:5150/admin`. Itération CSS pure = `trunk build` + hard refresh
  (ServeDir lit `dist/` à chaque requête, pas besoin de relancer le backend).
- Deux pièges shadcn-rs à garder en tête : `Switch` (contrôle visuel) et animation
  `Sheet` — cf. QUIRKS.

---

## 2026-06-24 — Phase 3 TERMINÉE (SPA Yew admin)

### Dernière chose faite
- Phase 3 (SPA Yew admin) complète et clôturée.
- Livrables principaux : crate `latch-dto` (DTO partagés back+front) ; API JSON re-préfixée sous `/api/*` ; serving SPA sous `/admin` via `nest_service` (ServeDir + fallback `index.html`, `LATCH_SPA_DIST`) ; SPA Yew complète (yew-router 0.18, BrowserRouter basename="/admin", gloo-net 0.6) : AuthProvider/Protected, pages Login/List/Detail, side-panels ProjectForm/DeployPanel/DeleteProjectPanel/DeleteVersionPanel, composants CopyButton/PinField, CSS shadcn-rs vendorisée (5 fichiers patchés).
- Parcours admin vérifié end-to-end : login → créer projet → détail + PIN → déployer → preview no-store → activer → supprimer version active refusée (400) → supprimer version inactive → cross-origin 403 → supprimer projet → logout 401. PIN absent de la liste confirmé. wasm-bindgen-test : 3 verts (T5). Backend nextest : 82 verts.
- Contrat `docs/contrat-deploy.md` amendé (§4 : API `/api/*`, SPA `/admin`, `latch-dto` ; §7 : side-panels, page détail RO, slug RO, URL via `window.location.origin`).
- Dockerfile + `.env.example` + `docs/ENVIRONMENT.md` documentent `LATCH_SPA_DIST`.

### Trucs en suspens
- e2e Playwright (Phase 4/6) : non exécutés (Phase 4 introduit `/c/<slug>`). Parcours vérifiés manuellement en Phase 3.
- `deploy_version` renvoie `{id, n}` côté backend — la SPA ignores le corps de réponse (reload de la page après déploiement). Comportement acceptable en v1.
- Minors déférés au BACKLOG : base de slug éditable, override `PUBLIC_BASE_URL`, couche de toast globale, remontée d'erreur `activate_version`, polish login.rs (clear error au re-submit).

### Prochaine chose à creuser
- **Phase 4** : serving `/c/<slug>` — deux états (libre vs. code + cookie), page de déverrouillage stylée (`brand_name`), `POST /c/<slug>/unlock` (verify_code + cookie signé HMAC), rate-limit sur unlock, tests d'intégration.

### Notes pour future Claude
- `yew-router = 0.18` (PAS 0.21) pour `yew 0.21` — numérotation divergente (cf. QUIRKS).
- `gloo-net 0.6` : un HTTP 401/404 est `Ok(Response)` — inspecter `.status()` ; `.json(&body)?` avant `.send().await?` (cf. QUIRKS).
- `<Sheet>` shadcn-rs est une coquille — piloter `<SheetContent open on_close>` directement (cf. QUIRKS).
- CSS shadcn-rs patchée (`--color-card*`/`--color-popover*`) sous `frontend/styles/` (cf. QUIRKS).
- La SPA est buildée par `trunk build` → `frontend/dist/`. Servie par Loco sous `/admin` via `nest_service`. En dev, lancer le backend depuis `backend/` avec `LATCH_SPA_DIST=../frontend/dist` (ou valeur par défaut). En prod, `LATCH_SPA_DIST=/app/frontend/dist` posé par le Dockerfile.
- Side-panels montés en permanence : `use_effect_with(props.open, ...)` pour réinitialiser les champs (cf. QUIRKS + CONVENTIONS).

---

## 2026-06-24 — Phase 2 TERMINÉE (Task 9 : vérification, env, contrat, clôture mémoire)

### Dernière chose faite
- Phase 2 (adaptateur web admin) complète et clôturée. Suite : **77/77 verts, 0 ignorés**.
- Garde d'architecture (`backend/tests/architecture.rs`) verte — le cœur `src/services/`
  ne contient aucun `use axum::` ni `use loco_rs::`.
- `cargo fmt --all` propre, `cargo clippy --all-targets -- -D warnings` : 0 warning.
- Décisions Phase 2 reportées dans `docs/contrat-deploy.md` (§4 session/cookie/CSRF/rate-limit,
  §9 invariants structurels).
- `.env.example` complété : `SESSION_SECRET` + `LATCH_STORAGE_ROOT`.
- Branche : `feat/phase-2-admin-web`, prête pour review / merge sur `main`.

### Trucs en suspens
- `cargo deny check licenses advisories` non exécutable localement (binaire absent).
  Vérification déléguée à la CI GitHub Actions — toutes les licences des nouvelles deps
  Phase 2 (axum_session, axum_session_sqlx, tower_governor, tower, time) sont MIT/Apache-2.0,
  couvertes par `deny.toml allow = [...]`.
- BACKLOG : nettoyage du fichier HTML sur `delete_version` (storage.delete non encore déclaré).
- BACKLOG : `same_host` — port par défaut/IPv6 non géré (acceptable derrière Caddy, cf. BACKLOG).

### Prochaine chose à creuser
- **Phase 3** : SPA Yew admin (login screen, liste projets, détail, side-panel création/édition,
  upload HTML + deploy depuis l'interface).

### Notes pour future Claude
- Les 77 tests incluent : 13 tests unitaires (middleware Origin), tests d'intégration Loco
  (admin CRUD, auth, deploy, versions, security_invariants), tests service (ProjectsService,
  DeployService), garde d'archi — tout dans `cargo test -p latch`.
- Pattern `request_with_config(RequestConfigBuilder::new().save_cookies(true).build(), ...)`
  obligatoire pour tout test qui enchaîne login + accès protégé (cf. QUIRKS).
- `is_prod = !matches!(env, Development | Test)` dans `web/mod.rs` — fail-secure,
  ne pas inverser en `matches!(..., Production)` (cf. QUIRKS).
- `session.destroy()` au logout (révocation serveur immédiate), pas `session.clear()`.

---

## 2026-06-24 — Task 8 Phase 2 : déploiement + versions (activate/delete/preview)

### Dernière chose faite
- 4 handlers ajoutés à `controllers/admin.rs` : `deploy`, `activate_version`, `delete_version`, `preview_version`.
- `deploy` : appelle `DeployService::new(ctx.db, storage_from_ctx(&ctx)).deploy(...)`, répond `{id, n}`.
- `activate_version` : charge la version par (project_id, n) → 404 si absente ; met `active_version_id` + `updated_at` manuellement sur le projet.
- `delete_version` : charge version → 404 si absente ; refuse si c'est la version active (400) ; sinon `delete_by_id`.
- `preview_version` : charge version → 404 ; lit le HTML via `storage.read(&version.html_path)` ; répond avec tuple axum `([(CACHE_CONTROL, "no-store"), (CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()` — sans passer par `format::html` (qui ne permet pas d'injecter un header custom sans builder).
- Routes câblées : 3 mutations avec `.layer(from_fn(require_same_origin))`, preview GET derrière `AdminAuth` sans garde Origin.
- Import ajouté : `axum::response::IntoResponse`, `DeployReq`, `DeployService`.
- 3 nouveaux tests d'intégration : `deploy_creates_version_and_preview_serves_html`, `activate_switches_active_version`, `delete_version_refuses_active_and_removes_inactive`.
- Suite complète 76/76 verts, 0 ignorés. fmt + clippy clean. Commit `6c732c1`.

### Trucs en suspens
- Nettoyage du fichier HTML sur le storage lors d'un `delete_version` : non implémenté (cf. BACKLOG).
- Phase 2 adaptateur web admin : toutes les routes sont maintenant couvertes.

### Prochaine chose à creuser
- Phase 3 : SPA Yew admin (login, liste projets, détail, side-panel création/édition, déploiement depuis l'interface).

### Notes pour future Claude
- `preview_version` utilise le pattern axum brut `(headers_array, body).into_response()` enveloppé dans `Ok(...)`. `IntoResponse` doit être importé explicitement (`use axum::response::IntoResponse`). `loco_rs::prelude::*` importe `Response` (= `axum::response::Response`) mais pas le trait `IntoResponse`.
- Les tests deploy/preview/activate/delete nécessitent `LATCH_STORAGE_ROOT` pointé sur un `tempfile::tempdir()` — garder la variable `tmp` vivante jusqu'à la fin du test (drop explicite à la fin ou par scope), sinon le répertoire est supprimé avant la fin des requêtes HTTP.
- `save_cookies(true)` est obligatoire pour les tests avec session (login → accès protégé).
- `Origin: http://127.0.0.1` (sans port) dans tous les tests de mutation.

---

## 2026-06-24 — Task 7 Phase 2 : API admin écriture (CRUD + code) + garde Origin

### Dernière chose faite
- 5 handlers d'écriture ajoutés à `controllers/admin.rs` : `create`, `update`, `delete`, `set_code`, `clear_code`.
- Routes câblées avec garde `require_same_origin` sur chaque mutation via `.layer(from_fn(...))` par handler (axum 0.8 fusionne les MethodRouter sur même chemin).
- Cascade manuelle versions→projet en transaction dans `delete` (QUIRKS — FK SQLite non enforced).
- `updated_at` posé manuellement dans `update` (cf. QUIRKS hook before_save).
- 3 tests ignorés activés : `mutation_rejected_on_cross_origin`, `pin_never_appears_in_project_list`, `pin_appears_on_project_detail`.
- Tests de mutation ajoutés : `create_then_get_and_delete_project`, `set_and_clear_code_via_api`.
- **Piège découvert** : harness Loco utilise `Host: 127.0.0.1:PORT`, pas `localhost` — Origin de test doit être `http://127.0.0.1` (cf. QUIRKS).
- Fallback URI dans `require_same_origin` pour le mode mock (où `Host` header peut être absent).
- Suite complète 72/72 verts, 0 ignorés. fmt + clippy clean.

### Trucs en suspens
- Aucun test ignoré restant (les 3 ont été activés et passent).

### Prochaine chose à creuser
- Phase 2 est complète côté adaptateur web admin (Tasks 2-7 terminées).
- Phase 3 : SPA Yew admin (login, liste, détail, side-panel création/édition, etc.).

### Notes pour future Claude
- `Origin: http://127.0.0.1` (sans port) matche `Host: 127.0.0.1:PORT` grâce à la règle "si l'un n'a pas de port, on accepte" dans `same_host`. Ne pas mettre `http://localhost` dans les tests de mutation.
- Plusieurs `.add(path, method_router)` sur le même chemin avec des verbes distincts fusionnent via axum `Router::route` (merge des MethodRouter). Le `.layer()` sur un MethodRouter s'applique uniquement aux verbes définis dans ce MethodRouter (pas aux autres).
- `axum::routing::delete(handler)` doit être utilisé (namespaced) si `delete` est aussi le nom du handler, pour éviter l'ambiguïté.

---

## 2026-06-24 — Task 6 Phase 2 : API admin lecture (liste + détail projets)

### Dernière chose faite
- `controllers/admin.rs` créé : `GET /admin/projects` (liste sans PIN) + `GET /admin/projects/{id}` (détail avec PIN + versions), protégés par `AdminAuth`.
- `controllers/mod.rs` mis à jour : déclare `pub mod admin`.
- `app.rs` mis à jour : monte `controllers::admin::routes()`.
- Les 2 tests ignorés de Task 4 (`protected_route_is_401_without_session`, `login_then_access_protected_route`) **re-activés et verts**.
- Nouveaux tests actifs : `list_projects_returns_empty_array_when_none`, `detail_returns_404_for_unknown_id`.
- `backend/tests/security_invariants.rs` créé avec `pin_never_appears_in_project_list` et `pin_appears_on_project_detail` (ignorés — attendent Task 7).
- **Bug corrigé dans `web/mod.rs`** : `is_prod` était `true` en environment `Test` (car `!Development`), activant `cookie_secure = true` et empêchant la propagation des cookies de session dans les tests. Corrigé : `is_prod` vrai uniquement en `Production`.
- Suite complète 67/67 verts, 3 ignorés. fmt + clippy clean.

### Trucs en suspens
- Les 3 tests ignorés :
  - `mutation_rejected_on_cross_origin` (admin_api.rs) — attend Task 7.
  - `pin_never_appears_in_project_list` (security_invariants.rs) — attend Task 7.
  - `pin_appears_on_project_detail` (security_invariants.rs) — attend Task 7.

### Prochaine chose à creuser
- Task 7 : `POST /admin/projects` (création) + mutations CRUD + `require_same_origin` câblé sur mutations. Activera les 3 tests ignorés.

### Notes pour future Claude
- `request_with_config(RequestConfigBuilder::new().save_cookies(true).build(), ...)` est requis pour tout test intégration qui fait login puis accès protégé — `request(...)` ne propage pas les cookies.
- `is_prod` dans `web/mod.rs` doit être `matches!(..., Production)`, pas `!matches!(..., Development)` — l'environnement de test est `Test`, pas `Development`.
- `save_cookies` de `axum-test` stocke les `Set-Cookie` response headers dans un `CookieJar` interne, et les réémet sur les requêtes suivantes. Fonctionne en mode Mock ET HTTP.
- Context7 a confirmé : Loco 0.16/axum 0.8 utilise `{id}` (pas `:id`) pour les path params.

---

## 2026-06-24 — Task 5 Phase 2 : middleware same-origin (CSRF guard)

### Dernière chose faite
- `controllers/middleware/mod.rs` créé : déclare `pub mod origin`.
- `controllers/middleware/origin.rs` créé : helpers `url_host` / `same_host` / `split_host_port` + middleware `require_same_origin` (axum `from_fn`).
- 403 produit via `Ok((StatusCode::FORBIDDEN, ...).into_response())` — pas via `loco_rs::Error::Unauthorized` (→401). Confirmé via lecture directe de `loco-rs-0.16.4/src/errors.rs` + `controller/mod.rs`.
- `controllers/mod.rs` mis à jour : déclare `pub mod middleware`.
- 13 tests unitaires des helpers (RED→GREEN, y compris bug corrigé sur ports différents).
- Test `mutation_rejected_on_cross_origin` ajouté dans `admin_api.rs`, `#[ignore = "needs POST /admin/projects (Task 7)"]`.
- Suite complète 56/56 passés, 3 ignorés. fmt + clippy clean. Commit `ee60df3`.

### Trucs en suspens
- Le middleware n'est PAS encore câblé sur des routes mutantes (Tasks 7/8).
- Test `mutation_rejected_on_cross_origin` reste `#[ignore]` jusqu'à ce que `POST /admin/projects` existe (Task 7).

### Prochaine chose à creuser
- Task 6 (si l'ordre du plan l'exige) ou directement Task 7 : `controllers/admin.rs` — handlers CRUD JSON protégés par `AdminAuth` + `require_same_origin` câblé sur mutations.

### Notes pour future Claude
- `loco_rs::Error::Unauthorized` → **401** (pas 403). Pour un 403 dans un middleware axum, utiliser `Ok((StatusCode::FORBIDDEN, "msg").into_response())` — idiomatique, sans dépendance sur `ErrorDetail` Loco.
- `same_host` utilise `rsplit_once(':')` pour séparer host/port — gère les cas `"example.com"` (pas de port) et `"example.com:8080"` (port explicite). Si les deux ont un port, ils doivent être égaux. Si l'un n'en a pas, on accepte.
- Bug potentiel IPv6 (`[::1]:port`) : `rsplit_once(':')` ne fonctionnerait pas correctement. Non adressé en v1 (pas de cas IPv6 dans le périmètre, noté dans les commentaires du code).

---

## 2026-06-24 — Task 4 Phase 2 : auth admin (login/logout, AdminAuth, rate-limit)

### Dernière chose faite
- `controllers/auth.rs` créé : `login`/`logout` handlers + extracteur `AdminAuth` (FromRequestParts sans async_trait, retourne 401 si session sans flag admin).
- Rate-limit `tower_governor 0.7` sur `/admin/login` uniquement via `.add("/login", post(login).layer(GovernorLayer { config }))` — type de retour inline pour éviter l'annotation verbeuse de `NoOpMiddleware`.
- `controllers/mod.rs` mis à jour : déclare `pub mod auth`.
- `app.rs` mis à jour : `.add_route(controllers::auth::routes())`.
- 3 tests actifs verts (boots, login_rejects_bad_credentials, login_is_rate_limited), 2 ignorés avec raison explicite (attendent Task 6 `/admin/projects`). Suite complète 43/43 passés, 2 ignorés. fmt + clippy clean. Commit en cours.

### Trucs en suspens
- Task 6 (controllers/admin.rs : CRUD projets JSON) est la prochaine étape.
- Les 2 tests ignorés (`protected_route_is_401_without_session`, `login_then_access_protected_route`) seront activés après Task 6.

### Prochaine chose à creuser
- Task 5 ou Task 6 selon l'ordre du plan : `controllers/admin.rs` — handlers GET/POST/PATCH/DELETE projets + deploy, protected par `AdminAuth`.

### Notes pour future Claude
- `GovernorLayer` se construit avec `GovernorLayer { config: Arc::new(...) }` (pas de `::new()`), le champ `config` est `pub`.
- `GovernorConfigBuilder::finish()` retourne `Option<GovernorConfig<K, M>>`, pas `Result` — utiliser `.expect(...)`.
- `Session<T>::from_request_parts` a un `Rejection = (StatusCode, &'static str)` → mapper avec `.map_err(|_| loco_rs::Error::Unauthorized(...))`.
- Annotation de type `GovernorLayer<SmartIpKeyExtractor, governor::middleware::NoOpMiddleware>` échoue car `governor` (sous-dep) n'est pas dans la crate root — construire inline dans `routes()` ou éviter l'annotation.
- `secure_compare` compare TOUJOURS les deux champs (user et pass) avant de décider, pour ne pas révéler quel champ a échoué (contrat §9).

---

## 2026-06-24 — Task 3 Phase 2 : mapping CoreError→HTTP + DTOs admin

### Dernière chose faite
- `controllers/error.rs` créé : `into_response(CoreError) → loco_rs::Error` (NotFound→404, Validation→400, Db/Io→500).
- `controllers/dto.rs` créé : `ProjectListItem` (sans PIN), `ProjectDetail` (avec PIN via `from_model`), `VersionItem`, `CreateProjectReq`, `UpdateProjectReq`, `SetCodeReq`, `DeployReq`.
- `controllers/mod.rs` mis à jour : déclare `dto` + `error` + `home` (pas encore `admin`/`auth`/`middleware`).
- 4 nouveaux tests verts (2 PIN-scoping, 2 error-mapping) ; suite totale 39/39. fmt + clippy clean. Commit `c61a817`.

### Trucs en suspens
- Task 4 (controllers/admin.rs : CRUD projets JSON) est la prochaine étape.
- `admin`/`auth`/`middleware` modules déclarés dans `mod.rs` quand créés par Tasks 4/5/6.

### Prochaine chose à creuser
- Task 4 : `controllers/admin.rs` — handlers GET/POST/PATCH/DELETE projets + deploy, utilise `ProjectListItem`/`ProjectDetail`/`DeployReq` etc., guard origin.

### Notes pour future Claude
- `loco_rs::Error` variantes confirmées via source 0.16.4 : `NotFound` (404), `BadRequest(String)` (400), `Message(String)` (500), `InternalServerError` (500 sans message).
- `ProjectListItem` n'a structurellement PAS de champ `pin` — invariant §9.2 renforcé par la structure de type, pas juste par un `#[serde(skip)]`.
- Déclarer dans `mod.rs` seulement les modules dont les fichiers existent (évite échec de compilation entre tâches).

---

## 2026-06-24 — Task 2 Phase 2 : câblage axum-session (after_routes + helpers web)

### Dernière chose faite
- `axum_session 0.16.0` + `axum_session_sqlx 0.5.0` + `tower_governor 0.7.0` + `tower 0.5` + `time 0.3` ajoutés — sqlx 0.8.6 partagé sans conflit.
- `backend/src/web/mod.rs` créé : `SessionPool` / `AdminSession` type aliases, `storage_from_ctx` (LATCH_STORAGE_ROOT → FsStorage), `build_session_store` (pool SQLite Loco → SessionLayer).
- `after_routes` câblé dans `backend/src/app.rs` : monte `SessionLayer` au démarrage.
- Smoke test `backend/tests/admin_api.rs` : vérifie que l'app boote avec la session layer + répond `/_ping` 200.
- Suite 35/35 verte, fmt + clippy clean. Commit `d1e9507`.

### Trucs en suspens
- Task 3 (controllers/auth.rs : login/logout session) est la prochaine étape de Phase 2.
- `cargo-deny` non installé localement — tourne en CI uniquement. Licences des nouvelles dépendances toutes MIT/Apache.

### Prochaine chose à creuser
- Task 3 : `controllers/auth.rs` — POST `/admin/login` (compare ADMIN_USER/ADMIN_PASS à temps constant, pose session, rate-limit), GET `/admin/logout` (détruit la session). Utilise `AdminSession` from `web::AdminSession`.

### Notes pour future Claude
- `with_session_name` (pas `with_cookie_name`) dans `SessionConfig` 0.16 — cf. QUIRKS.
- `SessionSqlitePool::from(pool)` (pas `::new`) — cf. QUIRKS.
- `SESSION_SECRET` doit faire ≥ 64 bytes en prod — cf. QUIRKS.
- `LATCH_STORAGE_ROOT` (défaut `data`) : racine du volume HTML — non encore utilisé en Phase 2, câblé ici pour Tasks suivantes.

---

## 2026-06-24 — Phase 1 mergée sur `main` + scrub d'historique (nom client)

### Dernière chose faite
- **Phase 1 mergée sur `main`** (fast-forward, `main` = `a06d90a`) et **force-pushée sur GitHub** ;
  branche `feat/phase-1-coeur` supprimée. 33 tests verts, fmt + clippy clean au moment du merge.
- **Incident confidentialité traité** : un **nom de client réel** traînait comme exemple de slug
  dans `docs/contrat-deploy.md` (hérité du bootstrap) et s'était propagé (tests slug, QUIRKS, plan).
  Purgé du working-tree (placeholder générique `Mon Projet` / `mon-projet`) **et de tout
  l'historique** via `git filter-repo --replace-text`, puis **force-push de `main`**.
  Règle non-négociable ajoutée dans `CLAUDE.md` (« jamais de nom de client dans le repo »).
- Phase 1 a été déroulée en **Subagent-Driven** (1 implémenteur + 1 reviewer par tâche, 3 cycles
  de fix, revue finale opus = « ready to merge »). Ledger : `.superpowers/sdd/progress.md` (gitignoré).

### Trucs en suspens / à savoir
- **L'historique de `main` a été RÉÉCRIT** (filter-repo) : tous les SHA d'avant `a06d90a` ont changé.
  Un clone/worktree antérieur à ce push **diverge** — re-cloner ou `git fetch && git reset --hard origin/main`.
  Backup de l'ancien historique : `scratchpad/latch-backup-before-scrub.bundle` (hors repo, session-local).
- **CI** : un run va tourner sur la `main` réécrite — confirmer le vert au prochain passage.
- Les anciens SHA peuvent rester accessibles côté GitHub (caches/PR/forks) un temps — support GitHub si purge totale requise.

### Prochaine chose à creuser
- **Phase 2** : adaptateur web admin (handlers Loco/axum, JSON, cookie-session via `axum-session`,
  table `sessions` créée ici, mapping `CoreError` → HTTP status, guard `Origin` sur mutations).

### Notes pour future Claude
- `cargo loco db entities` exige **`sea-orm-cli`** installé sur la machine (cf. QUIRKS + ENVIRONMENT).
- Le cœur `services/` est protégé par la garde `tests/architecture.rs` (récursive, détecte aussi `pub use`).
- Avant de coder une API Loco/sea-orm/rmcp/yew : **Context7** (versions épinglées).

---

## 2026-06-24 — Phase 1 TERMINÉE (Task 9 : garde d'archi + clôture mémoire)

### Dernière chose faite
- Garde d'architecture `backend/tests/architecture.rs` : scan de `src/services/`, fail si `use axum` ou `use loco_rs` détecté (contrat §1). Test PASS — le cœur est propre.
- Phase 1 entièrement livrée sur la branche `feat/phase-1-coeur` : services `slug`/`security`/`pin`/`storage`/`projects`/`deploy`, migrations + entités SeaORM, `test_support` in-memory, garde d'archi.
- Full suite 33/33 verte ; fmt + clippy clean. Clôture mémoire (INDEX, HANDOFF, CONVENTIONS, QUIRKS, BACKLOG) complète.

### Trucs en suspens
- Branch `feat/phase-1-coeur` prête pour review/merge avant d'attaquer Phase 2.

### Prochaine chose à creuser
- Phase 2 : adaptateur web admin (handlers Loco/axum, JSON, cookie-session, mapping `CoreError` → HTTP status, guard `Origin` sur mutations).

### Notes pour future Claude
- La garde d'archi est un test d'intégration (`--test architecture`), pas un `#[cfg(test)]` inline ; elle tourne dans `cargo test -p latch` automatiquement.
- L'ordre `storage.write` → `db.begin()` dans `deploy.rs` est intentionnel et non-négociable (contrat §8).
- `active_version_id` = FK logique (pas de contrainte DB) à cause de la référence circulaire `projects⇄versions` — voir QUIRKS.

---

## 2026-06-24 — Task 8 : DeployService

### Dernière chose faite
- `DeployService` implémenté dans `backend/src/services/deploy.rs`.
- Ordre imposé : `storage.write(...)` AVANT `db.begin()` → un fichier orphelin est inoffensif, un pointeur actif vers un fichier absent ne l'est pas.
- Transaction : insert `versions` row + flip `projects.active_version_id` si `activate=true`.
- 3 tests GREEN, full suite 32/32, fmt + clippy clean.
- Commit : `b329682` — `✨ feat: DeployService (ordre fichier→tx, flip pointeur transactionnel)`.

### Trucs en suspens
- Task 9 : garde d'archi (`no_axum_in_services`) + clôture mémoire Phase 1.

### Prochaine chose à creuser
- Task 9 : ajouter un test `#[test]` qui vérifie qu'aucun fichier sous `backend/src/services/` ne contient `use axum::` ou `use loco_rs::`.

### Notes pour future Claude
- Le n `max(n)+1` est calculé hors transaction. `UNIQUE(project_id,n)` est le backstop pour la concurrence.
- `project.updated_at` est mis à jour manuellement dans `deploy.rs` car le wrapper `before_save` du modèle Loco ne s'applique qu'en dehors des transactions directes sur `ActiveModel`.

---

## 2026-06-24 — Task 6 : Migrations + entités + test_support

### Dernière chose faite
- Migrations `projects` et `versions` écrites et appliquées via `cargo loco db migrate` (depuis `backend/`).
- Entités SeaORM générées via `cargo loco db entities` : `_entities/projects.rs` + `_entities/versions.rs` + wrappers Loco `models/projects.rs` + `models/versions.rs`.
- `test_support::test_db()` : SQLite in-memory migrée, `max_connections(1)`.
- Test `unique_project_n_is_enforced` : GREEN — UNIQUE(project_id,n) rejette le doublon.
- `sea-orm-cli` installé sur la machine (manquait, nécessaire pour `cargo loco db entities`).

### Trucs en suspens
- Tasks 7 (ProjectsService) et 8 (DeployService) à implémenter.

### Prochaine chose à creuser
- Task 7 : `ProjectsService` (create, list, get, update, delete) consommant `_entities::projects`.

### Notes pour future Claude
- Type date généré : `DateTimeWithTimeZone` — utiliser `chrono::Utc::now().into()` dans les `Set(...)`.
- Le wrapper `models/projects.rs` a un hook `before_save` qui touche `updated_at`, mais il ne s'applique que si le champ est `unchanged` ; les services (`set_code`/`clear_code`/`deploy`) posent `updated_at = Set(chrono::Utc::now().into())` explicitement (ceinture + bretelles, valeur cohérente). Donc : on continue de le set manuellement dans les services.
- `UNIQUE(project_id,n)` sur `versions` est géré par l'index `idx_versions_project_n` (SQLite l'honore correctement en-memory, testé).
- `sea-orm-cli` doit être présent sur la machine pour `cargo loco db entities`. Cf. QUIRKS.

---

## 2026-06-24 — Phase 0 livrée (scaffold & squelette CI/Docker)

### Dernière chose faite
- **Phase 0 du ROADMAP terminée, tous critères de sortie verts** (vérifiés réellement,
  pas sur parole) :
  - Workspace 2 membres : `backend/` (Loco 0.16.4, crate `latch`, bin `latch-cli`) +
    `frontend/` (crate `latch-ui`, Yew 0.21) + sous-crate `backend/migration`.
  - Scaffold généré via `loco new --db sqlite --bg none --assets none` → starter minimal
    **sans users/JWT** (rien à retirer), **sans worker/Redis**.
  - `libsqlite3-sys` en `bundled` (unifié avec sqlx 0.8 → `libsqlite3-sys 0.30.1`).
  - `cargo loco start` boote (depuis `backend/`), `trunk build` produit le bundle wasm.
  - fmt + clippy `-D warnings` verts (backend ET frontend wasm) ; `cargo test` vert.
  - Image Docker multi-stage construite (~85 Mo) + **smoke test conteneur** : `/_health`
    = `{"ok":true}`, auto-migrate au boot, `latch.sqlite` créé dans le volume.
  - Écrits : Dockerfile, `docker-compose.yml`, `deploy.sh`, `.env.example`, deny.toml,
    CI `.github/workflows/ci.yml`, dual-license MIT/Apache, README + badge.

### Versions épinglées (résolues via Context7 + crates.io)
- loco `0.16` (lock 0.16.4) · rmcp **pin 1.8.0** (≥1.4 CVE, pas encore dep → Phase 5) ·
  yew **0.21** (imposé par `shadcn-rs 0.1.0` qui requiert `yew ^0.21`) · shadcn-rs 0.1.0
  (compile en wasm, OK) · sea-orm 1.1 (aligné Loco).

### Trucs en suspens / à savoir
- **Lancer le serveur depuis `backend/`** (Loco lit `./config` au CWD) — cf. QUIRKS.
- `default-members = [backend, backend/migration]` : le frontend wasm est exclu des
  commandes natives (sinon `cargo build` tente de le compiler pour l'hôte) — cf. QUIRKS.
- **CI verte sur `main`** : pipeline **prouvé intégralement vert** sur le commit `c1b2126`
  (fmt/clippy, tests, build SPA, **cargo-deny** corrigé + désormais **bloquant**, docker
  build/push GHCR — tous SUCCESS). Le run du commit de versioning `f9c0361` n'a **pas été
  attendu** (abandonné sur demande) ; changement à faible risque (config `metadata-action`,
  YAML validé localement). À jeter un œil au prochain passage si besoin.
- **Images versionnées** (`docker/metadata-action`) : pour publier une release, **pousser
  un tag git `vX.Y.Z`** → produit `X.Y.Z`/`X.Y`/`latest`/`sha-`. Un push `main` ne produit
  que `main`+`sha-`. Déploiement pin via `LATCH_IMAGE_TAG` (`.env`).
- `Cargo.lock` est commité (pin réel). `.vscode/` toujours hors commit.

### Prochaine chose à creuser
- **Phase 1** : cœur `services/` (projects, deploy tx, slug, Storage, CoreError) +
  migrations `projects`/`versions`/`sessions` + tests unit. Agnostique HTTP.

### Notes pour future Claude
- Avant de coder une API Loco/sea-orm/rmcp/yew : **Context7** (versions épinglées).
- Le smoke test conteneur est reproductible : `docker run -p 5151:5150 -v <data>:/data ghcr.io/owlnext-fr/latch:dev`.

## 2026-06-24 — Bootstrap mémoire projet livré

### Dernière chose faite
- Rangé les docs normatifs sous `docs/` (ils traînaient à la racine, alors que
  `CLAUDE.md` les référençait déjà sous `docs/` — les liens sont maintenant corrects).
- Mis en place le système de mémoire persistante : bloc « Mémoire projet » dans
  `CLAUDE.md` (decision tree + règle de fin d'implémentation non-négociable), hook
  `SessionStart` (`.claude/hooks/load-memory.sh`) qui injecte le head de `HANDOFF.md`
  + `INDEX.md` au démarrage, `.gitignore` pour `.claude/settings.local.json`.
- Créé `docs/superpowers/{specs,plans}/` (specs & plans détaillés par feature
  non-triviale, fichiers `YYYY-MM-DD-<slug>.md`).

### Règle actée cette session
- **Convention de commit = gitmoji + conventionnel** (`<gitmoji> <type>: <desc>`,
  ex. `✨ feat:`, `🐛 fix:`). Consignée dans `docs/BOOTSTRAP.md §4`. Obligatoire.

### Trucs en suspens
- Bootstrap commité sur la branche **`chore/bootstrap-memoire`** (on était sur `main`).
- `.claude/settings.json` + `.claude/hooks/` + `.rtk/filters.toml` sont **commités**
  (setup partagé équipe). `.vscode/` laissé hors commit (spécifique éditeur).
- Contenu existant **préservé** (non écrasé par les templates vides du prompt) :
  `INDEX.md`, `ENVIRONMENT.md`, `CONVENTIONS.md`, `QUIRKS.md`, `BACKLOG.md` gardent
  leur contenu projet riche issu du cadrage.

### Prochaine chose à creuser
- Dérouler la **Phase 0** du ROADMAP (scaffold & squelette CI/Docker).

### Notes pour future Claude
- En début de session, le hook t'aura déjà injecté le head de `HANDOFF.md`. Lis-le,
  puis `docs/INDEX.md`, puis les normatifs (`contrat-deploy` → `BOOTSTRAP` → `ROADMAP`).
- Le hook ne montre que 80 lignes de `HANDOFF.md` (append-only, il grossit) ; si tu
  veux plus de contexte, lis le fichier entier.

## 2026-06-24 — Kit dérivé, avant tout code

Le cadrage archi est **clos**. Le kit (`CLAUDE.md`, `docs/contrat-deploy.md`,
`docs/BOOTSTRAP.md`, `docs/ROADMAP.md`) est la source de vérité. Rien n'est encore
codé : on entre en **Phase 0** (scaffold).

Décisions structurantes verrouillées : Loco/axum + SeaORM/SQLite (`bundled`) ;
frontend **Yew + shadcn-rs** servi en statique (choix assumé « PoC technique, fun >
simplicité », pas le plus simple — le plus simple aurait été du server-rendered) ;
admin **cookie-session** (pas le JWT Loco) ; `/c/<slug>` à **deux états** avec page de
déverrouillage stylée + PIN 6 chiffres + rate-limit *load-bearing* ; MCP **Modèle 1**
(`deploy_token` en argument) ; GHCR **public**, déploiement **manuel** via `deploy.sh`.

Prochaine action : dérouler la Phase 0 du ROADMAP. Avant de coder une API d'une crate
listée dans le tableau Context7 du `CLAUDE.md`, **résoudre la doc via Context7**.

À trancher quand ça deviendra concret (non bloquant) : longueur exacte du suffixe de
slug (cf. QUIRKS). Acté : nom du projet **`latch`** (repo `owlnext-fr/latch`), domaine
de serving **`latch.owlnext.fr`**.
