# Phase 7 — Lot 1 : Fondations transverses (i18n centralisé + thème)

> Design doc. Premier des 4 lots de la Phase 7 (« Peaufinage graphique / web »).
> Périmètre : les fondations transverses dont dépendent les autres lots.
> Statut : design validé (brainstorming 2026-06-25), à implémenter via un plan dédié.

## Contexte & motivation

La Phase 7 regroupe 11 chantiers de polish répartis sur 4 surfaces (SPA admin, login,
unlock public, serving backend). Le travail est découpé en 4 lots, chacun avec son cycle
spec → plan → implémentation. **Ce lot (1) livre les fondations transverses** dont
dépendent les lots suivants :

- le **vrai sélecteur de langue** (Lot 2) ne peut se peupler proprement que si l'i18n
  découvre ses locales seul ;
- le **toggle de thème** (Lot 2) suppose un `ThemeProvider` monté.

Faire ces fondations en premier évite de tout recâbler deux fois.

### État actuel (constaté)

- **i18n admin** : `src/i18n/index.ts`, imports statiques `./locales/en.json` + `fr.json`
  (108 clés plates, `keySeparator: false`), `LanguageDetector` (localStorage `latch.locale`
  puis navigator), `supportedLngs: ['en','fr']` **codé en dur**, `fallbackLng: 'en'`.
  `LocaleSwitcher` = toggle 2 boutons FR/EN.
- **i18n unlock** : `src/unlock/i18n.ts`, catalogue **inline** (8 clés), instance i18next
  **séparée** (`createInstance()`), bundle Vite distinct (`unlock.html` → `src/unlock/main.tsx`),
  `interpolation.escapeValue: false` (interpolation `{{brand}}` du `brand_name`).
- **thème** : `next-themes` (^0.4.6) en dépendance **mais aucun `ThemeProvider` monté**.
  CSS prêt : variables `oklch`, bloc `.dark`, `@custom-variant dark (&:is(.dark *))`
  (Tailwind v4, pas de `tailwind.config`). `src/components/ui/sonner.tsx` lit déjà
  `useTheme()` (qui retombe sur `undefined`/system faute de provider).
- **titres** : statiques dans `index.html` / `unlock.html` (hors périmètre de ce lot —
  c'est le Lot 3).

## Objectifs (ce que le lot livre)

1. **i18n auto-découvert** : ajouter une langue = déposer un (ou deux) fichier(s) JSON,
   **sans toucher au code**. Vrai pour l'admin **et** l'unlock.
2. **Métadonnées de langue auto-décrites** : chaque locale porte son nom affiché et son
   drapeau, prêts à être consommés par le sélecteur du Lot 2.
3. **`ThemeProvider` monté** sur le bundle admin (couvre login), défaut `system`,
   persistance, anti-FOUC. **Pas d'UI de bascule** dans ce lot (c'est le Lot 2).

### Non-objectifs (explicitement hors lot)

- Le **vrai sélecteur de langue** (`Select` + drapeaux) → **Lot 2**. `LocaleSwitcher`
  reste le toggle FR/EN actuel, inchangé.
- L'**UI de bascule de thème** (clair/sombre/système) → **Lot 2**.
- Le **thème sur l'unlock** et les surfaces publiques → décidé plus tard, avec le Lot 4
  (pages d'erreur). L'unlock reste **clair-only** ici, son bundle n'est pas touché côté thème.
- Titres dynamiques, logo, largeur admin, bouton GitHub → autres lots.

## Décisions de design (tranchées au brainstorming)

| # | Décision | Choix retenu |
|---|---|---|
| D1 | Structure des catalogues | **Deux dossiers** `locales/admin/` + `locales/unlock/`, chacun auto-découvert par `import.meta.glob`. L'unlock ne charge que ses 8 clés. |
| D2 | Source du nom + drapeau | **Clé `_meta`** auto-décrite dans chaque JSON (`{ "name": "Français", "flag": "FR" }`). Drapeau = code pays ISO (rendu emoji/lib tranché au Lot 2). |
| D3 | Périmètre thème | **Admin + login** seulement (même bundle). Unlock reste clair-only ; bundle public intouché. |

## Architecture

### Arborescence cible

```
src/i18n/
  locales/
    admin/   en.json  fr.json     ← 108 clés + _meta
    unlock/  en.json  fr.json     ← 8 clés + _meta
  available-locales.ts            ← logique pure partagée (parse + strip _meta + fallback)
  index.ts                        ← init i18next admin (glob locales/admin/*.json)
src/unlock/
  i18n.ts                         ← init i18next unlock (glob locales/unlock/*.json), instance séparée
index.html                        ← + script inline anti-FOUC (thème)
src/main.tsx                      ← + <ThemeProvider> wrap (admin uniquement)
```

### A. Découverte des locales (`import.meta.glob`)

`import.meta.glob('./locales/admin/*.json', { eager: true })` est **résolu au build par
Vite** : les JSON découverts entrent dans le bundle correspondant (admin **ou** unlock).
La séparation public/admin est donc garantie par construction — l'unlock ne peut pas tirer
les clés admin, même par accident.

Pour chaque module découvert :
- **code de langue** = dérivé du **nom de fichier** (`fr.json` → `fr`) ;
- **`_meta`** = extrait du JSON ;
- **`resources`** = le reste du JSON (sans `_meta`).

### B. `available-locales.ts` — fonction pure partagée

Le cœur testable du lot. **Sépare le « quoi parser » du « comment découvrir »** :
`import.meta.glob` (primitive Vite, indisponible sous Vitest sans config) est appelé dans
`index.ts`/`unlock/i18n.ts`, mais la transformation est une fonction pure prenant la map
de modules en argument.

```ts
export type LocaleInfo = { code: string; name: string; flag: string }

export type ParsedLocales = {
  resources: Record<string, { translation: Record<string, string> }>
  locales: LocaleInfo[]   // ordonné, source de vérité du sélecteur (Lot 2)
}

// glob: résultat de import.meta.glob('...', { eager: true })
export function parseLocales(glob: Record<string, unknown>): ParsedLocales
```

Règles :
- `resources[code].translation` = le JSON **sans `_meta`** (sinon `t('_meta.name')`
  polluerait l'espace de clés plates).
- `locales` = un `LocaleInfo` par fichier, **trié** (par code, ordre stable).
- **Robustesse** : si `_meta` manque ou est malformé → log `console.warn` + fallback
  `{ name: code.toUpperCase(), flag: code.toUpperCase() }`. Déposer un JSON sans `_meta`
  ne casse pas le build ; la langue apparaît sans joli nom/drapeau.

### C. Init i18next admin (`src/i18n/index.ts`)

- `locales/en.json` → `locales/admin/en.json`, `locales/fr.json` → `locales/admin/fr.json`
  (déplacement mécanique ; les 108 clés plates restent inchangées).
- Ajout de `_meta` en tête : `{ "name": "English", "flag": "GB" }` / `{ "name": "Français", "flag": "FR" }`.
- Init :
  ```ts
  const { resources, locales } = parseLocales(import.meta.glob('./locales/admin/*.json', { eager: true }))
  i18n.use(LanguageDetector).use(initReactI18next).init({
    resources,
    supportedLngs: locales.map(l => l.code),       // dérivé — plus de ['en','fr'] en dur
    fallbackLng: 'en',                              // inchangé
    keySeparator: false,                           // inchangé
    detection: { order: ['localStorage','navigator'], lookupLocalStorage: 'latch.locale' }, // inchangé
  })
  ```
- `locales` (avec `_meta`) est **exporté** pour le sélecteur du Lot 2.
- **Une seule instance** (celle de `initReactI18next`) ; le glob ne change que la *source*
  des ressources. Les 108 `t('…')` des composants continuent de marcher sans modification.
- `LocaleSwitcher` **non modifié** dans ce lot. Conséquence assumée : déposer une 3ᵉ langue
  avant le Lot 2 la rendrait active par détection mais pas encore offerte dans le toggle.

### D. Init unlock (`src/unlock/i18n.ts`) — symétrique, instance isolée

- Les 8 clés inline migrent vers `locales/unlock/{en,fr}.json` (+ `_meta`).
- Init :
  ```ts
  const { resources, locales } = parseLocales(import.meta.glob('../i18n/locales/unlock/*.json', { eager: true }))
  const instance = i18next.createInstance()         // instance SÉPARÉE — inchangé
  instance.use(LanguageDetector).init({
    resources,
    supportedLngs: locales.map(l => l.code),
    fallbackLng: 'en',
    interpolation: { escapeValue: false },          // {{brand}} — conservé
    detection: { order: ['localStorage','navigator'], lookupLocalStorage: 'latch.locale' },
  })
  ```
- `createInstance()` séparé et `escapeValue: false` **conservés à l'identique** ; seule la
  *source* des 8 clés change (inline → JSON découvert).
- Le helper `parseLocales` est **partagé** (pas de duplication de la logique strip/fallback).
- Clé localStorage volontairement identique (`latch.locale`) : cohérence du choix de langue
  sur un même navigateur entre surfaces, chaque bundle restant autonome (fallback navigateur).
- Conséquence du build-time glob : ajouter `locales/admin/de.json` **n'apparaît pas** sur la
  page publique tant que `locales/unlock/de.json` n'existe pas — comportement voulu.

### E. Thème (`ThemeProvider` next-themes)

Montage dans `src/main.tsx`, wrap de l'app **admin uniquement** (`src/unlock/main.tsx` non
touché) :
```tsx
<ThemeProvider attribute="class" defaultTheme="system" enableSystem
               storageKey="latch.theme" disableTransitionOnChange>
  <App />
</ThemeProvider>
```
- `attribute="class"` → pose `.dark` sur `<html>`, cible attendue par `@custom-variant dark`.
- `defaultTheme="system"` + `enableSystem` → respecte l'OS par défaut (critère ROADMAP).
- `storageKey="latch.theme"` → cohérent avec `latch.locale`.
- `disableTransitionOnChange` → pas de flash de transitions CSS au switch.
- Provider **au-dessus** du `<Toaster>` sonner → `useTheme()` y résout la bonne valeur.

**Anti-FOUC** : en SPA Vite pure (CSR), `<html>` n'a pas `.dark` avant le montage React →
flash possible si la préférence est dark. Mitigation : **script inline bloquant** dans
`index.html` (uniquement, pas `unlock.html`) qui lit `localStorage['latch.theme']` (ou
`prefers-color-scheme` si `system`) et pose la classe **avant** le premier paint (~8 lignes,
sans dépendance). L'injection anti-flash native de next-themes est une feature Next.js, d'où
le script manuel en CSR.

**Pas d'UI de bascule** dans ce lot : provider monté avec défaut `system`, persistance
active. Le contrôle (`setTheme()`) est le Lot 2. En attendant, le thème se teste via la
préférence OS ou en posant `.dark` à la main dans le devtools.

## Plan de tests

Lot **100 % frontend** (aucun changement backend → pas de `cargo`/MCP concerné). Batterie :
Vitest + Testing Library, lint, typecheck ; e2e Playwright doit rester vert.

- **`parseLocales` (cœur du lot)** :
  - parse correct `{en,fr}` → `resources` + `locales` ordonnés, code dérivé du nom de fichier ;
  - **strip `_meta`** : `t('_meta.name')` ne résout pas ;
  - **fallback `_meta` manquant/malformé** : langue présente avec `{name: CODE, flag: CODE}`, pas de throw ;
  - `supportedLngs` dérivé = liste des codes découverts.
- **i18n admin** : un composant rend `t('login.title')` ; `changeLanguage('fr')` → texte FR ;
  langue inconnue → fallback EN.
- **i18n unlock** : interpolation `{{brand}}` rendue ; 8 clés résolues ; fallback EN.
- **`ThemeProvider`** : un composant lisant `useTheme()` reçoit une valeur définie une fois
  le provider monté (test léger jsdom).
- **e2e** : pas de nouveau test (aucune UI nouvelle) ; les 4 e2e existants restent verts.

## Critères de sortie du Lot 1

1. **Démontrable par test** : ajouter `locales/admin/<lang>.json` (+ `unlock/<lang>.json`)
   le fait entrer dans `supportedLngs` et la détection **sans toucher au code**.
2. `ThemeProvider` monté, défaut `system`, persiste sur `latch.theme`, **anti-FOUC** en place.
3. Les **108 clés admin + 8 unlock résolvent toujours** (zéro régression).
4. `pnpm lint && pnpm typecheck && pnpm test` verts ; **SonarCloud `new_coverage ≥ 80 %`**
   sur le code neuf (`parseLocales`).
5. Mémoire à jour : `HANDOFF`, `INDEX`, `CONVENTIONS` (pattern glob + `_meta` réutilisable au
   Lot 2/3), `QUIRKS` si piège (FOUC CSR, glob eager sous Vitest).

## Risques & points de vigilance

- **`import.meta.glob` sous Vitest** : non disponible sans config. Mitigé par l'isolation de
  `parseLocales` en fonction pure (testée avec des maps factices). Le `glob()` lui-même n'est
  appelé que dans `index.ts`/`unlock/i18n.ts` (non couverts unitairement, triviaux).
- **next-themes hors Next** : valider les props exactes via **Context7** (version épinglée)
  au moment du plan ; vérifier le comportement CSR de l'anti-FOUC.
- **`react-i18next`** : vérifier via Context7 l'API `init`/`addResourceBundle`/`supportedLngs`
  pour la version épinglée avant d'implémenter.
- **Régression silencieuse des 108 clés** : le déplacement de fichiers + ajout `_meta` ne doit
  pas altérer une clé. Couvert par les tests i18n admin + les composants existants.

## Dépendances vers les lots suivants

- **Lot 2** consomme `locales` (export de `index.ts`) pour le `Select` de langue, et
  `setTheme()` du provider monté ici pour le toggle de thème.
- **Lot 3** réutilise le pattern glob + `_meta` consigné dans `CONVENTIONS`.
