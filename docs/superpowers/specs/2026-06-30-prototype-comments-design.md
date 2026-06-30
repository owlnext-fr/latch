# Spec — Commentaires ancrés sur les prototypes projetés

> **Type** : design / spec (entrée de la phase plan). **Date** : 2026-06-30.
> **Feature** : commentaires type Figma, ancrés à des éléments du DOM, sur la surface
> visiteur `/c/<slug>`, avec lecture + modération côté admin.
>
> **Doc-first (CLAUDE.md)** : c'est un **nouveau modèle de données ET une nouvelle surface
> de sécurité**. Le `docs/contrat-deploy.md` doit être amendé (§3, §6, §7, §9) **avant** le
> code. Cette spec décrit le quoi ; le plan d'implémentation décrira l'ordre.

## 1. Contexte & objectif

La surface visiteur `/c/<slug>` sert une **page-coquille (shell)** qui charge le prototype
réel dans un `<iframe src="/c/<slug>/raw">` (même origine, même binaire Loco). On ajoute une
couche de **commentaires ancrés à des éléments** : le visiteur active un mode commentaire
depuis une barre d'action flottante, cible un bloc du DOM du proto (surlignage au survol,
clic pour ancrer), et pose un **fil de commentaires** flottant près du bloc. Le commentaire
**suit** son élément au scroll et aux changements de scène (déplacement, disparition,
réapparition). L'admin **lit et modère** les commentaires par version, à la fois en liste et
en preview outillée (mêmes pastilles positionnées que le visiteur).

Référence de recherche technique (options A/B/C, verdicts de faisabilité) :
travail de cadrage en amont de cette spec (parent-reaches-in same-origin confirmé, ancrage par
descripteur stable confirmé, identité par cookie signé confirmée, divergence projets libres
confirmée, table `comments` sans impact sur `deploy()` confirmée).

## 2. Périmètre

**Dans la v1 :**
- Mode commentaire visiteur (desktop/curseur), picker d'élément, ancrage robuste, suivi live,
  popup/panneau, réduction en pastille, barre d'action 3 boutons.
- Persistance par version, identité visiteur anonyme (cookie signé), édition/suppression de ses
  propres commentaires.
- Toggle `comments_enabled` par projet (défaut sécurité-aware).
- Vue admin : liste textuelle par version **+** preview outillée (overlay positionné) **+**
  modération (suppression de n'importe quel message).

**Hors v1 (→ BACKLOG) :**
- Mode **collaboratif** (un visiteur voit les commentaires des autres) — la v1 est **privée**.
- Statut **« résolu »** (colonne réservée au schéma, sans UI).
- Commentaires **tactile / mobile** (le picker desktop d'abord ; lot mobile séparé).
- **Report inter-versions** des commentaires (re-ancrage v1→v2).
- **Authoring admin** (l'admin lit/modère mais ne crée pas de commentaire).
- Notifications admin de nouveaux commentaires ; filtres/tri admin avancés.
- Rendu **markdown** des corps (v1 = texte brut).

## 3. Décisions figées

| Sujet | Décision |
|---|---|
| Ambition technique | « B-full » : pins dormants, scorer de similarité, suivi complet, clustering, états orphelins — derrière des *seams* (interface `Picker` + descripteur d'ancrage W3C complet). |
| Rendu du corps | **Texte brut** (échappement JSX ; pas de `react-markdown` dans la couche commentaire). |
| Interaction iframe | **Parent-reaches-in** same-origin ; **aucune injection** dans `/raw`. |
| Visibilité | **Privé** : chaque relecteur ne voit que ses fils ; l'admin voit tout. Confidentialité = **filtre de lecture**, jamais contrainte de schéma (collaboratif = ajout non-breaking). |
| Activation | Toggle **`comments_enabled` par projet** ; défaut : code ON → commentaires ON, code OFF → commentaires OFF, « smart default jusqu'au premier touch » ; **pas de flip silencieux** ; avertissement à l'édition si retrait du code. |
| Structure | Un **pin** (point ancré) porte un **fil** de 1..N messages (modèle 2 tables). |
| Statut « résolu » | **Hors v1** ; colonne `status` (défaut `open`) réservée, sans UI. |
| Versioning | Commentaires **liés à leur version** (instantané figé) ; visiteur = version active ; archives en admin par version ; pas d'indicateur cross-version. |
| Identité / nom | **Lazy au 1ᵉʳ commentaire** (uniforme libres + à code) ; nom requis pour poster, pré-rempli ensuite (localStorage) ; cookie signé `HttpOnly` opaque (`owner_token`) pour éditer/supprimer les siens. |
| Mobile/tactile | **Hors v1**. |
| Barre d'action | **Overlay flottant** en bas ; 3 boutons : ✏️ mode commentaire · 👁️ pastilles + compteur · 💬 liste de mes commentaires. |
| Édition | Verbe **PUT** (cohérent avec `PUT /api/projects/{id}`). |
| Modération admin | **Incluse en v1** (l'admin supprime n'importe quel message, soft-delete). |
| Secret cookie identité | **Réutilise `UNLOCK_COOKIE_SECRET`** (la clé `SignedCookieJar` existante) ; pas de nouveau secret de prod. |
| Vue admin | **Liste textuelle + preview outillée** (module overlay partagé, adaptateur admin) ; admin = lecture/navigation/modération, **pas** d'authoring. |
| Bundle | Couche commentaire en **lazy (code-split Vite)** ; React Query confiné au module lazy. |

## 4. Contraintes d'architecture (vérifiées, à respecter)

- **Same-origin** : le shell et `/raw` sont deux routes du même hôte Loco → le parent peut lire
  `iframe.contentDocument`, `elementFromPoint`, `getBoundingClientRect`, écouter le scroll —
  **sans changement de header**. `frame-ancestors 'self'` régit *qui embarque*, pas l'accès DOM
  parent→enfant ; il doit **rester** `'self'`.
- **Proto immuable par version** : `/raw` sert l'HTML tel quel depuis le storage → un descripteur
  capturé se re-résout déterministe *dans une même version*. L'ennemi est le **runtime** (reflow,
  scènes JS, composants dupliqués), pas la dérive de source.
- **Gate unlock** : un seul helper `unlock_ok()` (`true` pour projet libre **sans cookie** ;
  sinon vérifie le cookie signé `latch_unlock`). À réutiliser tel quel pour gater les commentaires.
- **Projets libres ≠ à code** : un projet libre ne pose **aucun** cookie → identité + autorisation
  d'écriture des commentaires viennent d'un **chemin séparé** (identité lazy + rate-limit dédié).
- **`deploy()` storage-first → tx** : invariant inchangé ; une table `comments` FK→`versions`
  n'ajoute **rien** à ce chemin.
- **Admin code-first + drift** : DTO `ToSchema` + `#[utoipa::path]` → `openapi.json` (test de drift)
  → `pnpm gen:api` → `schema.d.ts`. « Terminé » = `UPDATE_OPENAPI=1 cargo test --test openapi_drift`
  **et** `pnpm gen:api`.
- **Invariants build-breaking** : aucune réponse ne contient de hash ; le PIN clair n'apparaît que
  sur le détail admin projet. Le `owner_token` doit suivre la même rigueur (jamais sérialisé).
- **Rate-limit** actuel : seulement `/unlock` → les writes commentaires exigent **de nouvelles**
  couches Governor.

## 5. Modèle de données

Migrations SeaORM hand-numbered sous `backend/migration/src/`, puis entités régénérées par
`sea-orm-cli` (`cargo loco db entities`).

### 5.1 `projects` — colonne ajoutée

- `comments_enabled` BOOLEAN NOT NULL. La migration **backfill** les projets existants avec
  `comments_enabled = code_enabled`. Ensuite, le service pose la valeur explicitement à la
  création/édition (cf. §10.1 défaut sécurité-aware).

### 5.2 `comment_pins` — point ancré, porte un fil

| Colonne | Type | Rôle |
|---|---|---|
| `id` | PK autoincrement | |
| `version_id` | INTEGER NOT NULL, FK→`versions(id)` **ON DELETE CASCADE** | lie le pin à une version |
| `owner_token` | TEXT NOT NULL | jeton opaque du créateur — **jamais sérialisé** |
| `anchor` | TEXT NOT NULL | descripteur JSON (§5.4) |
| `status` | TEXT NOT NULL DEFAULT `'open'` | réservé (`open`/`resolved`) — pas d'UI v1 |
| `created_at` / `updated_at` | timestamps | |
| `deleted_at` | timestamp NULL | soft-delete (tombstone) |

Index : `comment_pins(version_id)`.
`anchor_status` (`anchored`/`approximate`/`orphaned`) **n'est pas persisté** : résultat de
résolution au runtime, calculé à l'affichage.

### 5.3 `comments` — message d'un fil

| Colonne | Type | Rôle |
|---|---|---|
| `id` | PK autoincrement | |
| `pin_id` | INTEGER NOT NULL, FK→`comment_pins(id)` **ON DELETE CASCADE** | rattache au pin |
| `owner_token` | TEXT NOT NULL | jeton opaque de l'auteur du message |
| `author_name` | TEXT NOT NULL | nom auto-déclaré (≤ 80 chars, caractères de contrôle retirés) |
| `body` | TEXT NOT NULL | texte brut (≤ 2000 chars) |
| `created_at` / `updated_at` | timestamps | |
| `deleted_at` | timestamp NULL | soft-delete |

Index : `comments(pin_id)`.
En privé v1, tous les messages d'un pin partagent le même `owner_token` ; le garder au niveau
message rend le collaboratif futur (plusieurs auteurs sous un pin) purement additif.

### 5.4 Descripteur d'ancrage (`anchor`) — format de contrat, versionné

```json
{
  "v": 1,
  "selector": "main > section:nth-of-type(2) .card:nth-child(3) > button",
  "fingerprint": { "tag": "button", "text": "En savoir plus", "role": "button", "ordinal": 2 },
  "textQuote": { "exact": "En savoir plus", "prefix": "… avant", "suffix": "après …" },
  "offset": { "x": 0.42, "y": 0.60 },
  "fallbackPoint": { "x": 0.31, "y": 0.78 }
}
```

- `selector` (rung 1, lib `finder`, classes volatiles exclues du prédicat) ;
- `fingerprint` (rung 2 : désambiguïsation + base du scorer de similarité) ;
- `textQuote` (rung 3 W3C, robustesse sur nœuds de texte) ;
- `offset` = point du clic en **% de la boîte de l'élément** (placement du pin dans le bloc) ;
- `fallbackPoint` = coordonnée page **normalisée** (dernier recours, *orphaned/approximate*) ;
- `v` = version du format (évolutif).

## 6. Surfaces & endpoints

### 6.1 Public (visiteur) — sous `/c/{slug}`

Toutes en `Cache-Control: no-store`, **gardées par `unlock_ok` + `comments_enabled`** (sinon 404,
on ne révèle pas la feature). Les **écritures** portent en plus : garde **Origin** same-origin
(fail-closed si Origin absent), **cookie d'identité** (posé au 1ᵉʳ POST), **nouvelles couches
Governor** (par-IP + par-slug), header custom **`X-Comment-Client`**.

| Méthode | Route | Rôle |
|---|---|---|
| GET | `/c/{slug}/comments` | liste **mes** pins+fils de la **version active** (filtré par `owner_token`) |
| POST | `/c/{slug}/comments` | crée un pin + 1ᵉʳ message (`anchor` + `author_name` + `body`) ; pose le cookie d'identité si absent |
| POST | `/c/{slug}/comments/{pin}/replies` | ajoute un message à **mon** pin |
| PUT | `/c/{slug}/comments/messages/{id}` | édite **mon** message (remplace `body`) |
| DELETE | `/c/{slug}/comments/messages/{id}` | supprime **mon** message (soft) ; si dernier du fil → pin soft-deleted |
| DELETE | `/c/{slug}/comments/{pin}` | supprime **mon** pin (fil entier) |

Réponse GET visiteur — **jamais** de `owner_token` :

```json
{ "version": 3, "pins": [
  { "id": 12, "anchor": { "v": 1, "...": "..." }, "created_at": "...",
    "messages": [
      { "id": 31, "author_name": "Léa", "body": "...", "created_at": "...", "updated_at": "...", "editable": true }
    ] }
] }
```

`editable` calculé par appelant (toujours `true` en privé ; générique → prêt collaboratif).

### 6.2 Admin — sous `/api`, `AdminAuth`

Calqué sur `preview_version` (auth d'abord ; pas de garde Origin sur le GET).

| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/projects/{id}/versions/{n}/comments` | tous les pins+fils de la version `n` (DTO `CommentItem` — `author_name`, `body`, `anchor`, dates ; **pas** de `owner_token`) |
| DELETE | `/api/projects/{id}/comments/messages/{id}` | modération : supprime n'importe quel message (soft) — le handler **vérifie que le message appartient bien au projet `{id}`** (message → pin → version → projet) avant suppression |

### 6.3 DTO existants enrichis (→ régénérer `openapi.json` + `schema.d.ts`)

- `PublicMeta` (`GET /api/public/{slug}`) gagne **`comments_enabled: bool`** → le shell sait s'il
  affiche la barre commentaire.
- `VersionItem` (liste admin) gagne **`comment_count: i32`** (calculé en **une requête groupée**,
  pas de N+1, façon `list_with_versions`) → indicateur + action « Commentaires ».
- `ProjectDetail` / `ProjectListItem` exposent **`comments_enabled`** (toggle `ProjectForm`).

## 7. Identité & sécurité

- **`owner_token`** : ULID opaque (aléatoire), généré au 1ᵉʳ commentaire, posé dans un **cookie
  signé** `latch_comment` : `HttpOnly`, `Secure` (prod) + `SameSite=Lax`, `Path=/c/{slug}`, via
  `SignedCookieJar` (clé **`UNLOCK_COOKIE_SECRET`** réutilisée). Édition/suppression = **comparaison
  à temps constant** (`secure_compare`) du token du cookie vs `owner_token` de la ligne.
- **Garantie réelle** : « le même navigateur, porteur du cookie non-forgeable, édite ses propres
  commentaires ». **Frontière de confort, pas de sécurité** (le nom reste usurpable). Documenté.
- **Rate-limit** : nouvelles couches Governor sur les writes (par-IP + par-slug), calquées sur
  `serve_ratelimit.rs`, variables `LATCH_COMMENT_RL_*` (défauts sains). **Load-bearing** sur les
  projets libres (écriture non authentifiée).
- **CSRF** : garde Origin same-origin (fail-closed si Origin absent) + header `X-Comment-Client`
  exigé sur tous les writes ; en complément du `SameSite=Lax`.
- **Plafonds** (configurables) : `body` ≤ 2000 chars (`chars()`) → 422 ; `author_name` ≤ 80 chars,
  caractères de contrôle retirés ; max 200 pins / version / `owner_token` (anti-flood) → 429/422 ;
  `DefaultBodyLimit` sur la route (413).
- **Modèle de confiance projets libres** (assumé, documenté) : écriture non authentifiée, protégée
  par identité lazy + rate-limit + Origin + header ; niveau de confiance = celui du proto (public).

### 7.1 Invariants (tests build-breaking — `backend/tests/security_invariants.rs`)

1. `owner_token` **jamais** sérialisé (réponse publique **ou** admin) → `editable: bool` à la place.
2. Aucune réponse de commentaire ne contient de hash ni de PIN (§9.1/§9.2 préservés).
3. Le gate `unlock_ok` + `comments_enabled` couvre **toutes** les routes commentaires (projet à code
   non déverrouillé → 403 ; commentaires désactivés → 404).

## 8. Frontend — module partagé `src/comments/`

Couche à responsabilités isolées, consommée par le shell visiteur **et** l'admin. Chargée en
**lazy (code-split Vite)** ; React Query confiné à ce module.

### 8.1 `Picker` (la *seam*) — interface unique, 1 impl

```
getElementAt(x, y)        → élément du proto sous ce point (hit-test)
describe(el, clickPoint)  → descripteur d'ancrage JSON (§5.4)
resolve(anchor)           → { element, status: anchored|approximate|orphaned } | null
toShellRect(el)           → rect de l'élément transposé dans l'espace du shell
subscribe(cb)             → notifie sur scroll/resize/mutation/visibilité
```

Seule impl v1 : `SameOriginPicker` (lit `iframe.contentDocument`/`contentWindow`). Une future impl
`PostMessagePicker` (cross-origin) se branche sans toucher au reste.

### 8.2 Ancrage (`describe`/`resolve`) — échelle de résolution

- `describe` : sélecteur `finder` + empreinte + `textQuote` + offset% + point de secours.
- `resolve` (cascade) : sélecteur → 1 match = *anchored* ; plusieurs → empreinte+ordinal ; 0 →
  **scorer de similarité** sur les candidats (*approximate*) → sinon `textQuote` → sinon
  `fallbackPoint` (*orphaned/approximate*).

### 8.3 Pins dormants

`MutationObserver` sur le `<body>` du proto → **rejoue `resolve`** pour les pins *orphaned* quand le
DOM change ; un pin réapparaît tout seul quand son élément revient (onglet, modale, scène).

### 8.4 Contrôleur de suivi (partagé, 1 pour tous les pins)

- Sources : scroll passif (fenêtre proto + conteneurs scrollables), `ResizeObserver` par ancre,
  **un** `IntersectionObserver` (visibilité → affiche/masque), `MutationObserver` (scènes).
- Boucle : **un seul rAF à dirty-flag** ; sur signal → « sale » → frame suivante : **lire tous les
  rects puis écrire toutes les positions** (`translate3d`) ; idle si rien ne bouge. rAF continu
  réservé au sous-ensemble d'ancres animées par transform CSS.
- Transposition iframe→shell : rect interne + `getBoundingClientRect` de l'iframe (+ bordures).

### 8.5 Couche overlay (rendu)

Calque frère de l'iframe (`absolute inset-0`, `pointer-events` ajustés) :
- surlignage au survol = boîte à contour dégradé, `pointer-events:none` (mode pick) ;
- pastilles positionnées par le contrôleur ; **clustering** des pastilles denses en badge compteur ;
- popups via **`@floating-ui/dom`** (`computePosition` sur `VirtualElement` = rect suivi ;
  `offset`/`flip`/`shift`/`arrow`) ; panneau de fil **déployé** ⇄ **réduit** en pastille ;
  badge « a peut-être bougé » sur *approximate/orphaned*.

### 8.6 Barre d'action (overlay flottant bas)

✏️ mode commentaire · 👁️ pastilles + compteur · 💬 liste de mes commentaires (clic → saut au pin).
Affichée si `comments_enabled` ; capabilities-gated (mode pick masqué si `canAuthor=false`).

### 8.7 Machine à états du mode pick

`idle` → `pick` (curseur modifié, surlignage au survol) → clic = capture l'ancre + ouvre la popup
nouveau-commentaire (nom *lazy* pré-rempli + corps) → submit → POST → le pin apparaît → retour
`idle`.

### 8.8 Adaptateur de données + capabilities

Interface : `list()`, `createPin()`, `addReply()`, `editMessage()`, `deleteMessage()`,
`deletePin()` + objet `capabilities` (`canAuthor`, `canEditOwn`, `canModerate`).
- **Adaptateur visiteur** → `/c/{slug}/comments` (scopé `owner_token`) ; `canAuthor` + `canEditOwn`.
- **Adaptateur admin** → endpoints admin (tous les fils) ; `canModerate` seul (lecture + suppression
  de n'importe quel message ; pas d'authoring).

## 9. Frontend — montage visiteur (shell)

Le shell de base reste minuscule. Si `comments_enabled` (lu dans `PublicMeta`), il charge la barre
d'action minimale ; le **gros module** (picker/ancrage/suivi/overlay + React Query) se charge en
lazy **au besoin** : entrée en mode commentaire, ou `GET /comments` renvoie ≥ 1 pin à afficher.
Les fetches commentaire suivent la règle « swallow-on-error » du shell (un échec ne masque jamais le
proto). Nom pré-rempli depuis `localStorage`.

## 10. Frontend — montage admin

### 10.1 Toggle `comments_enabled` (`ProjectForm`)

- À la **création** : défaut suit le toggle « code » en direct (code ON → commentaires ON,
  code OFF → OFF) **tant que l'admin n'a pas touché** le toggle commentaires (RHF : `watch` le code,
  `setValue` commentaires sauf si le champ est `dirty`). Helper text : « Sans code d'accès, les
  commentaires sont publics en écriture (protégés par anti-spam). »
- En **édition** : toggles indépendants ; retirer le code ne désactive **pas** silencieusement les
  commentaires → **avertissement inline** si commentaires ON + code passé à OFF. **Zéro flip silencieux.**

### 10.2 Liste textuelle (`VersionCommentsPanel`)

`<Sheet>` lecture seule (calqué sur `version-detail-panel.tsx`), ouvert depuis la ligne de version
(action « Commentaires », désactivée si `comment_count === 0`). Pins groupés ; chaque pin = carte
avec **repère d'ancrage lisible** (dérivé de `fingerprint`, ex. « bloc `button` — “En savoir plus” »)
+ fil (auteur, body texte, dates) + **modération** (corbeille par message, confirmation).
Hook `useVersionComments(projectId, n)` (openapi-fetch + React Query, déjà présents côté admin).

### 10.3 Mode Review (preview outillée)

Nouvelle route SPA `/admin/projects/{id}/versions/{n}/review` : encadre le proto (iframe → route
preview raw existante, `AdminAuth`, same-origin) et monte le **module overlay partagé** avec
l'**adaptateur admin**. L'admin voit **tous les commentaires positionnés** sur le vrai proto,
navigue (liste latérale ↔ pins), et **modère**. Pas d'authoring admin.
**Ajustement backend** : la route preview raw doit autoriser l'encadrement same-origin
(`frame-ancestors 'self'`).

## 11. i18n

- `locales/shell/{en,fr}.json` → `shell.comments.*` (barre, popup, prompts nom, états).
- `locales/admin/{en,fr}.json` → `version_comments.*` (panneau + Review + modération + toggle).
- Auto-découverte JSON, clés plates (convention existante).

## 12. Tests (par couche — gate Sonar `new_coverage ≥ 80 %`)

- **Backend nextest** : migration ; `CommentsService` (cœur, sans axum/loco — garde d'archi) ;
  intégration endpoints (gate `unlock_ok` + `comments_enabled`, auth `owner_token` edit/delete,
  rate-limit, garde Origin, liste admin, modération admin) ; `security_invariants` (§7.1) ;
  `openapi_drift` régénéré.
- **Frontend Vitest + MSW** : ancrage `describe`/`resolve` sur DOM-fixtures (unique / dupliqué /
  scène changée) ; contrôleur de suivi (rects mockés) ; rendu overlay / popup / barre ; les deux
  adaptateurs ; vue Review admin ; logique du défaut `comments_enabled` dans `ProjectForm`. Shims
  jsdom (`getBoundingClientRect`, `IntersectionObserver`/`ResizeObserver`/`MutationObserver`) façon
  shims radix/input-otp existants.
- **e2e Playwright (desktop)** : visiteur (unlock → mode commentaire → cibler → écrire → pin
  apparaît → scroll suit → reload persiste → éditer → supprimer) ; projet libre ; vue Review admin
  (pins positionnés + modération).

## 13. Docs & contrat à amender

**Doc-first — `docs/contrat-deploy.md` amendé AVANT le code :**
- §3 : tables `comment_pins`/`comments`, colonne `comments_enabled`, format `anchor`.
- §6 : endpoints `/c/{slug}/comments` + gating + cookie d'identité ; note MCP `deploy_prototype`
  **inchangé**.
- §7 : toggle `comments_enabled`, vue admin liste + Review + modération.
- §9 : invariants §7.1 (owner_token jamais sérialisé, couverture du gate).

**Autres docs mémoire :**
- `docs/ENVIRONMENT.md` : `LATCH_COMMENT_RL_*` + plafonds + réutilisation du secret cookie +
  route Review.
- `docs/CONVENTIONS.md` : module partagé + adaptateur + capabilities + seam `Picker`.
- `docs/QUIRKS.md` : pièges (transposition iframe, shims jsdom observers, lazy code-split).
- `docs/INDEX.md` + `docs/HANDOFF.md` : à la livraison.

**`public_docs/` (Fumadocs) — nouvelle page ET passe sur les pages existantes impactées :**
- Nouvelle page « Commenter un prototype » (visiteur) + section admin Review.
- **Passe sur l'existant** (le shell+iframe et les surfaces changent) :
  - `how-it-works/architecture` (le shell héberge désormais la couche commentaire) ;
  - `how-it-works/security-model` (cookie d'identité, gating commentaires, invariant owner_token) ;
  - `admin/*` (toggle `comments_enabled`, vue Review, modération) ;
  - toute page décrivant la surface `/c` / le serving si elle mentionne le périmètre.
- Contraintes MDX/basePath/images déjà connues (cf. QUIRKS) à respecter.

## 14. Confidentialité

Fixtures et exemples en placeholders fictifs (`demo`, `ACME`, `mon-projet`) — **jamais** de nom de
client réel, nulle part (code, tests, docs, messages de commit).

## 15. Définition de « terminé » (rappel CLAUDE.md)

- `cargo fmt` + `cargo clippy --all-targets -D warnings` verts ;
- toutes les couches de test vertes (unit cœur, intégration, MCP inchangé, Vitest+MSW, e2e) ;
- gate SonarCloud `new_coverage ≥ 80 %` sur le code neuf ;
- `docs/contrat-deploy.md` cohérent avec le code (amendé en amont) ;
- `public_docs/` (page neuve + passe existant) à jour ;
- `docs/HANDOFF.md` (entrée datée) + `docs/INDEX.md` (livrables au vert) mis à jour.

## 16. Hors périmètre / BACKLOG (récap)

Mode collaboratif · statut « résolu » · commentaires tactile/mobile · report inter-versions ·
authoring admin · notifications admin · filtres/tri admin · rendu markdown des corps ·
visualisation admin par overlay déjà couverte en v1 (donc retirée du BACKLOG).
