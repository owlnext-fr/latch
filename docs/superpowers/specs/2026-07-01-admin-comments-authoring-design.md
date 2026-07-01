# Design — Authoring de commentaires côté admin

> Spec. Branche : `feat/prototype-comments` (feature commentaires ancrés).
> Date : 2026-07-01. Statut : validé (approche A), en attente relecture spec.

## 1. Problème & objectif

Aujourd'hui l'admin peut **lire** tous les fils d'une version (page Review) et **modérer**
(supprimer n'importe quel message), mais **ne peut pas écrire**. On veut que l'admin puisse
participer : démarrer ses propres fils, **répondre aux fils des visiteurs** (le geste
collaboratif clé), et éditer/supprimer ses propres messages.

Décidé au brainstorming :
- **Périmètre** : authoring complet (créer des pins, répondre aux visiteurs, éditer/supprimer les siens).
- **Identité** : libellé fixe « Admin » (i18n), forcé serveur — le client ne choisit pas le nom.
- **Visibilité des fils *propres* de l'admin** : **admin seulement** (notes privées de relecture).
  Les *réponses* de l'admin dans le fil d'un visiteur restent, elles, visibles de ce visiteur.
- **Marqueur visuel** : badge « Admin » discret à côté du nom (surfaces `/c` **et** Review).

Hors périmètre v1 (backlog) : diffusion des fils propres de l'admin à tous les visiteurs,
notifications, statut « résolu ».

## 2. Choix structurant : jeton sentinelle (approche A)

Le cœur `CommentsService` est entièrement indexé par `owner_token`. L'admin a besoin d'une
identité de propriété. Comme il y a **un seul compte admin**, on réserve un `owner_token`
sentinelle constant plutôt que d'ajouter une colonne de rôle :

```
pub const ADMIN_OWNER_TOKEN: &str = "__admin__";
```

- **Non-collision** : un `owner_token` visiteur est un ULID (26 chars Crockford base32,
  `[0-9A-HJKMNP-TV-Z]`). `"__admin__"` (underscores) ne peut jamais être produit par
  `mint_owner_token()`. Aucune migration DB.
- **Anti-usurpation** : les writes visiteur lisent le owner **uniquement** depuis le cookie
  signé `latch_comment` (`read_owner_token`) ; un client ne peut pas se forger
  `owner_token = "__admin__"`. Les writes admin sont derrière `AdminAuth` (session) et posent
  la sentinelle côté serveur.
- **Distinction** : dérivée à la sérialisation — `is_admin = (owner_token == ADMIN_OWNER_TOKEN)`.
  On ne sérialise **jamais** le token (invariant §9 du contrat), seulement ce booléen.

`ADMIN_OWNER_TOKEN` vit dans `services/comments.rs` (module cœur) ; le module `dto` l'importe
pour dériver `is_admin`.

## 3. Backend — service

Constante `ADMIN_AUTHOR: &str = "admin"` (stockée dans `author_name` des messages admin ;
**jamais affichée** — l'UI rend le libellé i18n via `is_admin`. On stocke une valeur non vide
pour satisfaire `sanitize_author_name`).

Méthodes :

- **Créer un fil admin** : réutilise `create_pin(version_id, ADMIN_OWNER_TOKEN, ADMIN_AUTHOR,
  anchor, body)`. Le plafond anti-flood (200 pins par `(version, owner_token)`) s'applique à
  son propre bucket — OK.
- **Répondre au fil d'un visiteur** : **nouvelle** méthode
  `admin_add_reply(project_id, pin_id, body) -> Result<comments::Model, CoreError>`. Résout
  `pin → version → projet` (comme `moderate_delete_message`, NotFound si hors projet) **sans**
  owner-check, puis insère un message `owner_token = ADMIN_OWNER_TOKEN`, `author_name = ADMIN_AUTHOR`.
  Applique `validate_body` (2000 chars).
- **Éditer les siens** : réutilise `edit_message(comment_id, ADMIN_OWNER_TOKEN, body)`. Le
  owner-check interne (`owned_live_message` → `secure_compare`) restreint naturellement l'admin
  à ses propres messages (message visiteur → NotFound). Zéro nouvelle logique.
- **Supprimer un message** : `moderate_delete_message(project_id, comment_id)` existe déjà et
  couvre **tout** message du projet (dont les siens) + soft-delete du pin si vide. Réutilisé tel quel.
- **Supprimer un fil propre** : réutilise `delete_pin(pin_id, ADMIN_OWNER_TOKEN)`. L'owner-check
  interne restreint l'admin à ses **propres** pins (pin visiteur → NotFound). Nécessaire car le
  bouton « supprimer le fil » du `ThreadPopup` est gaté sur `canEditOwn && messages[0].editable` :
  pour un fil propre de l'admin (`editable = is_admin = true`), ce bouton **apparaît** et appelle
  `deletePin` — il doit donc être fonctionnel, pas `UNSUPPORTED`.

## 4. Backend — DTO / OpenAPI

Ajouter le booléen dérivé `is_admin` aux **deux** surfaces :

- `CommentMessage` (visiteur) : `+ pub is_admin: bool`.
- `AdminCommentMessage` (Review) : `+ pub is_admin: bool`. On **n'ajoute pas** `editable` côté
  admin (le test d'invariant `!json.contains("editable")` reste vert) ; le front dérive
  `editable = is_admin` dans l'adaptateur.

Câblage :
- `message_base_fields` reste inchangé ; `to_comment_pin` et `to_admin_comment_pin` calculent
  `is_admin = m.owner_token == ADMIN_OWNER_TOKEN` par message.
- Régénérer `openapi.json` + `frontend/src/api/schema.d.ts` (test `openapi_drift` doit rester vert).

## 5. Backend — endpoints (admin.rs)

Tous sous `AdminAuth` + `require_same_origin` (comme la modération ; **pas** de `X-Comment-Client`
— c'est la garde visiteur). Ajoutés dans `routes()` :

| Verbe / chemin | Handler | Corps | Service |
|---|---|---|---|
| `POST /api/projects/{id}/versions/{n}/comments` | `admin_create_pin` | `NewAdminPin { anchor, body }` | `create_pin(version.id, ADMIN_OWNER_TOKEN, …)` |
| `POST /api/projects/{id}/comments/pins/{pin}/replies` | `admin_reply` | `NewAdminReply { body }` | `admin_add_reply(id, pin, body)` |
| `PUT /api/projects/{id}/comments/messages/{cid}` | `admin_edit_comment` | `EditCommentBody { body }` | `edit_message(cid, ADMIN_OWNER_TOKEN, body)` |
| `DELETE /api/projects/{id}/comments/pins/{pin}` | `admin_delete_pin` | — | `delete_pin(pin, ADMIN_OWNER_TOKEN)` |

- `admin_create_pin` résout la version via `find_version(&ctx, id, n)` (déjà utilisé par
  `list_version_comments`) → garantit l'appartenance au projet.
- Réponses : `admin_create_pin` renvoie un `AdminCommentPin` (le pin fraîchement créé) ;
  `admin_reply`/`admin_edit_comment` renvoient un `AdminCommentMessage` ; `admin_delete_pin`
  renvoie `OkResponse`. `#[utoipa::path]` sur les 4 handlers, schémas de requête enregistrés
  dans l'`ApiDoc`.
- Note axum : `POST` et `GET` sur `/projects/{id}/versions/{n}/comments` sont fusionnés par
  `Router::route` (même remarque que le code existant).

## 6. Frontend

### 6.1 Adaptateur admin (`data/admin-adapter.ts`)
- `capabilities = { canAuthor: true, canEditOwn: true, canModerate: true }`.
- `toMessage` : `editable = m.is_admin` (au lieu de `false` en dur).
- Implémenter :
  - `createPin({ anchor, body })` → `POST /api/projects/{id}/versions/{n}/comments`. `author_name`
    ignoré (forcé serveur) ; on ne l'envoie pas.
  - `addReply(pinId, { body })` → `POST /api/projects/{id}/comments/pins/{pin}/replies`.
  - `editMessage(messageId, body)` → `PUT /api/projects/{id}/comments/messages/{cid}`.
  - `deletePin(pinId)` → `DELETE /api/projects/{id}/comments/pins/{pin}` (supprime un fil
    **propre** ; l'owner-check serveur restreint aux pins de l'admin). Requis car le bouton
    « supprimer le fil » s'affiche sur les fils propres de l'admin (cf. §5).
  - `deleteMessage` inchangé (endpoint de modération existant).

### 6.2 Identité fixe dans le compose/reply
- Étendre `Capabilities` avec `fixedAuthorName: string | null`.
  - Visiteur : `null` (comportement actuel — champ nom + `getStoredName`/`setStoredName`).
  - Admin : le libellé i18n `comment.admin_author` (passé à `createAdminAdapter`).
- `compose-popup.tsx` et le composer de réponse de `thread-popup.tsx` : si `fixedAuthorName != null`,
  **masquer** le champ nom et sa validation, afficher « en tant que {nom} » (`comment.compose.as_label`),
  et soumettre `author_name = fixedAuthorName`.

### 6.3 Badge « Admin »
- Type front `CommentMessage` gagne `is_admin` (via `schema.d.ts` régénéré).
- `thread-popup.tsx` et `comments-drawer.tsx` : quand `m.is_admin`, afficher le libellé
  `comment.admin_author` **au lieu de** `author_name`, suivi d'un `<Badge variant="secondary">`
  (shadcn, déjà présent dans le repo) portant `comment.admin_badge`. Discret, sur `/c` et Review.

### 6.4 i18n (`src/i18n/locales/comments/{en,fr}.json`)
Nouvelles clés : `comment.admin_author` (« Admin »), `comment.admin_badge` (« Admin »),
`comment.compose.as_label` (« Commenting as {{name}} » / « Vous commentez en tant que {{name}} »).

## 7. Sécurité / invariants

- `owner_token` **jamais** sérialisé — on n'ajoute qu'un booléen dérivé `is_admin` (cohérent
  avec l'invariant §9 : on renvoie un booléen, pas le token).
- Pas d'escalade de privilège : la sentinelle n'est jamais acceptée d'un client ; les writes
  visiteur lisent le owner uniquement depuis le cookie signé.
- Identité non usurpable : `author_name` client ignoré sur les endpoints admin ; le serveur pose
  `ADMIN_OWNER_TOKEN`/`ADMIN_AUTHOR`.
- Gardes : `AdminAuth` + `require_same_origin` sur les 4 mutations (401 sans session, 403 cross-origin).

## 8. Tests (par couche)

**Cœur (`services/comments.rs`)**
- `admin_add_reply` sur un pin visiteur ajoute un message (owner = sentinelle) ; sur un pin d'un
  autre projet → `NotFound`.
- `create_pin` avec la sentinelle → pin possédé par l'admin.
- `edit_message(ADMIN_OWNER_TOKEN, …)` sur un message visiteur → `NotFound` (édition restreinte aux siens).
- `delete_pin(pin, ADMIN_OWNER_TOKEN)` : supprime un pin propre de l'admin ; sur un pin visiteur → `NotFound`.

**DTO (`dto/mod.rs`)**
- `is_admin` vrai ssi `owner_token == ADMIN_OWNER_TOKEN`, sur `to_comment_pin` **et**
  `to_admin_comment_pin` ; `owner_token` toujours absent du JSON ; pas d'`editable` côté admin.

**Intégration (contrôleurs)**
- POST pin / POST reply / PUT edit : 401 sans session, 403 cross-origin, 200 nominal.
- **Réponse admin visible côté visiteur** : après `admin_add_reply` sur le pin d'un visiteur,
  `GET /c/{slug}/comments` (cookie du visiteur) contient le message avec `is_admin = true`.

**Frontend (Vitest)**
- Adaptateur admin : `createPin`/`addReply`/`editMessage` tapent les bons endpoints ;
  `editable = is_admin`.
- `thread-popup` / `comments-drawer` : badge + libellé quand `is_admin`.
- `compose-popup` : champ nom masqué quand `fixedAuthorName` fourni ; soumet ce nom.

**e2e (Playwright, `comments-admin.spec.ts`)**
- L'admin crée un pin en Review (ciblage → écriture → pin ancré).
- L'admin répond à un fil visiteur seedé (API) ; le message apparaît dans le fil.

## 9. Doc

- `docs/contrat-deploy.md` : §6.4/§7 (endpoints admin d'écriture), §9 (note `is_admin` booléen
  dérivé + sentinelle `ADMIN_OWNER_TOKEN`, invariant `owner_token` inchangé).
- `public_docs/content/docs/admin/comments.mdx` : mention de l'authoring admin (créer / répondre /
  éditer ses messages, badge « Admin »).
- Mémoire projet (INDEX, HANDOFF, QUIRKS/CONVENTIONS si pertinent) en fin d'implémentation.

## 10. Définition de « terminé »

`cargo fmt`/`clippy` clean ; `cargo nextest` vert (cœur + intégration + `openapi_drift`) ;
`pnpm lint` + `pnpm typecheck` + Vitest verts ; e2e Playwright verts ; couverture new-code ≥ 80 %
(gate SonarCloud) ; doc + mémoire à jour.
