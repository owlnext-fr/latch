# Phase 7 — Lot 2 : Panneau Settings unifié (side-panel)

> Design doc. Deuxième des 4 lots de la Phase 7 (« Peaufinage graphique / web »).
> Dépend du Lot 1 (i18n auto-découvert + `ThemeProvider` montés).
> Statut : design validé (brainstorming 2026-06-25), à implémenter via un plan dédié.

## Contexte & motivation

Le Lot 1 a livré les fondations : i18n auto-découvert exposant `locales: LocaleInfo[]`
(`{ code, name, flag }`) et un `ThemeProvider` (next-themes, défaut `system`, storageKey
`latch.theme`) monté sur le bundle admin. Ce lot construit dessus le **panneau de réglages
unifié** demandé par le ROADMAP : transformer la route Settings plein écran en **side-panel**
(`<Sheet>`) regroupant infos MCP + **vrai sélecteur de langue (avec drapeaux)** + **toggle de
thème**, chaque réglage **explicité par un helper text**.

### État actuel (constaté)

- **Settings** = route plein écran `/settings` (`routes/settings.tsx`, `mx-auto max-w-2xl`),
  atteinte par l'icône Settings de la topbar via `router.navigate({ to: '/settings' })`.
  Affiche : `mcp_url` (copiable), `deploy_token` (`PinField`), `public_base_url`. Données via
  `useSettings()` (`/api/settings`). Clés i18n : `settings.title/mcp_intro/mcp_url/deploy_token/
  public_base_url/copy_mcp_url`.
- **`LocaleSwitcher`** (`components/locale-switcher.tsx`) = toggle 2 boutons FR/EN, **liste en
  dur `['en','fr']`** (dernier reliquat signalé par la revue Lot 1). Rendu dans la **topbar**
  ET sur la **page login**.
- **Pattern Sheet** rodé (`project-form.tsx`, `deploy-panel.tsx`…) : `<Sheet open onOpenChange>`,
  `<SheetContent className="w-full overflow-y-auto sm:max-w-md">`, corps `flex flex-col gap-5 p-4`,
  helper text `text-muted-foreground text-xs` sous chaque champ.
- **Composants UI** : pas de `Select` vendorisé. Le package unifié `radix-ui` (^1.6.0) est en
  dépendance — `ui/sheet.tsx` vendorise déjà `Dialog` depuis `radix-ui`. Le primitive `Select`
  s'y trouve aussi.
- **Thème** : `useTheme()/setTheme()` disponibles (provider monté au Lot 1). `sonner.tsx` lit
  déjà `useTheme()`.

## Décisions de design (tranchées au brainstorming)

| # | Décision | Choix retenu |
|---|---|---|
| D1 | Rendu des drapeaux | **`flag-icons`** (SVG, cohérent tous OS — les emojis drapeaux ne s'affichent pas sous Windows). Classe `fi fi-<code>`. |
| D2 | Contrôle de thème | **Segmented 3 boutons** (Système / Clair / Sombre, icônes lucide). |
| D3 | Structure | **Sheet seul** : route `/settings` **supprimée**, l'icône topbar ouvre le Sheet. `LocaleSwitcher` **retiré de la topbar**. La page **login** garde un sélecteur léger, **refactoré pour lire `locales`** (élimine le dernier `['en','fr']`). |
| D4 | Ordre des sections | **Connexion MCP en premier**, **Préférences en second** (choix utilisateur). |
| D5 | Pas de bouton « Enregistrer » | Langue/thème = préférences à effet immédiat (persistées seules) ; MCP = lecture seule. Le Sheet est un panneau d'état/préférences, pas un formulaire. |

## Objectifs (ce que le lot livre)

1. **Panneau Settings en side-panel** (`<Sheet>`) ouvert depuis l'icône topbar ; route
   `/settings` retirée.
2. **Vrai sélecteur de langue** (`Select` + drapeaux SVG), peuplé depuis `locales` (déposer un
   JSON admin → la langue apparaît, zéro code).
3. **Toggle de thème** 3 états (système/clair/sombre), persistant, « Système » correctement
   surligné.
4. **Helper text par réglage** (langue, thème, et chaque info MCP).
5. **`LocaleSwitcher` dé-hardcodé** (dérivé de `locales`), conservé pour le login uniquement.

### Non-objectifs (hors lot)

- Logo, titres dynamiques, largeur admin, bouton GitHub login, page d'erreur serving → autres lots.
- Thème sur l'unlock / surfaces publiques → reste hors périmètre (Lot 4 décidera).
- Subset/tree-shaking des drapeaux flag-icons → BACKLOG si la taille du bundle admin devient un sujet.

## Architecture

### Fichiers

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/src/components/ui/select.tsx` | Wrapper shadcn du `Select` radix (style d'import calqué sur `ui/sheet.tsx`) | **Créer** |
| `frontend/src/components/language-select.tsx` | `Select` peuplé depuis `locales`, rend drapeau + nom, `onValueChange → i18n.changeLanguage`. Importe la CSS flag-icons. | **Créer** |
| `frontend/src/components/theme-toggle.tsx` | Segmented 3 boutons via `useTheme()/setTheme()` | **Créer** |
| `frontend/src/components/settings-sheet.tsx` | Le panneau `<Sheet>` (sections MCP puis Préférences), `open`+`onOpenChange` | **Créer** |
| `frontend/src/components/topbar.tsx` | Icône Settings ouvre le Sheet (state local) ; retrait `LocaleSwitcher` | **Modifier** |
| `frontend/src/components/locale-switcher.tsx` | Dérive ses options de `locales` (supprime `['en','fr']`) ; conservé pour login | **Modifier** |
| `frontend/src/router.tsx` | Retrait de `settingsRoute` | **Modifier** |
| `frontend/src/routes/settings.tsx` | Contenu MCP migré dans `settings-sheet.tsx` | **Supprimer** |
| `frontend/src/hooks/use-settings.ts` | `enabled: isOpen` (fetch `/api/settings` à l'ouverture) | **Modifier** |
| `frontend/src/i18n/locales/admin/{en,fr}.json` | ~12 nouvelles clés `settings.*` | **Modifier** |
| `frontend/package.json` | Dépendance `flag-icons` | **Modifier** |
| `frontend/vitest.setup.ts` | Shims jsdom pour radix Select (`scrollIntoView`, `hasPointerCapture`, `releasePointerCapture`) | **Modifier** |
| Tests : `language-select.test.tsx`, `theme-toggle.test.tsx`, `settings-sheet.test.tsx`, `locale-switcher.test.tsx`, `topbar.test.tsx` (modifié) | Couverture | **Créer/Modifier** |

### Ancrage du Sheet

La `Topbar` est rendue par chaque page admin (`list`, `detail`). Elle porte l'état d'ouverture
(`useState`) et rend `<SettingsSheet open onOpenChange />`. Le panneau est ainsi disponible
partout sans toucher au routeur ni introduire un layout racine. La route `/settings` disparaît
du `router.tsx`.

### `useSettings` paresseux

`useSettings(enabled)` passe `enabled: isOpen` à `useQuery`. Conséquence importante : le
`deploy_token` (secret) n'est fetché qu'à **l'ouverture** du panneau — sans ce garde-fou, le
passage d'une route dédiée à un Sheet global déclencherait le fetch (et l'exposition du secret)
sur **toutes** les pages admin.

### `SettingsSheet` — contenu & layout

`SheetContent className="w-full overflow-y-auto sm:max-w-md"`, header `SheetTitle = t('settings.title')`,
corps `flex flex-col gap-5 p-4`. Deux sections (titres `text-xs font-medium text-muted-foreground
uppercase`), **MCP d'abord, Préférences ensuite** :

```
┌─ Réglages ─────────────────────────────── ✕ ─┐
│  CONNEXION MCP                                │
│  URL du endpoint MCP                           │
│  https://…/mcp                    [copier]    │
│  À renseigner dans le connecteur MCP Claude.  │
│                                               │
│  Deploy token              [•••••• 👁 copier] │
│  Secret validé par tous les tools MCP.        │
│                                               │
│  URL publique de base                         │
│  https://latch.owlnext.fr                     │
│  Racine publique de l'instance.               │
│  ───────────────────────────────────────────  │
│  PRÉFÉRENCES                                  │
│  Langue   [ ▱ fi-gb  English          ▾ ]     │
│  Langue de l'interface d'administration.      │
│                                               │
│  Thème   ┌────────┬───────┬────────┐          │
│          │▣ Syst. │ Clair │ Sombre │          │
│          └────────┴───────┴────────┘          │
│  « Système » suit la préférence de l'OS.      │
└───────────────────────────────────────────────┘
```

- **Connexion MCP** : contenu actuel de `settings.tsx` (mcp_url copiable, deploy_token via
  `PinField`, public_base_url) + **un helper text par champ**. États `loading`/`error` via
  `useSettings`.
- **Préférences** : `LanguageSelect` + `ThemeToggle`, chacun avec helper text. Effet immédiat,
  pas de `SheetFooter` d'actions.

### `LanguageSelect`

- `import { locales } from '@/i18n'` (Lot 1) → `locales.map(...)`, **zéro liste en dur**.
- Option : `<span className={`fi fi-${l.flag.toLowerCase()}`} />` + `l.name`, valeur `l.code`.
  (`flag` est `GB`/`FR` en ISO majuscule dans `_meta` → `.toLowerCase()` pour la classe `fi-gb`.)
- Courant = `i18n.language.slice(0,2)` ; `onValueChange={(code) => void i18n.changeLanguage(code)}`.
- Trigger : drapeau + nom de la langue active.
- `import 'flag-icons/css/flag-icons.min.css'` **dans ce module** (pas dans `index.css` partagé)
  → la CSS reste dans le bundle admin, l'unlock public n'est pas alourdi (invariant Lot 1).

### `ThemeToggle`

- `const { theme, setTheme } = useTheme()`. Lit **`theme`** (préférence : `system`/`light`/`dark`),
  **pas** `resolvedTheme` (sinon « Système » ne s'allume jamais).
- 3 boutons `system`/`light`/`dark`, icônes lucide `Monitor`/`Sun`/`Moon`, labels i18n.
- `variant={theme === v ? 'secondary' : 'ghost'}`, `aria-pressed`, `onClick={() => setTheme(v)}`.
- `fieldset` + `legend` sr-only (accessibilité), pattern de l'ancien `LocaleSwitcher`.

### `LocaleSwitcher` (refactor, conservé pour login)

- Dérive ses options de `locales` (`locales.map(l => l.code)`), supprime `const LOCALES = ['en','fr']`.
- Reste un toggle compact de boutons (code en majuscule), **sans drapeau** (léger, pré-auth).
- Retiré de la topbar ; conservé sur `routes/login.tsx`.

## i18n — clés à ajouter (admin `en` + `fr`)

On garde les 6 clés `settings.*` existantes. Ajouts :

| Clé | EN | FR |
|---|---|---|
| `settings.section_mcp` | MCP connection | Connexion MCP |
| `settings.section_preferences` | Preferences | Préférences |
| `settings.language` | Language | Langue |
| `settings.language_help` | Admin interface language. | Langue de l'interface d'administration. |
| `settings.theme` | Theme | Thème |
| `settings.theme_help` | "System" follows your OS preference. | « Système » suit la préférence de l'OS. |
| `settings.theme_system` | System | Système |
| `settings.theme_light` | Light | Clair |
| `settings.theme_dark` | Dark | Sombre |
| `settings.mcp_url_help` | Set this in Claude's MCP connector. | À renseigner dans le connecteur MCP de Claude. |
| `settings.deploy_token_help` | Secret validated by all MCP tools. | Secret validé par tous les tools MCP. |
| `settings.public_base_url_help` | Public root of this instance. | Racine publique de l'instance. |

Clés purement admin → uniquement dans `locales/admin/` (pas `unlock/`).

## flag-icons

- Dépendance `flag-icons` ajoutée au `package.json`.
- CSS importée **dans `language-select.tsx`** (scope bundle admin, hors unlock).
- Trade-off assumé : la CSS embarque les classes de tous les pays. Acceptable pour l'admin ;
  subset éventuel → BACKLOG.

## Plan de tests

Lot 100 % frontend. Vitest + Testing Library + MSW ; e2e Playwright reste vert.

- **`language-select.test.tsx`** : options dérivées de `locales` (en/fr), classe `fi-…` présente,
  sélection → `i18n.changeLanguage`, valeur courante reflète `i18n.language`.
- **`theme-toggle.test.tsx`** : 3 boutons, `aria-pressed` lit `theme` (pas `resolvedTheme`), clic
  → `setTheme(v)`. Wrap `ThemeProvider` + mock `matchMedia` (pattern `theme.test.tsx` Lot 1).
- **`settings-sheet.test.tsx`** : ouverture → champs MCP (MSW `/api/settings`), **helper text par
  réglage**, états loading/error, présence `LanguageSelect` + `ThemeToggle`.
- **`topbar.test.tsx`** (modifié) : icône Settings **ouvre le Sheet** (contenu du panneau visible)
  au lieu de naviguer ; plus de `LocaleSwitcher`.
- **`locale-switcher.test.tsx`** : options dérivées de `locales` (plus de `['en','fr']`), sélection
  → `changeLanguage`.
- Nettoyage des tests référençant la route `/settings`.

### Quirk anticipé — radix Select sous jsdom

Le `Select` radix utilise `scrollIntoView`, `hasPointerCapture`, `releasePointerCapture` (absents
de jsdom). Ajouter les shims dans `vitest.setup.ts` (comme les shims `ResizeObserver`/
`elementFromPoint` déjà présents). Les tests ciblent le **câblage** (option courante, `onValueChange`
→ `changeLanguage`), pas le cycle pointer interne de radix.

## Critères de sortie du Lot 2

1. Réglages accessibles **uniquement** via l'icône topbar → Sheet ; route `/settings` retirée,
   aucune navigation morte.
2. Sélecteur de langue avec drapeaux, **peuplé depuis `locales`** (déposer un JSON admin → langue
   visible) — démontrable.
3. Toggle thème 3 états fonctionnel + persistant ; « Système » surligné correctement.
4. **Chaque réglage a un helper text.**
5. **Plus aucun `['en','fr']` en dur** (LocaleSwitcher dérivé de `locales`).
6. `pnpm lint && typecheck && test` verts ; **SonarCloud new_coverage ≥ 80 %** ; build vert ;
   **isolation bundle public** confirmée (unlock sans CSS flag-icons ni chaînes `settings.*`).
7. Mémoire à jour : INDEX, HANDOFF, CONVENTIONS (Select + flag-icons, helper-text généralisé),
   QUIRKS (shims radix Select jsdom).

## Risques & points de vigilance

- **radix Select via le package unifié `radix-ui`** : vérifier l'API (`Select.Root/Trigger/…`) en
  miroir de `ui/sheet.tsx` ; valider via Context7 si surprise. jsdom shims requis pour les tests.
- **Régression i18n** : le refactor `LocaleSwitcher` ne doit pas casser le login. Couvert par test.
- **Isolation bundle public** : la CSS flag-icons ne doit pas entrer dans le bundle unlock (import
  scoping vérifié au build).
- **Secret `deploy_token`** : `enabled: isOpen` indispensable pour ne pas fetcher le token hors
  ouverture du panneau.

## Dépendances

- Consomme du Lot 1 : `locales` (export `@/i18n`) et le `ThemeProvider` monté (`useTheme/setTheme`).
- Prépare le Lot 3 : le `Select` vendorisé et le helper-text généralisé seront réutilisables.
