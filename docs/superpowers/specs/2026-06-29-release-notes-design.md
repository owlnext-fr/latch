# Spec — Notes de version (release notes)

> Date : 2026-06-29
> Statut : design validé, prêt pour plan d'implémentation.

## Intention

Permettre, à chaque déploiement d'une version de prototype, la saisie de **notes
de version en markdown léger** (typographie, listes, emphase ; pas de liens ni
d'images). Au premier affichage d'une nouvelle version par un visiteur de
`/c/<slug>`, une **popup overlay** montre ces notes ; au dismiss, le navigateur
mémorise que cette version a été vue et ne réaffiche plus la popup.

Ce chantier introduit aussi une **page-coquille (shell) systématique** sur
`/c/<slug>` : le prototype est désormais encapsulé dans une iframe servie par une
couche de rendu latch. Les notes de version sont le **premier module** de cette
couche ; de futures fonctionnalités de rendu (`/c`) s'y brancheront sans toucher
au serving.

## Décisions validées (résumé)

| Sujet | Décision |
|---|---|
| Affichage overlay | **Shell + iframe** : `/c/<slug>` sert une coquille qui encadre le proto dans `<iframe src="/c/<slug>/raw">`. |
| Portée du shell | **Systématique** (tous les protos, pas seulement ceux avec notes) — `/c` devient surface de rendu pérenne. |
| Édition des notes | **Saisie au déploiement uniquement** (admin upload ou MCP). Pas d'édition après coup. |
| Éditeur admin | **WYSIWYG léger** (Tiptap, restreint gras/italique/listes) **+ onglet Aperçu**. |
| Tracking « vu » | **`localStorage`** côté navigateur (cosmétique, pas de cookie signé). |
| Rendu markdown | `react-markdown` **restreint** au périmètre partagé (pas de `a`/`img`, `skipHtml`) — barrière XSS unique, partagée overlay client + aperçu admin. |
| Périmètre markdown | **Identique éditeur et rendu** : titres, gras, italique, listes (puces + numérotées), citation, paragraphes. Rien d'autre. |
| Stockage | **Markdown brut** en colonne `release_notes TEXT NULL`. |

## Compromis assumés

- **Tous les protos tournent désormais en iframe.** Un proto qui suppose être au
  top-level (`window.top`, fullscreen API, certaines redirections) peut être
  gêné. Choix délibéré au profit d'une couche de rendu pérenne. `/c/<slug>/raw`
  pose `Content-Security-Policy: frame-ancestors 'self'` pour que seul le shell
  latch puisse l'encadrer.
- Pas d'édition des notes après déploiement : pour corriger, redéployer une
  version. Réduit la surface (pas d'endpoint PATCH, pas de panel d'édition).

---

## 1. Modèle de données & validation (backend)

### Migration

`backend/migration/src/m20260629_000004_add_release_notes_to_versions.rs` —
ajoute la colonne **`release_notes TEXT NULL`** à la table `versions`. Nullable :
les versions existantes restent valides (aucune popup pour elles). Inscrire la
migration dans `migration/src/lib.rs`.

### Entité

- `backend/src/models/_entities/versions.rs` : champ `release_notes: Option<String>`.
- Régénération via SeaORM, ou ajout manuel cohérent avec le style existant.

### Validation à l'input

- **Longueur max : 10 000 caractères.** Au-delà → erreur de validation
  (`CoreError` métier, traduit en 4xx côté web/MCP). UTF-8 garanti par le type
  `String` Rust.
- **On ne valide PAS la syntaxe markdown** à l'écriture (fragile, contournable).
  La barrière de sécurité réelle est le **rendu restreint** (§5). Justification :
  le markdown brut est inoffensif tant qu'il n'est pas transformé en DOM ; et le
  MCP peut transmettre du markdown arbitraire (Claude), donc le rendu doit de
  toute façon être la barrière.

---

## 2. Service & contrats (backend)

### Service

`backend/src/services/deploy.rs` :

```rust
pub async fn deploy(
    &self,
    project_id: i32,
    html: &str,
    activate: bool,
    release_notes: Option<&str>,   // nouveau
) -> Result<versions::Model, CoreError>
```

- Validation de longueur des notes avant écriture.
- `release_notes` persisté dans le `versions::ActiveModel` **dans la même
  transaction** que l'insertion de version (ordre HTML→DB inchangé).

### DTO

- `backend/src/dto/mod.rs` :
  - `VersionItem` : + `release_notes: Option<String>` (détail projet
    **authentifié** → exposition OK).
  - `DeployReq` : + `notes: Option<String>` (web admin).
- `DeployResponse` inchangé (`{ id, n }`).

### Invariants de sécurité

- Les notes **ne contiennent jamais** de hash ni de PIN → aucun impact sur les
  invariants existants (« pas de hash en réponse », « PIN seulement en détail »).
- **Nouveau vecteur : XSS via notes** (servies à des visiteurs anonymes dans
  l'overlay). Couvert par le rendu restreint (§5) + test dédié (§7).

---

## 3. MCP (backend)

`backend/src/mcp/mod.rs` — `DeployArgs` :

```rust
/// Notes de version en markdown léger (typographie, listes, emphase).
/// Les liens et images sont ignorés au rendu. Optionnel.
#[serde(default)]
release_notes: Option<String>,
```

Passé tel quel à `DeployService::deploy(...)`. La description du tool
`deploy_prototype` est complétée pour mentionner le paramètre. `DeployResult`
inchangé.

---

## 4. Serving `/c/<slug>` — shell systématique (backend)

Le shell est une **nouvelle mini-SPA Vite** (`shell.html` → `src/shell/main.tsx`),
sur le même moule que `unlock.html`. La couche serving devient :

### Routes

- **`GET /c/<slug>`** : résout projet + version active.
  - Pas de projet / pas de version active → page d'erreur (inchangé).
  - **Gate unlock au niveau shell** : si `code_enabled` et pas de cookie unlock
    valide → sert la **page de déverrouillage top-level** (comportement actuel,
    inchangé — le PIN reste saisi hors iframe).
  - Sinon → sert **toujours le shell** (`shell.html`), `no-store`.
- **`GET /c/<slug>/raw`** : cible de l'iframe. Sert le **HTML brut** du proto
  (logique de serving actuelle), avec **les mêmes gates** (defense-in-depth :
  re-vérifie l'unlock) + en-têtes `Cache-Control: no-store` et
  `Content-Security-Policy: frame-ancestors 'self'`.
- **`GET /c/<slug>/notes`** : renvoie `{ n: number, notes_md: string }` pour la
  version active, ou **`204 No Content`** si aucune note. **Gardé par le même
  unlock** que le proto → pour un proto protégé, les notes ne fuitent jamais
  avant déverrouillage. `no-store`.

### Notes d'architecture

- Garder le gate unlock **au niveau du shell** (et pas uniquement dans l'iframe)
  préserve l'UX (PIN top-level) et garantit que les notes restent invisibles tant
  que le proto l'est.
- L'endpoint `/notes` expose le numéro `n` de la version active, mais **gardé par
  l'unlock** : visible seulement par qui voit déjà le proto. Cela ne viole pas
  l'invariant « pas de n° de version au public », qui vise la surface *pré-auth*
  (page unlock) et le MCP.
- **Abandonné** : l'optimisation « pas de notes → raw direct » et le
  « redirect-vers-raw une fois vu ». Elles court-circuiteraient la couche shell
  qu'on veut pérenniser. Sans notes (ou déjà vues), le shell affiche simplement
  l'iframe plein cadre, sans overlay.

### Build / routing

- `frontend/vite.config.ts` : ajouter l'entrée `shell: 'shell.html'` (à côté de
  `main`, `unlock`, `error`).
- `backend/src/web/mod.rs` : helper `shell_index()` (chemin `frontend/dist/shell.html`),
  sur le modèle de `unlock_index()`.
- `backend/src/app.rs` / `controllers/serve.rs` : enregistrer les routes `/raw` et
  `/notes`, router `/c/<slug>` vers le shell.

---

## 5. Overlay & tracking « vu » (frontend — shell)

### Tracking navigateur

- **`localStorage`**, scopé par slug. Clé : `latch:seen:<slug>`, valeur : dernier
  numéro `n` vu (entier).
- Choix `localStorage` plutôt que cookie : le « déjà vu » est **purement
  cosmétique** — pas besoin de signature ni d'aller-retour serveur à chaque
  requête. `localStorage` est par-origine ; toutes les pages `/c/*` partagent
  l'origine, d'où le **scoping par slug obligatoire** dans la clé.

### Logique du shell au chargement

1. Monter l'iframe `src="/c/<slug>/raw"` (plein cadre).
2. `fetch('/c/<slug>/notes')` :
   - `204` (pas de notes) → ne rien afficher.
   - `{ n, notes_md }` → comparer à `localStorage['latch:seen:<slug>']` :
     - si `seen >= n` → ne rien afficher.
     - sinon → afficher l'**overlay** par-dessus l'iframe avec `notes_md` rendu.
3. Au **dismiss** de l'overlay → `localStorage['latch:seen:<slug>'] = n`, masquer
   l'overlay (l'iframe reste).

### Rendu markdown (barrière XSS)

- `react-markdown` configuré **restreint** au **périmètre markdown partagé** —
  autorisés : paragraphes, titres (`h1`…`h6`), `strong`/`em`, listes
  `ul`/`ol`/`li`, `blockquote` ; **interdits** : `a`, `img`, `code`, et **HTML
  brut** (`skipHtml: true`). Ce périmètre est **identique** à ce que produit
  l'éditeur admin (§6) : ni l'admin ni le MCP ne peuvent faire apparaître autre
  chose au rendu.
- **Même configuration de rendu** réutilisée par l'overlay client *et* l'aperçu
  admin (§6) → fidélité garantie et barrière unique. À extraire dans un composant
  partagé (ex. `src/lib/markdown.tsx` ou équivalent réutilisable par les deux
  bundles).

---

## 6. Éditeur admin (frontend)

- `frontend/src/components/deploy-panel.tsx` : ajouter un champ notes sous le
  choix du fichier, en **WYSIWYG léger** :
  - **Tiptap** (`@tiptap/react`, `@tiptap/starter-kit`) avec schéma **restreint
    au périmètre markdown partagé** : titres, gras, italique, listes (puces +
    numérotées), citation. **Désactiver** explicitement tout le reste du
    StarterKit hors périmètre (liens, images, blocs de code, code inline, etc.)
    pour que l'éditeur ne puisse produire que ce que le rendu accepte.
  - **Sérialisation en markdown** (extension markdown Tiptap) → la valeur envoyée
    dans `DeployReq.notes` est du `.md`, cohérent avec le MCP.
  - **Onglet Aperçu** : rend la valeur courante via le **même** `react-markdown`
    restreint que l'overlay client.
- La mutation `useDeploy()` (`src/hooks/use-projects.ts`) transmet `notes` dans le
  body.
- i18n (`src/i18n/locales/admin/{en,fr}.json`, clés plates) : ajouter
  `deploy.notes`, `deploy.notes_help`, `deploy.notes_edit`, `deploy.notes_preview`
  (FR + EN).
- **Dans le périmètre** : afficher un indicateur « a des notes » dans le tableau
  des versions de `detail.tsx` (ex. icône / badge sur la ligne, au survol ou en
  colonne). S'appuie sur `VersionItem.release_notes`.

### Régénération du client typé

- Après modif des DTO backend : `pnpm gen:api` (régénère `src/api/schema.d.ts`
  depuis `openapi.json`). Veiller à ce que `openapi.json` backend soit régénéré au
  préalable selon le pipeline existant.

---

## 7. Tests (definition of done)

- **Rust unit** (`services/deploy.rs`) : `deploy()` persiste `release_notes` ;
  rejet si > 10 000 caractères ; `None` laissé tel quel.
- **Intégration** (Loco + SQLite test) :
  - `POST /api/projects/{id}/deploy` avec `notes` → persistées et exposées dans
    `VersionItem`.
  - `GET /c/<slug>` sert toujours le shell (200, HTML shell) ; gate unlock pour
    proto protégé.
  - `GET /c/<slug>/raw` sert le HTML brut + en-têtes `no-store` et
    `frame-ancestors 'self'` ; gardé par l'unlock.
  - `GET /c/<slug>/notes` → `{ n, notes_md }` / `204` ; **403 (ou redirection)**
    si proto protégé non déverrouillé.
- **MCP** : `deploy_prototype(release_notes=…)` persiste les notes ; token gate
  inchangé.
- **Frontend Vitest** :
  - Tiptap → markdown : le périmètre partagé (titres, gras, italique, listes,
    citation) produit le markdown attendu ; rien hors périmètre (pas de
    liens/images/code) n'est produisible.
  - Rendu == éditeur : un markdown contenant un lien/une image/du code est rendu
    **sans** ces éléments (cohérence du périmètre des deux côtés).
  - Rendu restreint : un texte de notes contenant `<script>…</script>`,
    `[x](javascript:…)`, `![](x onerror=…)` est rendu **inerte** (pas de balise
    active, pas de `href` javascript).
  - Overlay : s'affiche si `seen < n`, se cache si `seen >= n` ; le dismiss écrit
    `localStorage`.
- **Playwright e2e** : déployer une version avec notes → visiter `/c/<slug>` →
  overlay visible → dismiss → reload → overlay absent.
- **Gate SonarCloud** `new_coverage ≥ 80 %` sur le code neuf (bloquante).
- `cargo fmt` + `cargo clippy -D warnings` ; `pnpm lint` + `pnpm typecheck`.

---

## 8. Nouvelles dépendances

- **Frontend** :
  - `react-markdown` (+ ce qu'il faut pour restreindre les éléments / `skipHtml`).
  - `@tiptap/react`, `@tiptap/starter-kit`, extension markdown Tiptap.
- **Backend** : aucune (markdown stocké brut, rendu côté client). À confirmer au
  plan si une validation/normalisation serveur supplémentaire est souhaitée.

---

## 9. Mémoire projet à mettre à jour (fin d'implémentation)

- `docs/INDEX.md` : ligne « Notes de version » + lien spec/plan.
- `docs/HANDOFF.md` : entrée datée.
- `docs/QUIRKS.md` : le piège « tous les protos sont en iframe désormais »
  (impacts `window.top`, CSP) si découvert pertinent.
- `docs/CONVENTIONS.md` : pattern du composant de rendu markdown restreint
  partagé, si introduit.
- `docs/ENVIRONMENT.md` : si une env var (ex. limite de longueur configurable) est
  ajoutée.
- `docs/contrat-deploy.md` : documenter la nouvelle surface shell `/c`, l'endpoint
  `/notes` + son gating, et le champ `release_notes` dans le flux deploy, puisque
  ça touche l'archi du serving (le contrat fait loi).

## 10. Documentation publique (Fumadocs — `public_docs/`)

À mettre à jour **dans le même chantier** (la doc publique fait partie du
livrable), couvrant la création de version (UI + MCP) et la visualisation `/c` :

- **`content/docs/admin/versions.mdx`** : section sur la saisie des notes de
  version au déploiement (éditeur WYSIWYG léger + aperçu, périmètre markdown :
  titres, gras, italique, listes, citation) et l'indicateur « a des notes » dans
  la liste des versions.
- **`content/docs/publish-from-claude/tools-reference.mdx`** : ajouter l'argument
  `release_notes` à la signature et au tableau de `deploy_prototype` (markdown
  léger, optionnel ; liens/images/code ignorés au rendu).
- **`content/docs/how-it-works/`** : décrire la nouvelle surface de serving
  `/c/<slug>` = **shell + iframe** (`/raw`, `frame-ancestors 'self'`, `no-store`),
  l'endpoint `/notes` gardé par l'unlock, et l'**overlay de notes** côté visiteur
  (affiché au premier passage sur une nouvelle version, mémorisé en
  `localStorage`, masqué au dismiss). Extension de `architecture.mdx` /
  `security-model.mdx`, ou nouvelle page de serving — à trancher au plan.
- Mettre à jour les `meta.json` concernés si une page est ajoutée.
