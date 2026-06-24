# Design — Phase 3 : polish UX + i18n (admin SPA)

> Spec issue du brainstorming du 2026-06-24, en réponse à la punch-list post-test
> live (`2026-06-24-phase-3-punchlist-ux.md`). Couvre **les patchs UX (§1)** et le
> **chantier polish produit + i18n (§3)** de la punch-list, traités en une session.
> Le contrat (`docs/contrat-deploy.md`) reste la loi : rien ici ne change les
> invariants de sécurité ni le modèle de données — c'est exclusivement la couche
> présentation de la SPA Yew (`frontend/`).

## 1. Objectif & périmètre

Rendre l'admin distribuable : corriger les frictions UX relevées au test navigateur,
introduire une **vraie couche i18n multi-locale (FR + EN)** avec sélecteur, une
**couche de toasts** pour les retours d'action, et un **polish produit** (helper text,
intros de page, accessibilité).

**Hors périmètre** : backend (API `/api/*` inchangée), serving `/c/<slug>` (Phase 4),
MCP (Phase 5). Aucune modification de DTO (`latch-dto`) n'est requise.

## 2. Principe de projet acté — vendoriser shadcn-rs quand un composant est cassé

`shadcn-rs` est en **0.1** (lib jeune, API instable, plusieurs composants à moitié
implémentés — cf. QUIRKS). **Règle** : quand un composant shadcn-rs est cassé ou
bloquant, on le **vendorise et on le patche** dans `frontend/src/components/`, plutôt
que de le contourner par des hacks fragiles (ex. remount par `key`). On réutilise les
classes CSS déjà vendorisées pour garder un rendu identique. On garde shadcn-rs pour
tout ce qui fonctionne.

Précédents : la CSS shadcn-rs déjà vendorisée et patchée (5 fichiers sous
`frontend/styles/`), et désormais le `Switch` (cf. §5.3).

## 3. Architecture — modules transverses (le socle)

### 3.1 i18n (`rust-i18n`)

- **Dépendance** : `rust-i18n` ajoutée à `frontend/Cargo.toml`, + bloc
  `[package.metadata.i18n]` : `available-locales = ["en", "fr"]`,
  `default-locale = "en"`, `load-path = "locales"`.
- **Fichiers de traduction** : `frontend/locales/en.yml` + `frontend/locales/fr.yml`.
  Clés groupées par écran : `login.*`, `list.*`, `detail.*`, `form.*`, `deploy.*`,
  `common.*`, `toast.*`. Embarqués **à la compilation** via `rust_i18n::i18n!("locales")`
  dans `main.rs` (`#[macro_use] extern crate rust_i18n;`) → pur Rust à l'exécution,
  donc **compatible wasm** (pas d'accès fichier au runtime). Vérifié via Context7.
- **`LocaleProvider`** (`frontend/src/i18n.rs`, calqué sur `auth.rs::AuthProvider`) :
  un `ContextProvider` portant une struct `LocaleContext { locale: Locale,
  set_locale: Callback<Locale> }` où `Locale` est un enum `En` / `Fr`.
  - **Au boot** : lit `localStorage["latch.locale"]`, sinon dérive de
    `navigator.language` (préfixe `fr` → Fr, sinon En), sinon défaut `En`. Appelle
    `rust_i18n::set_locale(locale.as_str())` et initialise le state.
  - **Au changement** (`set_locale`) : `rust_i18n::set_locale(...)` +
    `localStorage.setItem("latch.locale", ...)` + bump du `use_state` → re-render des
    consommateurs.
- **`use_locale()`** : hook qui `use_context::<LocaleContext>()` (force l'abonnement,
  donc le re-render au changement de locale). Tout composant affichant du texte
  appelle `use_locale()` en tête puis `t!("login.submit")`. Comme `t!` lit la locale
  globale (déjà mise à jour synchrone par `set_locale` avant le re-render), l'affichage
  suit.
- **`LocaleSwitcher`** (`frontend/src/components/locale_switcher.rs`) : sélecteur FR/EN
  en **deux boutons maison** (`FR` / `EN`, l'actif marqué), pas de `Select` shadcn —
  on évite une nouvelle dépendance à un composant 0.1 potentiellement bancal. Monté
  dans la `topbar` (liste + détail) et sur l'écran de login.
- **Montage** : `<LocaleProvider>` enveloppe tout l'arbre dans `App`
  (`main.rs`), au-dessus de `AuthProvider`.

> **Réactivité — point d'attention** : `rust_i18n::set_locale` est un état global qui
> ne notifie pas Yew. La réactivité vient **exclusivement** de l'abonnement au
> `LocaleContext` via `use_locale()`. Donc : tout composant qui rend du texte traduit
> **doit** appeler `use_locale()`, même s'il n'utilise pas la valeur retournée (l'appel
> garantit le re-render). C'est une convention à documenter dans CONVENTIONS.

### 3.2 Toasts maison (`frontend/src/toast.rs`)

`shadcn-rs` expose `Toast`/`Sonner` en déclaratif **sans auto-dismiss** (cf. QUIRKS) →
couche maison.

- **`ToastProvider`** : `ContextProvider` portant `ToastContext { push_success:
  Callback<String>, push_error: Callback<String> }`. État interne : `Vec<Toast>` où
  `Toast { id: u32, kind: ToastKind, msg: String }`. À chaque `push_*`, on ajoute un
  toast et on arme un `gloo_timers::callback::Timeout` (~4 s) qui le retire par `id`.
- **Rendu** : un overlay `.toast-stack` (position fixe, coin haut-droit) rendu par le
  provider au-dessus des enfants, listant les toasts actifs.
- **`use_toast()`** : hook d'accès. Les pages/panels appellent
  `toast.push_success(t!("toast.project_created").to_string())` etc.
- **Montage** : `<ToastProvider>` sous `LocaleProvider`, au-dessus de l'arbre routé.
- **Câblage** (succès + erreur) : création, édition, déploiement, activation,
  suppression projet, suppression version, copie URL/PIN. Corrige le silence actuel de
  `activate_version` (cf. BACKLOG « Remontée d'erreur sur activate_version »).
- CSS : `.toast-stack` / `.toast` / `.toast--success` / `.toast--error` dans `app.css`
  (couleurs via les variables success/warning/destructive — cf. §5.2).

### 3.3 Composant `Toggle` vendorisé (`frontend/src/components/toggle.rs`)

Copie de `shadcn-rs-0.1.0/src/components/switch.rs`, avec **un seul changement de
logique** : remplacer

```rust
let is_checked = if checked { checked } else { *internal_checked };
```

par un état **contrôlé pur** :

```rust
let is_checked = checked;
```

(et suppression de `internal_checked` / `default_checked` devenus inutiles ; `onclick`
et `onkeydown` se contentent d'émettre `onchange`, le parent porte l'état). Réutilise
les classes CSS existantes `.switch` / `.switch-thumb` / `.switch-checked` /
`.switch-disabled` (déjà dans `components.css`) → **rendu visuellement identique**.
Gère `disabled` correctement. Remplace `<Switch>` dans `project_form.rs` et `deploy.rs`.

## 4. Fichiers touchés (vue d'ensemble)

**Nouveaux** :
- `frontend/src/i18n.rs` — `Locale`, `LocaleProvider`, `use_locale`.
- `frontend/src/toast.rs` — `ToastProvider`, `use_toast`, rendu overlay.
- `frontend/src/components/toggle.rs` — Switch vendorisé patché.
- `frontend/src/components/locale_switcher.rs` — sélecteur FR/EN.
- `frontend/locales/en.yml`, `frontend/locales/fr.yml` — traductions.

**Modifiés** :
- `frontend/Cargo.toml` — dép `rust-i18n` + `[package.metadata.i18n]`.
- `frontend/src/main.rs` — `i18n!`, `#[macro_use]`, montage `LocaleProvider` + `ToastProvider`.
- `frontend/src/pages/{login,list,detail}.rs` — i18n, toasts, accessibilité, intros, switcher.
- `frontend/src/panels/{project_form,deploy,delete_project,delete_version}.rs` — i18n, `Toggle`, toasts, PIN disabled, slug disabled, dropzone.
- `frontend/src/components/mod.rs` — déclare `toggle`, `locale_switcher`.
- `frontend/styles/variables.css` — ajout `--color-success*` / `--color-warning*` (`:root` + `.dark`).
- `frontend/styles/app.css` — login spacing, badges, dropzone, toasts, `.locale-switcher`.
- `frontend/index.html` — `lang` (mineur).

## 5. Patchs UX écran par écran (§1 de la punch-list)

### 5.1 Login (`pages/login.rs` + `app.css`)
- **Espacement** entre le champ « Mot de passe » et le bouton « Se connecter »
  (marge sur le bouton ou classe d'espacement sur le `CardContent`).
- Textes via `t!` ; `LocaleSwitcher` ajouté (au-dessus de la carte ou dans un coin).
- Polish reporté de BACKLOG : `error.set(None)` est déjà fait avant submit (vérifié) —
  rien à corriger côté re-submit.

### 5.2 Liste (`pages/list.rs`) — badges colorés
- **Finding** : `--color-success` / `--color-warning` **n'existent pas** dans la CSS
  vendorisée (contrairement à ce que supposait la punch-list — seules
  primary/secondary/destructive sont définies). **Décision** : les **ajouter** dans
  `variables.css` (`:root` clair + `.dark` sombre, HSL cohérent avec la palette), comme
  le patch card/popover documenté dans QUIRKS.
- Badge **code activé → vert** (`.badge--success`), **libre → orange**
  (`.badge--warning`), classes définies dans `app.css`. Mêmes badges réutilisés au
  détail (cohérence).
- Intro de page courte + textes via `t!`.

### 5.3 ProjectForm (`panels/project_form.rs`)
- `<Switch>` → `<Toggle>` (cf. §3.3) : le toggle « Code d'accès » bascule visuellement.
- **PIN toujours affiché** : retirer le `if *code_on { ... }` autour du champ PIN ;
  l'afficher en permanence et le passer `disabled` (grisé) + bouton régénérer
  `disabled` quand le code est off (plus de saut de layout). La validation PIN ne
  s'applique que si `code_on`.
- **Slug `disabled`** en mode édition (au lieu de `readonly={true}` qui restait
  éditable visuellement — cf. punch-list).
- Helper text EN sous les champs (nom, marque, PIN), via `t!`.

### 5.4 DeployPanel (`panels/deploy.rs`)
- **Dropzone drag-and-drop** : zone `.dropzone` stylée, handlers `ondragover`
  (preventDefault + état survol), `ondragleave`, `ondrop` (lit
  `DataTransfer.files`), et `onclick` qui déclenche un `<input type=file>` caché
  (`NodeRef` + `.click()`). Affiche nom + taille du fichier choisi, état survol. La
  lecture passe par le même `gloo_file::futures::read_as_text` qu'aujourd'hui.
- `<Switch>` → `<Toggle>` pour « activer immédiatement ».
- Toasts succès/erreur de déploiement.

## 6. Polish produit (§3 de la punch-list)

- **Helper text** sous chaque champ de formulaire (au-delà du toggle), via `t!`.
- **Intros de page** courtes (liste, détail) : ce que fait l'écran / à quoi sert
  chaque bloc, via `t!`.
- **Accessibilité** : remplacer les `<a onclick>` sans `href` (lignes de table dans
  `list.rs`, breadcrumb `‹ Projets` dans `detail.rs`) par des `<button>` stylés en lien
  (`.linkish`), `aria-label` cohérents, focus visible. Conserver les vrais `<a href>`
  (preview de version) tels quels.
- **Tout le texte via i18n** : EN par défaut, FR fourni — login, liste, détail, panels,
  messages d'erreur, helper text, toasts.

## 7. Tests & validation

- **wasm-bindgen-test** : un test qui vérifie que changer la locale fait basculer la
  sortie de `t!` (EN → FR sur une clé témoin). Conserver les 3 tests existants
  (pin/url/clipboard).
- **Validation Playwright (obligatoire — process punch-list §4)** : pour chaque patch,
  `browser_navigate` + `browser_take_screenshot` + `browser_evaluate` sur les styles
  calculés. Points à prouver au navigateur (invisibles aux tests SDD/curl) :
  - toggle « Code d'accès » et « activer immédiatement » basculent **visuellement** ;
  - badges liste **verts/oranges** (couleur calculée) ;
  - champ PIN **grisé** (disabled) quand code off, pas de saut de layout ;
  - slug non éditable en édition ;
  - dropzone : drop d'un fichier + clic pour parcourir ;
  - toasts : apparition + auto-dismiss sur création/déploiement/activation/suppression/copie ;
  - **switch de langue** FR↔EN re-rend l'UI et persiste après reload.
- `cargo fmt --all` + `cargo clippy` (frontend via sa cible wasm) verts. Backend
  inchangé.

## 8. Critères de sortie

- Tous les items §1 et §3 de la punch-list traités et **validés au navigateur**.
- i18n FR+EN opérationnelle avec sélecteur persistant ; aucun texte FR en dur résiduel.
- Toggle corrigé (vendorisé), toasts câblés sur tous les retours d'action, dropzone
  fonctionnelle, badges colorés, PIN/slug disabled, login espacé.
- Mémoire mise à jour : `INDEX.md` (livrables i18n/toasts/Toggle/dropzone),
  `HANDOFF.md` (entrée datée), `QUIRKS.md` (finding success/warning + patch Toggle +
  réactivité i18n), `CONVENTIONS.md` (patterns LocaleProvider/ToastProvider/Toggle +
  règle de vendorisation), `CLAUDE.md` si la règle de vendorisation mérite d'y figurer.
- Le contrat §7 (admin — rails par page) reste cohérent ; amendement si l'i18n ou le
  sélecteur de langue change un comportement décrit.
