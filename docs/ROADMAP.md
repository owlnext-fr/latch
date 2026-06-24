# ROADMAP — latch

> Phases, dépendances, critères de sortie. Identifier la phase courante avant de
> coder (cf. `docs/HANDOFF.md` et `docs/INDEX.md`). Une phase n'est close que si ses
> critères de sortie sont **verts** — alors on le consigne dans INDEX + HANDOFF.

L'ordre suit les dépendances : le cœur d'abord (testable sans HTTP), puis les
adaptateurs un par un, puis l'e2e qui valide le tout assemblé, puis le packaging.

---

## Phase 0 — Scaffold & squelette CI/Docker

Mettre en place le terrain sans logique métier.
- Workspace 2 crates : `backend/` (Loco, template **avec DB**) + `frontend/` (Yew).
- **Retirer l'auth users/JWT** générée par Loco (on n'utilise pas la table `users`).
- Désactiver Redis/worker.
- `Cargo.toml` : versions épinglées (Loco, rmcp ≥ 1.4.0, yew 0.21, shadcn-rs 0.1),
  `libsqlite3-sys` `bundled`.
- Squelette CI (fmt/clippy/test vides mais qui tournent), Dockerfile multi-stage,
  `docker-compose.yml`, `deploy.sh`, dual-license, README minimal.

**Sortie :** `cargo loco start` démarre, `trunk build` produit un bundle, l'image se
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

## Phase 3 — SPA admin

> **⚠️ 2026-06-25** : livrée en **Yew** (Phase 3 + polish UX/i18n complets), puis **décision de
> migrer vers React/Vite/shadcn-ui** (friction `shadcn-rs` 0.1 + wasm). Crate Yew retirée ;
> migration React = chantier en cours sur `feat/admin-react`. Le **comportement (contrat §7)
> ne change pas** — seul le rendu. Voir `docs/superpowers/specs/2026-06-25-admin-react-migration-decision.md`.
> **Item futur** : Fumadocs (mini-landing + doc complète, GH Pages) — chantier séparé.

Implémenter les rails du contrat §7 (à l'origine avec `shadcn-rs`, désormais shadcn/ui React).
- Login, liste, détail (accès / config / versions / déploiement), side-panel
  création/édition, modales de confirmation destructive, copie URL + PIN, prévisualisation.
- Build Trunk servi en statique par Loco + fallback SPA.

**Sortie :** parcours admin manuel complet ; `wasm-bindgen-test` verts (dose mesurée).

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
