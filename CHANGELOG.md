# Changelog

Toutes les évolutions notables de latch. Format inspiré de Keep a Changelog ; versionnage SemVer.
## [v0.3.1] — 2026-06-26

### Corrections

- Restaure la session admin en prod (cookie __Host-, bug axum_session 0.16)

### Documentation

- Note refonte .env.example (Phase 9)
## [v0.3.0] — 2026-06-26

### Ajouts

- Identité produit (logo currentColor, stone/oklch, nav)
- Landing produit (hero, features, CTA) + page 404
- Intro docs + ordre sidebar + nettoyage sample
- Landing — parcours 3 étapes + conversation Claude simulée

### Corrections

- Gate new-code — docs job --ignore-scripts + ternaires settings-sheet

### Documentation

- Spec site doc publique (Fumadocs/GitHub Pages)
- Déploiement doc dans la CI principale (ci.yml)
- Archive le brief d'origine du site doc public
- Doc servie à la racine (domaine custom docs.latch.owlnext.fr)
- Retour sous-chemin GH Pages (pas de domaine custom)
- Plan d'implémentation site doc publique (12 tasks)
- Section how-it-works (architecture, security, contributing)
- Section deploy (docker, compose, reverse-proxy, config, releases…)
- Section admin (projects, access-codes, versions, co-branding)
- Publish-from-claude (connect, tools, why-token) + quickstart
- Page troubleshooting (modes d'échec concrets)
- Schéma flux Claude (composant themeable) + captures
- Finitions — liens produit + mémoire projet
- Clôture — Phase 8 LIVRÉE (v0.3.0) + Phase 9 (passe polish)
- Régénère pour v0.3.0 (Phase 8 — site doc public)

### Interne

- Scaffold Fumadocs + export statique basePath /latch
- Build + déploiement Pages dans ci.yml (jobs docs + deploy-docs)

### Tests

- Aligne l'assertion DOCS_URL sur l'URL GH Pages
## [v0.2.0] — 2026-06-25

### Ajouts

- ParseLocales — découverte pure des locales + _meta
- ThemeProvider next-themes (admin, défaut system) + anti-FOUC
- LanguageSelect (Select radix + flag-icons, locales-driven)
- ThemeToggle segmenté (système/clair/sombre)
- SettingsSheet (MCP + préférences) + useSettings(enabled)
- Topbar ouvre le Sheet, route /settings supprimée
- Logo + useDocumentTitle + lib/links + favicon SVG, purge scaffold
- Titres de page dynamiques + largeur bornée max-w-6xl
- Logo + lien GitHub + titre de page
- Vrai logo GitHub (SVG inline) + Button asChild (Slot OK)
- Logo badge+texte + bouton ? vers la doc
- Logo + titre de page dynamique (brand)
- Page d'erreur stylée /c (3e entrée Vite error.html)
- Branches d'erreur /c en pages HTML stylées + log 500

### Divers

- SelectValue placeholder + ordre DOM ItemText/indicateur

### Documentation

- Cadrage Phase 7 en 4 lots + spec Lot 1 (fondations i18n/thème)
- Plan d'implémentation Lot 1 (fondations i18n/thème)
- Lot 1 livré — mémoire (INDEX/HANDOFF/CONVENTIONS/QUIRKS)
- Spec Lot 2 — panneau Settings unifié (side-panel)
- Plan d'implémentation Lot 2 (panneau Settings side-panel)
- Lot 2 livré — mémoire (INDEX/HANDOFF/CONVENTIONS/QUIRKS)
- Spec Lot 3 — identité visuelle & confort admin
- Plan d'implémentation Lot 3 (identité visuelle)
- Lot 3 livré — mémoire (INDEX/HANDOFF/CONVENTIONS/QUIRKS)
- Spec Lot 4 — page d'erreur stylée serving /c
- Plan d'implémentation Lot 4 (page d'erreur serving /c)
- Lot 4 livré — Phase 7 LIVRÉE (mémoire + ROADMAP)
- Clôture Phase 7 (v0.2.0) — HANDOFF + QUIRKS + CHANGELOG

### Interne

- Admin locales auto-découvertes (glob + _meta)
- Unlock locales auto-découvertes (glob), bundle public minimal
- LocaleSwitcher dérivé de locales (supprime ['en','fr'] en dur)
- Reflow Prettier + clarifie note QUIRKS lucide
- SVG inline currentColor (suit le thème) + favicon transparent adaptatif
- Ignore backend/data (storage dev) — évite le parasite

### Tests

- Durcit les tests parseLocales + documente les casts trusted-input
- MatchMedia mock configurable (évite redefine cross-test)
- Non-fetch panneau fermé + purge clé i18n morte mcp_intro
- Assertion précise du nom accessible (latch latch)
## [v0.1.0] — 2026-06-25

### Ajouts

- Scaffold Phase 0 — workspace Loco + Yew, Docker, CI
- CoreError + squelette de la couche service (cœur)
- Service slug (base lisible + suffixe 8 base62)
- Service security (secure_compare temps constant)
- Service pin (génération + validation 6 chiffres)
- Trait Storage + FsStorage (write atomique, read)
- Migrations projects/versions (+ unique project_id,n) + entités générées
- ProjectsService (CRUD + set/clear/verify code)
- DeployService (ordre fichier→tx, flip pointeur transactionnel)
- Migration table sessions (schéma axum-session)
- Câblage axum-session (after_routes) + helpers web (storage/session)
- Mapping CoreError→HTTP + DTO admin (PIN scopé au détail)
- Auth admin (login/logout, extracteur AdminAuth, rate-limit login)
- Middleware garde same-origin (CSRF) sur mutations admin
- API admin lecture projets (liste/détail) + tests invariant PIN
- API admin écriture projets (CRUD + code) + garde Origin + cascade versions
- Déploiement manuel + versions (activate/delete/preview no-store)
- Crate partagée latch-dto (contrat de fil back/front)
- Serving statique SPA sous /admin (ServeDir + fallback index)
- Utilitaires SPA (pin, url publique, presse-papier) + tests wasm
- Client API typé SPA (gloo-net, latch-dto, gestion 401)
- État d'auth dérivé SPA (AuthProvider, sonde boot, Protected)
- Page Login SPA (shadcn Card/Input/Button, erreur inline)
- Composants SPA CopyButton (Copié! éphémère) + PinField
- Page Liste SPA (table, copie URL, état vide, logout)
- Navigation sur toute la ligne projet (sauf cellule URL/copie) — liste
- Side-panel Créer/Éditer projet (Sheet contrôlé, code toggle, PIN)
- Side-panel Déployer (upload HTML via gloo-file, activer)
- Page Détail SPA + side-panels danger (supprimer projet/version)
- Fondation rust-i18n (locales en/fr, macro t!, enum Locale)
- LocaleProvider + use_locale + détection boot (localStorage/navigator)
- ToastProvider maison (gloo-timers) + câblage copie
- I18n + sélecteur de langue + espacement + toast erreur
- I18n + badges colorés (vars success/warning) + a11y + switcher + toast
- I18n + PIN disabled (au lieu de masqué) + slug disabled + helper text + toast
- Dropzone drag-and-drop + i18n + Toggle + toast
- I18n + badges colorés + a11y + toasts (activate/delete) + intro
- Réponses typées OkResponse/DeployResponse/ActivateResponse
- Annotations #[utoipa::path] sur toutes les routes /api
- ApiDoc agrège paths + schemas + tests de structure
- Swagger UI sous /api-docs en dev uniquement
- Client openapi-fetch typé + schema.d.ts généré + middleware 401
- App shell (router TanStack, Query, i18n FR/EN, sonner, providers)
- Route login (RHF+zod, 401→erreur, succès→liste) + useLogin/useLogout
- Route liste (table, badges accès colorés, état vide) + hooks Query + topbar
- ProjectForm side-panel (créer/éditer, PIN disabled si code off, slug RO, validation zod)
- Route détail (lecture seule) + DeployPanel dropzone + panels danger
- ProjectListItem expose active_version_n + version_count (affiche v{n} + compte)
- Jeton de cookie unlock liant le PIN (HMAC + expiration)
- PublicMeta (sans PIN) + UnlockReq pour la surface /c
- GET /api/public/{slug} (meta sans PIN) + OpenAPI
- GET /c/{slug} — proto actif no-store ou page de déverrouillage
- POST /c/{slug}/unlock — cookie signé liant le PIN (révocation par rotation §6)
- Rate-limit governor /unlock (IP+slug + plafond global slug)
- Page de déverrouillage /c (entrée Vite dédiée, React+shadcn)
- InputOTP segmenté + CardDescription i18n
- Bouton loading réutilisable + état pending sur les actions + auto-submit OTP
- Helpers deploy_token / public_base_url / host_authority (fail-secure)
- Squelette serveur rmcp Streamable HTTP monté sous /mcp (allowed_hosts dérivé)
- Tool deploy_prototype (gate token, slug préexistant, activate défaut true)
- Tool list_projects (gate token, résumé sans PIN ni hash §9.2)
- GET /api/settings (AdminAuth) expose infos MCP (deploy_token + urls)
- Panneau Settings React (infos MCP, token masqué via PinField)
- Annonce serverInfo.name="latch" (au lieu du défaut rmcp)

### Corrections

- Cargo-deny vert (licences + advisories scopés)
- Purge nom de client des sources/docs + règle de confidentialité (CLAUDE.md)
- Aligne expires sur INTEGER (fidélité schéma axum-session)
- Valide la longueur de SESSION_SECRET (erreur claire au lieu d'un panic)
- Logout invalide la session côté serveur (destroy au lieu de clear)
- Cookie Secure fail-secure (tout sauf Development/Test)
- Delete 404 avant commit + update renvoie les versions (+ test update)
- Garde same-origin sur logout + note contrat login-CSRF accepté
- DB de test in-memory (course migrations seaql sous nextest)
- Ajoute les variables CSS card/popover manquantes de shadcn-rs + Trunk addresses
- Reset du side-panel projet à la réouverture + état busy anti-double-submit
- Reset du side-panel Déployer à la réouverture + import gloo_file inliné
- Garde busy sur DeleteVersionPanel + fallback "PIN non défini" (détail)
- Chemin CSS absolu (deep-link) + polish login/pin/CONVENTIONS (revue finale)
- SPA routing (sans basename) + CSS layout + override Sheet — retours test live
- Toggle vendorisé (patch Switch shadcn-rs, état contrôlé pur)
- Glyphes boutons détail restaurés + lang=en + audit chaînes (clôture polish)
- Badges success/warning battus par .badge.variant-secondary (spécificité)
- Toast création unique + erreurs API i18n + clés deploy câblées (revue finale)
- Résoudre fetch à l'appel (interception MSW) + baseUrl origine absolue
- Revue Plan 2 — test §9.2 structurel, version active non trompeuse, login MSW réel, toasts cohérents
- Pin packageManager pnpm@9.15.9 (corepack/Docker — évite pnpm 11 + minimumReleaseAge)
- Scoper Vitest à src/ (n'exécute plus les specs Playwright e2e)
- Générer le PIN via CSPRNG (crypto.getRandomValues) au lieu de Math.random()
- Body limit configurable (LATCH_BODY_LIMIT, défaut 5mb) — gros HTML déployait en 413
- Bind 127.0.0.1 explicite — CI Playwright flaky (localhost → ::1 IPv6)
- AbortController sur le fetch brand de la page unlock (race au démontage)
- Fail-secure sur UNLOCK_COOKIE_SECRET et SESSION_SECRET (refus de boot en prod sans secret explicite)
- Corrige commentaire rate-limit auth + chemins doc unlock + BACKLOG storage
- Cases OTP en rouge + message d'erreur centré sous l'input
- Retire le favicon /vite.svg (404 sur /admin depuis base '/') + BACKLOG broutilles UI
- Unlock_cookie request-path sans #[allow] (unreachable!), commentaires governor auto-suffisants
- Avertir si ./data non-root échoue + clarifie dataprep bind-mount (revue finale)
- État d'erreur du panneau Settings (plus de panneau blanc silencieux)

### Documentation

- Passe mémoire de fin de session (Phase 0 + CI + versioning)
- Décisions Phase 1 (sessions→Phase 2, slug 8 base62) + plan d'implémentation
- Mémoire task 6 (migrations + entités + test_support)
- Mémoire Task 7+8 (INDEX + HANDOFF DeployService)
- Corrige note HANDOFF sur updated_at (set manuel volontaire, hook ≠ suffisant)
- Clôture de session — Phase 1 mergée/poussée, scrub d'historique, sea-orm-cli en toolchain
- Plan d'implémentation Phase 2 (adaptateur web admin)
- Mémoire Task 2 (axum-session câblé, QUIRKS SessionPool/Key, ENV SESSION_SECRET)
- QUIRKS — clear() vs destroy() au logout (axum_session 0.16)
- Mémoire Task 5 (middleware same-origin, QUIRKS loco 403 vs 401)
- Backlog same_host (port par défaut + IPv6 sans crochets)
- Mémoire Task 8 (deploy+versions web, preview no-store, BACKLOG storage delete)
- Clôture Phase 2 (adaptateur web admin) + report décisions au contrat
- Design Phase 3 (SPA Yew admin) — API /api, latch-dto, side-panels
- Plan d'implémentation Phase 3 (SPA Yew admin) — 14 tâches
- Corrige doc-comments /api + restaure messages d'assertion sécurité (T2)
- Corrige le doc-comment de Protected + match exhaustif explicite (T7)
- Clôture Phase 3 (SPA Yew admin) — contrat §4/§7, mémoire, Docker SPA
- Spec design polish UX + i18n (Phase 3) — punch-list post-test live
- Plan d'implémentation polish UX + i18n (Phase 3) — 10 tâches TDD
- Mémoire Task 3 (ToastProvider + CopyButton rewired)
- Clôture polish UX + i18n (mémoire à jour, punch-list cochée)
- Design base technique admin React (stack, OpenAPI, CI pistes, Docker)
- Plan 1 migration React — backend OpenAPI (utoipa, openapi.json, drift)
- Mémoire T2 (retrait latch-dto) — HANDOFF + INDEX
- Clôture mémoire Plan 1 (backend OpenAPI livré)
- HANDOFF — retrait des labels Tn ambigus + guard Swagger dev/test précisé
- HANDOFF — clôture Plan 1 (revue finale + fix) + amorce reprise Plan 2
- Plan 2 migration React — frontend app (Vite/TanStack/shadcn, 9 tasks)
- HANDOFF + INDEX (Plan 2 T6 livré)
- Plan 3 migration React — CI/Docker/e2e Playwright/docs (5 tasks)
- Alignement mémoire sur la stack React (contrat §2/§4, BOOTSTRAP, ROADMAP, ENV, QUIRKS, CONVENTIONS, INDEX, BACKLOG, README)
- HANDOFF — migration React livrée (Plans 1-3), serveur prêt pour validation
- Mémoire post-validation (body-limit, liste enrichie, CSPRNG, pré-vol CI)
- HANDOFF — fix e2e bind commité (464eb94) + CI verte (run 28153192320)
- Spec Phase 4 — serving /c/<slug> + déverrouillage
- Plan d'implémentation Phase 4 — serving /c/<slug>
- Task 5 — mémoire projet (HANDOFF + INDEX)
- Config unlock + mémoire (ENV, QUIRKS, INDEX, ROADMAP, HANDOFF, BACKLOG)
- Mémoire post-itération UI unlock (InputOTP + assets + quirk jsdom)
- HANDOFF — CI verte sur main (run 28164197300, Phase 4 + itérations)
- Spec durcissement toolchain & CI (SonarQube Cloud + cargo-chef + lints)
- §2 spec — inventaire réel du backlog Sonar (pré-scan)
- Spec — non-root container (S6471) intégré au stage runtime
- Plan d'implémentation — durcissement toolchain & CI (9 tasks)
- Mémoire Task 5 — cargo-chef + non-root (HANDOFF, INDEX, QUIRKS)
- Clôture chantier toolchain & CI + garde deploy.sh /data non-root
- HANDOFF — revue finale opus (Ready to merge) + polish deploy.sh, prêt à merger main
- CI VERTE sur main (run 28175334921, 8/8) + BACKLOG bump actions Node 20
- Spec Phase 5 — endpoint MCP + panneau Settings (design validé)
- Plan d'implémentation Phase 5 — MCP + panneau Settings (8 tâches TDD)
- Phase 5 livrée — contrat §5/§9, mémoire, scan Sonar local + règle couverture 80% dans la toolchain
- Clarif note §9 settings (sans collision de numéro) + clé metric new_coverage
- Noter le point sonar.tests / tests Rust à trancher à la finalisation
- HANDOFF + INDEX après T3 e2e serving /c
- Script de capture Playwright (skip sauf CAPTURE=1) + captures admin/unlock
- T5 captures — mise à jour HANDOFF/INDEX/QUIRKS
- CHANGELOG via git-cliff (preprocessor gitmoji) — v0.1.0
- Refonte complète (badges Sonar, captures, quickstart, archi, sécurité)
- Phase 6 LIVRÉE — vérif finale verte + mémoire + stub Phase 8
- Revue finale opus (merge) + polish serverInfo + entrée merge main

### Interne

- Bootstrap du système de mémoire projet
- Tags d'image versionnés (semver + sha) + pin du déploiement
- Tmp d'écriture unique (atomicité honnête sous concurrence)
- Retire dep tokio dev redondante + hook before_save no-op (versions)
- API admin re-préfixée /api/* + DTO via latch-dto
- Scaffold SPA (router, deps, CSS shadcn-rs vendorisée)
- Lock rust-i18n 3.1.5 + deps transitives (résolution Cargo.lock)
- Thème de marque OWLNEXT (export shadcn oklch → triplets HSL)
- Retrait du front Yew + décision de migration admin → React/Vite/shadcn
- Inline latch-dto dans backend/src/dto + dérive utoipa::ToSchema
- Retrait de la crate latch-dto (inlinée dans backend/src/dto)
- Nettoyer les doc-comments handlers (summaries propres /api)
- Scaffold Vite+React+TS+shadcn (thème bJfDPe2y), Tailwind v4, ESLint a11y
- Stage build SPA Node/pnpm (Vite) en remplacement de Trunk/wasm
- Pistes back/front parallèles + drift OpenAPI/schema + supply-chain front + e2e gate docker
- Allowlist licences front + OFL-1.1 (Inter) + MPL-2.0 (calibré au pré-vol local)
- Pin pnpm version (9.15.9) sur action-setup — l'action root ne lit pas frontend/package.json
- Helpers cookie unlock (secret≥64, Key signée, no-store path) + axum-extra
- Découple /assets du préfixe /admin (base '/' + mount /assets)
- Centre l'InputOTP + espace au-dessus aligné sur le bouton
- Bordure OTP plus foncée (oklch 0.85, même teinte que --input)
- Retire l'opérateur void (Sonar S3735 ×21)
- Props Readonly (S6759) + globalThis vs window (S7764)
- Dé-imbrication ternaires (S3358) + singletons Sonar
- Cargo-chef + runtime non-root + durcissements Sonar
- Durcissement (pin actions SHA S7637, --ignore-scripts S6505) + confort
- Unwrap_used/expect_used = warn (workspace), enforcé en CI
- Job SonarQube Cloud bloquant (gate wait) + properties
- Couverture Rust sur Sonar (cargo-llvm-cov → lcov), clippy reste l'autorité
- Classer backend/tests comme tests + supply-chain vérifiée
- Purge d'un nom client résiduel + ignore .playwright-mcp

### Sécurité

- Clôture Phase 4 + Phase 7 (peaufinage graphique/web) + patterns & quirks
- Spec finalisation (e2e, durcissement, packaging) + notes Phase 7
- Plan d'implémentation détaillé (8 tâches, TDD, setup e2e via API)
- Robots.txt + X-Robots-Tag servis par l'app + tests

### Tests

- Garde d'archi (cœur sans axum/loco) + docs: clôture Phase 1
- Garde d'archi récursive + détecte les ré-exports (pub use)
- Couvre Db/Io→500 + consomme versions dans from_model
- Durcit url_host (userinfo) + tests d'intégration du middleware same-origin
- Preview non authentifié → 401 (invariant admin-only)
- Latch-dto — sémantique absent/null de UpdateProjectReq.brand_name
- Serving direct d'un asset SPA existant (/admin/app.js)
- Durcir list_schema_has_no_pin_field (assertion JSON structurée)
- Openapi.json commité + verrou anti-drift
- Harness Vitest+MSW + PinField/CopyButton/LocaleSwitcher (tests verts)
- Smoke Playwright admin (login → créer → déployer) + harness stack réelle
- Couverture Vitest en lcov (pour Sonar)
- Couvre le new code (hooks mutation, detail, deploy-panel) → gate Sonar vert
- Garde anti-chaîne-vide de resolve_required (fail-secure)
- Deploy_prototype rejette le mauvais token via le gate (assert message)
- E2e du transport Streamable HTTP réel (initialize, tools, gate token)
- Renforce les assertions (protocolVersion, no-side-effect deploy, exactly-two tools)
- Serving /c libre + unlock par PIN + bascule de version
- Synchronise sur la réponse /unlock (anti-flaky)
