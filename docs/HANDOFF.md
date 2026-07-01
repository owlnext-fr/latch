# Handoff — état courant

> Notes informelles pour la prochaine session (humaine ou Claude). Format libre,
> chronologique inverse (le plus récent en haut). À mettre à jour en fin de session
> significative — l'idée est de se resituer en 30 secondes.

## 2026-07-01 (quater) — Issue #1 : publication doc GitHub Pages APRÈS le push Docker (CI)

### Dernière chose faite
Fix mécanique CI (`.github/workflows/ci.yml`) : le job `deploy-docs` (déploiement GitHub Pages)
passe de `needs: [docs]` à `needs: [docs, docker]`. Avant, la doc se publiait dès que son build
(`docs`) était vert, indépendamment du reste — donc une doc pouvait partir en prod alors que le
`docker build`/push échouait ensuite. Maintenant la publication est gatée par le job `docker`, qui
dépend lui-même de toute la gate (`fmt-clippy`, `test-backend`, `supply-chain`, `frontend`,
`supply-chain-front`, `e2e`, `e2e-vite`, `sonar`) → la doc n'est publiée que si l'intégralité de la
CI, build d'image compris, est verte. Le job `docs` (build/validation à chaque push/PR) est
**inchangé**. Commentaire YAML réécrit (il vantait le « couplage faible » de la Phase 8 §6.2, décision
désormais inversée par l'issue #1). Branche `fix/1-deploy-docs-after-docker`.

### Trucs en suspens
- QA/gate = la CI elle-même (pas de QA locale possible sur un changement de workflow). À vérifier au
  merge : sur un push `main`, `deploy-docs` attend bien `docker` puis se déclenche (ordre visible dans
  l'onglet Actions).
- **Effet de bord assumé** : la doc est désormais couplée aux jobs Rust/e2e. Un échec e2e flaky sans
  rapport avec la doc bloquera aussi sa publication — c'est le compromis explicitement demandé par #1
  (on préfère ne pas publier que publier une doc « orpheline » d'une release cassée).

### Notes pour future Claude
- `deploy-docs` ne tourne que sur push `main` (`if: github.event_name == 'push' && ref == main`) ;
  sur PR il est skippé, donc ce changement n'affecte pas le feedback PR (le build `docs` valide
  toujours à chaque PR). Sur un tag `v*`, `deploy-docs` ne tourne pas non plus (l'`if` exige `main`).
- Un `needs:` non satisfait **skippe** le job (ne le met pas en échec) : si `docker` casse, Pages
  n'est simplement pas redéployé (l'ancienne doc reste en ligne).

## 2026-07-01 (ter) — Fix-wave revue finale commentaires admin (DRY, doc, tests durcis)

### Dernière chose faite
Lot de correctifs cosmétiques/couverture issus de la revue finale de l'authoring admin, aucun
changement de comportement produit :
- **Backend** : helper privé `is_admin_owner(owner_token: &str) -> bool` (`backend/src/dto/mod.rs`)
  factorise la comparaison `owner_token == ADMIN_OWNER_TOKEN` dupliquée dans `to_comment_pin` et
  `to_admin_comment_message`. `backend/tests/comments_admin.rs` : assertion intermédiaire prouvant
  que le `body` ne change pas après une édition 404 cross-projet (symétrie avec `still_there` sur la
  suppression) ; nouveau test `admin_write_endpoints_rejected_on_cross_origin` (403 sur les 4
  mutations d'authoring avec origin étranger) + extension de `admin_write_endpoints_require_session`
  aux 3 mutations restantes (reply/edit/delete pin, 401 sans session).
- **Frontend** : docstring périmée de `CommentsAdapter` corrigée (mentionnait encore « et plus tard
  l'admin, Plan 3 » — l'admin authoring est livré) ; fixture `adminPin` de `thread-popup.test.tsx`
  annotée `CommentPin` (cohérence avec les autres fixtures du fichier) ; `data-testid="comment-message"`
  ajouté sur le `<li>` de message dans `thread-popup.tsx`, utilisé par `comments-admin.spec.ts` à la
  place d'un scoping par texte (`page.locator('li', { hasText })`) — sélecteur robuste.
- `docs/BACKLOG.md` : entrée ajoutée pour la convergence différée des 3 walks `pin → version →
  projet` vers `assert_version_in_project` (référencée par le fix précédent, absente du backlog
  jusqu'ici).

### Trucs en suspens
Aucun — lot 100% cosmétique/couverture, aucune route/DTO/OpenAPI changée (pas de régén
openapi.json/schema.d.ts).

### Prochaine chose à creuser
Rien d'ouvert sur ce fix-wave. Sujet naturel suivant, inchangé : décider du sort de la branche
`feat/prototype-comments` (merge dans `main` ou PR) — décision humaine.

### Notes pour future Claude
Gate complète relancée : backend (`cargo fmt` + `cargo clippy --all-targets -- -D warnings` 0 warning
+ `cargo nextest run` → 195 passed, +1 vs les 194 précédents) ; frontend (`pnpm lint` + `pnpm
typecheck` clean, Vitest → 233 passed) ; e2e (`pnpm exec playwright test comments-admin` → 3 passed,
dont le test utilisant le nouveau `data-testid`). Rapport détaillé :
`.superpowers/sdd/fixwave-report.md`.

## 2026-07-01 (bis) — Fix : scope projet sur edit/delete-pin admin (parité reply/moderate)

### Dernière chose faite
La revue finale de l'authoring admin (session précédente le même jour) avait noté une asymétrie :
`admin_edit_comment` et `admin_delete_pin` (`backend/src/controllers/admin.rs`) recevaient l'`id`
projet du path mais ne le vérifiaient jamais — ils résolvaient uniquement par owner-check
sentinelle via `edit_message`/`delete_pin` (génériques, partagés avec le visiteur). Corrigé :
nouvelles méthodes de service `admin_edit_message(project_id, comment_id, body)` et
`admin_delete_own_pin(project_id, pin_id)` (`backend/src/services/comments.rs`) qui ajoutent la
vérification `message/pin → pin → version → projet` (helper privé `assert_version_in_project`),
NotFound en cas de mismatch — parité avec `admin_add_reply`/`moderate_delete_message`. Signatures
HTTP inchangées (aucune régén `openapi.json`). TDD : 6 tests unitaires + 1 test d'intégration
(2 projets, tentative cross-projet → 404 + fil intact, bon projet → 200). Doc normative mise à
jour : `docs/contrat-deploy.md` §6.4 (mapping des 2 endpoints pointe désormais vers les nouvelles
méthodes dédiées, plus `edit_message`/`delete_pin`). Rapport détaillé :
`.superpowers/sdd/fix-projectscope-report.md`.

### Trucs en suspens
- Aucun — `edit_message`/`delete_pin` génériques restent utilisés tels quels par le flux visiteur
  (non touchés, pas de régression possible).
- Duplication mineure assumée : `admin_add_reply`/`moderate_delete_message` gardent leur propre
  inline pin→version→projet plutôt que d'être migrés vers `assert_version_in_project` (hors scope
  de ce fix, diff minimal ; refactor de convergence possible plus tard si souhaité, cf. BACKLOG).

### Prochaine chose à creuser
Rien d'ouvert sur ce fix. Sujet naturel suivant, inchangé : décider du sort de la branche
`feat/prototype-comments` (merge dans `main` ou PR) — décision humaine.

### Notes pour future Claude
- Gate relancée : `cargo fmt` + `cargo clippy --all-targets -- -D warnings` (0 warning) +
  `cargo nextest run` → 194 passed. Pas de changement frontend/OpenAPI → suites `pnpm`/Playwright
  non ré-exécutées (aucun fichier frontend touché par ce fix).

## 2026-07-01 — Authoring commentaires admin (clôture Task 9 : doc + mémoire + gate finale)

### Dernière chose faite
Tasks 1-8 (déjà committées, commits `dda592e..c24502f`) ont livré l'**authoring de commentaires
côté admin** : l'admin ne se contente plus de lire/modérer, il peut écrire. Cette session (Task 9)
clôt la feature par la doc + la mémoire + la gate finale complète, **sans toucher au code** :
- `docs/contrat-deploy.md` : §7 (bullet Commentaires) mentionne l'authoring dans `createAdminAdapter` ;
  §6.4 documente les **4 endpoints admin d'écriture** (create pin / reply / edit / delete pin) avec
  leur mapping service (`create_pin`/`admin_add_reply`/`edit_message`/`delete_pin` + sentinelle) ;
  §9 (invariant 7) documente le **jeton sentinelle `ADMIN_OWNER_TOKEN = "__admin__"`** (aucune
  migration DB, non collisionnable avec un ULID visiteur) et le booléen dérivé `is_admin` sur les
  deux DTO (`CommentMessage` + `AdminCommentMessage`), en réaffirmant que `owner_token` n'est
  toujours jamais sérialisé.
- `public_docs/content/docs/admin/comments.mdx` : nouvelle section « Leaving comments as an
  administrator » (EN) — créer un fil = note privée visible seulement en Review ; répondre à un
  visiteur = visible de ce visiteur ; éditer/supprimer ses messages ; badge « Admin ». **Retiré**
  l'ancienne phrase devenue fausse : « Administrators cannot post comments themselves ». Vérifié :
  `pnpm types:check` (fumadocs-mdx + next typegen + tsc) passe.
- `docs/INDEX.md` : ligne livrable « Authoring commentaires admin » (Phase 10), avec renvoi au
  commit range et à la spec/au plan.
- Gate finale (backend + frontend) relancée entièrement — voir résultats ci-dessous.

### Trucs en suspens
- Branche `feat/prototype-comments` toujours **non mergée** dans `main` — décision de merge/PR = humain.
- Hors périmètre v1 (backlog, déjà noté dans la spec) : diffusion des fils propres de l'admin à
  tous les visiteurs, notifications, statut « résolu ».

### Prochaine chose à creuser
Rien d'ouvert côté authoring admin — la feature est fonctionnellement complète et documentée.
Prochain sujet naturel : décider du sort de la branche (merge dans `main` ou PR) — c'est une
décision humaine, pas technique.

### Notes pour future Claude
- **`ADMIN_OWNER_TOKEN = "__admin__"`** (`backend/src/services/comments.rs`) est LE point d'entrée
  pour comprendre comment l'admin « possède » des commentaires sans colonne de rôle ni migration :
  c'est un `owner_token` constant, jamais produit par `mint_owner_token()` (ULID), réservé au
  compte admin unique. `is_admin = (owner_token == ADMIN_OWNER_TOKEN)` est calculé à la
  sérialisation dans `dto/mod.rs`, jamais stocké tel quel dans le JSON de sortie.
- **Seam `fixedAuthorName`** (posé sur `CommentsAdapter`, pas sur `Capabilities` — écart assumé
  vs la spec initiale pour minimiser la churn des littéraux `capabilities` dans les tests
  existants, même effet fonctionnel) : quand non-null (cas admin), `compose-popup.tsx` et le
  composer de réponse de `thread-popup.tsx` masquent le champ nom et soumettent ce libellé fixe —
  c'est le mécanisme qui empêche le client de choisir le nom affiché pour l'admin.
- Si une future tâche touche à nouveau l'authoring admin, relire d'abord
  `docs/superpowers/specs/2026-07-01-admin-comments-authoring-design.md` (§2 sentinelle, §5 table
  des 4 endpoints, §7 sécurité) — c'est la source de vérité design, le contrat n'en est qu'un résumé.

## 2026-07-01 — Polish fil + listes commentaires (icônes/rouge + date+heure)

### Dernière chose faite
Ajustements UX demandés pendant la validation (commit `e86999c`) :
- **Fil** (`ThreadPopup`, partagé /c + /admin) : modifier/supprimer déplacés en **boutons-icône**
  (`Pencil`/`Trash2`) en haut à droite du message ; suppression message ET suppression du fil en
  variante **`destructive`** (rouge). Tests via `data-variant="destructive"` (robuste).
- **Listes** (drawer `/c` + `/admin` Review, panneau admin `VersionCommentsPanel`) : passage du temps
  relatif à une **date absolue AVEC heure** — nouveau helper pur `comments/ui/format-datetime.ts`
  (`formatDateTime(iso, locale)`). Format retenu (choix humain) : **mois en lettres + jour/heure
  zéro-paddés** (`toLocaleString {day:'2-digit', month:'long', year:'numeric', hour:'2-digit',
  minute:'2-digit'}`) → « 05 mars 2026 à 08:03 » / « March 05, 2026 at 08:03 AM ». Le helper
  `timeAgo` (+ test) a été **retiré** (plus aucun consommateur).
Gate : vitest **227**, lint/typecheck clean, `dist/` rebuild → serveur validation `:5150` à jour.

### Notes pour future Claude
Choix « date absolue partout » tranché avec l'humain (vs relatif / infobulle). Si un affichage relatif
redevient souhaité quelque part, `format-datetime.ts` reste le point d'entrée date des commentaires.

## 2026-07-01 — Statut commentaires dans la carte Configuration (détail projet)

### Dernière chose faite
Petit correctif UX remonté pendant la validation : la carte **Configuration** du détail projet
(`frontend/src/routes/detail.tsx`) affichait le statut du **code** (activé/libre) mais pas celui des
**commentaires**. Ajout d'une ligne « Commentaires : activés/désactivés » symétrique de la ligne Code
(clés i18n `detail.comments_*` EN/FR, test `detail.test.tsx`). Commit `c552e05`. Gate : vitest 225,
lint/typecheck clean. `dist/` rebuild → serveur de validation `:5150` à jour.

### Notes pour future Claude
`project.comments_enabled` est déjà dans le DTO `ProjectDetail` (aucun backend touché). Le toggle
d'édition vit dans `ProjectForm` (déjà là) ; ici c'est purement l'affichage lecture-seule du détail.

## 2026-07-01 — Popups commentaires ancrés au pin

### Dernière chose faite
Popups de commentaires (`ThreadPopup`+`ComposePopup`) ancrés au **point du pin** plutôt qu'au bounding box
de l'élément ciblé (Tasks 1-3, commits `42ca948`, `ecc2c03`, `f7ed17c`, `ab7ca45`) : helper pur `anchorPoint`
(dédup avec `PinBadge`), nouveau hook `useFloatingPoint` (VirtualElement de taille nulle au point du pin
via `@floating-ui/dom`), câblage dans `ThreadPopup`/`ComposePopup`/`comments-app`, offset =
`PIN_RADIUS + GAP` pour garder le pin visible à côté du popup, fix anti-boucle (dépendance sur les
primitives `[x, y]` plutôt que sur la référence de l'objet `point`). Cette Task 4 clôt avec la gate finale :
- `pnpm lint` → 0 erreur.
- `pnpm typecheck` → 0 erreur (« TypeScript: No errors found »).
- `pnpm test` (Vitest) → **222 passed** (53 fichiers), inclut `anchor-point.test.ts`,
  `use-floating-point.test.ts` (dont le test anti-boucle).
- `pnpm exec playwright test` → **8 passed / 2 skipped** (suite commentaires inchangée, pas de régression).
- `cargo nextest run` (backend, contrôle) → **181 passed** (inchangé).

### Trucs en suspens
- Branche `feat/prototype-comments` toujours **non mergée** dans `main` — décision de merge/PR = humain.
- **Vérification au navigateur** (Step 3 du brief : clic sur gros conteneur → popup collé au clic ; clic
  pin → `ThreadPopup` collé avec pin visible ; pin près d'un bord → flip/shift sans sortir de l'écran ;
  même comportement côté admin/Review) **non automatisable en jsdom, reste à faire par l'humain**.

### Prochaine chose à creuser
Si la vérif navigateur (Step 3) révèle un écart visuel (offset insuffisant, flip qui recouvre le pin sur
un cas de bord réel), ajuster `POPUP_OFFSET` ou la stratégie `flip`/`shift` de `useFloatingPoint` — mais
d'abord constater le problème au navigateur avant de retoucher le code.

### Notes pour future Claude
- Pattern **`VirtualElement` de taille nulle** : pour ancrer un popup `@floating-ui/dom` sur un point
  plutôt qu'un élément réel, fournir une `reference` dont `getBoundingClientRect()` renvoie un rect
  `width:0, height:0` centré sur le point — floating-ui gère ça nativement (cf. `useFloatingPoint`).
- `POPUP_OFFSET = PIN_RADIUS + GAP` est **load-bearing** pour que le pin reste « visible à côté » du
  popup (choix Figma) — ne pas le réduire sans revalider visuellement.
- Le hook dépend des **primitives `[x, y]`** (pas de la référence de l'objet `point`) dans son
  `useEffect`/`useMemo` : sinon un `anchorPoint()` recréé à chaque rendu (nouvel objet `{x,y}`) reboucle
  l'effet en continu (repro RED : 220 appels vs GREEN : 1 appel).

## 2026-07-01 — **Fix commentaires « hors écran » (proto multi-vues)**

### Dernière chose faite
Bug remonté par l'humain (testé sur un vrai proto CRM multi-vues, code protégé) : commenter sur une vue,
changer de « page » (JS `switchScene` → `display:none`), ouvrir la liste, cliquer le commentaire → popup
en (0,0) et pas de retour sur la bonne vue. **Reproduit au navigateur (Playwright) + cause racine + fix
livré** (commit `1338163`) :
- `FollowController` marque `PinPosition.hidden` (élément résolu mais rect d'aire nulle = vue masquée).
- `OverlayLayer` ne rend plus les pins `hidden` ; `CommentsDrawer` : badge « hors écran » + note inline
  au clic (pas de fil fantôme) ; `ThreadPopup` se ferme si l'ancre passe hors écran. i18n EN/FR.
- Gate : vitest **216**, e2e comments **3/3**, lint+typecheck clean. Vérifié end-to-end au navigateur
  (pin disparaît hors vue, badge + note, retour vue → pin réapparaît et clic ouvre le fil normalement).

### Trucs en suspens
- Serveur build de démo tourne sur `:5150` (DB/stockage jetables `/tmp/latch-judge.*`) — projet démo
  `Mon Projet` (id 1) + le proto réel de l'humain (id 2, protégé). À stopper quand plus utile.
- Fix `hidden` non commité dans une branche séparée : il est **sur `feat/prototype-comments`** à la suite
  du refactor UX. Décision merge/PR de toute la branche = humain (cf. entrée suivante).

### Prochaine chose à creuser
Cf. BACKLOG : `visibility:hidden` non détecté (rect non nul), e2e multi-vues, retour-sur-vue best-effort.

### Notes pour future Claude
Confidentialité : le proto réel porte un nom client → **jamais dans le repo** (tests/fixtures = placeholders).
Les captures de repro/fix ont été gardées **hors** du working-tree (scratchpad).

## 2026-07-01 — **Refactor UX commentaires LIVRÉ — Task 8 (gate finale + mémoire)**

### Dernière chose faite
Refactor UX de la couche commentaires (frontend-only, 7 tasks code + Task 8 vérification) livré sur
`feat/prototype-comments` :
- **Pins** : couleur bleu fluo **fixe** `COMMENT_FLUO = '#18A0FB'` (`comments/ui/colors.ts`) + label = 1ʳᵉ
  lettre de l'auteur (`firstLetter`). Ambre `#f59e0b` conservé pour `orphaned`/`moved`.
- **Ciblage DOM** : bordure fluo + inset glow capé (`glowShadow`, cap 30px) dans `overlay-layer.tsx`.
- **Popups bornés au viewport** : `use-floating-rect.ts`, pipeline `offset→flip→shift(crossAxis+limitShift)→size(maxHeight+overflowY:auto)`.
- **Fix décalage pins admin** : root `OverlayLayer` passé de `absolute inset-0` à `fixed inset-0` — les
  coordonnées de `toShellRect` sont en espace VIEWPORT, un conteneur `absolute` décalé sous la topbar admin
  (`h-14`=56px) causait un double-comptage. Cf. `docs/QUIRKS.md`.
- **`CommentsDrawer`** (`comments/ui/comments-drawer.tsx`) : liste des threads, tri orphelins en bas, clic →
  scroll+ouverture du thread ; helper `timeAgo`. Câblé dans `comments-app.tsx` (bouton « My comments »
  toggle, `focusPinFromList`).
- e2e : drawer visiteur + assertion d'alignement des pins admin.

Gate finale (Task 8), toute verte :
- `pnpm lint` : 0 erreur.
- `pnpm typecheck` : 0 erreur.
- `pnpm test` (Vitest) : **212 passed** (52 fichiers).
- `pnpm exec playwright test` : **8 passed / 2 skipped** (screenshots.capture.ts, skip volontaire hors `CAPTURE=1`), 0 failed.
- `cargo nextest run` (backend inchangé, vérifié quand même) : **181 passed** (15 binaires).

### Trucs en suspens
- Décision merge/PR `feat/prototype-comments` → `main` à prendre par l'humain (rien poussé).
- Gate SonarCloud CI reste l'autorité finale après push (non relancée en local pour cette tâche mémoire-only).

### Prochaine chose à creuser
Merge `feat/prototype-comments` → `main` (ou nouvelle PR) + 1ᵉʳ déploiement prod avec le refactor UX. Pas
de suite fonctionnelle identifiée côté commentaires — la feature est fonctionnellement complète (backend
Plan 1 + visiteur Plan 2 + admin Plan 3 + refactor UX).

### Notes pour future Claude
- **Overlay commentaires = espace viewport** : si tu retouches `OverlayLayer` ou tout composant qui
  positionne des pins par-dessus un iframe/proto, vérifie que le conteneur racine est `position: fixed`
  (pas `absolute`) — cf. `docs/QUIRKS.md` pour le pourquoi exact (double-comptage d'offset).
- `COMMENT_FLUO` est une constante hex fixe, volontairement **jamais** un token `--primary` — l'overlay est
  rendu sur un proto au thème arbitraire (cf. `docs/CONVENTIONS.md`).
- Le gate de cette tâche n'a pas touché `vite.config.ts` → `pnpm test:vite` (smoke Vite dedie) non requis
  par la règle du CLAUDE.md ; seule la suite Playwright par défaut faisait partie du scope.

---

## 2026-07-01 — **Consolidation mémoire — fin de session (Plan 3 + P1/P2 + DX Vite + P3)**

### Dernière chose faite
Consolidation de la mémoire projet en fin de session + commit docs.

Session complète résumée :
- **Plan 3 (admin commentaires)** livré : `CommentsApp` adaptateur injectable (`{cacheKey, frame, adapter}`) ; `createAdminAdapter` (maps `AdminCommentMessage`→`CommentMessage`, `editable:false`, `canModerate:true`) ; modération dans `ThreadPopup` ; toggle `comments_enabled` dans `ProjectForm` (smart default + warning) ; hooks `useVersionComments`/`useModerateComment` + `VersionCommentsPanel` ; page Review `/admin/projects/{id}/versions/{n}/review` (iframe + overlay lazy) ; `frame-ancestors 'self'` sur `preview_version` ; i18n partagé via `src/i18n/locales/comments/` + `mergeFragmentGlob` ; e2e admin `comments-admin.spec.ts` ; docs Fumadocs `admin/comments.mdx`. Gate verte (vitest 195, e2e 8, lint/typecheck clean, nextest 181, Sonar local PASSED). Revue finale whole-branch = **Ready to merge YES**.
- **P1** : rate-limit login `/api/login` rendu tunable (`LATCH_LOGIN_RL_BURST`/`LATCH_LOGIN_RL_PER_SECOND`, défauts 5/2 inchangés) ; webServer Playwright pose `LATCH_LOGIN_RL_BURST=100000` ; retry-on-429 retiré des helpers e2e.
- **P2** : lot cosmétique (garde redondante, 5 clés i18n mortes retirées, `mergeFragmentGlob` refactoré, `data-testid="pin-badge"`, doc "My comments" corrigée).
- **Fix dev-server Vite** (commit `550560c`) : `vite.config.ts` — `changeOrigin+setHeader origin` (corrige CSRF 403 en dev) + proxy `/assets` → backend (corrige MIME assets visiteur en dev). 100% dev-only, sans impact prod.
- **P3** : config Playwright isolée `playwright.vite.config.ts` + smoke `e2e-vite/vite-smoke.spec.ts` + script `pnpm test:vite` + job CI `e2e-vite`. Couvre l'angle mort du proxy Vite.
- **Feature commentaires ancrés TERMINÉE bout-en-bout** : backend (Plan 1) + frontend visiteur (Plan 2) + admin (Plan 3).

### Trucs en suspens
- Décision merge/PR `feat/prototype-comments` → `main` à prendre par l'humain (rien poussé).
- **Minors différés** (non bloquants) : G1 return type JSX.Element sur CommentsApp (moot si moduleDetection:force) ; G2 garde redondante (moot P2) ; I1 warning aussi en create/assertion redondante/commentaire ref-dep ; J2 `open={x!==null}` redondant + pas d'`onError` sur `moderate.mutate` + `toLocaleDateString` sans locale ; L2 `mergeFragmentGlob` filter+cast vs destructure + `codeFromPath` dupliqué inline.
- **Backlog** : item `onError` sur la modération (J2).
- Gate SonarCloud CI reste l'autorité finale après push.

### Prochaine chose à creuser
**FIXES UX** (demandé par l'humain — focus de la prochaine session).

### Notes pour future Claude
- **Deux modèles de serving en dev** : admin SPA = Vite `:5173` (HMR) ; visiteur/unlock/shell/error = backend `:5150` (sert `dist/`). La suite e2e principale teste le BUILD sur `:5150` — elle NE voit PAS les bugs du proxy Vite.
- **Lancer le backend** : `cd backend && ADMIN_USER=admin ADMIN_PASS=secret LATCH_BINDING=127.0.0.1 cargo loco start` (port :5150).
- **Lancer le frontend dev** : `cd frontend && pnpm dev` (port :5173, proxy → :5150).
- **Pour couvrir le parcours Vite** : `cd frontend && pnpm test:vite` (smoke :5173). À lancer EN PLUS de la suite principale.
- La feature est terminée. La branche `feat/prototype-comments` est en avance sur `main` de ~25 commits. Merge + CI SonarCloud = prochaines étapes opérationnelles.

---

## 2026-07-01 — Task P3 : **smoke e2e Vite dev-server (:5173) — couvre proxy CSRF + assets MIME**

### Dernière chose faite
Gate complète validée (commit `125ba08`) :
- `frontend/playwright.vite.config.ts` : config Playwright dédiée, 2 webServers (backend :5150 + Vite :5173), baseURL :5173.
- `frontend/e2e-vite/vite-smoke.spec.ts` : 1 test couvrant login UI (CSRF) + création projet UI (CSRF) + déploiement via `page.request` (CSRF) + page visiteur `/c/<slug>` (MIME assets).
- `pnpm test:vite` → **1 passed / 0 failed** (serveurs dev réutilisés).
- Preuve de morsure sur :5199 cassé : `/assets/unlock-*.js` → `text/html` ; `POST /api/projects` avec `Origin: :5199` → `403`. Réparé via :5173 : `text/javascript` + `200`.
- CI : job `e2e-vite` ajouté, `docker` needs mis à jour.
- `pnpm lint` + `pnpm typecheck` : 0 erreur.

### Trucs en suspens
- La suite par défaut (`pnpm exec playwright test`) montre des failures en **local dev** (rate-limit non désarmé sur le backend partagé + données obsolètes). C'est une contrainte pré-existante du dev local, pas un régressif : en CI (backend frais + LATCH_LOGIN_RL_BURST=100000) → 8/0/2. Cf. QUIRKS "rate-limit /api/login".

### Prochaine chose à creuser
Merge `feat/prototype-comments` → `main` + 1ᵉʳ déploiement en prod.

### Notes pour future Claude
- `page.request` partage les cookies du navigateur (après `pageLogin`), résout par rapport à baseURL (:5173) → traverse le proxy Vite. Contrairement au fixture `request` (APIRequestContext isolé).
- Le smoke Vite échouerait si : (a) le proxy `/assets` est retiré de `vite.config.ts` → `#pin` absent ; (b) `changeOrigin`/`setHeader origin` est retiré → toast "Project created." absent (403 silencieux).
- `vite.config.broken.mjs` est temporaire (repro isolée preuve de morsure) et NE doit PAS être commité.

## 2026-07-01 — Task P1 : **rate-limit login tunable par env + désarmement e2e (retire retry-429)**

### Dernière chose faite
Gate complète validée (commit `571fa88`) :
- Backend : `cargo fmt` clean, `clippy` 0 warning, `cargo nextest` 181 passed (dont `login_is_rate_limited` PASS — défaut burst=5 conservé).
- Frontend : `pnpm lint` 0 err, `pnpm typecheck` 0 err, Playwright **8 passed / 0 failed / 2 skipped** × 2 runs (dont `CI=1`).
- Grep e2e : `429`/`setTimeout` ne renvoient plus que des commentaires explicatifs — aucune logique de retry.

### Ce qui a changé
- `env_u32`/`env_u64` passés `pub(crate)` dans `serve.rs` → réutilisés dans `auth.rs` sans duplication.
- `auth.rs` lit `LATCH_LOGIN_RL_BURST` (défaut 5) et `LATCH_LOGIN_RL_PER_SECOND` (défaut 2).
- `playwright.config.ts` webServer.command : `LATCH_LOGIN_RL_BURST=100000` ajouté.
- `apiLogin`/`pageLogin` dans les 3 specs : retour à la forme simple (pas de retry).
- `.env.example`, `docs/ENVIRONMENT.md`, `docs/QUIRKS.md` mis à jour.

### Trucs en suspens
Rien — la branche `feat/prototype-comments` est complète. Prête pour review/merge sur `main`.

### Prochaine chose à creuser
Merge `feat/prototype-comments` → `main` + 1ᵉʳ déploiement en prod (cf. `docs/INDEX.md` item Post-merge).

### Notes pour future Claude
- Le défaut `burst=5` / `per_second=2` est load-bearing : NE PAS le modifier sans adapter `login_is_rate_limited`.
- Si un 429 en e2e réapparaît, vérifier les logs webServer (la var doit apparaître au démarrage du serveur loco).

## 2026-06-30 — Task N1 : **Plan 3 (admin Review + toggle + docs) LIVRÉ — feature commentaires TERMINÉE bout-en-bout**

### Dernière chose faite
Gate complète validée + Sonar local passé + mémoire consolidée (Task N1, clôture Plan 3). Gate finale :
- Frontend : lint 0 err, typecheck 0 err, vitest **195 passed**, playwright **8 passed / 0 failed / 2 skipped**.
- Backend : `cargo fmt` clean, `clippy` 0 warning, `cargo nextest` **181 passed**.
- Sonar local : `QUALITY GATE STATUS: PASSED` (scan Docker `sonarsource/sonar-scanner-cli`, branche `feat/prototype-comments`).
- Couverture locale proxy : frontend 82.22% (statements), backend via `cargo llvm-cov` 181/181.

La feature commentaires ancrés est **terminée bout-en-bout** : backend (Plan 1) + frontend visiteur (Plan 2) + admin (Plan 3). Livrés dans Plan 3 : `createAdminAdapter` + modération `ThreadPopup`, toggle `comments_enabled` dans `ProjectForm`, hooks `useVersionComments`/`useModerateComment` + `VersionCommentsPanel`, page Review `/admin/projects/{id}/versions/{n}/review`, `frame-ancestors 'self'` sur preview, i18n partagé `locales/comments/` + `mergeFragmentGlob`, e2e admin déterministe (retry-on-429), docs Fumadocs `admin/comments`.

> **Note sur les entrées précédentes K2/L1** : elles affirmaient que la page Review affichait les clés `comment.*` en texte littéral. C'était vrai jusqu'à L2 — **corrigé par le commit `49dc0f2`** (partage i18n via `mergeFragmentGlob`). Voir QUIRKS "RÉSOLU par L2".

### Trucs en suspens
- Décision merge/PR → `main` à prendre par l'humain (branche `feat/prototype-comments` en avance sur `main`).
- **Minors différés** (non bloquants, à corriger avant ou après merge) :
  - G1 : return type `JSX.Element` manquant sur `CommentsApp`.
  - G2 : garde externe redondante `(canEditMsg||canDeleteMsg)` ≡ `canDeleteMsg` (thread-popup.tsx).
  - I1 : warning aussi en mode create (UX) ; assertion intermédiaire redondante test2 ; commentaire ref-dep manquant.
  - J2 : `open={x!==null}` redondant ; pas de `onError` sur `moderate.mutate` ; `toLocaleDateString` sans locale.
  - L2 : `mergeFragmentGlob` filter+cast vs destructure ; test2 `apiLogin` deadweight ; `codeFromPath` dupliqué inline.
- Gate SonarCloud CI reste l'autorité finale (cette session a validé le scan local Docker — les résultats CI après push seront les vrais chiffres de référence).

### Prochaine chose à creuser
- Créer la PR `feat/prototype-comments` → `main` + surveiller la CI SonarCloud.
- Corriger les Minors si désirés avant merge.

### Notes pour future Claude
- `mergeFragmentGlob(resources, glob)` est la clé pour partager des clés i18n entre bundles — voir CONVENTIONS et QUIRKS (entrée RÉSOLU).
- `pageLogin(page)` obligatoire pour tout e2e admin browser (cf. QUIRKS "session request ≠ page").
- Les helpers login font retry-on-429 (≤6×, 800 ms) — ne pas toucher au rate-limit prod.
- La feature est terminée. Le prochain chantier est le merge + éventuellement une release taggée.

## 2026-06-30 — Task L1 : **e2e admin Review + modération livré**

### Dernière chose faite
`frontend/e2e/comments-admin.spec.ts` créé et vert (2 passed, 5.7s). Stratégie B (API directe, Option B) retenue pour le seed du commentaire : POST `/c/{slug}/comments` avec un `AnchorDescriptor` v1 ciblant `#cta` (présent dans le proto HTML) → pin status `anchored` dans la Review iframe. Login admin dans la page via `pageLogin(page)` (formulaire `/admin/login`) car la session `axum_session` posée par `apiLogin(request)` n'est PAS partagée avec le contexte browser `page` (cf. QUIRKS). Deux bugs TypeScript de build bloquants corrigés au passage : `admin-adapter.test.ts` manquait les imports `vi/it/expect` vitest ; `version-comments-panel.tsx` utilisait `JSX.Element` (namespace non résolu avec `moduleDetection:force`). Commit `6fd346d`.

### Trucs en suspens
Plan 3 reste : toggle `comments_enabled` dans `ProjectForm` (K3), passe `public_docs` (K4/M). La gate passe (tsc/lint/vitest 189/e2e).

### Prochaine chose à creuser
Plan 3 Task K3 : connecter le champ `comments_enabled` dans `ProjectForm` (front) → `UpdateProjectReq` déjà has le champ, il reste à câbler le switch et les clés i18n dans le formulaire (le champ `comments_enabled` est déjà dans le form schema `project-form.tsx`, potentiellement complet).

### Notes pour future Claude
- `pageLogin(page)` est le helper à utiliser pour tous les tests e2e admin qui naviguent dans le SPA. `apiLogin(request)` seul ne suffit pas pour la navigation browser (voir QUIRKS).
- ~~Les clés `comment.thread.*` / `comment.bar.*` s'affichaient en texte littéral en Review admin.~~ **OBSOLÈTE depuis L2** : corrigé par `mergeFragmentGlob` (commit `49dc0f2`, cf. QUIRKS §i18n "RÉSOLU par L2"). Les clés `comment.*` sont désormais fusionnées dans le bundle admin via `src/i18n/locales/comments/`.
- `delete_message` côté backend supprime aussi le pin si c'était le dernier message (`soft_delete_pin_if_empty`).

## 2026-06-30 — Task K2 : **Page Review admin livrée**

### Dernière chose faite
Route SPA `/admin/projects/$id/versions/$n/review` créée (`frontend/src/routes/review.tsx`). La page est full-screen : breadcrumb retour projet, iframe sur `previewUrl(id, n)` (same-origin, `frame-ancestors 'self'`), overlay lazy `CommentsApp` avec `createAdminAdapter` — même pattern reloadKey que `CommentsMount` (bump sur event `load` iframe, ref callback `useState<HTMLIFrameElement | null>`). Bouton « Review » (`MessagesSquare`, `Link` TanStack) ajouté dans `detail.tsx` (entre Comments et Preview). `reviewPath` helper dans `lib/utils.ts`. Clés i18n `review.*` + `detail.review_aria` EN+FR. Route câblée dans `router.tsx`. Test Vitest `review.test.tsx` vert (stub `@/comments` via `vi.mock`). Gate : lint 0 err, typecheck 0 err, 26 tests verts. Commit `6bb8275`.

### Trucs en suspens
Plan 3 reste : toggle `comments_enabled` dans `ProjectForm` (§10.1), passe `public_docs` (§13). La feature n'est pas terminée bout-en-bout (l'admin peut naviguer en Review mais ne peut pas activer/désactiver les commentaires depuis le formulaire projet).

### Prochaine chose à creuser
Toggle `comments_enabled` dans `ProjectForm` : champ boolean à ajouter + clé i18n `form.comments`/`form.comments_help` déjà dans en.json/fr.json — à câbler via `UpdateProjectReq`. Puis passe docs publiques.

### Notes pour future Claude
- Le pattern `useState<HTMLIFrameElement | null>(null)` + `ref={setFrameEl}` est documenté dans `docs/CONVENTIONS.md` (section shell) ; ne pas utiliser `useRef` pour éviter les re-renders manquants.
- `createAdminAdapter` a `canModerate: true`, `canAuthor/canEditOwn: false` — c'est l'adaptateur modération uniquement.
- La page Review n'a pas de Topbar (plein écran intentionnel, breadcrumb minimal).

## 2026-06-30 — Commentaires ancrés : **Plan 2 (FRONTEND VISITEUR) LIVRÉ**

### Dernière chose faite
Module `src/comments/` complet + montage shell lazy, exécuté en subagent-driven (Tasks 0,A1-A4,B0-B2,C1-C2,D0-D9,E1,F0-F2). Branche `feat/prototype-comments`.

Livré (frontend visiteur) :
- **Module partagé `src/comments/`** : seam `Picker` + `SameOriginPicker` (transposition iframe→shell), moteur d'ancrage `describe`/`resolve`/`similarity`, contrôleur de suivi rAF, adaptateur visiteur + hooks React Query confinés (`commentsKey(slug)`), overlay/pastilles/popups `@floating-ui`, barre d'action, machine pick.
- **Shell visiteur** : `/c/<slug>` lit `PublicMeta.comments_enabled` et charge le module en lazy (`React.lazy`, 1er du repo) ; React Query confiné au chunk (son propre `QueryClient`).
- **e2e Playwright** : `e2e/comments.spec.ts` — cibler un élément, écrire un commentaire, pin ancré, persistance après reload.
- **Gate finale verte** : lint 0 err, typecheck 0 err, vitest **173 passed** (44 fichiers), playwright **6 passed**.

### Trucs en suspens
**Plan 3 = admin Review (§10 spec), toggle `comments_enabled` dans `ProjectForm` (§10.1), docs publiques Fumadocs (§13) — PAS commencés.** La feature n'est PAS terminée bout-en-bout (le visiteur marche, l'admin ne lit/modère pas encore via UI).

Une revue finale whole-branch + un lot de petits nettoyages (Minors) restent à passer avant merge (voir `.superpowers/sdd/progress.md` section « MINORS DIFFÉRÉS »).

### Prochaine chose à creuser
Écrire et exécuter le **Plan 3** : toggle `comments_enabled` dans `ProjectForm`, vue Review admin (monte le même module `src/comments/` côté admin), passe `public_docs`. Spec : `docs/superpowers/specs/2026-06-30-prototype-comments-design.md`.

### Notes pour future Claude
- Corps des commentaires = texte brut (pas de markdown) ; `owner_token` jamais reçu côté client (booléen `editable`).
- Le gros module se charge en lazy ; React Query est confiné au chunk.
- Transposition iframe→shell : le picker ajoute le rect de l'iframe (voir QUIRKS). Le clic de pick se calcule en espace shell (`e.clientX`), pas `el.getBoundingClientRect()` (espace iframe).
- En-tête `X-Comment-Client: '1'` requis sur tous les writes commentaire (anti-CSRF).

---

## 2026-06-30 — Commentaires ancrés : **Plan 1 (Backend) LIVRÉ** (T1-T11), frontend à venir

### Dernière chose faite
Backend complet de la feature **commentaires ancrés type Figma** sur les prototypes, exécuté en
subagent-driven (11 tâches, branche `feat/prototype-comments`). **Périmètre = backend uniquement** ;
les **Plans 2 (frontend module partagé + shell visiteur)** et **3 (admin Review + docs publiques)**
ne sont PAS commencés — la feature N'EST PAS terminée bout-en-bout.

Livré (backend) :
- **Schéma** : tables `comment_pins` + `comments` (FK→versions/comment_pins CASCADE, soft-delete `deleted_at`) + colonne `projects.comments_enabled` (backfill = `code_enabled`).
- **Cœur** `services/comments.rs` : `CommentsService` (create_pin/add_reply/list/count + edit/delete/delete_pin/moderate, owner-check via `secure_compare`→NotFound, validation 2000/80, plafond 200 pins).
- **Identité visiteur** : cookie signé `latch_comment` (ULID opaque, réutilise `UNLOCK_COOKIE_SECRET`), garde `X-Comment-Client`.
- **Endpoints publics** `/c/{slug}/comments` (GET + POST + replies + PUT edit + DELETE message/pin), gated `unlock_ok`+`comments_enabled` (403 verrouillé / 404 désactivé), rate-limit `LATCH_COMMENT_RL_*`, Origin guard.
- **Admin** : `GET /api/projects/{id}/versions/{n}/comments` (`list_version_comments`) + `DELETE …/comments/messages/{cid}` (modération, walk projet), `comment_count` live.
- **DTOs + OpenAPI** régénérés (`openapi.json` + `schema.d.ts`), drift GREEN. Contrat `docs/contrat-deploy.md` amendé (§3/§6.4/§7/§9 invariant 7).
- **Invariant** : `owner_token` JAMAIS sérialisé (structurel + test 3 surfaces).
- **Gate finale verte** : `cargo fmt`/`clippy` clean, `nextest` **181 passed**, `openapi_drift` green, `cargo-deny` PASS, `pnpm typecheck` clean. **Revue finale opus = Ready to merge YES.** **Gate SonarCloud PASSED** (new_coverage **97.7 %**, new_duplicated_lines_density **2.1 %** après 2 passes de dédup, ratings A/A/A).

### Trucs en suspens
- **Branche `feat/prototype-comments` CONSERVÉE telle quelle** (décision : on enchaîne Plans 2-3 dessus, pas de merge/PR pour l'instant). HEAD `0634108`, ~20 commits devant `main`.
- **Plans 2 et 3 (frontend) à écrire puis exécuter SUR CETTE BRANCHE** — c'est là que vit toute l'UX (picker, ancrage, suivi, overlay, barre d'action, vue Review admin). Le backend les attend.
- Avant de coder le Plan 2 : faire une **recon frontend** (shell/Vite/entrées, React Query, shadcn, bundles unlock/shell isolés) pour un plan sans placeholder — comme la recon backend du Plan 1.
- Minors non bloquants consignés dans le ledger SDD (`.superpowers/sdd/progress.md`) pour la revue finale : filtre `DeletedAt.is_null()` défensif sur le lookup pin de `moderate_delete_message` ; quelques tests à durcir ; formatage.

### Prochaine chose à creuser
Écrire le **Plan 2 (frontend)** : module `src/comments/` (interface `Picker` same-origin, échelle
d'ancrage W3C, contrôleur de suivi, overlay/popup `@floating-ui/dom`, barre d'action, adaptateur de
données) chargé en lazy dans le shell visiteur ; puis Plan 3 (toggle `ProjectForm`, vue Review admin
montant le même module, passe `public_docs`). Spec : `docs/superpowers/specs/2026-06-30-prototype-comments-design.md`.

### Notes pour future Claude
- Le **corps des commentaires est en texte brut** (pas de markdown) — décision produit. Ne pas réintroduire de rendu HTML serveur.
- `moderate_delete_comment` : `require_same_origin` (mutation) mais pas `X-Comment-Client` (endpoint admin, pas visiteur) — voulu.
- `comments_gate` renvoie `Result<_, Response>` (statuts 403/404 exacts), PAS `loco_rs::Error` (qui transformerait 403→401).
- Le frontend consommera les types depuis `schema.d.ts` régénéré (admin GET = `list_version_comments`, distinct du serve GET `list_comments`).

---

## 2026-06-30 — Micro-feature : redirection `GET /` → `/admin`

### Dernière chose faite
Ajout d'une **redirection `GET / → /admin`** (307 temporaire). Avant : `/` tombait sur le
fallback Loco (page welcome en dev, 404 en prod). Implémentation minimale :
- `backend/src/app.rs` : handler `root_redirect()` (`axum::response::Redirect::temporary("/admin")`)
  + route `router.route("/", get(root_redirect))` dans `after_routes` (à côté de `robots.txt`).
- `backend/tests/hardening.rs` : test `root_redirects_to_admin` (assert 307 + `Location: /admin`).
  axum-test ne suit pas les redirections (`Policy::none`) → on observe le 307 brut.

**Gate** : `cargo fmt` OK, `clippy --all-targets -D warnings` 0 issue, `cargo nextest run` **147 passed**.

### Trucs en suspens
- Commit + push à faire (backend-only, aucun impact frontend).
- 307 **temporaire** choisi à dessein (réversible si `/` doit un jour servir une landing) — ne pas
  passer en 308 sans raison (cache navigateur dur).

### Notes pour future Claude
- La racine `/` n'était capturée par aucune route (les contrôleurs `home` sont préfixés `/api`,
  l'admin est `nest_service("/admin", …)`). La route `/` est donc libre — pas de conflit axum au boot.

---

## 2026-06-30 — Resync post-release v1.1.0 + nettoyage repo

### Dernière chose faite
Resync de l'état réel du repo après la **release des notes de version + patchs UX, taggée
`v1.1.0` et poussée hier**. Les entrées datées 2026-06-29 ci-dessous (feat/release-notes,
feat/release-notes-ux) sont **toutes mergées et releasées** — elles parlaient encore de « à
merger » / « v0.4.0 », c'était périmé. `main` == `origin/main` (synchro).

Nettoyage effectué :
- **Commit `5de6c8b`** : fix `LATCH_STORAGE_ROOT` (`./data` → `/data`, chemin absolu) +
  doc du piège (QUIRKS) + déclencheur incident rattaché à la Phase 9 (ROADMAP). C'étaient les
  3 fichiers qui traînaient non commités dans le working-tree.
- **6 branches locales mergées supprimées** : `chore/toolchain-ci-hardening`, `feat/admin-react`,
  `feat/phase-3-spa-yew-admin`, `feat/phase-4-serving`, `feat/phase-5-mcp`,
  `feat/phase-7-lot-1-fondations`. Les branches `feat/release-notes*` avaient déjà été nettoyées.

### Trucs en suspens
- **`5de6c8b` non poussé** : commit local sur `main`, à pousser (`git push`).
- La passe **Phase 9** reste ouverte (cf. ROADMAP) — c'est le prochain gros chantier.

### Prochaine chose à creuser
- **Phase 9 — passe de polish** : (1) login → `LanguageSelect` (Select + drapeaux) au lieu du
  vieux `LocaleSwitcher` ; (2) corrections pages de doc `public_docs/` ; (3) zoom des images
  (docs `ImageZoom` + lightbox landing) ; (4) **audit relatif-vs-absolu de toutes les variables
  de chemin** (`LATCH_SPA_DIST`, `DATABASE_URL`…) + garde-fou code envisagé sur `storage_from_ctx`
  (refus de boot si chemin relatif hors Dev/Test, fail-secure comme les secrets de cookie).
- **Post-merge Phase 8** (toujours coché [ ] dans INDEX) : confirmer le déploiement GitHub Pages
  vert + basePath sur l'URL live.

### Notes pour future Claude
- Le **storage prod DOIT être un chemin absolu** (`/data`) — un relatif part sur la couche
  éphémère `/app/data`, perdue à la recréation du conteneur (DB sur volume, HTML perdus). Cf. QUIRKS.
- Si une donnée HTML a été perdue par ce piège : re-déployer chaque proto (MCP ou upload admin).

---

## 2026-06-29 — UX patchs release-notes (feat/release-notes-ux)

### Dernière chose faite
Documentation (Fumadocs + mémoire) des 3 patchs UX implémentés sur `feat/release-notes-ux` :

1. **Preview depuis la liste projets** — chaque ligne de la liste admin a une action Preview (icône)
   qui ouvre la version active dans un nouvel onglet via la route admin `GET /api/projects/{id}/versions/{n}/preview`
   (`no-store`, derrière la session) ; l'action est désactivée si le projet n'a pas de version active.
2. **Icône notes** — l'indicateur « a des notes » dans la liste des versions est désormais une icône
   lucide `FileText` au lieu de l'emoji 📝.
3. **Panel Détail** — un bouton « Détail » sur chaque ligne de version ouvre un side-panel read-only
   via `Sheet` affichant le numéro de version, la date de déploiement, le statut (actif) et les notes
   rendues via `MarkdownView` (identiques à ce que voit le visiteur).

**Fichiers doc mis à jour :**
- `public_docs/content/docs/admin/projects.mdx` (section « The project list » — action Preview)
- `public_docs/content/docs/admin/versions.mdx` (icône notes + action Details dans la liste)
- `docs/INDEX.md` (entrée « Admin UX patchs »)
- `docs/CONVENTIONS.md` (helper `previewUrl` + pattern panel read-only Sheet + MarkdownView)
- `docs/HANDOFF.md` (cette entrée)

### Trucs en suspens
- Tests frontend (Preview liste, panel Détail, icône `FileText`) = à la charge de la branche code.
- CI SonarCloud gate `new_coverage ≥ 80%` à vérifier après l'implémentation.
- Merge `feat/release-notes-ux` → `main` quand tests verts.

### Prochaine chose à creuser
- Vérifier le rendu du panel Détail sur mobile (plein écran / dismiss accessible).
- Post-merge : régénérer le CHANGELOG (`git-cliff --tag vX.Y.Z`).

### Notes pour future Claude
- `previewUrl(projectId, n)` est dans `@/lib/utils` — réutiliser pour tout lien de preview admin.
- Le panel Détail utilise `<Sheet>` (Radix) avec le bouton de fermeture X intégré — pattern
  identique aux autres panels read-only (cf. CONVENTIONS « Side-panel via Radix `<Sheet>` »).
- `MarkdownView` est partagé entre admin (aperçu déploiement, panel Détail) et shell (overlay
  visiteur) — tout changement de l'allow-list affecte les deux.
- La route preview `/api/projects/{id}/versions/{n}/preview` est derrière `AdminAuth` + `no-store`.
  Elle est déjà câblée côté backend (Phase 2) ; la nouveauté est l'exposition dans la liste.

---

## 2026-06-29 — Fix duplication Sonar (feat/release-notes)

### Dernière chose faite
Gate Sonar `new_duplicated_lines_density` ramenée sous 3 % (était 4,7 %).

**Refactors appliqués :**
- Frontend : factory `src/i18n/create-bundle-i18n.ts` — shell/i18n.ts, unlock/i18n.ts, error/i18n.ts réduits à 5 lignes chacun.
- Backend : helpers `resolve_project_html` / `resolve_project_status` dans `serve.rs` — bloc `get_by_slug` factorisé (serve, raw, notes).

**Gate résultat :** `QUALITY GATE STATUS: PASSED` (scan local SonarCloud confirmé).  
**Commit :** `59c7b45` — tous tests verts (146 nextest + 94 vitest), clippy OK, build OK.

### Trucs en suspens
Branche `feat/release-notes` prête à merger (gate Sonar passée).

### Prochaine chose à creuser
Merger feat/release-notes dans main et créer la release v0.4.0.

### Notes pour future Claude
La factory `createBundleI18n` est dans `src/i18n/create-bundle-i18n.ts`.
Le glob DOIT rester un littéral statique chez l'appelant (contrainte Vite).
Chaque bundle garde son propre `createInstance()` — isolation requise par Sonar et par le design multi-bundle.

---

## 2026-06-29 — Phase 9 : notes de version livrées (feat/release-notes)

### Dernière chose faite
Feature **notes de version** implémentée et documentée sur la branche `feat/release-notes`.

**Ce qui est livré :**
- Colonne `versions.release_notes` (TEXT NULL, max 10 000 chars). Markdown brut stocké, jamais rendu serveur.
- Validation 400 / `invalid_params` MCP au-delà de 10 000 caractères (comptage `chars()`).
- Nouvel argument `release_notes` optionnel sur `deploy_prototype` (MCP).
- Panneau de déploiement admin : éditeur Tiptap (WYSIWYG léger) + onglet Aperçu ; indicateur 📝 sur les lignes de version.
- `MarkdownView` restreint (`react-markdown` + `skipHtml + allowedElements`) partagé entre aperçu admin et overlay visiteur. Périmètre : paragraphes, titres, gras, italique, listes, citation. Interdits : liens, images, code, HTML brut.
- `GET /c/<slug>` sert désormais un **shell** (iframe vers `/c/<slug>/raw`).
- `GET /c/<slug>/raw` : HTML brut, `frame-ancestors 'self'` + `no-store`, gardé par l'unlock.
- `GET /c/<slug>/notes` : `{ n, notes_md }` ou 204, `no-store`, gardé par l'unlock.
- Overlay visiteur : affiché au premier passage sur une nouvelle version, mémorisé via `localStorage['latch:seen:<slug>']` = dernier `n` vu.
- Shell = bundle Vite isolé (`src/shell/`) avec sa propre instance i18n (`src/shell/i18n.ts` + `locales/shell/`).

**Documentation mise à jour :**
- `docs/contrat-deploy.md` (§3 versions.release_notes, §5.1 deploy_prototype, §6 refondu shell+iframe+notes)
- `public_docs/content/docs/admin/versions.mdx` (section release notes : éditeur, aperçu, périmètre, indicateur 📝, overlay visiteur)
- `public_docs/content/docs/publish-from-claude/tools-reference.mdx` (release_notes dans signature + tableau deploy_prototype)
- `public_docs/content/docs/how-it-works/architecture.mdx` (shell+iframe, endpoints /raw+/notes, overlay, avertissement iframe)
- `public_docs/content/docs/how-it-works/security-model.mdx` (section Markdown rendering : allow-list, rendu client-only, gate /notes, CSP frame-ancestors)
- `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`

### Trucs en suspens
- Tests : les tests backend (migration, service deploy avec release_notes, endpoint /raw + /notes, invariants) et frontend (MarkdownView, overlay shell, éditeur Tiptap, indicateur 📝) sont à la charge du code livré sur la branche — la présente tâche est documentation uniquement.
- CI : vérifier que la gate SonarCloud `new_coverage ≥ 80%` passe après l'implémentation.
- Merge : `feat/release-notes` → `main` quand tests verts.

### Prochaine chose à creuser
- Vérifier le rendu de l'overlay sur mobile (plein écran, dismiss accessible).
- Envisager une limite de longueur côté éditeur Tiptap (feedback en temps réel au lieu d'une erreur serveur).
- Post-merge : régénérer le CHANGELOG (`git-cliff --tag vX.Y.Z`).

### Notes pour future Claude
- **Tous les protos tournent désormais en iframe** : `window.top` accessible depuis le proto = shell, fullscreen API réduite, postMessage scope différent. Cf. QUIRKS.
- **`release_notes` est stocké brut** (markdown) ; le rendu se fait UNIQUEMENT côté client. Ne jamais rendre les notes en HTML serveur.
- **`MarkdownView` restreint** = composant partagé entre admin (aperçu) et shell (overlay). Ce que l'admin voit = ce que le visiteur voit.
- **Le shell est une mini-SPA Vite isolée** (même moule que `unlock` et `error`) — sa propre instance i18n, son propre `locales/shell/`, pas de dépendance au bundle admin.
- **`localStorage['latch:seen:<slug>']` = dernier `n` vu** : clé par slug, valeur = numéro de version. L'overlay se ré-affiche si `n` change (nouvelle version avec notes).

## 2026-06-26 — Hotfix prod `v0.3.1` : session admin non restaurée derrière HTTPS (bug axum_session)

### Dernière chose faite
**Diagnostic + fix d'un bug bloquant en prod : impossible de se connecter à l'admin** (déploiement
`https://latch.3.tools.owlnext.fr`). Symptôme : login OK puis retour à `/admin/login` **sans message
d'erreur**. Diagnostic au curl : `POST /api/login` → **200** (cookie posé), puis `GET /api/projects`
avec le cookie → **401**, et l'**UUID de session change à chaque requête** → le serveur crée une
session neuve à chaque fois.

**Cause (lue dans la source de la lib)** : bug `axum_session 0.16.0` (`src/headers.rs`). Avec
`with_prefix_with_host(true)`, l'**écriture** préfixe `__Host-` (`NameType::get_name`) mais la
**lecture** (`get_headers_and_key`) lit le champ brut `session_name` (= `latch_admin`) sans repasser
par `get_name` → cherche `latch_admin`, ne trouve jamais `__Host-latch_admin`. Ne se déclenche qu'en
prod (`is_prod=true`) ; invisible en dev/test (CI verte) car pas de préfixe.

**Fix** (`backend/src/web/mod.rs::build_session_store`) : ne plus utiliser `with_prefix_with_host` ;
poser nous-mêmes les noms `__Host-latch_admin` / `__Host-store` via `with_session_name`/`with_store_name`
**en prod uniquement** (HTTP en dev rejetterait un cookie `__Host-`), `prefix_with_host=false` → noms
symétriques lecture/écriture. Durcissement `__Host-` préservé (convention de nom policée par le navigateur :
Secure + Path=/ + pas de Domain, déjà fournis). Gate : compile + fmt + clippy + tests web verts.

### Trucs en suspens
- **Release `v0.3.1` en cours** : branche `fix/admin-session-host-prefix` → merge `main` → tag `v0.3.1`
  → CI build l'image (`0.3.1`/`latest`) → sur la box `./deploy.sh` (pull `latest`). **Re-tester ensuite**
  les 2 curls (login 200 puis `/api/projects` 200 avec le cookie) **et** le login navigateur réel.
- **`.env.server` + `nginx.conf`** (copies de la box, posées à la racine pour debug) : ajoutés au
  `.gitignore` (le `.env.server` contient les **secrets prod réels** — ne jamais committer ; envisager
  une rotation puisqu'ils ont transité par le working-tree).
- Le bug de session ne pouvait PAS être attrapé par les tests existants (Test force `is_prod=false`).
  Piste : test d'intégration « prod-like » (env custom + assert round-trip cookie) — non fait, à évaluer.

### Prochaine chose à creuser
- Après confirmation que l'admin se connecte en prod : reprendre la **vérif post-merge Phase 8** (Pages
  live + basePath) puis la **Phase 9 (polish)**.

### Notes pour future Claude
- **Ne jamais réactiver `with_prefix_with_host`** tant qu'axum_session n'est pas patché en amont (la
  lecture n'applique pas le préfixe). Le préfixe `__Host-` se pose via le **nom littéral** du cookie.
- Pour ce genre de bug « marche en dev, casse en prod » : le différentiel est presque toujours
  `is_prod` (cookie Secure + `__Host-`). Reproduire au curl (UUID de session qui change = session
  jamais restaurée) avant de toucher au code.

## 2026-06-26 — Phase 8 (site doc Fumadocs) MERGÉE sur `main` (tag **v0.3.0**)

### Dernière chose faite
**Phase 8 — site de documentation publique — implémentée, mergée sur `main` (fast-forward), taggée `v0.3.0` et poussée.** CHANGELOG régénéré (git-cliff `--tag v0.3.0`). Build statique vert (66 pages). Le merge déclenche le job CI **`deploy-docs`** (1ᵉʳ déploiement Pages).

- **App** : `public_docs/` (Fumadocs 16 / Next 16 / React 19, template `+next+fuma-docs-mdx+static`, layout `src/`). Export statique, **basePath `/latch`** + `assetPrefix` + `public/.nojekyll` (sous-chemin GitHub Pages, **pas de domaine custom**).
- **Landing** : hero + **parcours 3 étapes** (étape 2 = **conversation Claude simulée en CSS**, `ClaudeChat`) + features + CTA + footer. Identité produit (preset `shadcn.css`, tokens stone/oklch, logo inline `currentColor`, Inter). Captures réutilisées de Phase 6 (`docs/assets/*.png` → `public/img/`).
- **Docs EN** (sourcées du contrat/BOOTSTRAP, jamais le `docs/` interne) : how-it-works, deploy (reverse-proxy 4 serveurs + config 17 clés), admin, publish-from-claude (2 tools), quickstart, troubleshooting. Recherche statique Orama.
- **CI** : jobs **`docs`** (build push/PR) + **`deploy-docs`** (Pages, `main` only) ajoutés à `ci.yml` (pas de workflow séparé). Pages = GitHub Actions **déjà activé**.
- **Liens produit** : `README` + `frontend/src/lib/links.ts` `DOCS_URL` → `https://owlnext-fr.github.io/latch/docs`.

### Trucs en suspens
- **Surveiller la CI sur `main`** : 1ᵉʳ run post-merge avec les jobs `docs` + `deploy-docs`. Vérifier le déploiement Pages vert puis **charger une page profonde live et confirmer que `_next/` se charge** (piège basePath) → cocher la ligne « post-merge » de `docs/INDEX.md`.
- **Serveurs dev laissés tournés** : site doc Fumadocs sur `:3000` (`/latch/`), app latch backend `:5150` + frontend Vite `:5173` (`/admin/`, login `admin`/`secret`).
- e2e/visual : la landing référence les vraies captures Phase 6 ; si l'UI admin change, régénérer via le harnais Playwright (`CAPTURE=1`) et recopier dans `public/img/`.

### Prochaine chose à creuser
- **Phase 9 — passe de polish** (cf. ROADMAP) : (1) **sélecteur de langue du login** en ancien modèle (`LocaleSwitcher` boutons FR/EN dans `frontend/src/routes/login.tsx`) → migrer vers `LanguageSelect` (Select + drapeaux, Phase 7 Lot 2) ; (2) **corrections de pages de doc** (à préciser à la relecture) ; (3) **zoom des images** docs (Fumadocs `ImageZoom`) + landing (lightbox sur les captures du parcours).

### Notes pour future Claude
- **public_docs = app isolée** : son `package.json`/lockfile, **hors** workspace Rust et **hors** `frontend/`. Node 24, pnpm 9.15.9. `pnpm dev` → `http://localhost:3000/latch/`.
- **Pièges Fumadocs/basePath** consignés dans QUIRKS (scaffold via PTY, images en import statique, `.nojekyll`, MDX = JSX, Shiki `caddy`→`text`, recherche statique).
- **Bascule domaine custom plus tard** = poser `DOCS_BASE_PATH=''` + CNAME (env-driven, zéro code à toucher).

## 2026-06-26 — Phase 7 MERGÉE sur `main` (tag **v0.2.0**) + logo adaptatif + flow MCP validé en vrai

### Dernière chose faite
**Phase 7 (4 lots) mergée sur `main` en fast-forward, taggée `v0.2.0`, poussée.** CHANGELOG régénéré (git-cliff `--tag v0.2.0`). CI surveillée au push.

Après la clôture du Lot 4, trois choses ajoutées avant le merge :
- **Logo adaptatif au thème** (commit `4954f97`) : le composant `Logo` passe d'un `<img src>` à un **SVG inline en `currentColor`** (fond blanc retiré) → suit `text-foreground`, donc mark sombre en clair / clair en sombre. Le **favicon** (`src/assets/latch-logo.svg`) devient transparent + adaptatif via `<style> @media (prefers-color-scheme: dark)`. Tests : `getByAltText('latch')` → `getByRole('img', { name: 'latch' })` (5 fichiers). *(Rappel : `currentColor` ne marche QUE sur SVG inline, jamais via `<img src>` — d'où l'inlining ; idem icône GitHub Lot 3.)*
- **Flow MCP validé de bout en bout (vrai endpoint live)** : backend lancé servant le build, puis `initialize` → `tools/list` → `list_projects` → `deploy_prototype(slug="azerazer-IiLHqghy", activate=true)` sur `/mcp` (transport Streamable HTTP réel, token gate dev). Résultat : `DeployResult { url, version: 2, code_protected: true }` (aucun PIN/hash → invariant §9 OK), v1→v2 active en DB, HTML stocké, et **servi après unlock** (PIN). Caveat Claude Code : `claude mcp add` en cours de session ne charge PAS les tools (chargés au démarrage) ; `/mcp` ne reconnecte que les serveurs déjà chargés → test fait via client HTTP sur le même endpoint live (identique).
- **Fix `.gitignore`** (commit `ef9eb3f`) : `backend/data/` (storage dev quand on lance depuis `backend/`) n'était PAS ignoré (`/data` n'attrape que le volume Docker prod racine) → comblé. `backend/data` + scratchpad de test purgés.

### Trucs en suspens
- **CI sur `main`** : 1ʳᵉ exécution post-merge Phase 7 (v0.2.0) — vérifier gate SonarCloud `new_coverage ≥ 80%` + build/push image GHCR (`main` + `v0.2.0` → tags `0.2.0`, `0.2`, `latest`).
- **Scan Sonar local** non lancé (la CI fait foi).
- e2e Playwright « page d'erreur /c stylée » = optionnel non fait → BACKLOG.
- Lien doc du bouton « ? » topbar = URL Phase 8 (Fumadocs) pas encore en ligne (assumé).

### Prochaine chose à creuser
- **Phase 8** — Documentation publique (Fumadocs / GitHub Pages). Le README + le bouton « ? » pointent vers une URL doc à publier.

### Notes pour future Claude
- Phase 7 = 4 lots (fondations i18n/thème · panneau Settings side-panel · identité visuelle · page d'erreur serving), chacun spec+plan+subagents, 4 revues finales opus = YES.
- Pour une icône/logo qui suit le thème : **SVG inline `currentColor`** (pas `<img>`). Favicon adaptatif : `@media (prefers-color-scheme: dark)` dans le SVG.
- Storage dev = `backend/data` (désormais gitignoré). Prod = volume `/data`.

## 2026-06-26 — Phase 7 Lot 4 : Page d'erreur serving /c — LIVRÉE (Phase 7 ✅ COMPLÈTE)

### Dernière chose faite
**Phase 7 Lot 4 — page d'erreur stylée serving `/c` — clôturée et vérifiée.** Task 3 : gate complète + mémoire projet.

**Gate finale (repo root)** :
- `cargo fmt --all --check` : ✅ OK
- `cargo clippy --all-targets --all-features -- -D warnings` : ✅ **No issues found**
- `cargo nextest run` : ✅ **137/137 tests passed** (13 binaries, +1 test)
- `cd frontend && rtk lint` : ✅ **No issues found**
- `pnpm typecheck` : ✅ OK
- `rtk vitest run --coverage` : ✅ **87/87 tests passed** ; couverture error-page **≥ 80%** (new-code total OK)
- `pnpm build` : ✅ **build OK** ; `dist/error.html` **présent** (607 B)
- Bundle isolation vérifiée : ✅ `grep -rl "ProjectForm\|deploy_token\|use-projects" dist/assets/*.js | grep -i error || echo "OK"` → **OK: bundle error sans code admin**

**Livré** :
- 3ᵉ entrée Vite `error.html` (3 fichiers : `src/error/{main,error-page,i18n}.tsx`)
- `locales/error/{en,fr}.json` auto-découverts (2 clés : `title`, `message`)
- `web::error_index()` → `PathBuf` vers `dist/error.html`
- `serve.rs::serve_error_page(status)` : lit HTML, renvoie HTML + `no-store` + status code, fallback texte inline si manque
- 5 branches Err terminales → `Ok(serve_error_page(...))` (404 slug inconnu, 404 pas de version, 500 DB/storage/version manquante)
- Logs `tracing::error!` sur 500 (observabilité backend)
- Page générique (zéro injection, pas de leak d'existence de slug)
- `fake_dist()` écrit `error.html` ; test fallback inline quand absent

**Mémoire projet mise à jour (6 fichiers)** :
- `docs/CONVENTIONS.md` : section « Page d'erreur serving /c » (pattern 3ᵉ entrée Vite, serve_error_page, fallback)
- `docs/QUIRKS.md` : entrée « fake_dist écrit unlock.html ET error.html » (fake_dist pose les 2 ; fallback testé)
- `docs/BACKLOG.md` : item Phase 4 « Erreur opaque + sans log » marqué ~~RÉSOLU (Phase 7 Lot 4)~~
- `docs/INDEX.md` : ligne Phase 7 Lot 4 (8 items, gate + isolation)
- `docs/ROADMAP.md` : **Phase 7 ✅ LIVRÉE (2026-06-26)** — 4 lots complets (Lot 1/2/3/4)
- `docs/HANDOFF.md` : cette entrée

### Trucs en suspens
- **Merge Lots 1+2+3+4 groupé** → piste CI séparée recommandée ; 4 branches Phase 7 à fusionner (feat/phase-7-lot-1-fondations, feat/phase-7-lot-2-settings, feat/phase-7-lot-3-identite, feat/phase-7-lot-4-error-page sur feat/phase-7-lot-1-fondations par ordre dépendance)

### Prochaine chose à creuser
- **Phase 8 — Documentation publique (Fumadocs)** : landing + doc détaillée, GitHub Pages, stub dans ROADMAP

### Notes pour future Claude
- **3ᵉ entrée Vite** : pattern réutilisable (2ᵉ = unlock, 3ᵉ = error) pour toute surface public à typage réactif. Procédé : dossier `src/<name>/`, globs locales JSON auto-découverte, entrée vite.config.ts `build.rollupOptions.input`.
- **serve_error_page** : lire HTML, renvoyer Response + status (ne pas utiliser loco_rs::Error pour les pages publiques).
- **Bundle isolation** : 2 globs Vite distincts (admin/error) garantissent aucun code admin dans error.js.
- **Phase 7 fusionnée** : 4 lots, tous verts, prête pour merge → production.

---

## 2026-06-26 — Phase 7 Lot 3 : Identité visuelle (Logo, titres, largeur, GitHub, favicon SVG) — LIVRÉE

### Dernière chose faite
**Phase 7 Lot 3 complète et vérifiée.** Passe 1 via implémentation Tasks 1-5 (spec/plan/exécution), validation complète en Task 6 :
- Logo : composant `components/logo.tsx` (SVG immutualisé), topbar badge+texte (size-6), login/unlock (size-12)
- Favicon : SVG-only `/src/assets/latch-logo.svg` → bundlé `/assets/<hash>.svg` par Vite dans les deux HTML (index.html + unlock.html)
- Titres de page : hook `hooks/use-document-title.ts` appelé par route, schéma « Page — latch admin », clés i18n `title.*`
- Liens externes : centralisés `lib/links.ts` (`GITHUB_URL`, `DOCS_URL`), rendus `Button asChild` avec `<a target="_blank" rel="noopener noreferrer">`
- Largeur admin : `max-w-6xl` sur list/detail main (layout conforme)
- Bouton GitHub : présent login (`lib/links.ts` GITHUB_URL)
- Bouton « ? » doc topbar : présent (`lib/links.ts` DOCS_URL)
- Icône GitHub : inline SVG composant `components/github-icon.tsx` (lucide-react 1.21.0 n'a pas `Github`)
- Purge scaffold : vite.svg + react.svg retirés, favicon bundlé confirmé

**Vérification finale depuis `frontend/` — gate complète :**
- `rtk lint` : **0 erreur**
- `pnpm typecheck` : **0 erreur**
- `rtk vitest run --coverage` : **85/85 tests verts**, couverture Logo/useDocumentTitle/GithubIcon **100% new-code**
- `pnpm build` : **build OK**, favicon `/assets/latch-logo-<hash>.svg` dans dist/index.html ET dist/unlock.html
- `ls public/vite.svg src/assets/react.svg 2>&1 | grep "no such\|cannot"` → **OK: scaffold purgé**

**Mémoire projet mise à jour (4 fichiers) :**
- `docs/CONVENTIONS.md` : section Logo/titres/links/favicon + pattern /assets bundling
- `docs/QUIRKS.md` : entrée favicon /assets + note lucide-react 1.21.0 sans brand icons
- `docs/INDEX.md` : ligne Phase 7 Lot 3 (Logo/titres/largeur/GitHub/bouton doc/favicon SVG/purge)
- `docs/HANDOFF.md` : cette entrée + note button.tsx asChild fix (suite Task 1-5)

### Trucs en suspens
- **Lot 4** : page d'erreur stylée `/c` + fusionnelle Lots 1–3 → deploy
- **Note production** : lien doc pointe URL Phase 8 pas encore en ligne (BACKLOG)

### Prochaine chose à creuser
**Phase 7 Lot 4** : page d'erreur serving `/c` + merge Lots 1+2+3 groupé

### Notes pour future Claude
- **Favicon via /assets** : SVG-only assumé (past l'outil interne noindex). Référencer via `/src/assets/latch-logo.svg` dans HTML → Vite rewrites `/assets/<hash>.svg` servi par backend mount.
- **Logo mutualisé** : `components/logo.tsx` réutilisé partout (taille CSS variable `size-*`). Pas de duplication SVG.
- **lib/links.ts** : source unique d'URLs externes (GITHUB_URL, DOCS_URL) → appel par composants/tests centralisé.
- **button.tsx fix** : `asChild` prop passait `false` à Slot.Root au lieu de `true` — suite Task 1, corrigé, test régression verte.

---

## 2026-06-25 — Phase 7 Lot 2 : Panneau Settings unifié + Select + LanguageSelect + ThemeToggle — LIVRÉE

### Dernière chose faite
**Phase 7 Lot 2 clôturée et vérifiée.** Implémentation complète en 6 tâches :
- `components/ui/select.tsx` : wrapper Select radix (style shadcn identique à `ui/sheet.tsx`)
- `components/language-select.tsx` : Select + CSS flag-icons (import isolé = bundle unlock clean), locales-driven auto-découverte
- `components/theme-toggle.tsx` : Segmented 3-state (system/light/dark), lecture `theme` context
- `components/settings-sheet.tsx` : MCP section + Preferences, helper text par champ, `useSettings(open)` lazy fetch
- Topbar rewired : icône Settings ouvre le Sheet ; route `/settings` supprimée ; effacement `routes/settings.tsx`/`settings.test.tsx` ; `test/utils.tsx` nettoyé
- i18n : ~12 nouvelles clés `settings.*` (EN+FR) ; jsdom shims radix Select (scrollIntoView, hasPointerCapture, releasePointerCapture)

**Vérification finale complète depuis `frontend/` :**
- `rtk lint` : **0 erreur**
- `pnpm typecheck` : **0 erreur**
- `rtk vitest run --coverage` : **76/76 tests verts**, couverture new-code (language-select, theme-toggle, settings-sheet) **≥ 80%**
- `pnpm build` : **build OK**, 2 entrées (`main`, `unlock`) ; assets isolés `/assets`

**Isolation bundle public vérifiée :**
- `grep -rl "flag-icons\|fi-gb\|section_preferences" dist/assets/ | grep -i unlock || echo "OK"` → **OK: unlock bundle sans flag-icons ni settings.***
- flag-icons CSS **présente dans main-*.css** (admin bundle, sanity vérifiée)

**Mémoire projet mise à jour :**
- `docs/CONVENTIONS.md` + section Select+helper-text (pattern radix, CSS isolée par composant)
- `docs/QUIRKS.md` + entrée radix Select jsdom (shims pour Radix)
- `docs/INDEX.md` + ligne Phase 7 Lot 2 livrée (9 items, gate + isolation)
- `docs/HANDOFF.md` (cette entrée)

### Trucs en suspens
- **Lot 3** : logo/titres de page/largeur layout (TBD priorité produit)
- **Lot 4** : page d'erreur stylée `/c`
- **Fusion Lot 1+2** : prévoir un merge groupé au moment du passage en prod (piste CI séparée recommandée)

### Prochaine chose à creuser
**Phase 7 Lot 3** : peaufinage visuel (logo projet, titres de page, ajustements largeur, GitHub links, etc.)

### Notes pour future Claude
- **Select radix vendorisé** : `import { Select as SelectPrimitive } from "radix-ui"` (même pattern que Sheet). Pattern câblage : `flex flex-col gap-1.5` → label + contrôle + helper text `text-muted-foreground text-xs`.
- **flag-icons CSS isolation** : n'importer que dans le composant qui l'utilise (`language-select.tsx`), pas dans `index.css` partagé. Bundle unlock n'aura pas `flag-icons` grâce aux 2 globs Vite distincts (admin/unlock).
- **Gate couverte** : new-code ≥ 80% Sonar = bloquante. Lot 2 passe (new components ≥ 80%).

---

## 2026-06-25 — Phase 7 Lot 1 : Fondations i18n/thème — LIVRÉE

### Dernière chose faite
**Phase 7 Lot 1 clôturée et prête à merge.** Vérification finale complète de **toutes les gates** depuis `frontend/` :
- `rtk lint` : **0 erreur**
- `pnpm typecheck` : **0 erreur**
- `rtk vitest run --coverage` : **69/69 tests verts**, couverture `src/i18n/available-locales.ts` = **100%**, `src/i18n` = **100%**, new-code total = **79.82%** (porte → 80% SonarCloud) ; **unlock-CxjyuO2k.js n'a pas les clés admin** (vérification `grep "common.new_project" dist/assets/unlock-*.js` → 0 matches) ; **anti-FOUC script dans index.html (1 occurrence), absent de unlock.html** ✓
- `pnpm build` : **build OK en 205ms**, 2 bundles distincts générés (`main`, `unlock`), assets isolés en `/assets` (~327 kB main-admin, ~14 kB unlock-public)

**Mémoire projet mise à jour :**
- `docs/CONVENTIONS.md` + section i18n (pattern glob+_meta auto-découverte)
- `docs/QUIRKS.md` + 2 entrées (import.meta.glob Vitest, anti-FOUC CSR)
- `docs/INDEX.md` + ligne Phase 7 Lot 1 livrée (8 items)
- `docs/HANDOFF.md` (cette entrée) + docs spec/plan : corrections 108→**106 clés** (5 références dans les deux docs)

**Commit prêt :** message brief (docs seulement, aucun code changé) ; tous les fichiers sont des `.md`.

### Trucs en suspens
- **Toggle thème** (Lot 2) : provider monté, défaut `system`, persistance active ; le contrôle UI (bouton) = Lot 2.
- **Vrai sélecteur langue** (Lot 2) : `locales` exporté prêt, sélecteur déclaratif à implémenter (Lot 2).
- **Unlock clair-only** : toujours valide, bundle unlock isolé garanti par 2 globs distincts Vite.
- **Context7 non connecté** : les APIs i18next / next-themes / react-i18next utilisées sans vérif formelle ; pas de régression identifiée.

**Review minors depuis branche (T3/T4)** : T3 premier test unlock (`parseLocales` sync vs async init = benign) ; T4 anti-FOUC script placé près du top de `<head>` (safe, bloquant avant render).

### Prochaine chose à creuser
**Phase 7 Lot 2** : toggle thème UI (bouton clair/sombre/système), vrai sélecteur langue (Select + flags), `setTheme()`/`changeLanguage()` câblés en composants respectifs.

### Notes pour future Claude
- **Pattern glob+_meta** : clé réutilisable (Lot 2 l10n flags UI, Lot 3+ autres catalogues) → documenté CONVENTIONS.
- **Couverture parseLocales** : 100% unitaire. Admin i18n / unlock i18n / theme provider = intégrés, pas de tests unitaires sinon (trivial relay).

---

## 2026-06-25 — Phase 6 : revue finale opus + polish serverInfo + MERGE main

### Dernière chose faite
**Phase 6 mergée sur `main`.** Après la clôture T8 (entrée ci-dessous) :
- **Revue finale de branche (opus)** sur `main..feat/phase-6-finalisation` (15 commits) : **Ready to merge = YES**, 0 Critical / 0 Important. Vérifié : invariants §9 au transport MCP réel (gate token AVANT write path → `versions == 0` ; aucune fuite PIN/hash) ; layer `X-Robots-Tag` englobe toutes les surfaces ; **les 2 captures PNG rendues visuellement = aucun nom client, PIN non affiché** ; mémoire cohérente.
- **Polish post-revue (reco opus)** commit `c943853` : `get_info()` annonce désormais `serverInfo.name = "latch"` (au lieu du défaut `rmcp`) via `with_server_info(Implementation::new("latch", env!("CARGO_PKG_VERSION")))` ; test `mcp_initialize_handshake` renforcé (`assert serverInfo.name == "latch"`) ; entrée BACKLOG marquée RÉSOLU ; QUIRKS à jour. 6/6 mcp_http, 136/136 nextest, clippy `--all-features` propre.
- **Merge `feat/phase-6-finalisation` → `main`** (fast-forward) + push origin. CI surveillée (gate SonarCloud `new_coverage ≥ 80%` + build/push image GHCR).

### Trucs en suspens
- **CI sur `main`** : 1ʳᵉ exécution post-merge Phase 6 — vérifier gate Sonar verte + image GHCR publiée.
- **Scan Sonar local** non lancé en T8 (optionnel) — la CI fait foi.
- `deploy.sh` testé sur la box réelle = responsabilité humaine.
- Lien Phase 8 (Fumadocs) dans le README = TBD assumé.

### Prochaine chose à creuser
- **Phase 7** (peaufinage : titres de page, logo, Settings en side-panel, page d'erreur stylée `/c`, thème) **ou Phase 8** (Fumadocs : landing + doc détaillée, cible du lien TBD du README) selon priorité produit.
- Éventuel tag `v0.1.0` (le CHANGELOG est prêt) si on veut figer une release.

### Notes pour future Claude
- **Subagents dispatchés** : leur passer une consigne explicite « tu es dispatché, IGNORE le protocole load-memory du CLAUDE.md, ne renvoie pas "Mémoire chargée" » — sinon le `## Protocole obligatoire` du CLAUDE.md les détourne (vécu en T7, relancé via SendMessage).
- **e2e MCP transport** : `Host: localhost:5150` requis sur les POST `/mcp` (le harness loco envoie `localhost` nu → rejet allowlist rmcp `host_authority`) ; résultat tool dans `result.structuredContent` ; header session `mcp-session-id` ; SSE keepalive = ligne `data:` vide à sauter. Tout dans QUIRKS.

---

## 2026-06-25 — Phase 6 LIVRÉE (T8 : vérification finale + mémoire)

### Dernière chose faite
**Phase 6 complète et clôturée.** Vérification finale de toutes les gates :
- `cargo fmt --all --check` : OK
- `cargo clippy --all-targets --all-features -- -D warnings` : 0 issue
- `cargo nextest run` : **136/136 verts** (13 binaires)
- `pnpm lint + pnpm typecheck + pnpm test` : 0 erreur, **54/54 Vitest verts**
- `pnpm exec playwright test` : **4 passés, 2 skippés** (captures skippées sans CAPTURE=1, normal)

Revue `deploy.sh` : `set -euo pipefail` présent, `docker compose pull/up -d/image prune -f` présents,
garde `chown 65532:65532 data` idempotente (best-effort avec message d'aide si droits insuffisants),
aucun secret en dur.

**Réconciliations T8 effectuées :**
- INDEX.md : "5 tests" → "6 tests" MCP + "135/135" → "136/136" + description complète des 6 tests
- INDEX.md : CAPTURE condition précisée ("skip sauf `CAPTURE=1`" ; `CI=1` = `reuseExistingServer`, indépendant)
- `.env.example` : commentaire `SESSION_SECRET` aligné avec `UNLOCK_COOKIE_SECRET` (≥ 64 octets, fail-secure)
- ROADMAP.md : Phase 6 LIVRÉE + stub Phase 8 (Fumadocs) ajouté
- ENVIRONMENT.md : toolchain git-cliff, CAPTURE=1 vs CI=1, badges Sonar visibilité publique
- CONVENTIONS.md : pattern e2e MCP transport HTTP + pattern durcissement X-Robots-Tag global
- QUIRKS.md : CAPTURE=1 ≠ CI=1 (rôles distincts)
- BACKLOG.md : git-cliff en CI (release automatisée)
- INDEX.md : Phase 6 ajoutée dans "Phases closes"

### Trucs en suspens
- **Revue finale de branche** (`main..feat/phase-6-finalisation`) à passer avant merge/PR (via `finishing-a-development-branch`). Ne pas merger sans revue verte.
- **Scan Sonar local** (optionnel) : `ENVIRONMENT.md §Scan local` pour vérifier la gate `new_coverage ≥ 80%` avant push si des doutes subsistent.
- `deploy.sh` testé sur la box réelle = responsabilité humaine (hors CI par construction).
- Lien Phase 8 (Fumadocs) dans le README = TBD (indiqué comme tel, non bloquant).

### Prochaine chose à creuser
- **Revue de branche** (opus / `superpowers:finishing-a-development-branch`) puis merge/PR de `feat/phase-6-finalisation` sur `main`.
- **Phase 7** (peaufinage graphique : titres de page, logo, menu Settings side-panel, pages d'erreur `/c` stylées) selon priorité produit.
- **Phase 8** (Fumadocs / GitHub Pages) : landing + doc détaillée.

### Notes pour future Claude
- CAPTURE=1 et CI=1 sont indépendants : CAPTURE contrôle le skip, CI active reuseExistingServer. Ne jamais documenter comme indissociables.
- `serverInfo.name = "rmcp"` (pas "latch") est un comportement connu de rmcp 1.8 (`from_build_env()`). Ne pas asserter ce champ dans les tests. Fix BACKLOG.
- La branche `feat/phase-6-finalisation` part de `main` (après Phase 5 mergée). Les 136 tests nextest incluent les 6 tests MCP HTTP réels (transport Streamable HTTP).
- git-cliff installé localement via `cargo install git-cliff`. Cliff.toml à la racine. Régénérer : `git cliff --output CHANGELOG.md`.

---

## 2026-06-25 — Phase 6 T5 : captures Playwright (hors CI) — DONE

### Dernière chose faite
`frontend/e2e/screenshots.capture.ts` créé — 2 tests de capture conditionnels :
- `capture liste admin` : crée 2 projets fictifs ("Mon Projet" protégé + "ACME" libre)
  via API, navigue vers `/admin`, prend `docs/assets/admin-list.png`.
- `capture page unlock` : crée le projet protégé, navigue vers `/c/<slug>`, attend
  `#pin`, prend `docs/assets/unlock.png`.

Skippés par défaut via `test.skip(!process.env.CAPTURE, …)`. Actifs avec `CAPTURE=1 CI=1`.

`playwright.config.ts` mis à jour : `testMatch: /.*\.(spec|capture)\.ts$/` pour
que les fichiers `.capture.ts` soient découverts (par défaut Playwright ne cherche que
`*.spec.ts` — cf. QUIRKS).

PNGs générés et inspectés visuellement : rendu correct, données 100 % fictives.
Skip sans `CAPTURE` confirmé : 2 skipped. Commit `fb1379b`.

### Trucs en suspens
- **T6** : CHANGELOG git-cliff
- **T7** : README refonte
- **T8** : vérif finale + mémoire

### Prochaine chose à creuser
- **T6** : CHANGELOG (`git-cliff`, cf. brief `task-6-brief.md`)

### Notes pour future Claude
- Playwright `testMatch` par défaut : `*.spec.ts` uniquement. Pour découvrir
  `.capture.ts`, il faut ajouter `testMatch: /.*\.(spec|capture)\.ts$/` dans la
  config — sinon `No tests found` silencieux.
- Régénérer les captures : `cd frontend && CAPTURE=1 CI=1 pnpm exec playwright test screenshots.capture`

---

## 2026-06-25 — Phase 6 T3 : tests e2e Playwright serving `/c` + unlock + bascule (4/4 verts)

### Dernière chose faite
`frontend/e2e/serve-unlock.spec.ts` créé — 3 tests Playwright navigateur réel sur la surface `/c` :
- `projet libre : /c sert le proto en no-store` : setup API, vérifie status 200 + `Cache-Control: no-store` + contenu "Demo proto"
- `projet protégé : unlock par PIN puis proto servi` : sans cookie → page unlock visible ; mauvais PIN → reste sur unlock ; bon PIN (135790) → auto-submit OTP via `pressSequentially` → proto servi
- `bascule de version : /c reflète la v2 activée` : deploy v1 → vérifie v1 ; deploy v2 → vérifie v2, v1 absente

Setup entièrement API-driven (login + create + deploy via `request` fixture, `Origin: baseURL` sur les mutations). PIN explicite à la création = déterministe. Fixture `proto-v2.html` créée (titre "Prototype v2" — sans "Demo proto" pour éviter les faux positifs sur `not.toContain`).

**Résultats :** 4/4 e2e verts (1 smoke admin + 3 serve-unlock). OTP auto-submit via `pressSequentially` fonctionnel. `pnpm lint` + `pnpm typecheck` propres. Commit `59a694e`.

### Trucs en suspens
- **T4** : sonar.tests + supply-chain audit (cargo-deny, CHANGELOG)
- **T5** : captures Playwright (screenshots en CI)
- **T6** : CHANGELOG git-cliff
- **T7** : README refonte
- **T8** : vérif finale + mémoire

### Prochaine chose à creuser
- **T4** : supply-chain audit (`cargo deny check`, `cargo audit`) + ajustement `sonar.tests` si souhaité

### Notes pour future Claude
- Piège proto-v2.html : le titre ne doit pas contenir le marqueur de v1 ("Demo proto") sinon l'assertion `not.toContain` échoue même quand v2 est bien servie. Titre neutre : "Prototype v2".
- OTP auto-submit : `pressSequentially` suffit — pas besoin du fallback `fill` + click bouton.
- La fixture `proto.html` ne doit PAS être modifiée (son marqueur "Demo proto" est utilisé par le smoke admin ET le serve-unlock spec).

---

## 2026-06-25 — Phase 6 T2 : tests e2e MCP transport HTTP réel (5 tests verts)

### Dernière chose faite
`backend/tests/mcp_http.rs` créé — 5 tests e2e du transport Streamable HTTP réel via le harness loco `request::<App>` :
- `mcp_initialize_handshake` : handshake + header `mcp-session-id` + instructions "latch"
- `mcp_tools_list_exposes_two_tools` : 2 tools exposés (`deploy_prototype`, `list_projects`)
- `mcp_deploy_prototype_creates_version` : deploy + structuredContent + invariant §9 (PIN absent)
- `mcp_list_projects_is_object_envelope` : enveloppe `{projects:[...]}` confirmée
- `mcp_bad_token_is_rejected` : gate token invalide → `isError` ou erreur JSON-RPC

**Incertitudes résolues** : résultat dans `structuredContent` (pas `content[0].text`) ; header session = `mcp-session-id` ; `protocolVersion = "2025-06-18"` accepté ; SSE commence par `data:` vide (keepalive) puis `data: {json}` ; `Host: localhost:5150` requis dans les requêtes (sinon 403 `allowed_hosts`) ; `serverInfo.name = "rmcp"` (rmcp lui-même, pas notre crate).

`axum-test = { version = "17.3" }` ajouté en dev-dep directe (requis pour typer `axum_test::TestServer` dans le helper). 135/135 tests backend verts. `cargo fmt` + `cargo clippy --all-features -- -D warnings` propres.

### Trucs en suspens
- **T3** : tests e2e Playwright `/c/<slug>` + unlock + bascule (priorité Phase 6)
- **T4** : sonar.tests + supply-chain audit
- Reste de la Phase 6 : CHANGELOG, README, vérif finale

### Prochaine chose à creuser
- **T3** : tests e2e Playwright (voir brief `task-3-brief.md`)

### Notes pour future Claude
- Tests e2e MCP : 4 pièges QUIRKS ajoutés (Host header, SSE `data:` vide, `serverInfo.name="rmcp"`, `axum_test::TestServer` non réexporté)
- Pattern test MCP : `setup_env()` pose `DEPLOY_TOKEN`, `LATCH_PUBLIC_BASE_URL`, `LATCH_STORAGE_ROOT` ; chaque requête porte `host: localhost:5150` ; `parse_mcp_body` ignore les lignes `data:` vides ; session capturée du header `mcp-session-id` de la réponse `initialize`

---

## 2026-06-25 — Phase 5 LIVRÉE : endpoint MCP + panneau Settings

### Dernière chose faite
**Phase 5 complète** (endpoint MCP + panneau Settings) livrée et validée. Récapitulatif :

- **Helpers `web/mod.rs`** : `deploy_token(ctx)`, `public_base_url(ctx)` (trailing-slash normalisé), `host_authority(base)` — fail-secure ; helper privé `resolve_required` (rejette chaîne vide).
- **Adaptateur MCP** (`backend/src/mcp/mod.rs`) : `LatchMcp` + `#[tool_router]`/`#[tool_handler]`/`ServerHandler`, monté via `nest_service("/mcp", StreamableHttpService)` + `LocalSessionManager` dans `app.rs::after_routes`. `rmcp` épinglé `"1.4"` (floor CVE-2026-42559), résout **1.8.0**. `allowed_hosts` = `web::host_authority(public_base_url)`.
- **Tools** : `deploy_prototype` (gate token FIRST, slug préexistant, `activate` défaut `true`, retourne `DeployResult { url, version, code_protected }`) ; `list_projects` (gate FIRST, retourne **enveloppe objet** `{ projects: [...] }` — pas de tableau racine, cf. quirk rmcp 1.8).
- **`LATCH_PUBLIC_BASE_URL`** : nouvelle variable runtime fail-secure (source hôte public → `allowed_hosts` + `url` deploy_prototype). **`DEPLOY_TOKEN`** aussi fail-secure au boot.
- **`GET /api/settings`** (AdminAuth) : `{ deploy_token, mcp_url, public_base_url }`, enregistré dans `openapi.rs`, `openapi.json` + `schema.d.ts` régénérés.
- **Frontend** : `hooks/use-settings.ts`, `routes/settings.tsx` (topbar icon, PinField pour deploy_token, CopyButton pour mcp_url, public_base_url texte, loading/error), route `/settings`, i18n `settings.*` EN+FR.
- **Tests** : 127 backend (gate token, deploy_prototype crée version, slug inconnu → erreur, invariants sécu, settings 401), 54 frontend. Clippy `--all-features` clean. Cargo-deny OK.
- **SonarCloud gate PASSED** sur la branche : new_coverage ~94.8%, ratings A.
- **Mémoire mise à jour** (Task 8) : contrat §5/§9, ROADMAP, INDEX, ENVIRONMENT, QUIRKS, CONVENTIONS, BOOTSTRAP, .env.example, CLAUDE.md.

### Trucs en suspens
- **Branchement réel Claude web à confirmer** : l'endpoint MCP a été testé via tests handler-level (gate token, déploiement) mais le branchement au cloud Anthropic (Claude web → connecteur MCP → `/mcp`) reste à valider au 1er test prod. Déduit de la doc rmcp (cf. ROADMAP §5). Procédure dans ENVIRONMENT §Connexion connecteur MCP.
- **e2e `/mcp` via transport HTTP** reporté Phase 6 (les tests actuels sont handler-level, sans serveur HTTP).
- **Phase 7 (locale/thème)** : le panneau Settings Phase 5 affiche deploy_token/mcp_url/public_base_url — locale et thème restent Phase 7 (cf. ROADMAP).
- **À TRANCHER À LA FINALISATION — `sonar.tests` & tests Rust** : `sonar-project.properties` ne déclare que `sonar.tests=frontend/src`. Conséquence (vérifiée API) : (a) les tests Rust **unitaires** sont inline `#[cfg(test)]` dans `backend/src/*.rs` → comptés comme du **code de prod** par Sonar (impossible à isoler — granularité fichier) ; c'est pourquoi des bras `panic!` de test jamais exécutés rognent la couverture ; (b) les tests d'**intégration** `backend/tests/*.rs` ne sont ni dans `sources` ni dans `tests` → **totalement ignorés** par Sonar. La **couverture** Rust, elle, est bien prise en compte (canal séparé `sonar.rust.lcov.reportPaths`, indépendant de `sonar.tests` — gate verte, global 91.1 %). Amélioration **cosmétique** possible à la finalisation : ajouter `backend/tests` à `sonar.tests` (`sonar.tests=frontend/src,backend/tests`) pour que les tests d'intégration soient *classés comme tests* — aucun impact couverture, ne corrige PAS le cas inline. Décision reportée (gate verte en l'état).

### Prochaine chose à creuser
- **Phase 6** (e2e, durcissement, packaging publiable) : flux Playwright `/c/<slug>` complet + tests e2e MCP + README + CHANGELOG + cargo-deny audit.
- **OU Phase 7** (titres de page, logo, menu Settings complet avec locale+thème) selon priorité produit.

### Notes pour future Claude
- **rmcp 1.8 quirks** (3 pièges, tous documentés dans QUIRKS.md) :
  1. `ServerInfo` est `#[non_exhaustive]` → `::default()` + fields + `.with_instructions()`.
  2. Tool output de type `array` racine → panique au boot → toujours envelopper dans un struct objet.
  3. `#[tool]` réécrit en `Pin<Box<dyn Future>>` → directement `await`-able dans les tests handler-level.
- **Scan Sonar local** : `backend-lcov.info` a des chemins absolus (`/srv/owlnext/latch/…`) → le container `sonar-scanner-cli` (qui monte sous `/usr/src`) les ignore silencieusement → faux échec de gate. **Fix** : `sed -i "s#$(pwd)/#/usr/src/#g" backend-lcov.info` avant le scan. CI n'a pas ce problème. Détail dans QUIRKS + ENVIRONMENT.
- **Règle 80% new-coverage** : gate bloquante CI, consignée dans CLAUDE.md + BOOTSTRAP §5. Tests substantiels requis (branches + cas d'erreur + invariants).

---

## 2026-06-25 — Chantier toolchain & CI LIVRÉ (branche `chore/toolchain-ci-hardening`)

### Dernière chose faite
Chantier complet **durcissement toolchain & CI** livré sur `chore/toolchain-ci-hardening` (T1→T8c + T9). Récapitulatif :
- **Remédiation 64 issues Sonar front** (T1–T3) : 22 `void` S3735 retirés, props `Readonly` (S6759) + `globalThis` (S7764), 4 ternaires (S3358) + FormEvent (S1874) + fieldset (S6819) + condition positive (S7735) + assertion (S5906).
- **Couverture Vitest lcov** (T4) : `@vitest/coverage-v8`, bloc `coverage`, script `test:cov`, lcov.info → SonarCloud.
- **Dockerfile cargo-chef + runtime non-root** (T5) : stage `cook` (couche deps cachée), `gcr.io/distroless/cc-debian12:nonroot` (uid 65532), stage `dataprep`, `--locked ×2`, `--ignore-scripts`, rust:1.96 épinglé.
- **CI confort** (T6) : 15 uses SHA-pinned, `--ignore-scripts ×3`, `concurrency cancel-in-progress`, cache Playwright, `clippy --all-features`.
- **Lints Rust no-unwrap** (T7) : `[workspace.lints.clippy] unwrap_used/expect_used=warn`, `[lints] workspace=true` ×2 crates, `#[allow]` groupés (tests), `fingerprint` refactoré en `unwrap_or_else(unreachable!)`.
- **SonarQube Cloud bloquant** (T8c) : `sonar-project.properties` (backend/src, lcov paths, clippy=false), job `sonar` CI (gate PASSED, front+IaC+couv Rust), `cargo-llvm-cov nextest` → `backend-lcov.info` (artefact CI), 0 issue Rust, couv globale 77.2%.
- **Vérif finale** (T9) : `cargo fmt` OK, clippy 0 warning, nextest 113/113, Vitest 52/52, lint/typecheck/build front verts. `eslint.config.js` : `coverage/` ajouté aux ignores. `deploy.sh` : garde `chown 65532:65532 data` (best-effort idempotente).
- **Revue finale de branche (opus) : « Ready to merge: Yes »** (0 Critical, 0 Important bloquant). Polish appliqué (commit `251d260`) : message d'aide explicite si le `chown ./data` échoue (au lieu d'un échec silencieux) + commentaire `dataprep` clarifié (s'applique aux volumes nommés ; le bind-mount le shadow).

### Trucs en suspens
- **Mergé sur `main` (ff `a42bd03..c32010f`) + CI VERTE** : run [28175334921](https://github.com/owlnext-fr/latch/actions/runs/28175334921), **8/8 jobs success** (dont `SonarQube` gate bloquant, flux artefact lcov Rust `test-backend`→`sonar`, `docker build` cargo-chef+non-root → GHCR). Chantier validé end-to-end.
- **Annotation non bloquante** : plusieurs actions épinglées (SHA v4 de checkout/setup-node/pnpm/action-setup/cache/download-artifact/docker-*) ciblent **Node 20 déprécié** (forcées sur Node 24). À traiter par un bump des majors (checkout v5, setup-node v5…) → cf. BACKLOG. Avertissement, pas une erreur.
- **2 issues Sonar non-bloquantes à clôturer en UI** (won't-fix) :
  - `typescript:S1874` ×2 (`deploy-panel.tsx:2,71`) — `FormEvent` déprécié @types/react 19 ; fix T3 (imports nommés) ne l'éteint pas → won't-fix UI.
  - `githubactions:S6505` (`ci.yml:132` `playwright install`) — faux positif (install navigateurs, pas paquets npm) → won't-fix UI.

### Prochaine chose à creuser
- **Phase 5 — Endpoint MCP** : `mcp/`, `rmcp ≥ 1.4.0`, `allowed_hosts`, `deploy_prototype` + `list_projects`, token validé sur tous les tools.

### Notes pour future Claude
- `sonar.rust.clippy.enabled=false` est OBLIGATOIRE dans `sonar-project.properties` : sans ça le scanner tente de lancer `cargo` (absent dans le container sonar-scanner-cli) → erreur. Clippy reste bloquant dans le job `fmt-clippy`.
- La gate Sonar new-code à 80% est sur le **new-code uniquement** (pas la couverture totale). Ne pas confondre avec `--fail-under` de `cargo-llvm-cov` (non utilisé).
- `cargo llvm-cov nextest` exige le component `llvm-tools-preview` sur la toolchain Rust ET `taiki-e/install-action@v2` avec `tool: cargo-llvm-cov,nextest` (v1 ne supporte qu'un seul outil).
- SonarCloud : **Automatic Analysis EXCLUSIVE du scanner CI** — les deux ne peuvent pas coexister ; désactiver l'Automatic Analysis dans les settings SonarCloud avant d'activer le job CI.
- `[lints] workspace=true` doit être répété dans **chaque** `Cargo.toml` de crate (backend + backend/migration) — le workspace root seul ne suffit pas.
- Bind-mount `./data` : le `chown` du stage `dataprep` ne s'applique PAS aux bind-mounts existants. `deploy.sh` contient maintenant la garde idempotente.

---

## 2026-06-25 — Toolchain/CI hardening Task 8c : couverture Rust sur SonarQube (commit `197fcec`)

### Dernière chose faite
- **`sonar-project.properties`** : `backend/src` ajouté à `sonar.sources`, `sonar.rust.lcov.reportPaths=backend-lcov.info`, `sonar.rust.clippy.enabled=false` (clippy reste l'autorité lint dans `fmt-clippy`).
- **`.github/workflows/ci.yml`** : job `test-backend` — `llvm-tools-preview` ajouté à `rust-toolchain`, `taiki-e/install-action` passé en **v2** (SHA `ace6ebe`) avec `tool: cargo-llvm-cov,nextest`, `cargo nextest run` remplacé par `cargo llvm-cov nextest --lcov --output-path backend-lcov.info`, upload artefact `backend-lcov` (`actions/upload-artifact@ea165f8  # v4`). Job `sonar` : `needs: [test-backend]` ajouté, download artefact (`actions/download-artifact@d3f86a1  # v4`) avant `pnpm test:cov`.
- **`.gitignore`** : `backend-lcov.info` ajouté.
- **Vérification locale** : gate PASSED, 113 tests Rust verts, 0 issue Rust sur SonarCloud, YAML OK, actionlint propre.

### Trucs en suspens
- CI non encore exécutée sur cette branche pour le job sonar avec le nouveau wiring (à vérifier quand le run CI démarre).
- Task 9 (vérif finale + mémoire + push CI) reste ouverte.

### Prochaine chose à creuser
- **Task 9** : push de la branche `chore/toolchain-ci-hardening` + vérification CI verte + clôture mémoire.

### Notes pour future Claude
- `sonar.rust.clippy.enabled=false` est **obligatoire** : sans ça le scanner tente de lancer `cargo` (absent dans le container sonar-scanner-cli) → erreur. Clippy bloquant reste dans le job `fmt-clippy`.
- `taiki-e/install-action@v2` installe plusieurs outils avec `tool: cargo-llvm-cov,nextest` (virgule-séparé) — le `@nextest` ref de la v1 ne supportait qu'un seul outil.
- `cargo llvm-cov nextest` exige le component `llvm-tools-preview` sur la toolchain Rust — à ajouter via `with: { components: llvm-tools-preview }` sur `dtolnay/rust-toolchain`.

---

## 2026-06-25 — Toolchain/CI hardening Task 5 : Docker cargo-chef + non-root (commit `916f0b8`)

### Dernière chose faite
- **Dockerfile réécrit** : stage Rust en **cargo-chef** (couche deps cachée), runtime **distroless cc-debian12:nonroot** (uid 65532), stage `dataprep` pour `chown 65532 /data`. Rust épinglé `1.96-bookworm`.
- **Durcissements Sonar** : S8549 (`--locked`), S6471 (non-root), S6596 (tags mineurs figés), S6505 (`--ignore-scripts`).
- **Build vérifié** : `DOCKER_BUILDKIT=1 docker build` → OK (~110s cook + ~21s build final).
- **Runtime non-root confirmé** : `docker inspect -f '{{.Config.User}}'` → `nonroot`. Migrations jouées + `latch.sqlite` créé sous uid 65532.
- **Cache cook confirmé** : `touch backend/src/app.rs` + rebuild → `cargo chef cook … CACHED`.
- Commit `916f0b8` sur `chore/toolchain-ci-hardening`.

### Trucs en suspens
- `docker-compose.yml` non modifié : bind-mount `./data:/data`. Un répertoire `./data` préexistant possédé par root nécessitera `chown 65532:65532 ./data` manuel (noté QUIRKS).
- CI non encore exécutée sur cette branche pour le job docker (à vérifier quand le workflow tourne).

### Prochaine chose à creuser
- **Task 6** : CI pin SHA + ignore-scripts + confort (`.github/workflows/`)
- **Task 7** : Rust workspace lints no-unwrap

### Notes pour future Claude
- `cargo chef cook -p latch --locked` fonctionne avec `recipe.json` issu de `cargo chef prepare` — pas de problème `--locked`.
- Le stage `dataprep` (debian-slim) est nécessaire car distroless n'a pas de shell pour `chown`.
- QUIRKS : voir entrée volume `/data` non-root ci-dessous.

---

## 2026-06-25 — PHASE 4 LIVRÉE + itérations UI + revues + Phase 7 planifiée → merge `main`

### Dernière chose faite
- **Phase 4 (serving `/c/<slug>` + déverrouillage) complète**, exécutée en Subagent-Driven (10 tâches,
  1 implémenteur + 1 reviewer chacune). Backend : `services/unlock_cookie.rs` (cœur pur, empreinte HMAC du
  PIN), `controllers/serve.rs` (GET /c arbre de décision, POST /unlock cookie signé `SignedCookieJar`+empreinte,
  GET /api/public/{slug} meta sans PIN), `controllers/serve_ratelimit.rs` (2 layers governor in-memory IP+slug
  & slug-global). Front : page de déverrouillage = **2ᵉ entrée Vite isolée** React+shadcn.
- **Fix sécu HIGH (revue auto)** : `UNLOCK_COOKIE_SECRET` **et** `SESSION_SECRET` étaient fail-OPEN en prod
  (fallback hardcodé si env absente) → corrigé en **fail-secure** (`resolve_cookie_secret`, refus de boot hors
  Dev/Test sans secret explicite). Commit `1b309d8`.
- **Revue finale de branche (opus)** : Ready to merge (fixes appliqués : commentaire rate-limit `auth.rs`, chemins doc).
- **Itérations UI post-livraison (demande humaine, validées au navigateur)** : InputOTP segmenté (6 cases, collage,
  auto-submit sur `onComplete`) + texte explicatif + **découplage assets `/admin/assets` → `/assets`** (base Vite `/`
  + mount backend ; corrige le couplage public↔/admin) + centrage OTP + **boutons `loading`** (spinner + disabled,
  câblés sur toutes les mutations) + état d'erreur (cases rouges, message centré) + bordure OTP foncée + retrait
  favicon `/vite.svg`. **Revue ciblée du diff d'itération (opus) : Ready to merge: Yes** (assets vérifiés contre
  les artefacts réels, e2e admin vert).
- **Phase 7 ajoutée à la ROADMAP** (peaufinage graphique/web) : titres de page, logo, menu Settings
  (locale + thème system/dark/light via `next-themes` à recâbler), i18n centralisé (détection JSON de locales).
- **Mémoire à jour** : ROADMAP (Phase 4 LIVRÉE + Phase 7), QUIRKS (fail-secure, cookie-signed, in-memory RL,
  elementFromPoint, /assets), CONVENTIONS (adaptateur serve, Button loading, 2ᵉ entrée Vite), INDEX, BACKLOG, ENV.

### Trucs en suspens
- **CI VERTE sur `main`** (run `28164197300`, commit `5dda87c`) — 7/7 jobs OK : fmt+clippy, tests backend,
  front (lint/typecheck/test/build), front supply-chain (audit+licences), **cargo-deny** (nouvelles deps
  `axum-extra`/`hmac`/`sha2`/`hex` OK), **e2e Playwright admin** (valide base Vite `/` + mount `/assets`),
  docker build (GHCR). Plus aucun point de vigilance ouvert.
- e2e complet `/c/<slug>` (déverrouillage) en Playwright = reporté **Phase 6** (le smoke admin couvre l'admin).

### Prochaine chose à creuser
- **Phase 5 — Endpoint MCP** (`mcp/`, `rmcp ≥ 1.4.0`, token sur tous les tools) — prochaine phase métier.
- OU **Phase 7** (peaufinage) selon priorité produit.

### Notes pour future Claude
- Secrets cookie : **fail-secure** désormais — en prod, `UNLOCK_COOKIE_SECRET` (≥ 64 o) ET `SESSION_SECRET` sont
  obligatoires, sinon le boot échoue (volontaire). Dev/Test gardent un fallback déterministe.
- `axum-extra` feature = **`cookie-signed`** (pas `cookie` — n'expose pas `Key`).
- `tower_governor` `per_second(n)` = **période de n secondes** (1/n req/s), pas n req/s (source 0.7.0).
- Rate-limit `/unlock` **in-memory** : compteurs perdus au reboot (limite §9.5 assumée, pas de table).
- Détail tâche-par-tâche + findings : `.superpowers/sdd/progress.md` (gitignoré).

---

## 2026-06-25 — Itération UX : Button loading + OTP auto-submit

### Dernière chose faite
- **`Button` loading réutilisable** : prop `loading?: boolean` ajoutée dans `frontend/src/components/ui/button.tsx`. Quand `true` : spinner `Loader2 animate-spin` injecté avant les children, button `disabled` effectif (`loading || disabled`). `asChild` non affecté (nav links).
- **Câblage sur 7 sites** : `routes/login.tsx`, `components/deploy-panel.tsx`, `components/delete-project-panel.tsx`, `components/delete-version-panel.tsx`, `components/topbar.tsx`, `components/project-form.tsx` (aggregate 4 mutations), `routes/detail.tsx` (activate, per-row via `activateVersion.variables?.n`).
- **Labels stables** : suppression de tous les text-swaps "…ing" — le spinner seul convoit l'état en cours.
- **OTP auto-submit** (`frontend/src/unlock/unlock-page.tsx`) : `doUnlock()` extrait, `onComplete={() => void doUnlock()}` sur `<InputOTP>`, guard `busy` anti-double-fire, `setPin('')` sur 401 (pas sur 429).
- **Tests** : 30 verts (était 29). Test modifié pour auto-submit (6ème chiffre → `reloadPage` sans clic bouton), nouveau test auto-submit sans bouton, assert clear-on-error 401. Lint/typecheck/build propres.

### Trucs en suspens
- Phase 5 MCP toujours la prochaine étape.
- Clés i18n inutilisées (`login.submitting`, `deploy.deploying`, `danger.deleting`) laissées dans le catalogue (inoffensives).

### Prochaine chose à creuser
- **Phase 5 — Endpoint MCP** : `mcp/` (`deploy_prototype` + `list_projects`), `rmcp ≥ 1.4.0`, `allowed_hosts`, token validé sur tous les tools.

### Notes pour future Claude
- `activateVersion.variables` (TanStack Query v5) expose les dernières variables passées à `.mutate()` — permet un spinner par ligne sans état local.
- La prop `loading` sur `Button` ne fait rien quand `asChild=true` (le spinner n'est pas injecté, `disabled` passe le raw prop, pas `isDisabled`) — intentionnel pour les nav links.
- `onComplete` sur `<InputOTP>` se déclenche aussi sur paste d'un code complet — guard `busy` empêche le double-fire.

---

## 2026-06-25 — Itération UI unlock : InputOTP + CardDescription + découplage /assets

### Dernière chose faite
- **InputOTP shadcn** (6 slots, `REGEXP_ONLY_DIGITS`, `id="pin"` forwardé au hidden input) remplace `<Input>` dans `unlock-page.tsx`. Installé via `pnpm dlx shadcn@latest add input-otp` (version `^1.4.2`). Supprimé import `Input`. Désactivation submit strict : `pin.length < 6` (au lieu de `=== 0`).
- **CardDescription** ajoutée dans CardHeader avec clé `unlock.instructions` (EN + FR) dans `i18n.ts`.
- **Base Vite** changée de `/admin/` à `/` dans `vite.config.ts`. Les deux bundles (`main`, `unlock`) référencent désormais `/assets/...`.
- **Mount `/assets`** ajouté dans `backend/src/app.rs` `after_routes` : `ServeDir::new(dist.join("assets"))` monté avant `/admin`.
- **Mock `document.elementFromPoint`** ajouté dans `vitest.setup.ts` (requis par `input-otp` en jsdom — cette API n'existe pas en jsdom).
- 4 tests Vitest unlock verts ; lint/typecheck/build propres ; cargo nextest 113 passed ; cargo clippy clean.
- e2e Playwright : le test admin-smoke ne pouvait pas tourner en local (serveur hérité sans les nouvelles routes `/assets`). En CI, `reuseExistingServer: false` → serveur neuf avec backend recompilé → test passera.

### Trucs en suspens
- e2e local : nécessite de redémarrer le backend pour que le mount `/assets` soit actif (le serveur hérité tourne toujours).
- Phase 5 MCP toujours la prochaine étape.

### Prochaine chose à creuser
- **Phase 5 — Endpoint MCP** : `mcp/` (`deploy_prototype` + `list_projects`), `rmcp ≥ 1.4.0`, `allowed_hosts`, token validé sur tous les tools.

### Notes pour future Claude
- `input-otp` utilise `document.elementFromPoint` pour le positionnement du caret → jsdom ne l'a pas → ajouter le mock dans `vitest.setup.ts`. Pattern déjà présent pour `ResizeObserver`.
- La `base: '/'` Vite implique que le bundle admin SPA est servi par `/admin` (ServeDir sur dist root), et les assets sont disponibles via le nouveau mount `/assets`. Les deux cohabitent sans conflit car Axum cherche d'abord la route `/assets` exacte avant de tomber dans le `ServeDir` admin (nest_service strip le préfixe).

---

## 2026-06-25 — Phase 4 LIVRÉE : serving `/c/<slug>` + déverrouillage

> **Remplace l'entrée provisoire « Task 5 » (fusionnée ici).** Phase 4 complète : Tasks 1-9
> livrées, validées navigateur (Task 9), vérification finale Task 10 ✅.

### Dernière chose faite
- **Phase 4 entière livrée** : `services/unlock_cookie.rs` (cœur pur, `issue_token`/`verify_token`,
  empreinte HMAC du PIN pour révocation par rotation) ; `controllers/serve.rs` (GET /c/{slug}
  arbre de décision 5 branches, POST /c/{slug}/unlock, GET /api/public/{slug}) ;
  `controllers/serve_ratelimit.rs` (deux governor layers via `ServiceBuilder`) ;
  `frontend/src/unlock/` (`main.tsx`/`unlock-page.tsx`/`i18n.ts`/`reload.ts`) + `unlock.html` (2ᵉ entrée Vite, page formulaire PIN).
- Task 10 : `.env.example` corrigé (`UNLOCK_COOKIE_SECRET` ≥ 64 bytes, 5 knobs RL) ;
  `docs/ENVIRONMENT.md` / `QUIRKS.md` / `INDEX.md` / `ROADMAP.md` / `BACKLOG.md` mis à jour.
- Vérification finale : `cargo nextest`, `cargo clippy`, `cargo deny`, `pnpm lint/typecheck/test/build` — tous verts.

### Trucs en suspens
- **e2e Playwright complet** (flux `/c/<slug>` + unlock + cookie) reporté en **Phase 6** (e2e, durcissement, packaging).
- Minors BACKLOG Phase 4 : `unlock.html` `lang` statique ; clarification sémantique `RL_IP_PER_SECOND` ; test isolé plafond slug-global ; erreur opaque `storage.read` dans `serve.rs`.

### Prochaine chose à creuser
- **Phase 5 — Endpoint MCP** : `mcp/` (`deploy_prototype` + `list_projects`), `rmcp ≥ 1.4.0`, `allowed_hosts`, token validé sur tous les tools.

### Notes pour future Claude
- **Cookie unlock** = `SignedCookieJar` (feature **`cookie-signed`** d'`axum-extra`, pas `cookie` seul) + empreinte HMAC du PIN dans la valeur (révocation par rotation de PIN). `Key::from()` exige ≥ 64 bytes. Construire via `SignedCookieJar::from_headers(&headers, key)` (manuellement depuis HeaderMap).
- **Rate-limit in-memory** : compteurs perdus au reboot (assumé §9.5). Deux layers governor montés via `tower::ServiceBuilder` car `.layer().layer()` sur MethodRouter casse l'inférence axum 0.8.9.
- **Fail-secure secrets** : `UNLOCK_COOKIE_SECRET` ET `SESSION_SECRET` refusent le boot en prod si absents/vides (helper `resolve_cookie_secret` hors Dev/Test). Garde en octets, pas chars.
- Le `?` ne peut pas vivre dans une closure `.map()` — utiliser `match` explicite (voir `serve` handler).

---

## 2026-06-25 — Fix CI e2e flaky (bind localhost/IPv6 → 127.0.0.1)

### Dernière chose faite
- **Diagnostic du flake `e2e Playwright (smoke admin)`** (runs FAIL/ok alternés) : le serveur Loco démarrait
  bien (`listening on http://localhost:5150`) ~75 s avant le `Timed out waiting 180000ms from config.webServer`.
  Cause : `development.yaml` avait `binding: localhost` → résolution non déterministe vers `::1` (IPv6) sur les
  runners GitHub, alors que Playwright poll `127.0.0.1/_health` (IPv4) → `ECONNREFUSED` → timeout.
- **Fix** : `binding` rendu réglable par env (`LATCH_BINDING`, défaut `localhost` inchangé pour le dev) via Tera
  dans `backend/config/development.yaml` ; la commande `webServer` de `frontend/playwright.config.ts` exporte
  `LATCH_BINDING=127.0.0.1`. Vérifié local : serveur loge `listening on http://127.0.0.1:5150`, `/_health` → 200,
  `1 passed` (9.6 s).
- Mémoire à jour : `QUIRKS.md` (nouvelle entrée), `ENVIRONMENT.md` (`LATCH_BINDING`).
- **Commité + poussé sur `main`** : `f90eb21`. **CI validée verte** (run `28153192320`) — tous les jobs OK,
  dont `e2e Playwright (smoke admin)` qui était la source du flake.

### Trucs en suspens
- Le flake était probabiliste : surveiller 2-3 runs CI verts d'affilée pour confirmer la disparition complète
  (cause racine éliminée par le bind IPv4 cohérent des deux côtés, donc faible risque).

### Prochaine chose à creuser
- Rien de bloquant. Éventuellement aligner `test.yaml`/`production.yaml` si un besoin de bind explicite apparaît
  (prod bind déjà `0.0.0.0`, OK).

---

## 2026-06-25 — Post-validation : fixes + enrichissement liste + MERGE main + CI

### Dernière chose faite
- **Validé au navigateur par l'humain** (« 100x mieux »). Correctifs livrés après validation :
  - **Bug gros HTML** : deploy d'un proto > 2 Mo → **413** (middleware Loco `limit_payload`, défaut 2 Mo).
    Rendu configurable : env **`LATCH_BODY_LIMIT`** (défaut `5mb`, `disable` possible) dans
    `backend/config/{development,production,test}.yaml` via `get_env`. Test de régression (deploy ~2,5 Mo → 200). Commit `d1087a2`.
  - **PIN via CSPRNG** : `generatePin()` → `crypto.getRandomValues` (hygiène ; vraie barrière = rate-limit `/unlock`, §9.5). Commit `bc2d2dd`.
  - **Vitest scope** : `include: ['src/**']` pour ne plus ramasser les specs Playwright `e2e/*.spec.ts`. Commit `0387724`.
  - **Liste enrichie (résorbe l'item BACKLOG)** : `ProjectListItem` expose **`active_version_n`** (n° réel) +
    **`version_count`** au lieu d'`active_version_id` (PK trompeuse). Service `list_with_versions` (2 requêtes,
    pas de N+1). `openapi.json` + `schema.d.ts` régénérés. Liste affiche « v2 · 3 versions » (pluriel i18next). Commit `797e56b`.
  - **CI** : allowlist licences front calibrée au pré-vol (`OFL-1.1` = police Inter, `MPL-2.0`). Commit `6583fd5`.
- **Pré-vol CI local** (avant push) : cargo-deny `licenses ok, advisories ok` (`Zlib` déjà dans `deny.toml`) ;
  `pnpm audit --audit-level=high` exit 0 (1 modérée only) ; license-checker exit 0 ; drift openapi/schema nul ;
  back 89/89 ; front 25/25 ; e2e Playwright vert.
- **MERGE sur `main`** (fast-forward — `main` était ancêtre, 84 commits) + **push origin** + CI surveillée.

### Trucs en suspens
- **CI sur `main`** : 1ʳᵉ exécution réelle de la CI réécrite (pistes back/front → e2e → docker push GHCR).
  Risque non pré-volable : le job **e2e** démarre le backend via `cargo loco start` dans le webServer Playwright
  (timeout 180 s) — compile CI au cache froid possiblement lent → à surveiller.
- `docs/ENVIRONMENT.md` « Box de déploiement » toujours « à remplir » (déploiement géré par l'humain).

### Notes pour future Claude
- Après un changement DTO/handler : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` (back) **et**
  `cd frontend && pnpm gen:api` (front). Les deux ont un drift-check CI.
- `LATCH_BODY_LIMIT` : protos > 5 Mo (base64) → remonter (`10mb`/`32mb`) ou `disable`.

---

## 2026-06-25 — MIGRATION REACT LIVRÉE (Plans 1-3) — prête pour validation humaine (feat/admin-react)

> Session autonome de nuit. Admin SPA migrée **Yew → React/Vite/shadcn** de bout en bout.
> Détail tâche-par-tâche : `.superpowers/sdd/progress.md` (ledger). Plans :
> `docs/superpowers/plans/2026-06-25-migration-react-plan-{1,2,3}-*.md`. Le backend Rust est inchangé.

### Dernière chose faite — un SERVEUR TOURNE pour ta validation
- **Serveur lancé et joignable** : `http://127.0.0.1:5150/admin` — login `admin` / `secret`.
  (backend `cargo loco start` en dev, sert le `dist/` React buildé sous `/admin` ; DB neuve `/tmp/latch-dev.sqlite`,
  storage `/tmp/latch-dev-data`.) Vérifié : `/_health`=ok, `/admin/` sert le React, `/api/projects` sans session = 401.
  Si le process n'est plus là au réveil, relancer :
  `cd frontend && pnpm build` puis `cd backend && LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret LATCH_STORAGE_ROOT=/tmp/latch-dev-data DATABASE_URL='sqlite:///tmp/latch-dev.sqlite?mode=rwc' cargo loco start`.

### Ce qui est livré (3 plans, tout vert local)
- **Plan 1 (déjà fait avant la nuit)** : backend OpenAPI (utoipa) → `openapi.json` commité + drift test. 88 tests nextest.
- **Plan 2 — app React** (11 commits) : Vite+React+TS strict, TanStack Router (code-based, basepath `/admin`) + Query,
  client **openapi-fetch** typé depuis `openapi.json` (→ `frontend/src/api/schema.d.ts`), shadcn/ui (Radix, base **stone**,
  preset oklch `bJfDPe2y`), Tailwind v4, RHF+zod, react-i18next (FR/EN, défaut EN, catalogue porté), sonner.
  Pages contrat §7 : login, liste (badges colorés, état vide), détail (lecture seule, PIN masqué, versions),
  side-panels ProjectForm (créer/éditer, PIN disabled si code off, slug RO) / DeployPanel (dropzone) / danger.
  **25 tests Vitest+MSW verts**, typecheck/lint(jsx-a11y)/build verts. Revue finale opus : 0 Critical, 4 Important + 2 Minor CORRIGÉS.
- **Plan 3 — pipeline + e2e + docs** : Dockerfile stage **Node 24/pnpm** (Vite) → **image buildée OK 105 Mo** distroless ;
  `ci.yml` réécrit en pistes parallèles back/front → e2e → docker (+ drift OpenAPI & schema, supply-chain front) ;
  **e2e Playwright smoke VERT** (login → créer projet → déployer) contre la stack réelle ; doc mémoire alignée
  (contrat §2/§4, BOOTSTRAP, ROADMAP, ENV, QUIRKS+CONVENTIONS avec archive « Historique Yew », INDEX, BACKLOG, README).

### Trucs en suspens / à trancher (TOI, demain)
- **PUSH + PR non faits** : la branche `feat/admin-react` est **locale** (pas d'upstream). Rien n'est poussé. La CI
  réécrite n'a donc PAS tourné sur GitHub — à valider au 1er push. (Volontaire : tu voulais valider d'abord.)
- **GAP contrat §7 (décision API)** : `ProjectListItem` ne porte que `active_version_id` (PK), pas le n° de version ni le
  compte. La colonne « version active » affiche désormais **« Deployed »/—** (honnête) au lieu d'un faux `v{id}`. Pour
  afficher le vrai n° + compte, **enrichir le DTO backend** (`active_version_n` + `version_count`) → régénérer
  `openapi.json` + `schema.d.ts` + drift. C'est un changement du **contrat OpenAPI (loi)** → à décider/blesser. → BACKLOG.
- **CI license allowlist** (`supply-chain-front`) possiblement à calibrer au 1er run réel (SPDX non listé mais légitime).
- Minors Plan 2 (BACKLOG) : bouton « activer » sans état pending ; bundle JS 604 kB (code-splitting) ; reusable workflows CI ;
  `deny.toml` transitives utoipa-swagger-ui (zlib-rs « Zlib »).

### Notes pour future Claude (quirks React découverts — aussi dans QUIRKS.md)
- **openapi-fetch capture `globalThis.fetch` au load** → `client.ts` passe `fetch:(input)=>globalThis.fetch(input)` pour que MSW intercepte en test.
- **Pinner `packageManager: pnpm@9.15.9`** (`frontend/package.json`) : sinon corepack tire pnpm 11 dont la politique `minimumReleaseAge` rejette le lockfile (Docker/CI rouges).
- **ResizeObserver polyfill** dans `vitest.setup.ts` (Radix en jsdom).
- **shadcn `init --preset bJfDPe2y`** nécessite `npm_config_ignore_workspace_root_check=true` (le template Vite pose un `pnpm-workspace.yaml`).
- Le ledger `.superpowers/sdd/progress.md` (gitignoré) détaille chaque task + les findings de revue.

### Prochaine chose à creuser
- Valider l'UX au navigateur (serveur ci-dessus), puis **push + PR** de `feat/admin-react`, vérifier la CI verte.
- Ensuite **Phase 4** (serving `/c/<slug>` + unlock) — backend, indépendant du front.

---

## 2026-06-25 — Migration React Plan 1 : Backend OpenAPI livré (feat/admin-react)

> Plan 1/3 exécuté en Subagent-Driven (8 tâches). Détail tâche-par-tâche dans
> `.superpowers/sdd/progress.md` ; plan dans `docs/superpowers/plans/2026-06-25-migration-react-plan-1-backend-openapi.md`.

### Dernière chose faite
- **DTO inlinés dans `backend/src/dto/`** et crate `latch-dto` supprimée du workspace (`Cargo.toml` membres + `backend/Cargo.toml`, `git rm -r latch-dto`). Workspace réduit à 2 membres : `backend` + `backend/migration`.
- **Réponses typées** : structs `OkResponse`/`DeployResponse`/`ActivateResponse` dans `crate::dto` remplacent les `serde_json::json!` ad-hoc. Tous les handlers retournent des types `ToSchema`.
- **Dérivation `utoipa::ToSchema`** sur tous les DTOs. Dépendances `utoipa 5` + `utoipa-swagger-ui 9` (axum 0.8 natif — v8 tire axum 0.7) ajoutées à `backend/Cargo.toml`.
- **`#[utoipa::path]` sur toutes les routes `/api/*`** (placées AVANT `#[debug_handler]`) + `ApiDoc` (`paths(...)`, `components(schemas(...))`) dans `backend/src/openapi.rs`.
- **`openapi.json` committé à la racine + test de drift + Swagger UI dev.** Régénérable via `UPDATE_OPENAPI=1 cargo test --test openapi_drift`. Swagger sous `/api-docs` en dev/test uniquement (guard `is_prod`). Test drift `backend/tests/openapi_drift.rs` dans la suite nextest.
- **Revue finale de branche (Opus) passée** sur tout le Plan 1 (`db58d28..`) : 0 Critical, fondation saine pour le Plan 2. Un Important corrigé (commit **`d80833a`**) : les doc-comments `///` des handlers fuitaient des paths `/admin/...` périmés + des notes internes (Context7/QUIRKS) dans `openapi.json` → auraient pollué le JSDoc du client TS. Summaries réduits à une ligne `/api`, notes internes passées en `//`, `openapi.json` régénéré. Sanity : 0 occurrence de `/admin/projects` et de `Context7` dans le schéma.
- Vérification finale : `cargo fmt --all` propre, `cargo clippy --all-targets -- -D warnings` 0 warning, **`cargo nextest run` = 88 verts** (intégration, security_invariants, openapi_drift, dto::tests, openapi::tests). Aucune référence résiduelle à `latch_dto`. **HEAD = `d80833a`**, working tree propre.

### Trucs en suspens
- **Plan 2 (PROCHAINE ÉTAPE)** : app React (Vite + TanStack Router SPA). PAS DE BRAINSTORM — le design est déjà tranché. Il reste à **écrire le plan** (writing-plans) puis l'exécuter en Subagent-Driven.
- **CI / Docker rouges PAR DESIGN** sur `feat/admin-react` (Dockerfile stage Trunk/wasm, job CI frontend wasm, `web/mod.rs` défaut `../frontend/dist`, `.env.example`/`.gitignore`) — seront retravaillés au **Plan 3** (CI pistes node + Docker stage pnpm). Ne pas s'en alarmer.
- **BACKLOG (non bloquant, ajoutés ce jour)** : `SecurityScheme` cookie dans l'OpenAPI ; allowlist `deny.toml` pour les transitives de `utoipa-swagger-ui 9` (dont `zlib-rs` licence « Zlib ») → à traiter avec la supply-chain du Plan 3.

### Prochaine chose à creuser — DÉMARRAGE PLAN 2 (à froid)
- **Écrire le Plan 2** (`docs/superpowers/plans/`) via writing-plans, à partir du design déjà validé.
- **Design de référence = `docs/superpowers/specs/2026-06-25-admin-react-stack-design.md`** (LA source : stack Vite+React+TanStack Router, OpenAPI→openapi-typescript+openapi-fetch, Query/RHF+zod/react-i18next/sonner, structure `frontend/`, `.nvmrc`, tests Vitest+MSW). La décision/périmètre amont est dans `2026-06-25-admin-react-migration-decision.md`.
- **Input figé du front = `openapi.json`** (racine) : le build front lancera `openapi-typescript` dessus → `frontend/src/api/schema.d.ts` + client `openapi-fetch`. Le schéma est propre (revue finale).
- **Recycler depuis la branche Yew** : catalogue i18n FR/EN via `git show feat/phase-3-spa-yew-admin:frontend/locales/en.yml` (et `fr.yml`) → JSON ; comportement UX = contrat §7 (`docs/contrat-deploy.md`) ; thème oklch (preset shadcn `bJfDPe2y`).
- **Plan 3 ensuite** : CI pistes (back/front/(fuma)→e2e→docker), supply-chain front, Docker stage Node/pnpm, smoke e2e Playwright, alignement docs (BOOTSTRAP/contrat §4/ROADMAP/ENVIRONMENT/README).

### Notes pour future Claude
- Le workspace n'a plus de crate `latch-dto`. Tout est dans `crate::dto` (`backend/src/dto/mod.rs`). Les types sont identiques, juste inlinés.
- Pour régénérer `openapi.json` après un changement de handler ou de DTO : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` (depuis la racine). Un test de drift casse la suite si on oublie.
- **Les `///` des handlers deviennent les `summary` OpenAPI → JSDoc du client TS.** Garder ces doc-comments concis/orientés API ; mettre les notes internes en `//`. (Cf. QUIRKS.)
- Le Swagger UI (`/api-docs`) ne s'expose qu'en dev/test (`is_prod = !matches!(env, Development | Test)`, fail-secure : tout env inconnu = prod = pas de Swagger). Ne pas inverser ce guard.
- Épingler `utoipa-swagger-ui = "9"` : v8 tire `axum 0.7` (conflit de types avec l'axum 0.8 du projet). Cf. QUIRKS.
- Ledger d'exécution Subagent-Driven du Plan 1 (détail tâche-par-tâche, findings) : `.superpowers/sdd/progress.md` (gitignoré, scratch).

---

## 2026-06-25 — DÉCISION : migration admin Yew → React/Vite/shadcn (pause, reprise à froid)

### Dernière chose faite
- **Décision actée** (après le polish Yew) : **migrer l'admin SPA de Yew vers React + Vite +
  shadcn/ui + Tailwind**. Raison : `shadcn-rs` 0.1 + outillage wasm = trop de friction pour
  peu de gain ; écosystème JS mature = vélocité + qualité produit ; cohérent avec Fumadocs prévu.
  **Le backend reste Rust.** Discussion complète + périmètre + recyclage + questions ouvertes :
  **`docs/superpowers/specs/2026-06-25-admin-react-migration-decision.md`** (à lire en premier).
- **Fait cette session** : branche **`feat/admin-react`** créée ; crate Yew **`frontend/` supprimée**
  (`git rm`), retirée des `members` du workspace racine. Backend compile, **86 tests verts**.
  Le backend Phase 3 (API `/api/*`, serving `/admin`, garde Origin, session, `latch-dto`, tests
  `spa_serving`/`security_invariants`) est **gardé** (agnostique du front).
- **Branche Yew `feat/phase-3-spa-yew-admin`** conservée comme référence (conserve `frontend/`
  Yew + locales + composants). `main` intouché.

### Trucs en suspens (volontairement, pour la session neuve)
- **CI/Docker rouges attendus** sur `feat/admin-react` : Dockerfile stage `trunk`, job CI
  `frontend` (wasm), `web/mod.rs` défaut `../frontend/dist`, `.env.example`/`.gitignore` — à
  retravailler vers un pipeline **node/pnpm (vite build)** PENDANT la migration (cf. doc §6).
- BOOTSTRAP/contrat §4 (stack/rendu) à mettre à jour une fois la stack React tranchée.

### Prochaine chose à creuser (SESSION NEUVE, contexte vide)
- Brainstormer la **base technique React** (routeur, types TS depuis `latch-dto`, data layer,
  i18n lib, tests/MSW, pipeline build, dossier) — cf. doc §5. Puis spec → plan → impl.
- **Recycler** : contrat §7 (UX exacte), catalogue i18n FR/EN (depuis la branche Yew), endpoints
  `/api/*`, shapes `latch-dto`, thème oklch (se colle direct dans shadcn — plus de conversion),
  décisions UX du polish (badges, toasts, PIN/slug disabled, dropzone, a11y, sélecteur langue).
- **Fumadocs** (landing + doc GH Pages) = chantier séparé, après l'admin React.

### Notes pour future Claude
- Ne PAS repartir de `main` (n'a pas le backend Phase 3 : ni `/api`, ni serving SPA, ni `latch-dto`).
  Partir de `feat/admin-react` (backend Phase 3 + thème, sans le front Yew).
- Le serving Loco sert n'importe quel dist statique sous `/admin` (`spa_serving.rs` = faux dist).
  Le React Vite : `base: '/admin/'` + basename routeur ; cookies envoyés (same-origin), pas de token.

---

## 2026-06-25 — Polish UX + i18n SPA TERMINÉ (punch-list post-test-live, 10 tâches SDD)
> ⚠️ Réalisé en **Yew** — désormais **superseed** par la migration React (voir entrée ci-dessus).
> Reste la **référence comportementale/UX** à porter en React (contrat §7 + catalogue i18n).

### Dernière chose faite
- Chantier **polish UX + i18n** clos sur `feat/phase-3-spa-yew-admin` (spec
  `docs/superpowers/specs/2026-06-24-phase-3-ux-polish-design.md`, plan
  `docs/superpowers/plans/2026-06-24-phase-3-ux-polish.md`). Déroulé en **Subagent-Driven**
  (1 implémenteur + 1 reviewer par tâche). Ledger : `.superpowers/sdd/progress.md`.
- **Livré** : (1) **i18n FR+EN** via `rust-i18n 3` — `LocaleProvider` réactif + `use_locale()`,
  fichiers `frontend/locales/{en,fr}.yml`, **sélecteur FR/EN** (`LocaleSwitcher`) persistant
  (localStorage `latch.locale`) + détection `navigator.language` au boot, défaut **EN** ;
  (2) **couche de toasts maison** (`toast.rs`, `ToastProvider`/`use_toast`, gloo-timers 4 s)
  câblée sur tous les retours d'action (création/édition/déploiement/activation/suppression/copie) ;
  (3) **`Toggle` vendorisé** (`components/toggle.rs`, patch du `Switch` shadcn-rs cassé — état
  contrôlé pur, classe `size-md` load-bearing) ; (4) **badges colorés** (vert PIN requis / orange
  libre — vars `--color-success`/`--color-warning` ajoutées à `variables.css`) ; (5) **dropzone
  drag-and-drop** (deploy.rs) ; (6) **PIN disabled** (au lieu de retiré du DOM) + **slug disabled**
  en édition ; (7) **helper text** + **intros de page** ; (8) **accessibilité** (`<a onclick>` →
  `<button class="linkish">`, breadcrumb en `<button>`) ; (9) login espacé.
- **Validé end-to-end au navigateur (Playwright)** : i18n réactif FR↔EN + persistance reload ;
  login espacé ; badges orange ET **vert** ; toasts (copie/création/déploiement) verts + auto-dismiss ;
  Toggle bascule visuellement ; PIN grisé quand code off (sans saut de layout) ; **dropzone : drop
  d'un fichier** lu + `human_size` ; détail EN avec glyphes `✎/⬆/🗑` ; panel danger interpolé
  (`Delete "…"`, `its N version(s)`).
- **Bug trouvé en validation live (invisible aux reviews unitaires) + corrigé** : badges
  `Variant::Secondary + badge--success` s'affichaient **gris** — `.badge.variant-secondary`
  (spécificité 0,2,0) de shadcn-rs écrasait `.badge--success` (0,1,0). Fix : doubler la classe
  (`.badge.badge--success/--warning`). Commit `8ff8bb7`. **Leçon : toujours valider les couleurs/CSS
  au navigateur** (cf. QUIRKS).
- Qualité finale (checkout réel) : `cargo fmt` clean, `clippy -p latch-ui --target wasm32 -D warnings`
  **0 issue**, `wasm-pack test` **5/5** (pin×2, url, i18n×2), `trunk build` OK.

### Trucs en suspens
- Revue finale de branche (opus) à passer avant merge/PR.
- BACKLOG : flicker `ondragleave` de la dropzone sur les enfants (cosmétique) ; un éventuel
  vrai i18n multi-locale au-delà de FR/EN (la couche est prête, ajouter une locale = un YAML).
- `cargo deny` (CI) : `rust-i18n 3.1.5` + 10 deps transitives ajoutées au lockfile (`9b2b3dd`) —
  vérifier qu'aucune nouvelle licence ne casse `deny.toml` au prochain run CI.

### Prochaine chose à creuser
- Merge/PR de `feat/phase-3-spa-yew-admin` sur `main` (toute la Phase 3 + le polish). Puis **Phase 4**
  (serving `/c/<slug>`).

### Notes pour future Claude
- **Réactivité i18n** : tout composant qui rend du texte traduit DOIT appeler `use_locale()` en
  tête (même `let _loc = use_locale();` non utilisé) — l'abonnement au contexte force le re-render ;
  `t!` lit la locale globale rust-i18n déjà mise à jour par `set_locale`. Cf. QUIRKS/CONVENTIONS.
- **Badges colorés** : utiliser `.badge.badge--success/--warning` (double classe) sinon shadcn écrase.
- **shadcn-rs cassé → vendoriser** (CSS, puis `Switch`→`Toggle`). Règle de projet (CONVENTIONS).
- Stack de validation live : `trunk build` (frontend) puis backend depuis `backend/` avec
  `LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret DATABASE_URL='sqlite://…'`.

---

## 2026-06-24 — Task 3 : ToastProvider + use_toast + câblage CopyButton

### Dernière chose faite
- Implémenté `frontend/src/toast.rs` (remplace le stub) : `ToastProvider`, `use_toast()`,
  `ToastHandle { push_success, push_error }`, auto-dismiss 4 s via `gloo_timers::Timeout`.
- Monté `<ToastProvider>` entre `<LocaleProvider>` et `<AuthProvider>` dans `main.rs`.
- Ajouté styles `.toast-stack`/`.toast`/`.toast--success`/`.toast--error` dans `app.css`.
- Rewired `copy_button.rs` : `use_toast()` + `t!("toast.copied")` + `t!("common.copied")`
  (les deux bras `Cow<'static, str>`). Warning `#[macro_use]` résolu.
- Build trunk : ✅. wasm-pack test 5/5 verts.
- Commit : `96bca80` — `✨ feat(toast): ToastProvider maison (gloo-timers) + câblage copie`

### Trucs en suspens
- `--color-success` non défini jusqu'à Task 6 → `.toast--success` sans fond coloré (attendu).
- Validation visuelle du toast (Playwright) déléguée au contrôleur (step 5 du brief).
- Prochaine tâche dans la SDD : **Task 4** (Toggle vendorisé — patch Switch shadcn-rs).

### Prochaine chose à creuser
- Task 4 : patch du `Switch` shadcn-rs (toggle visuel qui ne bascule pas).

### Notes pour future Claude
- Pattern toast : `use_toast()` dans tout composant sous `<ToastProvider>`.
- `make_push` retourne `Callback<String>` — pas de `Rc<Fn>` (évite les pitfalls de capture).
- `Timeout::forget()` : timer non trackable, no-op si composant démonté. Sûr.

---

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
