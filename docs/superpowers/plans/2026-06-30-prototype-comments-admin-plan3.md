# Commentaires ancrés — Plan 3 (Admin Review + toggle + docs) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compléter la feature commentaires côté **admin** : toggle `comments_enabled` au formulaire projet, liste textuelle modérable par version, page **Review** (proto encadré + overlay de pins partagé + modération), i18n, e2e, docs Fumadocs.

**Architecture :** Le backend est **intégralement livré** (endpoints admin `list_version_comments` + `moderate_delete_comment`, DTOs `comments_enabled`/`comment_count`, schéma OpenAPI régénéré). Plan 3 est **frontend + docs**, plus **un seul durcissement backend** (1 header CSP). Le cœur est de **rendre le module `src/comments/` réutilisable** : l'adaptateur de données devient injectable dans `CommentsApp`, un `createAdminAdapter` est ajouté, et une route SPA Review monte le même module avec les capacités admin (`canModerate` seul, pas d'authoring).

**Tech Stack :** React 19 + Vite + TypeScript · TanStack Router (code-based, basepath `/admin`) / Query · shadcn/ui (Radix, Sheet/Switch) · react-hook-form/zod · react-i18next (FR/EN, CLDR `_one`/`_other`) · openapi-fetch (`@/api/client`) typé depuis `schema.d.ts` · Vitest + Testing Library + MSW · Playwright · Fumadocs (MDX). Backend : Rust/axum (1 header).

## Global Constraints

- **Confidentialité (NON-NÉGOCIABLE)** : aucun nom de client réel nulle part. Placeholders fictifs : `Mon Projet`/`mon-projet`, `ACME`, `demo`.
- **Invariant sécurité** : `owner_token` JAMAIS reçu côté client. L'admin ne lit que `AdminCommentMessage` (pas d'`editable`, pas d'`owner_token`). La modération s'appuie sur `capabilities.canModerate`, pas sur une propriété de la donnée.
- **Corps de commentaire = texte brut** (échappement JSX). Jamais de rendu HTML/markdown serveur ni client dans la couche commentaire.
- **Toutes les commandes frontend se lancent DEPUIS `frontend/`.** Vitest `globals:true`, alias `@ → src`, jsdom.
- **Prettier** : `semi:false`, `singleQuote:true`, `trailingComma:all`. Props composants typées `Readonly<…>`.
- **« Terminé » frontend = `pnpm lint` ET `pnpm typecheck` ET `pnpm test`** (pas seulement Vitest). `eslint-plugin-react-hooks` v7 strict (`react-hooks/refs`, `set-state-in-effect`) + `erasableSyntaxOnly` (pas de parameter properties) passent à travers Vitest mais cassent la CI. Cf. `docs/QUIRKS.md`.
- **NE PAS relancer `pnpm gen:api`** : `openapi.json`/`schema.d.ts` sont figés et déjà à jour. Le seul changement backend (Task K1) **ne touche pas** l'OpenAPI (un header HTTP n'est pas dans le schéma).
- **Backend** : si Task K1 touche `backend/`, gate `cargo fmt --all` + `cargo clippy --all-targets -- -D warnings` + `cargo nextest run` depuis la racine. Cœur sans axum/loco (contrat §1) — mais K1 est dans un *controller*, pas le cœur.
- **i18n** : clés plates, auto-découverte JSON. Pluriels CLDR `_one`/`_other` (PAS `_plural`). Locales admin : `frontend/src/i18n/locales/admin/{en,fr}.json` ; shell : `…/shell/{en,fr}.json`. Défaut EN.

---

## File Structure

**Frontend — module partagé (`frontend/src/comments/`)**
- Modifié `comments-app.tsx` — `CommentsApp` accepte `adapter` + `cacheKey` injectés (au lieu de `createVisitorAdapter(slug)` en dur).
- Modifié `ui/thread-popup.tsx` — supporte la suppression de modération (`canModerate` → corbeille sur tout message, pas seulement `editable`).
- Créé `data/admin-adapter.ts` — `createAdminAdapter(projectId, n)`, mapping admin→UI, caps `canModerate` seul.
- Créé `data/admin-adapter.test.ts`.

**Frontend — shell visiteur (`frontend/src/shell/`)**
- Modifié `comments-mount.tsx` — injecte `createVisitorAdapter(slug)` + `cacheKey=slug` (préserve le comportement actuel).

**Frontend — admin (`frontend/src/`)**
- Modifié `components/project-form.tsx` — toggle `comments_enabled` + smart default + warning.
- Créé `hooks/use-version-comments.ts` — `useVersionComments(projectId, n)` + `useModerateComment(projectId)`.
- Créé `components/version-comments-panel.tsx` — Sheet liste textuelle + modération.
- Créé `routes/review.tsx` — page Review (iframe preview + `CommentsApp` admin).
- Modifié `router.tsx` — route `/projects/$id/versions/$n/review`.
- Modifié `routes/detail.tsx` — actions « Commentaires » (panneau) + « Review » (lien) par ligne de version.
- Modifié `lib/utils.ts` — helper `reviewPath(id, n)`.
- Modifié `i18n/locales/admin/{en,fr}.json` — clés `version_comments.*`, `review.*`, `form.comments*`.

**Frontend — tests**
- Créés/modifiés : `project-form.test.tsx`, `version-comments-panel.test.tsx`, `review.test.tsx`, `use-version-comments.test.tsx`, `comments-app.test.tsx` (régression injection), `thread-popup.test.tsx` (modération).
- e2e `frontend/e2e/comments-admin.spec.ts`.

**Backend**
- Modifié `backend/src/controllers/admin.rs` — header `frame-ancestors 'self'` sur `preview_version`.
- Modifié `backend/tests/` (test du header).

**Docs**
- `public_docs/content/docs/` : nouvelle page « commenter » + passe sur `how-it-works/architecture.mdx`, `how-it-works/security-model.mdx`, `admin/projects.mdx`, `admin/versions.mdx`.
- Mémoire : `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`, `docs/ENVIRONMENT.md`, `docs/contrat-deploy.md` (vérifier cohérence §7/§10).

---

## Phase G — Rendre le module réutilisable (seam adapter injectable)

### Task G1 : `CommentsApp` accepte un adaptateur + une clé de cache injectés

**Files:**
- Modify: `frontend/src/comments/comments-app.tsx`
- Modify: `frontend/src/shell/comments-mount.tsx`
- Test: `frontend/src/comments/comments-app.test.tsx` (existant — adapter), `frontend/src/shell/comments-mount.test.tsx` (existant — vérifier régression)

**Interfaces:**
- Consumes : `CommentsAdapter` (`data/adapter.ts`), `createVisitorAdapter(slug)` (`data/visitor-adapter.ts`), hooks `useCommentList(cacheKey, adapter)` etc. (`data/use-comments.ts` — le 1ᵉʳ param est une **clé de cache opaque**, pas un slug).
- Produces : nouveau contrat de props
  ```ts
  interface CommentsAppProps {
    cacheKey: string                 // clé de cache React Query (ex. slug visiteur, `admin:{id}:{n}` admin)
    frame: FrameRef
    adapter: CommentsAdapter         // injecté (visiteur OU admin)
  }
  export function CommentsApp(props: Readonly<CommentsAppProps>): JSX.Element
  ```
  `CommentsApp` ne construit plus `createVisitorAdapter` ; il consomme `props.adapter`. Le pick/authoring reste piloté par `adapter.capabilities` (déjà le cas via `ActionBar`/`ComposePopup` capabilities-gated).

- [ ] **Step 1 : Adapter le test de régression**

Dans `comments-app.test.tsx`, le montage doit désormais passer un `adapter` explicite. Remplacer la construction interne par un adaptateur factice injecté :
```tsx
import { CommentsApp } from './comments-app'
import type { CommentsAdapter } from './data/adapter'

const fakeAdapter: CommentsAdapter = {
  capabilities: { canAuthor: true, canEditOwn: true, canModerate: false },
  list: async () => ({ version: 1, pins: [] }),
  createPin: async () => { throw new Error('unused') },
  addReply: async () => { throw new Error('unused') },
  editMessage: async () => { throw new Error('unused') },
  deleteMessage: async () => {},
  deletePin: async () => {},
}

it('monte la barre d’action quand l’adaptateur autorise l’authoring', async () => {
  render(<CommentsApp cacheKey="demo" frame={fakeFrame} adapter={fakeAdapter} />)
  expect(await screen.findByRole('button', { name: /pick|commenter|comment/i })).toBeInTheDocument()
})
```
(Réutiliser le `fakeFrame`/helpers déjà présents dans le fichier ; ne garder qu’un test couvrant l’injection. Conserver les autres tests existants en leur passant `adapter={fakeAdapter}`.)

- [ ] **Step 2 : Lancer le test → échec de typage/props**

Run (depuis `frontend/`): `pnpm vitest run src/comments/comments-app.test.tsx`
Expected : FAIL (prop `adapter` inconnue / `slug` manquant).

- [ ] **Step 3 : Refactor `comments-app.tsx`**

- Remplacer l’interface `CommentsAppProps` (`slug` → `cacheKey` + ajout `adapter`).
- Supprimer `import { createVisitorAdapter }` et le `useMemo(() => createVisitorAdapter(slug), [slug])` ; utiliser `const adapter = props.adapter`.
- Remplacer toutes les occurrences de `slug` passées aux hooks par `cacheKey` (ce sont des clés de cache). Concrètement, `useCommentList(slug, adapter)` → `useCommentList(cacheKey, adapter)`, idem pour les 5 mutations.
- `CommentsInner({ cacheKey, frame, adapter })` ; `CommentsApp` conserve son `QueryClient` confiné (inchangé).

Extrait du nouveau cœur :
```tsx
interface CommentsAppProps {
  cacheKey: string
  frame: FrameRef
  adapter: CommentsAdapter
}

function CommentsInner({ cacheKey, frame, adapter }: Readonly<CommentsAppProps>) {
  const picker = useMemo(() => new SameOriginPicker(frame), [frame])
  const list = useCommentList(cacheKey, adapter)
  const createPin = useCreatePin(cacheKey, adapter)
  const addReply = useAddReply(cacheKey, adapter)
  const editMessage = useEditMessage(cacheKey, adapter)
  const deleteMessage = useDeleteMessage(cacheKey, adapter)
  const deletePin = useDeletePin(cacheKey, adapter)
  // … reste identique (positions, pick machine, overlay, popups, ActionBar) …
}

export function CommentsApp(props: Readonly<CommentsAppProps>) {
  const client = useMemo(
    () => new QueryClient({ defaultOptions: { queries: { retry: false } } }),
    [],
  )
  return (
    <QueryClientProvider client={client}>
      <CommentsInner {...props} />
    </QueryClientProvider>
  )
}
```

- [ ] **Step 4 : Mettre à jour le shell visiteur**

Dans `shell/comments-mount.tsx`, là où `<CommentsApp slug={slug} frame={frame} />` est monté, injecter l’adaptateur visiteur :
```tsx
import { createVisitorAdapter } from '@/comments/data/visitor-adapter'
// …
const adapter = useMemo(() => createVisitorAdapter(slug), [slug])
return <CommentsApp cacheKey={slug} frame={frame} adapter={adapter} />
```
(Si le fichier monte `CommentsApp` via le default export lazy, conserver le `import()` ; n’ajouter que les props. Garder le `key`-bump sur `load` de l’iframe.)

- [ ] **Step 5 : Gate**

Run : `pnpm lint && pnpm typecheck && pnpm vitest run src/comments src/shell`
Expected : PASS (suite visiteur inchangée, comportement identique).

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/comments/comments-app.tsx frontend/src/comments/comments-app.test.tsx frontend/src/shell/comments-mount.tsx
git commit -m "♻️ refactor(comments): adaptateur injectable dans CommentsApp (réutilisation admin)"
```

### Task G2 : `ThreadPopup` — suppression de modération (`canModerate`)

**Files:**
- Modify: `frontend/src/comments/ui/thread-popup.tsx`
- Test: `frontend/src/comments/ui/thread-popup.test.tsx`

**Interfaces:**
- Consumes : `Capabilities` (`canEditOwn`, `canModerate`), `CommentMessage` (`editable`).
- Produces : règle d’affichage des actions par message :
  - **Éditer** un message : visible si `capabilities.canEditOwn && message.editable`.
  - **Supprimer** un message : visible si `(capabilities.canEditOwn && message.editable) || capabilities.canModerate`.
  - **Supprimer le pin entier** (`onDeletePin`) : visible **uniquement** côté visiteur propriétaire (`canEditOwn && pin.messages[0]?.editable`) — l’admin ne supprime PAS de pins (modération message-par-message, spec §6.2/§10.2). Donc masqué si `canModerate && !canEditOwn`.

- [ ] **Step 1 : Test modération**

Dans `thread-popup.test.tsx`, ajouter :
```tsx
it('affiche la corbeille de modération sur un message non-editable quand canModerate', () => {
  const caps = { canAuthor: false, canEditOwn: false, canModerate: true }
  const pin = { id: 1, anchor: '{}', created_at: '', messages: [
    { id: 9, author_name: 'Léa', body: 'salut', created_at: '', updated_at: '', editable: false },
  ] }
  render(<ThreadPopup pin={pin} position={pos} capabilities={caps} busy={false}
    onReply={vi.fn()} onEdit={vi.fn()} onDelete={vi.fn()} onDeletePin={vi.fn()} onClose={vi.fn()} />)
  expect(screen.getByRole('button', { name: /delete|supprimer/i })).toBeEnabled()
  // pas de bouton "supprimer le fil" en modération
})

it('ne montre PAS la suppression quand visiteur non-auteur (editable=false, canEditOwn)', () => {
  const caps = { canAuthor: true, canEditOwn: true, canModerate: false }
  const pin = { id: 1, anchor: '{}', created_at: '', messages: [
    { id: 9, author_name: 'Léa', body: 'salut', created_at: '', updated_at: '', editable: false },
  ] }
  render(<ThreadPopup pin={pin} position={pos} capabilities={caps} busy={false}
    onReply={vi.fn()} onEdit={vi.fn()} onDelete={vi.fn()} onDeletePin={vi.fn()} onClose={vi.fn()} />)
  expect(screen.queryByRole('button', { name: /delete|supprimer/i })).toBeNull()
})
```
(Adapter `pos`/imports aux helpers existants du fichier.)

- [ ] **Step 2 : Run → fail** : `pnpm vitest run src/comments/ui/thread-popup.test.tsx` → FAIL.

- [ ] **Step 3 : Implémenter la règle**

Dans `thread-popup.tsx`, dériver par message :
```tsx
const canEditMsg = capabilities.canEditOwn && m.editable
const canDeleteMsg = canEditMsg || capabilities.canModerate
const canDeleteThread = capabilities.canEditOwn && (pin.messages[0]?.editable ?? false)
```
Afficher le bouton éditer si `canEditMsg`, le bouton supprimer-message si `canDeleteMsg` (appelle `onDelete(m.id)`), le bouton supprimer-fil si `canDeleteThread`. Les confirmations existantes restent. Le reply reste gardé par… (le visiteur seul a `canAuthor`; en admin la zone reply est masquée si `!canAuthor`). Si la zone reply n’était pas déjà gated, la gater sur `capabilities.canAuthor`.

- [ ] **Step 4 : Run → pass** : `pnpm vitest run src/comments/ui/thread-popup.test.tsx` → PASS.

- [ ] **Step 5 : Gate + commit**

```bash
pnpm lint && pnpm typecheck && pnpm vitest run src/comments
git add frontend/src/comments/ui/thread-popup.tsx frontend/src/comments/ui/thread-popup.test.tsx
git commit -m "✨ feat(comments): suppression de modération dans ThreadPopup (canModerate)"
```

---

## Phase H — Adaptateur admin + hook

### Task H1 : `createAdminAdapter(projectId, n)`

**Files:**
- Create: `frontend/src/comments/data/admin-adapter.ts`
- Test: `frontend/src/comments/data/admin-adapter.test.ts`

**Interfaces:**
- Consumes : `api` (`@/api/client`), types `AdminCommentList`/`AdminCommentPin`/`AdminCommentMessage` + `CommentList`/`CommentPin`/`CommentMessage` (depuis `@/api/schema`), `CommentsAdapter`/`Capabilities`.
- Produces :
  ```ts
  export function createAdminAdapter(projectId: number, n: number): CommentsAdapter
  ```
  - `capabilities = Object.freeze({ canAuthor:false, canEditOwn:false, canModerate:true })`
  - `list()` → `GET /api/projects/{id}/versions/{n}/comments` → mappe chaque `AdminCommentMessage` en `CommentMessage` avec `editable:false` ; renvoie `{ version, pins }` au format `CommentList`.
  - `deleteMessage(id)` → `DELETE /api/projects/{id}/comments/messages/{cid}` (params `{ id: projectId, cid: id }`). **Pas** de header `X-Comment-Client` (endpoint admin, cf. backend) ; openapi-fetch envoie le cookie de session.
  - `createPin`/`addReply`/`editMessage`/`deletePin` → `throw new Error('admin:unsupported')` (jamais appelés : caps masquent l’UI).

- [ ] **Step 1 : Test du mapping + endpoints**

```ts
import { createAdminAdapter } from './admin-adapter'

vi.mock('@/api/client', () => ({
  api: { GET: vi.fn(), DELETE: vi.fn() },
}))
import { api } from '@/api/client'

it('list() mappe AdminCommentMessage → CommentMessage editable:false', async () => {
  ;(api.GET as Mock).mockResolvedValue({
    data: { version: 2, pins: [
      { id: 7, anchor: '{}', created_at: 'x', messages: [
        { id: 11, author_name: 'Léa', body: 'hi', created_at: 'a', updated_at: 'b' },
      ] },
    ] },
    error: undefined,
  })
  const a = createAdminAdapter(3, 2)
  const out = await a.list()
  expect(api.GET).toHaveBeenCalledWith('/api/projects/{id}/versions/{n}/comments', {
    params: { path: { id: 3, n: 2 } },
  })
  expect(out.pins[0].messages[0].editable).toBe(false)
  expect(out.version).toBe(2)
})

it('deleteMessage() appelle l’endpoint de modération', async () => {
  ;(api.DELETE as Mock).mockResolvedValue({ error: undefined })
  await createAdminAdapter(3, 2).deleteMessage(11)
  expect(api.DELETE).toHaveBeenCalledWith('/api/projects/{id}/comments/messages/{cid}', {
    params: { path: { id: 3, cid: 11 } },
  })
})

it('capabilities = canModerate seul', () => {
  expect(createAdminAdapter(1, 1).capabilities).toEqual({
    canAuthor: false, canEditOwn: false, canModerate: true,
  })
})

it('createPin/editMessage throw (non supporté en admin)', async () => {
  const a = createAdminAdapter(1, 1)
  await expect(a.createPin({ anchor: '', author_name: '', body: '' })).rejects.toThrow()
  await expect(a.editMessage(1, 'x')).rejects.toThrow()
})
```

- [ ] **Step 2 : Run → fail** : `pnpm vitest run src/comments/data/admin-adapter.test.ts` → FAIL (module absent).

- [ ] **Step 3 : Implémenter `admin-adapter.ts`**

```ts
import { api } from '@/api/client'
import type {
  Capabilities,
  CommentList,
  CommentMessage,
  CommentPin,
  CommentsAdapter,
} from './adapter'
import type { components } from '@/api/schema'

type AdminCommentMessage = components['schemas']['AdminCommentMessage']
type AdminCommentPin = components['schemas']['AdminCommentPin']

const ADMIN_CAPS: Capabilities = Object.freeze({
  canAuthor: false,
  canEditOwn: false,
  canModerate: true,
})

function toMessage(m: AdminCommentMessage): CommentMessage {
  return {
    id: m.id,
    author_name: m.author_name,
    body: m.body,
    created_at: m.created_at,
    updated_at: m.updated_at,
    editable: false,
  }
}

function toPin(p: AdminCommentPin): CommentPin {
  return {
    id: p.id,
    anchor: p.anchor,
    created_at: p.created_at,
    messages: p.messages.map(toMessage),
  }
}

const UNSUPPORTED = 'admin:unsupported'

export function createAdminAdapter(projectId: number, n: number): CommentsAdapter {
  return {
    capabilities: ADMIN_CAPS,

    async list(): Promise<CommentList> {
      const { data, error } = await api.GET('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
      })
      if (error || !data) throw new Error('comments:admin:list')
      return { version: data.version, pins: data.pins.map(toPin) }
    },

    async createPin() { throw new Error(UNSUPPORTED) },
    async addReply() { throw new Error(UNSUPPORTED) },
    async editMessage() { throw new Error(UNSUPPORTED) },
    async deletePin() { throw new Error(UNSUPPORTED) },

    async deleteMessage(messageId: number): Promise<void> {
      const { error } = await api.DELETE('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
      })
      if (error) throw new Error('comments:admin:deleteMessage')
    },
  }
}
```

- [ ] **Step 4 : Run → pass** : `pnpm vitest run src/comments/data/admin-adapter.test.ts` → PASS.

- [ ] **Step 5 : Gate + commit**

```bash
pnpm lint && pnpm typecheck
git add frontend/src/comments/data/admin-adapter.ts frontend/src/comments/data/admin-adapter.test.ts
git commit -m "✨ feat(comments): adaptateur admin (list mappé + modération, canModerate)"
```

---

## Phase I — Toggle `comments_enabled` au `ProjectForm` (§10.1)

### Task I1 : toggle + smart default + warning + persistance

**Files:**
- Modify: `frontend/src/components/project-form.tsx`
- Modify: `frontend/src/i18n/locales/admin/{en,fr}.json` (clés `form.comments`, `form.comments_help`, `form.comments_warn_code_off`)
- Test: `frontend/src/components/project-form.test.tsx`

**Interfaces:**
- Consumes : `useCreateProject`/`useUpdateProject` (le body accepte `comments_enabled?: boolean | null` — déjà dans `CreateProjectReq`/`UpdateProjectReq`). `ProjectDetail.comments_enabled` (lecture seed édition).
- Produces : `FormValues` gagne `comments_enabled: boolean` + `commentsTouched` (suivi du smart default).

Règles §10.1 :
- **Création** : `comments_enabled` suit `code_enabled` en direct **tant que l’admin n’a pas touché** le toggle commentaires. Implémenter via un flag local `commentsTouched` (state) : `useEffect` qui, sur changement de `codeEnabled`, fait `setValue('comments_enabled', codeEnabled)` **si `!commentsTouched`**. Le `onCheckedChange` du toggle commentaires met `commentsTouched=true`.
- **Édition** : toggles indépendants ; seed depuis `project.comments_enabled`. **Avertissement inline** affiché si `comments_enabled === true && code_enabled === false` (retrait du code avec commentaires actifs) : « Sans code d’accès, les commentaires sont publics en écriture (protégés par anti-spam). » **Zéro flip silencieux** (on ne désactive jamais les commentaires automatiquement).
- **Persistance** : ajouter `comments_enabled: values.comments_enabled` au body de `createProject.mutate` ET `updateProject.mutate`.

- [ ] **Step 1 : Tests (3 comportements)**

Dans `project-form.test.tsx` :
```tsx
it('création : comments suit le code tant que non touché', async () => {
  renderForm({ mode: 'create' })
  const code = screen.getByRole('switch', { name: /code/i })
  const comments = screen.getByRole('switch', { name: /comment/i })
  expect(comments).toBeChecked()           // code ON par défaut → comments ON
  await userEvent.click(code)              // code OFF
  expect(comments).not.toBeChecked()       // suit
})

it('création : une fois touché, comments n’est plus piloté par le code', async () => {
  renderForm({ mode: 'create' })
  const code = screen.getByRole('switch', { name: /code/i })
  const comments = screen.getByRole('switch', { name: /comment/i })
  await userEvent.click(comments)          // touché → false
  await userEvent.click(code)              // code OFF
  expect(comments).not.toBeChecked()
  await userEvent.click(code)              // code ON
  expect(comments).not.toBeChecked()       // ne re-suit plus
})

it('édition : warning si commentaires ON et code passé OFF', async () => {
  renderForm({ mode: 'edit', project: { ...baseProject, code_enabled: true, comments_enabled: true } })
  await userEvent.click(screen.getByRole('switch', { name: /code/i })) // code → OFF
  expect(screen.getByText(/anti-spam|anti-?spam|public/i)).toBeInTheDocument()
})

it('soumission création envoie comments_enabled', async () => {
  const create = mockCreate()
  renderForm({ mode: 'create' })
  await userEvent.type(screen.getByLabelText(/name|nom/i), 'Mon Projet')
  await userEvent.click(screen.getByRole('button', { name: /save|enregistrer/i }))
  expect(create).toHaveBeenCalledWith(
    expect.objectContaining({ comments_enabled: true }),
    expect.anything(),
  )
})
```
(Réutiliser les helpers de rendu existants du fichier ; `mockCreate`/`renderForm` à aligner sur le style en place. `baseProject` = `ProjectDetail` factice avec placeholders fictifs.)

- [ ] **Step 2 : Run → fail** : `pnpm vitest run src/components/project-form.test.tsx` → FAIL.

- [ ] **Step 3 : i18n (3 clés, EN + FR)**

`admin/en.json` (ajouter aux clés `form.*`) :
```json
"form.comments": "Comments",
"form.comments_help": "Let reviewers leave anchored comments on the prototype.",
"form.comments_warn_code_off": "Without an access code, comments are publicly writable (spam-protected)."
```
`admin/fr.json` :
```json
"form.comments": "Commentaires",
"form.comments_help": "Permettre aux relecteurs de laisser des commentaires ancrés sur le prototype.",
"form.comments_warn_code_off": "Sans code d’accès, les commentaires sont publics en écriture (protégés par anti-spam)."
```

- [ ] **Step 4 : Implémenter le toggle**

- Ajouter `comments_enabled: boolean` à `FormValues`, au `schema` zod (`z.boolean()`), aux `defaultValues` (`true`), et aux deux branches du `reset` (`mode === 'edit'` → `project.comments_enabled` ; create → `true`).
- Ajouter `const [commentsTouched, setCommentsTouched] = useState(false)` ; réinitialiser `setCommentsTouched(false)` dans le `useEffect` d’ouverture (à chaque (re)open).
- `const commentsEnabled = useWatch({ control, name: 'comments_enabled' }) ?? true`.
- Smart default (effet dédié, gardé pour éviter `set-state-in-effect` sec — utiliser une condition) :
  ```tsx
  useEffect(() => {
    if (mode === 'create' && !commentsTouched) {
      setValue('comments_enabled', codeEnabled)
    }
  }, [codeEnabled, commentsTouched, mode, setValue])
  ```
- Bloc UI après le toggle code :
  ```tsx
  <div className="flex flex-col gap-1.5">
    <div className="flex items-center justify-between">
      <Label htmlFor="project-comments">{t('form.comments')}</Label>
      <Switch
        id="project-comments"
        checked={commentsEnabled}
        onCheckedChange={(checked) => {
          setCommentsTouched(true)
          setValue('comments_enabled', checked, { shouldValidate: true })
        }}
      />
    </div>
    <p className="text-muted-foreground text-xs">{t('form.comments_help')}</p>
    {commentsEnabled && !codeEnabled && (
      <p className="text-xs text-amber-600 dark:text-amber-500">
        {t('form.comments_warn_code_off')}
      </p>
    )}
  </div>
  ```
- Ajouter `comments_enabled: values.comments_enabled` aux deux `mutate` (create body + update body).

- [ ] **Step 5 : Run → pass + gate**

Run : `pnpm lint && pnpm typecheck && pnpm vitest run src/components/project-form.test.tsx` → PASS.

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/components/project-form.tsx frontend/src/components/project-form.test.tsx frontend/src/i18n/locales/admin/en.json frontend/src/i18n/locales/admin/fr.json
git commit -m "✨ feat(admin): toggle comments_enabled au formulaire projet (smart default + warning)"
```

---

## Phase J — Liste textuelle modérable (§10.2)

### Task J1 : hooks `useVersionComments` + `useModerateComment`

**Files:**
- Create: `frontend/src/hooks/use-version-comments.ts`
- Test: `frontend/src/hooks/use-version-comments.test.tsx`

**Interfaces:**
- Consumes : `api` (`@/api/client`), React Query (le repo a un `QueryClient` admin global — réutiliser le provider de test existant). Type `AdminCommentList`.
- Produces :
  ```ts
  export function versionCommentsKey(projectId: number, n: number): unknown[]
  export function useVersionComments(projectId: number, n: number): UseQueryResult<AdminCommentList>
  export function useModerateComment(projectId: number, n: number): UseMutationResult<void, Error, number>
  ```
  `useModerateComment` supprime un message (`DELETE …/comments/messages/{cid}`) et invalide `versionCommentsKey(projectId,n)` **et** la query projet (pour rafraîchir `comment_count`). Clé projet existante : repérer la clé utilisée par `useProject(id)` dans `hooks/use-projects.ts` et invalider la même.

- [ ] **Step 1 : Test (succès + invalidation)**

```tsx
it('useVersionComments fetch la liste admin', async () => {
  server.use(http.get('/api/projects/3/versions/2/comments', () =>
    HttpResponse.json({ version: 2, pins: [] })))
  const { result } = renderHook(() => useVersionComments(3, 2), { wrapper })
  await waitFor(() => expect(result.current.isSuccess).toBe(true))
  expect(result.current.data?.version).toBe(2)
})

it('useModerateComment DELETE le message', async () => {
  let called = false
  server.use(http.delete('/api/projects/3/comments/messages/11', () => {
    called = true
    return HttpResponse.json({ ok: true })
  }))
  const { result } = renderHook(() => useModerateComment(3, 2), { wrapper })
  await result.current.mutateAsync(11)
  expect(called).toBe(true)
})
```
(Utiliser le `wrapper` MSW + QueryClient du repo ; `server` = setup MSW existant.)

- [ ] **Step 2 : Run → fail** : `pnpm vitest run src/hooks/use-version-comments.test.tsx` → FAIL.

- [ ] **Step 3 : Implémenter**

```ts
import { useMutation, useQuery, useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api/client'
import type { components } from '@/api/schema'

type AdminCommentList = components['schemas']['AdminCommentList']

export function versionCommentsKey(projectId: number, n: number): unknown[] {
  return ['admin-version-comments', projectId, n]
}

export function useVersionComments(projectId: number, n: number): UseQueryResult<AdminCommentList> {
  return useQuery({
    queryKey: versionCommentsKey(projectId, n),
    queryFn: async () => {
      const { data, error } = await api.GET('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
      })
      if (error || !data) throw new Error('admin:version-comments')
      return data
    },
  })
}

export function useModerateComment(projectId: number, n: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (messageId: number) => {
      const { error } = await api.DELETE('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
      })
      if (error) throw new Error('admin:moderate')
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: versionCommentsKey(projectId, n) })
      void qc.invalidateQueries({ queryKey: ['project', projectId] }) // aligner sur use-projects.ts
    },
  })
}
```
> **Vérifier** la clé exacte de `useProject` dans `hooks/use-projects.ts` et la répliquer pour l’invalidation `comment_count`.

- [ ] **Step 4 : Run → pass + gate** : `pnpm lint && pnpm typecheck && pnpm vitest run src/hooks/use-version-comments.test.tsx` → PASS.

- [ ] **Step 5 : Commit**

```bash
git add frontend/src/hooks/use-version-comments.ts frontend/src/hooks/use-version-comments.test.tsx
git commit -m "✨ feat(admin): hooks useVersionComments + modération"
```

### Task J2 : `VersionCommentsPanel` + action sur la ligne de version

**Files:**
- Create: `frontend/src/components/version-comments-panel.tsx`
- Modify: `frontend/src/routes/detail.tsx` (action « Commentaires » par ligne)
- Modify: `frontend/src/i18n/locales/admin/{en,fr}.json` (clés `version_comments.*`)
- Test: `frontend/src/components/version-comments-panel.test.tsx`

**Interfaces:**
- Consumes : `useVersionComments`, `useModerateComment` (J1) ; `Sheet*` (`ui/sheet`) ; type `AdminCommentPin`/`AdminCommentMessage`. Helper d’ancrage lisible (dérivé du JSON `anchor`) — implémenter inline :
  ```ts
  function anchorLabel(anchorJson: string): string {
    try {
      const a = JSON.parse(anchorJson) as { fingerprint?: { tag?: string; text?: string } }
      const tag = a.fingerprint?.tag ?? 'element'
      const text = a.fingerprint?.text
      return text ? `${tag} — “${text}”` : tag
    } catch { return 'element' }
  }
  ```
- Produces :
  ```tsx
  interface VersionCommentsPanelProps {
    projectId: number
    version: number        // n
    open: boolean
    onOpenChange: (open: boolean) => void
  }
  export function VersionCommentsPanel(props: Readonly<VersionCommentsPanelProps>): JSX.Element
  ```
  `<Sheet>` lecture seule calqué sur `version-detail-panel.tsx`. Pins groupés ; chaque pin = carte (repère `anchorLabel(anchor)` + fil : `author_name`, `body` texte brut échappé, dates). Corbeille par message → `useModerateComment(projectId, version).mutate(messageId)` avec confirmation (utiliser un `window.confirm` est interdit — réutiliser le pattern de confirmation inline déjà présent dans `thread-popup`/`delete-version-panel` : un état `confirmingId`). Le panneau gère vide (`version_comments.empty`) et loading.

- [ ] **Step 1 : i18n (clés EN + FR)**

`admin/en.json` :
```json
"version_comments.title": "Comments — v{{n}}",
"version_comments.empty": "No comments on this version.",
"version_comments.loading": "Loading comments…",
"version_comments.anchor_label": "Anchor",
"version_comments.delete_aria": "Delete this message",
"version_comments.confirm_delete": "Delete this message?",
"version_comments.confirm_yes": "Delete",
"version_comments.confirm_no": "Cancel",
"version_comments.action": "Comments",
"detail.comments_aria": "View comments for this version"
```
`admin/fr.json` :
```json
"version_comments.title": "Commentaires — v{{n}}",
"version_comments.empty": "Aucun commentaire sur cette version.",
"version_comments.loading": "Chargement des commentaires…",
"version_comments.anchor_label": "Ancrage",
"version_comments.delete_aria": "Supprimer ce message",
"version_comments.confirm_delete": "Supprimer ce message ?",
"version_comments.confirm_yes": "Supprimer",
"version_comments.confirm_no": "Annuler",
"version_comments.action": "Commentaires",
"detail.comments_aria": "Voir les commentaires de cette version"
```

- [ ] **Step 2 : Test du panneau**

```tsx
it('liste les pins et permet la modération', async () => {
  server.use(
    http.get('/api/projects/3/versions/2/comments', () => HttpResponse.json({
      version: 2, pins: [
        { id: 7, anchor: JSON.stringify({ fingerprint: { tag: 'button', text: 'En savoir plus' } }),
          created_at: '2026-06-30T10:00:00Z', messages: [
            { id: 11, author_name: 'Léa', body: 'à revoir', created_at: '2026-06-30T10:00:00Z', updated_at: '…' },
          ] },
      ],
    })),
    http.delete('/api/projects/3/comments/messages/11', () => HttpResponse.json({ ok: true })),
  )
  render(<VersionCommentsPanel projectId={3} version={2} open onOpenChange={vi.fn()} />, { wrapper })
  expect(await screen.findByText(/En savoir plus/)).toBeInTheDocument()
  expect(screen.getByText('à revoir')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: /delete|supprimer/i }))
  await userEvent.click(screen.getByRole('button', { name: /^delete$|^supprimer$/i })) // confirm
  await waitFor(() => expect(screen.queryByText('à revoir')).toBeNull())
})

it('affiche l’état vide', async () => {
  server.use(http.get('/api/projects/3/versions/9/comments', () =>
    HttpResponse.json({ version: 9, pins: [] })))
  render(<VersionCommentsPanel projectId={3} version={9} open onOpenChange={vi.fn()} />, { wrapper })
  expect(await screen.findByText(/no comments|aucun commentaire/i)).toBeInTheDocument()
})
```

- [ ] **Step 3 : Run → fail** : `pnpm vitest run src/components/version-comments-panel.test.tsx` → FAIL.

- [ ] **Step 4 : Implémenter `version-comments-panel.tsx`**

Calquer la structure de `version-detail-panel.tsx` (Sheet read-only). Utiliser `useVersionComments(projectId, version)` + `useModerateComment(projectId, version)`. Rendre `body` en texte brut (`<p>{m.body}</p>` — JSX échappe). Confirmation inline via état local `confirmingId: number | null`. Titre via `t('version_comments.title', { n: version })`.

- [ ] **Step 5 : Câbler l’action dans `detail.tsx`**

- Importer `VersionCommentsPanel` + l’icône `MessageSquare` (lucide).
- State : `const [commentsVersion, setCommentsVersion] = useState<VersionItem | null>(null)`.
- Dans la cellule d’actions de chaque ligne (à côté de Detail/Preview), ajouter **avant** Preview :
  ```tsx
  <Button
    type="button" variant="ghost" size="icon-sm"
    aria-label={t('detail.comments_aria')} title={t('version_comments.action')}
    disabled={v.comment_count === 0}
    onClick={() => setCommentsVersion(v)}
  >
    <MessageSquare />
  </Button>
  ```
- Monter le panneau près des autres :
  ```tsx
  {commentsVersion && (
    <VersionCommentsPanel
      projectId={id}
      version={commentsVersion.n}
      open={commentsVersion !== null}
      onOpenChange={(o) => { if (!o) setCommentsVersion(null) }}
    />
  )}
  ```

- [ ] **Step 6 : Run → pass + gate**

Run : `pnpm lint && pnpm typecheck && pnpm vitest run src/components/version-comments-panel.test.tsx src/routes` → PASS.

- [ ] **Step 7 : Commit**

```bash
git add frontend/src/components/version-comments-panel.tsx frontend/src/components/version-comments-panel.test.tsx frontend/src/routes/detail.tsx frontend/src/i18n/locales/admin/en.json frontend/src/i18n/locales/admin/fr.json
git commit -m "✨ feat(admin): panneau liste de commentaires par version + modération"
```

---

## Phase K — Mode Review (§10.3)

### Task K1 : durcissement backend — `frame-ancestors 'self'` sur la preview admin

**Files:**
- Modify: `backend/src/controllers/admin.rs:407-420` (handler `preview_version`)
- Test: `backend/tests/` — repérer le test d’intégration de la preview (grep `preview`) et y ajouter l’assertion ; sinon ajouter dans le fichier de tests admin.

**Interfaces:** aucun changement de signature, d’OpenAPI ni de DTO. Header HTTP seulement.

- [ ] **Step 1 : Test (assert header)**

Ajouter dans le test d’intégration qui frappe `GET /api/projects/{id}/versions/{n}/preview` (login admin → deploy → preview) :
```rust
assert_eq!(
    res.headers().get("content-security-policy").map(|v| v.to_str().unwrap()),
    Some("frame-ancestors 'self'"),
);
```

- [ ] **Step 2 : Run → fail** (depuis racine) : `cargo nextest run -p latch preview` → FAIL (header absent).

- [ ] **Step 3 : Ajouter le header**

Dans `preview_version`, ajouter la 3ᵉ entrée au tableau de headers (calqué sur `serve.rs::raw_html_response`) :
```rust
(
    axum::http::header::CONTENT_SECURITY_POLICY,
    axum::http::HeaderValue::from_static("frame-ancestors 'self'"),
),
```

- [ ] **Step 4 : Run → pass + gate backend**

Run (racine) : `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run -p latch preview` → PASS.

- [ ] **Step 5 : Commit**

```bash
git add backend/src/controllers/admin.rs backend/tests
git commit -m "🔒 hard(admin): frame-ancestors 'self' sur la preview (encadrement Review same-origin)"
```

### Task K2 : route SPA Review + page

**Files:**
- Create: `frontend/src/routes/review.tsx`
- Modify: `frontend/src/router.tsx` (route `/projects/$id/versions/$n/review`)
- Modify: `frontend/src/lib/utils.ts` (helper `reviewPath`)
- Modify: `frontend/src/routes/detail.tsx` (lien « Review » par ligne)
- Modify: `frontend/src/i18n/locales/admin/{en,fr}.json` (clés `review.*`)
- Test: `frontend/src/routes/review.test.tsx`

**Interfaces:**
- Consumes : `previewUrl(id, n)` (`lib/utils.ts`), `CommentsApp` (default export lazy `@/comments`), `createAdminAdapter(id, n)` (H1), `useParams` (TanStack).
- Produces :
  ```ts
  // lib/utils.ts
  export function reviewPath(projectId: number, n: number): string // '/projects/{id}/versions/{n}/review' (sans basepath)
  ```
  Page Review : layout plein écran avec `<iframe src={previewUrl(id, n)} title="…">` + `<CommentsApp cacheKey={`admin:${id}:${n}`} frame={...} adapter={createAdminAdapter(id, n)} />` monté en overlay (`React.lazy` + `Suspense`, comme le shell). Le `frame` est une `FrameRef` pointant l’iframe (réutiliser le même contrat que le shell — repérer comment `comments-mount.tsx` construit la `FrameRef` et le répliquer).

- [ ] **Step 1 : i18n (EN + FR)**

`admin/en.json` :
```json
"review.title": "Review — {{name}}",
"review.action": "Review",
"review.back": "← Back to project",
"detail.review_aria": "Open review mode for this version"
```
`admin/fr.json` :
```json
"review.title": "Review — {{name}}",
"review.action": "Review",
"review.back": "← Retour au projet",
"detail.review_aria": "Ouvrir le mode review de cette version"
```

- [ ] **Step 2 : Test de la page**

```tsx
it('monte l’iframe de preview et la couche commentaire admin', async () => {
  render(<ReviewPage />, { wrapper: routerWrapper({ id: '3', n: '2' }) })
  const frame = await screen.findByTitle(/review|preview|prototype/i)
  expect(frame).toHaveAttribute('src', expect.stringContaining('/api/projects/3/versions/2/preview'))
})
```
(Stub `@/comments` default export par un composant trivial dans le test pour éviter de charger tout le module lazy ; vérifier que `ReviewPage` lit bien `id`/`n` des params et calcule `previewUrl`.)

- [ ] **Step 3 : Run → fail** : `pnpm vitest run src/routes/review.test.tsx` → FAIL.

- [ ] **Step 4 : `reviewPath` dans `lib/utils.ts`**

```ts
export function reviewPath(projectId: number, n: number): string {
  return `/projects/${projectId}/versions/${n}/review`
}
```

- [ ] **Step 5 : Implémenter `review.tsx`**

Page plein écran : breadcrumb retour (`t('review.back')` → `router.navigate({ to: '/projects/$id', params: { id } })`), un conteneur `relative` contenant l’`<iframe>` plein cadre + le montage lazy de `CommentsApp` (overlay). Construire la `FrameRef` comme le shell (réf à l’iframe + accès `contentWindow`/`contentDocument` same-origin). Adapter `createAdminAdapter(Number(id), Number(n))`, `cacheKey={`admin:${id}:${n}`}`.

> **Note iframe** : la preview est same-origin (`/api/...`), donc `SameOriginPicker` lit `contentDocument`. Le header `frame-ancestors 'self'` (K1) autorise l’encadrement. Attendre l’event `load` de l’iframe avant de monter `CommentsApp` (même `key`-bump que le shell).

- [ ] **Step 6 : Brancher la route dans `router.tsx`**

```tsx
import { ReviewPage } from './routes/review'
const reviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id/versions/$n/review',
  component: ReviewPage,
})
const routeTree = rootRoute.addChildren([loginRoute, listRoute, detailRoute, reviewRoute])
```

- [ ] **Step 7 : Lien « Review » dans `detail.tsx`**

Ajouter une action (icône `Eye`/`SquarePen` lucide, distincte de Preview) par ligne, qui navigue :
```tsx
<Button asChild variant="ghost" size="icon-sm"
  aria-label={t('detail.review_aria')} title={t('review.action')}>
  <Link to="/projects/$id/versions/$n/review" params={{ id: String(id), n: String(v.n) }}>
    <MessagesSquare />
  </Link>
</Button>
```
(Importer `Link` de `@tanstack/react-router` et l’icône.)

- [ ] **Step 8 : Run → pass + gate**

Run : `pnpm lint && pnpm typecheck && pnpm vitest run src/routes` → PASS.

- [ ] **Step 9 : Commit**

```bash
git add frontend/src/routes/review.tsx frontend/src/routes/review.test.tsx frontend/src/router.tsx frontend/src/lib/utils.ts frontend/src/routes/detail.tsx frontend/src/i18n/locales/admin/en.json frontend/src/i18n/locales/admin/fr.json
git commit -m "✨ feat(admin): page Review (proto encadré + overlay commentaires admin)"
```

---

## Phase L — e2e Playwright (§12)

### Task L1 : e2e admin (toggle + Review + modération)

**Files:**
- Create: `frontend/e2e/comments-admin.spec.ts`
- (Référence helpers : `frontend/e2e/serve-unlock.spec.ts` — `apiLogin`/`createProject`/`deploy` inline ; `createProject` accepte `comments_enabled`.)

**Interfaces:** Playwright `baseURL 127.0.0.1:5150`, webServer auto (`pnpm build && cargo loco start`), `reuseExistingServer` hors CI, `workers:1`. testids module : `comments-mount`, `pick-surface`, `data-status`. Admin login `ADMIN_USER=admin/ADMIN_PASS=secret`.

- [ ] **Step 1 : Écrire le scénario**

Flux réaliste, sans dépendre de l’UI visiteur pour seeder (poster un commentaire via l’API `/c/{slug}/comments` directement, en posant le cookie d’identité + header `X-Comment-Client`, OU réutiliser le helper visiteur de `comments.spec.ts`) :
```ts
import { test, expect } from '@playwright/test'
// helpers inline (login admin, createProject{comments_enabled:true}, deploy) repris de serve-unlock.spec.ts

test('admin : toggle comments, Review montre le pin, modération le supprime', async ({ page, request }) => {
  // 1. login admin + créer projet comments_enabled:true + déployer un proto HTML simple
  // 2. seed 1 commentaire via l’API publique (anchor minimal + author_name + body) sur la version active
  // 3. naviguer /admin/projects/{id} → cliquer l’action Review de la version
  // 4. attendre l’iframe preview + le montage [data-testid=comments-mount]
  // 5. activer l’affichage des pastilles → assert au moins 1 pin positionné visible
  // 6. ouvrir le fil → supprimer (modération) → le pin disparaît
  // 7. rouvrir le panneau liste depuis detail → assert vide / compteur à 0
})
```
> Si l’ancrage via API headless est trop fragile (le `resolve` dépend du DOM proto), préférer : poster le commentaire **via l’UI visiteur** réutilisant le flux de `comments.spec.ts` (pick souris), puis basculer en admin dans le même test. Choisir l’option qui passe de façon déterministe ; documenter le choix dans le ledger.

- [ ] **Step 2 : Lancer l’e2e**

Run (depuis `frontend/`): `pnpm exec playwright test comments-admin` (à froid, prévoir le build ; en itération `CI=1` pour réutiliser le serveur).
Expected : 1 passed.

- [ ] **Step 3 : Commit**

```bash
git add frontend/e2e/comments-admin.spec.ts
git commit -m "✅ test(e2e): parcours admin Review + modération des commentaires"
```

---

## Phase M — Docs Fumadocs (§13)

### Task M1 : nouvelle page « Commenter un prototype » + passe sur l’existant

**Files:**
- Create: `public_docs/content/docs/admin/comments.mdx` (ou section visiteur — choisir l’emplacement cohérent avec `meta.json` voisin ; ajouter au `meta.json` du dossier).
- Modify: `public_docs/content/docs/how-it-works/architecture.mdx` (le shell héberge la couche commentaire).
- Modify: `public_docs/content/docs/how-it-works/security-model.mdx` (cookie d’identité `latch_comment`, gating commentaires, invariant `owner_token`).
- Modify: `public_docs/content/docs/admin/projects.mdx` (toggle `comments_enabled`).
- Modify: `public_docs/content/docs/admin/versions.mdx` (liste de commentaires + Review + modération).

**Contraintes MDX (cf. QUIRKS §Fumadocs)** : `{…}` = JS, `<mot>` = balise → mettre `<slug>`, `{token}` etc. en backticks. Liens internes **root-relative** (`/docs/...`), jamais `/latch` en dur. Images via import statique (le fichier doit exister au build). Blocs sans grammaire Shiki → ` ```text `.

- [ ] **Step 1 : Rédiger la page neuve**

Page « Commenter un prototype » : ce que voit le relecteur (barre d’action 3 boutons, mode pick, fil, persistance par version, privé), prérequis (`comments_enabled`), confidentialité (chaque relecteur ne voit que ses fils). Section admin : liste par version + Review + modération. Placeholders fictifs uniquement.

- [ ] **Step 2 : Passe sur l’existant** — insérer les paragraphes décrits ci-dessus dans les 4 pages, en respectant le ton et la structure des pages voisines.

- [ ] **Step 3 : Build docs**

Run (depuis `public_docs/`): `pnpm build` (export statique). Expected : build OK (aucune erreur MDX/JSX, pas d’image manquante).

- [ ] **Step 4 : Commit**

```bash
git add public_docs/content/docs
git commit -m "📝 docs(public): commentaires ancrés — page visiteur + passe architecture/sécurité/admin"
```

---

## Phase N — Gate complète, Sonar & mémoire

### Task N1 : gate finale + Sonar local + docs mémoire

**Files:** `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`, `docs/ENVIRONMENT.md`, `docs/contrat-deploy.md`, `.superpowers/sdd/progress.md`.

- [ ] **Step 1 : Gate complète**

```bash
# Frontend (depuis frontend/)
pnpm lint && pnpm typecheck && pnpm test && pnpm exec playwright test
# Backend (depuis racine, si K1 touché)
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run
```
Expected : tout vert. Noter les compteurs (vitest N passed, playwright M passed).

- [ ] **Step 2 : Scan Sonar local** (cf. `docs/ENVIRONMENT.md §Scan local`)

Produire la couverture front (`pnpm test:cov`) + remap lcov backend si K1 (`sed -i "s#$(pwd)/#/usr/src/#g" backend-lcov.info`), lancer le scanner Docker, vérifier `new_coverage ≥ 80 %` sur le code neuf. Couvrir tout fichier neuf sous le seuil avant de clore.

- [ ] **Step 3 : Mémoire projet**

- `docs/INDEX.md` : lignes « Toggle `comments_enabled` admin », « Liste commentaires + modération », « Page Review admin », « Docs publiques commentaires » — Plan 3 — 2026-06-30.
- `docs/HANDOFF.md` : entrée datée en haut (Plan 3 livré ; feature commentaires **terminée bout-en-bout** ; compteurs de gate ; trucs en suspens éventuels).
- `docs/CONVENTIONS.md` : `CommentsApp` adaptateur injectable + `createAdminAdapter` + `cacheKey` ; pattern Review.
- `docs/QUIRKS.md` : pièges rencontrés (FrameRef admin, iframe preview load, etc.).
- `docs/ENVIRONMENT.md` : route Review `/admin/projects/{id}/versions/{n}/review`.
- `docs/contrat-deploy.md` : vérifier §7/§10 cohérents (la vue admin liste + Review + modération existe désormais réellement).
- `.superpowers/sdd/progress.md` : ledger Plan 3 à jour.

- [ ] **Step 4 : Commit mémoire**

```bash
git add docs .superpowers/sdd/progress.md
git commit -m "📝 docs(memory): Plan 3 commentaires (admin Review + toggle + docs) livré"
```

---

## Self-Review (couverture spec)

- §10.1 toggle `comments_enabled` + smart default + warning → **Task I1**. ✅
- §10.2 liste textuelle + modération + action ligne (disabled si `comment_count===0`) + repère d’ancrage → **Task J1/J2**. ✅
- §10.3 route Review + iframe preview + module overlay admin + `frame-ancestors 'self'` → **Task K1/K2**. ✅
- §8.8 adaptateur admin + capabilities `canModerate` → **Task H1** (+ seam injectable **G1**, modération **G2**). ✅
- §11 i18n `version_comments.*` / `review.*` / `form.comments*` → **I1/J2/K2**. ✅
- §12 e2e admin Review + modération → **Task L1**. ✅
- §13 docs Fumadocs (page neuve + passe existant) → **Task M1**. ✅
- §15 « terminé » : gate complète + Sonar + mémoire → **Task N1**. ✅

**Invariants** : `owner_token` jamais reçu (admin DTO sans `editable`/`owner_token`, mapping `editable:false`) — préservé. Texte brut — préservé. Régression visiteur couverte par G1 (suite verte) + e2e existant.
