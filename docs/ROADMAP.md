# ROADMAP — latch

> Phases, dépendances, critères de sortie. Identifier la phase courante avant de
> coder (cf. `docs/HANDOFF.md` et `docs/INDEX.md`). Une phase n'est close que si ses
> critères de sortie sont **verts** — alors on le consigne dans INDEX + HANDOFF.

L'ordre suit les dépendances : le cœur d'abord (testable sans HTTP), puis les
adaptateurs un par un, puis l'e2e qui valide le tout assemblé, puis le packaging.

---

## Phase 0 — Scaffold & squelette CI/Docker

Mettre en place le terrain sans logique métier.
- Workspace : `backend/` (Loco, template **avec DB**) + `frontend/` (app React, Vite + pnpm).
- **Retirer l'auth users/JWT** générée par Loco (on n'utilise pas la table `users`).
- Désactiver Redis/worker.
- `Cargo.toml` : versions épinglées (Loco, rmcp ≥ 1.4.0), `libsqlite3-sys` `bundled`.
- Squelette CI (fmt/clippy/test vides mais qui tournent), Dockerfile multi-stage (Node + Rust + runtime),
  `docker-compose.yml`, `deploy.sh`, dual-license, README minimal.

**Sortie :** `cargo loco start` démarre, `pnpm build` produit un bundle React, l'image se
construit, la CI passe au vert sur un projet vide.

## Phase 1 — Cœur (services) + modèle + migrations

Le métier, agnostique HTTP.
- Migrations SeaORM : `projects`, `versions`. _(La table `sessions` est reportée
  en Phase 2 : elle ne sert qu'à l'auth admin via `axum-session` ; la créer à vide
  ici donnerait une table morte et un risque de conflit de schéma. Décision actée
  2026-06-24.)_
- `services/` : `projects` (create/list/get_by_slug/set_code/clear_code/verify_code),
  `deploy` (tx insert version + flip pointeur, ordre fichier→DB du contrat §8),
  `slug` (base + suffixe), trait `Storage` + `FsStorage`, `CoreError`.
- `verify_code` à temps constant ; PIN auto-généré 6 chiffres.

**Sortie :** tests **unit** verts sur slug, code, bascule, `deploy_token`. Un test de
`deploy()` avec un `Storage` sur tempdir (jamais le disque de prod). Aucun `use axum`
ni `use loco_rs` dans `src/services/`.

## Phase 2 — Adaptateur web admin (API JSON + session)

- Migration `sessions` (store de session admin), créée ici — soit auto-gérée par
  `axum-session`, soit migration SeaORM dédiée, à trancher au câblage. _(Reportée
  de la Phase 1, cf. décision 2026-06-24.)_
- `controllers/auth.rs` : login/logout, cookie de session (`axum-session` dans
  `after_routes`), compte unique env, comparaison à temps constant, rate-limit login.
- `controllers/admin.rs` : API JSON — projets CRUD, deploy manuel, switch de version,
  config code, suppression. Vérif `Origin` sur les mutations.

**Sortie :** tests **intégration** (Loco + SQLite de test) verts sur chaque endpoint,
401 sans session, deploy transactionnel, switch, **test-invariant de sécu** (pas de
hash en réponse, pas de PIN en liste).

## Phase 3 — SPA admin ✅ LIVRÉE (2026-06-25)

> Livrée en deux temps : (a) Yew + polish UX/i18n complets ; (b) **migration React/Vite/shadcn-ui**
> décidée (friction `shadcn-rs` 0.1 + wasm), exécutée en Plans 1-3 sur `feat/admin-react`.
> Crate Yew (`latch-ui`) retirée du workspace (reste dans l'historique git).
> Le **comportement (contrat §7) n'a pas changé** — seul le rendu.
> Détail du choix : `docs/superpowers/specs/2026-06-25-admin-react-migration-decision.md`.

Livrables React (Plans 1-3, tous verts) :
- **Plan 1** — Backend : DTO inlinés `backend/src/dto/`, annotations utoipa, `openapi.json` commité
  + test drift, Swagger UI dev.
- **Plan 2** — Frontend React : scaffold Vite/pnpm/shadcn, client typé openapi-fetch, shell TanStack
  Router/Query, harness Vitest+MSW, Login, liste projets, ProjectForm, détail/deploy/danger panels,
  hooks `use-projects`.
- **Plan 3** — Infra : stage Docker Node 24, CI pistes back/front→e2e→docker, Playwright e2e.

**Critères de sortie :** parcours admin manuel complet ; Vitest verts ; Playwright e2e verts.

## Phase 4 — Serving `/c/<slug>` (deux états)

- `controllers/serve.rs` : GET deux états (libre / cookie valide / page de
  déverrouillage), `POST /unlock` (vérif + cookie signé HMAC), `no-store` partout,
  page de déverrouillage stylée portant `brand_name`.
- **Rate-limit *load-bearing*** sur `/unlock` (backoff IP+slug + plafond global slug).

**Sortie :** tests verts — projet libre servi, projet protégé → page de
déverrouillage, unlock pose le cookie et sert l'active, rate-limit effectif.

## Phase 5 — Endpoint MCP

- `mcp/` : `deploy_prototype` + `list_projects`, montés via `after_routes`
  (`nest_service("/mcp", …)`), `rmcp ≥ 1.4.0`, `allowed_hosts` incluant
  `latch.owlnext.fr`. **Token validé sur tous les tools.**
- `deploy_prototype` appelle le même `services::deploy()` que l'admin.

**Sortie :** tests verts — gate token sur tous les tools (lecture comprise),
`deploy_prototype` crée une version. **À confirmer au premier branchement réel :** que
Claude web se connecte à un serveur MCP sans auth HTTP (déduit de la doc, non testé).

## Phase 6 — E2E, durcissement, packaging publiable

- **E2E Playwright** : flux complet de bout en bout (contrat §7 + §6).
- Durcissement : `cargo deny`/`audit` au vert, en-têtes (`X-Robots-Tag`), `robots.txt`,
  vérif `Origin`, scoping/expiration des cookies.
- Publiable : README complet (quickstart, capture, archi), badge CI, CHANGELOG,
  dual-license, `.env.example`.

**Sortie :** e2e vert en CI, image publiée sur GHCR public, `deploy.sh` testé sur la
box. Repo présentable comme référence FOSS.
