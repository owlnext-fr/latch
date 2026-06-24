# Phase 3 — Polish UX + i18n Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rendre l'admin SPA distribuable — i18n FR/EN avec sélecteur persistant, couche de toasts, toggle corrigé, dropzone, badges colorés, accessibilité — en traitant la punch-list post-test-live (§1 patchs + §3 polish).

**Architecture :** Trois modules transverses nouveaux (`i18n.rs` = LocaleProvider + `t!`, `toast.rs` = ToastProvider, `components/toggle.rs` = Switch shadcn-rs vendorisé patché), puis migration écran par écran (login → list → form → deploy → detail) vers i18n + toasts + accessibilité. Backend et DTO inchangés.

**Tech Stack :** Yew 0.21 (CSR), yew-router 0.18, rust-i18n 3 (YAML embarqués à la compilation), gloo-timers 0.3, shadcn-rs 0.1 (partiellement vendorisé), Trunk.

## Global Constraints

- **Cible** : crate `latch-ui` (`frontend/`), cible `wasm32-unknown-unknown`. Build via `trunk build`, tests via `wasm-pack test --headless --firefox`.
- **Confidentialité (NON-NÉGOCIABLE)** : aucun nom de client réel nulle part. Placeholders fictifs uniquement (`Mon Projet`/`mon-projet`, `ACME`, `demo`).
- **Sécurité (contrat §9)** : ne jamais afficher de hash ; le PIN n'apparaît qu'au détail projet. Ce chantier ne touche pas ces invariants — ne pas les régresser.
- **Pas d'`unwrap`/`expect`** hors tests et init de boot. Erreurs propagées.
- **Commits** : `<gitmoji> <type>: <desc>` (ex. `✨ feat:`, `🐛 fix:`, `♻️ refactor:`). Préfixer les commandes avec `rtk` (RTK golden rule).
- **shadcn-rs cassé → vendoriser + patcher** (précédent : CSS ; ici : Switch). Pas de hacks `key`/remount.
- **i18n réactivité** : tout composant qui rend du texte traduit DOIT appeler `use_locale()` en tête (l'abonnement au contexte force le re-render au changement de locale ; `t!` lit la locale globale rust-i18n déjà mise à jour).
- **`t!` est disponible crate-wide** via `#[macro_use] extern crate rust_i18n;` dans `main.rs` — pas d'import par fichier.
- **Validation Playwright obligatoire** par patch UI (process punch-list §4) : `browser_navigate` + `browser_take_screenshot` + `browser_evaluate` sur styles calculés. Les 3 bugs précédents étaient invisibles aux tests SDD/curl.

**Commande de stack live (pour la validation Playwright)** — après `rtk trunk build` dans `frontend/` :
```bash
cd backend && LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret \
  DATABASE_URL='sqlite://latch_dev_uxpolish.sqlite?mode=rwc' cargo loco start
# SPA : http://127.0.0.1:5150/admin  (SESSION_SECRET a une clé de secours dev dans web/mod.rs)
# Itération CSS pure : rtk trunk build + hard refresh (ServeDir relit dist/ à chaque requête).
```

---

## Task 1 : Fondation i18n (crate + fichiers de locale + macro)

**Files:**
- Modify: `frontend/Cargo.toml` (dépendance + metadata)
- Create: `frontend/locales/en.yml`
- Create: `frontend/locales/fr.yml`
- Modify: `frontend/src/main.rs` (macro `i18n!`, `#[macro_use]`)
- Test: `frontend/src/i18n.rs` (test wasm sur `t!`) — créé minimal ici, étoffé Task 2

**Interfaces:**
- Produces: macro `t!("clé")` et `t!("clé", var = valeur)` disponibles crate-wide ; locales `en`/`fr` chargées ; fonction `rust_i18n::set_locale(&str)` / `rust_i18n::locale()`.

- [ ] **Step 1 : Ajouter la dépendance et la metadata i18n**

Modifier `frontend/Cargo.toml`. Sous `[dependencies]`, ajouter après la ligne `shadcn-rs = "0.1"` :
```toml
rust-i18n = "3"
```
À la fin du fichier, ajouter :
```toml
[package.metadata.i18n]
available-locales = ["en", "fr"]
default-locale = "en"
load-path = "locales"
```

- [ ] **Step 2 : Écrire `frontend/locales/en.yml`** (clés plates `_version: 1`)

```yaml
_version: 1

common.loading: "Loading…"
common.cancel: "Cancel"
common.save: "Save"
common.saving: "Saving…"
common.delete: "Delete"
common.deploy: "Deploy"
common.edit: "Edit"
common.logout: "Log out"
common.new_project: "+ New project"
common.copied: "Copied!"
common.regenerate: "⟳ Regenerate"
common.active: "active"
common.dash: "—"

login.title: "latch — admin"
login.user: "Username"
login.pass: "Password"
login.submit: "Sign in"
login.submitting: "Signing in…"
login.error_invalid: "Invalid credentials."

list.intro: "Your prototypes. Click a project to manage its versions and access."
list.col_name: "Name"
list.col_url: "Public URL"
list.col_code: "Access"
list.col_version: "Active version"
list.badge_code_on: "PIN required"
list.badge_free: "Open"
list.empty: "No projects yet."
list.create_first: "+ Create the first project"
list.copy_url_aria: "Copy the URL"
list.active: "active"

detail.back: "‹ Projects"
detail.intro: "Read-only overview. Use the actions to edit, deploy or delete."
detail.access_title: "Public access"
detail.url_label: "Public URL"
detail.code_label: "Access code"
detail.pin_undefined: "PIN not set"
detail.free_access: "Open access"
detail.config_title: "Configuration"
detail.brand_label: "Brand name"
detail.code_on: "enabled"
detail.code_off: "open"
detail.versions_title: "Versions"
detail.col_num: "#"
detail.col_date: "Date"
detail.col_status: "Status"
detail.activate_aria: "Activate"
detail.preview_aria: "Preview"
detail.delete_aria: "Delete"
detail.copy_url_aria: "Copy the URL"
detail.copy_pin_aria: "Copy the PIN"
detail.reveal_pin: "Reveal PIN"
detail.hide_pin: "Hide PIN"

form.title_create: "New project"
form.title_edit: "Edit project"
form.name: "Name"
form.name_help: "Shown in the admin list. Not visible to visitors."
form.slug: "Slug (auto)"
form.slug_help: "Auto-generated, read-only. It is the public URL suffix."
form.brand: "Brand name (optional)"
form.brand_help: "Shown on the unlock page: “Prototype prepared for …”."
form.code: "Access code"
form.code_help: "When enabled, visitors enter a 6-digit PIN before accessing the prototype. Disabled = open access by URL."
form.pin: "PIN (6 digits)"
form.pin_help: "Visitors type this to unlock. You can copy it from the detail page."
form.err_name: "Name is required."
form.err_pin: "The PIN must be 6 digits."

deploy.title: "Deploy a version"
deploy.file: "HTML file"
deploy.dropzone_idle: "Drag an HTML file here, or click to browse"
deploy.dropzone_hover: "Drop the file to load it"
deploy.file_chosen: "%{name} (%{size})"
deploy.activate: "Activate immediately"
deploy.activate_help: "The new version becomes the one served on the public URL."
deploy.err_no_file: "Choose an HTML file."
deploy.err_read: "Could not read the file."
deploy.btn: "Deploy"
deploy.deploying: "Deploying…"

danger.del_project_title: "Delete “%{name}”"
danger.del_project_intro: "This action is irreversible. The following will be permanently deleted:"
danger.del_project_li1: "the project and its configuration;"
danger.del_project_li2: "its %{count} version(s) and their HTML files;"
danger.del_project_li3: "the public URL (404 afterwards)."
danger.del_project_confirm: "Yes, delete permanently"
danger.del_version_title: "Delete version v%{n}"
danger.del_version_intro: "This version and its HTML file will be deleted. Irreversible."
danger.del_version_confirm: "Yes, delete"
danger.deleting: "Deleting…"

toast.project_created: "Project created."
toast.project_updated: "Project updated."
toast.project_deleted: "Project deleted."
toast.version_deployed: "Version deployed."
toast.version_activated: "Version activated."
toast.version_deleted: "Version deleted."
toast.copied: "Copied to clipboard."
```

- [ ] **Step 3 : Écrire `frontend/locales/fr.yml`** (mêmes clés, valeurs FR)

```yaml
_version: 1

common.loading: "Chargement…"
common.cancel: "Annuler"
common.save: "Enregistrer"
common.saving: "Enregistrement…"
common.delete: "Supprimer"
common.deploy: "Déployer"
common.edit: "Éditer"
common.logout: "Se déconnecter"
common.new_project: "+ Nouveau projet"
common.copied: "Copié !"
common.regenerate: "⟳ régénérer"
common.active: "active"
common.dash: "—"

login.title: "latch — admin"
login.user: "Identifiant"
login.pass: "Mot de passe"
login.submit: "Se connecter"
login.submitting: "Connexion…"
login.error_invalid: "Identifiants invalides."

list.intro: "Vos prototypes. Cliquez un projet pour gérer ses versions et son accès."
list.col_name: "Nom"
list.col_url: "URL publique"
list.col_code: "Accès"
list.col_version: "Version active"
list.badge_code_on: "PIN requis"
list.badge_free: "Libre"
list.empty: "Aucun projet pour l'instant."
list.create_first: "+ Créer le premier projet"
list.copy_url_aria: "Copier l'URL"
list.active: "active"

detail.back: "‹ Projets"
detail.intro: "Vue en lecture seule. Utilisez les actions pour éditer, déployer ou supprimer."
detail.access_title: "Accès public"
detail.url_label: "URL publique"
detail.code_label: "Code d'accès"
detail.pin_undefined: "PIN non défini"
detail.free_access: "Accès libre"
detail.config_title: "Configuration"
detail.brand_label: "Nom de marque"
detail.code_on: "activé"
detail.code_off: "libre"
detail.versions_title: "Versions"
detail.col_num: "#"
detail.col_date: "Date"
detail.col_status: "Statut"
detail.activate_aria: "Activer"
detail.preview_aria: "Prévisualiser"
detail.delete_aria: "Supprimer"
detail.copy_url_aria: "Copier l'URL"
detail.copy_pin_aria: "Copier le PIN"
detail.reveal_pin: "Révéler le PIN"
detail.hide_pin: "Masquer le PIN"

form.title_create: "Nouveau projet"
form.title_edit: "Éditer le projet"
form.name: "Nom"
form.name_help: "Affiché dans la liste admin. Invisible pour les visiteurs."
form.slug: "Slug (auto)"
form.slug_help: "Auto-généré, en lecture seule. C'est le suffixe de l'URL publique."
form.brand: "Nom de marque (optionnel)"
form.brand_help: "Affiché sur la page de déverrouillage : « Prototype préparé pour … »."
form.code: "Code d'accès"
form.code_help: "Quand activé, les visiteurs saisissent un PIN à 6 chiffres avant d'accéder au prototype. Désactivé = accès libre par l'URL."
form.pin: "PIN (6 chiffres)"
form.pin_help: "Les visiteurs le saisissent pour déverrouiller. Copiable depuis la page détail."
form.err_name: "Le nom est requis."
form.err_pin: "Le PIN doit faire 6 chiffres."

deploy.title: "Déployer une version"
deploy.file: "Fichier HTML"
deploy.dropzone_idle: "Glissez un fichier HTML ici, ou cliquez pour parcourir"
deploy.dropzone_hover: "Déposez le fichier pour le charger"
deploy.file_chosen: "%{name} (%{size})"
deploy.activate: "Activer immédiatement"
deploy.activate_help: "La nouvelle version devient celle servie sur l'URL publique."
deploy.err_no_file: "Choisissez un fichier HTML."
deploy.err_read: "Lecture du fichier impossible."
deploy.btn: "Déployer"
deploy.deploying: "Déploiement…"

danger.del_project_title: "Supprimer « %{name} »"
danger.del_project_intro: "Cette action est irréversible. Seront supprimés définitivement :"
danger.del_project_li1: "le projet et sa configuration ;"
danger.del_project_li2: "ses %{count} version(s) et leurs fichiers HTML ;"
danger.del_project_li3: "l'URL publique (404 ensuite)."
danger.del_project_confirm: "Oui, supprimer définitivement"
danger.del_version_title: "Supprimer la version v%{n}"
danger.del_version_intro: "Cette version et son fichier HTML seront supprimés. Action irréversible."
danger.del_version_confirm: "Oui, supprimer"
danger.deleting: "Suppression…"

toast.project_created: "Projet créé."
toast.project_updated: "Projet mis à jour."
toast.project_deleted: "Projet supprimé."
toast.version_deployed: "Version déployée."
toast.version_activated: "Version activée."
toast.version_deleted: "Version supprimée."
toast.copied: "Copié dans le presse-papier."
```

- [ ] **Step 3b : Créer `frontend/src/i18n.rs` minimal avec un test wasm**

```rust
//! Couche i18n : enum `Locale`, détection au boot, `LocaleProvider`, `use_locale`.
//! (Le provider est complété en Task 2 ; ici on pose l'enum + un test sur `t!`.)

/// Locales supportées par l'admin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    En,
    Fr,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Fr => "fr",
        }
    }

    /// Dérive la locale d'un code (`navigator.language` ou valeur stockée).
    pub fn from_code(code: &str) -> Locale {
        if code.to_ascii_lowercase().starts_with("fr") {
            Locale::Fr
        } else {
            Locale::En
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn from_code_maps_fr_and_defaults_en() {
        assert_eq!(Locale::from_code("fr-FR"), Locale::Fr);
        assert_eq!(Locale::from_code("FR"), Locale::Fr);
        assert_eq!(Locale::from_code("en-US"), Locale::En);
        assert_eq!(Locale::from_code("de"), Locale::En);
    }

    #[wasm_bindgen_test]
    fn t_macro_resolves_per_locale() {
        rust_i18n::set_locale("en");
        assert_eq!(&*t!("login.submit"), "Sign in");
        rust_i18n::set_locale("fr");
        assert_eq!(&*t!("login.submit"), "Se connecter");
        rust_i18n::set_locale("en"); // restaure pour les autres tests
    }
}
```

- [ ] **Step 4 : Câbler la macro dans `main.rs`**

Remplacer le haut de `frontend/src/main.rs` (lignes 1-13) par :
```rust
#[macro_use]
extern crate rust_i18n;

use yew::prelude::*;
use yew_router::prelude::*;

mod api;
mod auth;
mod components;
mod i18n;
mod pages;
mod panels;
mod routes;
mod toast;
mod util;

use auth::AuthProvider;
use routes::{switch, Route};

rust_i18n::i18n!("locales");
```
> Note : `mod toast;` est déclaré ici mais le fichier est créé en Task 3. Pour que Task 1 compile seule, créer un `frontend/src/toast.rs` **stub vide** maintenant (`// placeholder — rempli en Task 3`) ; il sera écrasé en Task 3. Idem ne PAS encore brancher `<LocaleProvider>`/`<ToastProvider>` dans le `html!` (Tasks 2 et 3).

- [ ] **Step 5 : Build + tests wasm**

Run: `cd frontend && rtk trunk build`
Expected: build OK, `dist/` produit, aucune erreur de compilation.

Run: `cd frontend && rtk proxy wasm-pack test --headless --firefox`
Expected: tests verts, dont `from_code_maps_fr_and_defaults_en` et `t_macro_resolves_per_locale`.

- [ ] **Step 6 : Commit**

```bash
rtk git add frontend/Cargo.toml frontend/locales frontend/src/i18n.rs frontend/src/main.rs frontend/src/toast.rs
rtk git commit -m "✨ feat(i18n): fondation rust-i18n (locales en/fr, macro t!, enum Locale)"
```

---

## Task 2 : LocaleProvider + use_locale + montage + détection au boot

**Files:**
- Modify: `frontend/src/i18n.rs` (ajout provider/hook/détection)
- Modify: `frontend/Cargo.toml` (web-sys feature `Storage`)
- Modify: `frontend/src/main.rs` (montage `<LocaleProvider>`)

**Interfaces:**
- Consumes: `Locale` (Task 1), `rust_i18n::set_locale`.
- Produces: `LocaleProvider` (composant), `LocaleContext { locale: Locale, set_locale: Callback<Locale> }`, hook `use_locale() -> LocaleContext`.

- [ ] **Step 1 : Ajouter la feature web-sys `Storage`**

Dans `frontend/Cargo.toml`, dans `[dependencies.web-sys] features = [...]`, ajouter `"Storage"` à la liste (pour `Window::local_storage`).

- [ ] **Step 2 : Étoffer `frontend/src/i18n.rs`** (ajouter au-dessus du bloc `#[cfg(test)]`)

```rust
use yew::prelude::*;

const LS_KEY: &str = "latch.locale";

fn read_stored() -> Option<String> {
    let win = web_sys::window()?;
    let store = win.local_storage().ok()??;
    store.get_item(LS_KEY).ok()?
}

fn write_stored(code: &str) {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(store)) = win.local_storage() {
            let _ = store.set_item(LS_KEY, code);
        }
    }
}

fn browser_lang() -> Option<String> {
    web_sys::window()?.navigator().language()
}

/// Locale au démarrage : localStorage → navigator.language → EN.
fn detect_initial() -> Locale {
    if let Some(stored) = read_stored() {
        return Locale::from_code(&stored);
    }
    if let Some(lang) = browser_lang() {
        return Locale::from_code(&lang);
    }
    Locale::En
}

#[derive(Clone, PartialEq)]
pub struct LocaleContext {
    pub locale: Locale,
    pub set_locale: Callback<Locale>,
}

#[hook]
pub fn use_locale() -> LocaleContext {
    use_context::<LocaleContext>().expect("LocaleProvider manquant au-dessus de l'arbre")
}

#[derive(Properties, PartialEq)]
pub struct LocaleProviderProps {
    pub children: Html,
}

#[function_component(LocaleProvider)]
pub fn locale_provider(props: &LocaleProviderProps) -> Html {
    // Initialise une seule fois : détecte la locale et applique la locale globale
    // rust-i18n de façon synchrone (avant le premier rendu des enfants → pas de flash).
    let locale = use_state(|| {
        let l = detect_initial();
        rust_i18n::set_locale(l.as_str());
        l
    });

    let set_locale = {
        let locale = locale.clone();
        Callback::from(move |l: Locale| {
            rust_i18n::set_locale(l.as_str());
            write_stored(l.as_str());
            locale.set(l);
        })
    };

    let ctx = LocaleContext {
        locale: *locale,
        set_locale,
    };

    html! {
        <ContextProvider<LocaleContext> context={ctx}>
            { props.children.clone() }
        </ContextProvider<LocaleContext>>
    }
}
```

- [ ] **Step 3 : Monter `<LocaleProvider>` dans `main.rs`**

Remplacer le corps de `fn app()` (le `html! { ... }`) par :
```rust
    html! {
        <BrowserRouter>
            <i18n::LocaleProvider>
                <AuthProvider>
                    <Switch<Route> render={switch} />
                </AuthProvider>
            </i18n::LocaleProvider>
        </BrowserRouter>
    }
```
> `<ToastProvider>` sera intercalé en Task 3.

- [ ] **Step 4 : Build + tests**

Run: `cd frontend && rtk trunk build`
Expected: build OK.

Run: `cd frontend && rtk proxy wasm-pack test --headless --firefox`
Expected: tests Task 1 toujours verts.

- [ ] **Step 5 : Commit**

```bash
rtk git add frontend/Cargo.toml frontend/src/i18n.rs frontend/src/main.rs
rtk git commit -m "✨ feat(i18n): LocaleProvider + use_locale + détection boot (localStorage/navigator)"
```

---

## Task 3 : ToastProvider + use_toast + overlay + CSS + câblage CopyButton

**Files:**
- Modify: `frontend/src/toast.rs` (remplace le stub)
- Modify: `frontend/src/main.rs` (montage `<ToastProvider>`)
- Modify: `frontend/styles/app.css` (styles `.toast-stack`/`.toast`)
- Modify: `frontend/src/components/copy_button.rs` (toast + i18n du libellé)

**Interfaces:**
- Consumes: rien (indépendant), `gloo_timers::callback::Timeout`.
- Produces: `ToastProvider` (composant), `ToastHandle { push_success: Callback<String>, push_error: Callback<String> }`, hook `use_toast() -> ToastHandle`.

- [ ] **Step 1 : Écrire `frontend/src/toast.rs`** (remplace le stub de Task 1)

```rust
//! Couche de toasts maison. shadcn-rs `Toast`/`Sonner` sont déclaratifs et sans
//! auto-dismiss (cf. QUIRKS) → provider maison : Vec<Toast> + gloo-timers (4 s).

use std::cell::RefCell;
use std::rc::Rc;

use gloo_timers::callback::Timeout;
use yew::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum ToastKind {
    Success,
    Error,
}

#[derive(Clone, PartialEq)]
struct Toast {
    id: u32,
    kind: ToastKind,
    msg: String,
}

#[derive(Clone, PartialEq)]
pub struct ToastHandle {
    pub push_success: Callback<String>,
    pub push_error: Callback<String>,
}

#[hook]
pub fn use_toast() -> ToastHandle {
    use_context::<ToastHandle>().expect("ToastProvider manquant au-dessus de l'arbre")
}

#[derive(Properties, PartialEq)]
pub struct ToastProviderProps {
    pub children: Html,
}

fn make_push(
    toasts: UseStateHandle<Vec<Toast>>,
    next_id: Rc<RefCell<u32>>,
    kind: ToastKind,
) -> Callback<String> {
    Callback::from(move |msg: String| {
        let id = {
            let mut n = next_id.borrow_mut();
            *n += 1;
            *n
        };
        let mut v = (*toasts).clone();
        v.push(Toast { id, kind, msg });
        toasts.set(v);

        let toasts = toasts.clone();
        Timeout::new(4000, move || {
            let v: Vec<Toast> = (*toasts).iter().filter(|t| t.id != id).cloned().collect();
            toasts.set(v);
        })
        .forget();
    })
}

#[function_component(ToastProvider)]
pub fn toast_provider(props: &ToastProviderProps) -> Html {
    let toasts = use_state(Vec::<Toast>::new);
    let next_id = use_mut_ref(|| 0u32);

    let handle = ToastHandle {
        push_success: make_push(toasts.clone(), next_id.clone(), ToastKind::Success),
        push_error: make_push(toasts.clone(), next_id.clone(), ToastKind::Error),
    };

    let items = (*toasts)
        .iter()
        .map(|t| {
            let cls = match t.kind {
                ToastKind::Success => "toast toast--success",
                ToastKind::Error => "toast toast--error",
            };
            html! { <div key={t.id} class={cls} role="status">{ t.msg.clone() }</div> }
        })
        .collect::<Html>();

    html! {
        <ContextProvider<ToastHandle> context={handle}>
            { props.children.clone() }
            <div class="toast-stack">{ items }</div>
        </ContextProvider<ToastHandle>>
    }
}
```

- [ ] **Step 2 : Monter `<ToastProvider>` sous `<LocaleProvider>` dans `main.rs`**

Remplacer le corps de `fn app()` par :
```rust
    html! {
        <BrowserRouter>
            <i18n::LocaleProvider>
                <toast::ToastProvider>
                    <AuthProvider>
                        <Switch<Route> render={switch} />
                    </AuthProvider>
                </toast::ToastProvider>
            </i18n::LocaleProvider>
        </BrowserRouter>
    }
```

- [ ] **Step 3 : Styles toasts dans `frontend/styles/app.css`** (ajouter en fin de fichier)

```css
/* ---- Toasts (overlay coin haut-droit) ---- */
.toast-stack {
  position: fixed;
  top: 16px;
  right: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 1000;
  pointer-events: none;
}
.toast {
  pointer-events: auto;
  min-width: 220px;
  max-width: 360px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  font-size: 13.5px;
  color: white;
  box-shadow: 0 4px 12px rgb(0 0 0 / 0.18);
  animation: toast-in 0.18s ease-out;
}
.toast--success {
  background: hsl(var(--color-success));
}
.toast--error {
  background: hsl(var(--color-destructive));
}
@keyframes toast-in {
  from { opacity: 0; transform: translateY(-6px); }
  to   { opacity: 1; transform: translateY(0); }
}
```
> `--color-success` est ajouté en Task 6 (Step 1). D'ici là le toast succès n'aura pas de fond coloré — non bloquant ; la validation couleur des toasts se fait en Task 6+.

- [ ] **Step 4 : Câbler un toast sur la copie + i18n du libellé (`copy_button.rs`)**

Remplacer le contenu de `frontend/src/components/copy_button.rs` par :
```rust
//! Bouton-icône « copier » : confirmation éphémère inline + toast global.

use gloo_timers::callback::Timeout;
use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::toast::use_toast;
use crate::util::clipboard;

#[derive(Properties, PartialEq)]
pub struct CopyButtonProps {
    pub value: String,
    #[prop_or_default]
    pub aria_label: Option<AttrValue>,
}

#[function_component(CopyButton)]
pub fn copy_button(props: &CopyButtonProps) -> Html {
    let _loc = crate::i18n::use_locale(); // abonnement i18n (re-render au switch de langue)
    let toast = use_toast();
    let copied = use_state(|| false);

    let onclick = {
        let (value, copied, toast) = (props.value.clone(), copied.clone(), toast.clone());
        Callback::from(move |_| {
            clipboard::copy(value.clone());
            copied.set(true);
            toast.push_success.emit(t!("toast.copied").to_string());
            let copied = copied.clone();
            Timeout::new(2000, move || copied.set(false)).forget();
        })
    };

    html! {
        <Button variant={Variant::Ghost} size={Size::Sm} onclick={onclick}
                aria_label={props.aria_label.clone()}>
            { if *copied { t!("common.copied") } else { std::borrow::Cow::Borrowed("⧉") } }
        </Button>
    }
}
```
> `t!` renvoie `Cow<'static, str>` → rendu directement par yew. Le bras `else` retourne `Cow::Borrowed("⧉")` pour que les deux bras aient le même type.

- [ ] **Step 5 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK.

Lancer la stack live (cf. Global Constraints). Avec Playwright :
1. `browser_navigate` → `http://127.0.0.1:5150/admin` ; se connecter (`admin`/`secret`).
2. Créer un projet, aller au détail, cliquer le bouton copier de l'URL.
3. `browser_snapshot` : un toast « Copied to clipboard. » apparaît en haut à droite et disparaît après ~4 s.
Expected: toast visible puis auto-dismiss.

- [ ] **Step 6 : Commit**

```bash
rtk git add frontend/src/toast.rs frontend/src/main.rs frontend/styles/app.css frontend/src/components/copy_button.rs
rtk git commit -m "✨ feat(toast): ToastProvider maison (gloo-timers) + câblage copie"
```

---

## Task 4 : Composant Toggle vendorisé (patch du Switch shadcn-rs)

**Files:**
- Create: `frontend/src/components/toggle.rs`
- Modify: `frontend/src/components/mod.rs`
- Modify: `frontend/src/panels/project_form.rs` (swap `Switch` → `Toggle`)
- Modify: `frontend/src/panels/deploy.rs` (swap `Switch` → `Toggle`)

**Interfaces:**
- Produces: composant `Toggle` avec props `{ checked: bool, disabled: bool, id: Option<AttrValue>, onchange: Option<Callback<Event>>, aria_label: Option<AttrValue> }`. État **contrôlé pur** (pas d'état interne).

- [ ] **Step 1 : Créer `frontend/src/components/toggle.rs`**

```rust
//! Toggle — `Switch` shadcn-rs 0.1 vendorisé et patché.
//! Bug d'origine (switch.rs) : `is_checked = if checked { checked } else { *internal }`
//! → quand le parent passe `checked=false`, le composant retombe sur son état interne
//! (déjà basculé) et ne revient jamais visuellement à off (cf. QUIRKS). Ici : état
//! 100% contrôlé (`is_checked = checked`), zéro état interne. Réutilise les classes
//! CSS `.switch` / `.size-md` / `.switch-thumb` / `.switch-checked` / `.switch-disabled`
//! déjà vendorisées (components.css).

use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ToggleProps {
    #[prop_or(false)]
    pub checked: bool,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or_default]
    pub id: Option<AttrValue>,
    #[prop_or_default]
    pub onchange: Option<Callback<Event>>,
    #[prop_or_default]
    pub aria_label: Option<AttrValue>,
}

#[function_component(Toggle)]
pub fn toggle(props: &ToggleProps) -> Html {
    let ToggleProps {
        checked,
        disabled,
        id,
        onchange,
        aria_label,
    } = props.clone();

    let onclick = {
        let onchange = onchange.clone();
        Callback::from(move |e: MouseEvent| {
            if !disabled {
                if let Some(cb) = onchange.as_ref() {
                    cb.emit(e.into());
                }
            }
        })
    };
    let onkeydown = {
        let onchange = onchange.clone();
        Callback::from(move |e: KeyboardEvent| {
            if !disabled && (e.key() == " " || e.key() == "Enter") {
                e.prevent_default();
                if let Some(cb) = onchange.as_ref() {
                    cb.emit(e.into());
                }
            }
        })
    };

    // size-md est LOAD-BEARING : `.switch` seul n'a ni hauteur ni largeur (cf. components.css).
    let classes = classes!(
        "switch",
        "size-md",
        checked.then_some("switch-checked"),
        disabled.then_some("switch-disabled"),
    );

    html! {
        <button
            type="button"
            role="switch"
            class={classes}
            aria-checked={checked.to_string()}
            aria-label={aria_label}
            disabled={disabled}
            onclick={onclick}
            onkeydown={onkeydown}
            id={id}
        >
            <span class="switch-thumb" aria-hidden="true"></span>
        </button>
    }
}
```

- [ ] **Step 2 : Déclarer le module** dans `frontend/src/components/mod.rs`

```rust
pub mod copy_button;
pub mod pin_field;
pub mod toggle;
```

- [ ] **Step 3 : Swap dans `project_form.rs`**

Dans l'import shadcn (ligne ~4-7), retirer `Switch` de la liste. Ajouter sous les `use` : `use crate::components::toggle::Toggle;`. Remplacer la balise `<Switch id="pf-code" checked={*code_on} onchange={on_code_toggle} />` par :
```rust
                <Toggle id={AttrValue::from("pf-code")} checked={*code_on} onchange={on_code_toggle.clone()} />
```

- [ ] **Step 4 : Swap dans `deploy.rs`**

Idem : retirer `Switch` de l'import shadcn, ajouter `use crate::components::toggle::Toggle;`. Remplacer `<Switch id="dp-activate" checked={*activate} onchange={on_toggle} />` par :
```rust
                <Toggle id={AttrValue::from("dp-activate")} checked={*activate} onchange={on_toggle.clone()} />
```

- [ ] **Step 5 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK (plus aucune référence à `Switch`).

Playwright (stack live) :
1. Connexion → « New project » → le toggle « Code d'accès » est ON par défaut.
2. Cliquer le toggle → `browser_evaluate` : `getComputedStyle($switch).backgroundColor` change (perd la couleur primary) **et** le thumb revient à gauche (`transform` ≈ none). Re-cliquer → revient ON visuellement.
Expected: bascule visuelle effective dans les deux sens (le bug est corrigé).

- [ ] **Step 6 : Commit**

```bash
rtk git add frontend/src/components/toggle.rs frontend/src/components/mod.rs frontend/src/panels/project_form.rs frontend/src/panels/deploy.rs
rtk git commit -m "🐛 fix(ui): Toggle vendorisé (patch Switch shadcn-rs, état contrôlé pur)"
```

---

## Task 5 : LocaleSwitcher + Login (i18n, espacement, toast)

**Files:**
- Create: `frontend/src/components/locale_switcher.rs`
- Modify: `frontend/src/components/mod.rs`
- Modify: `frontend/src/pages/login.rs`
- Modify: `frontend/styles/app.css` (`.locale-switcher`, espacement login)

**Interfaces:**
- Consumes: `use_locale()` (Task 2), `Locale`.
- Produces: composant `LocaleSwitcher` (sans props).

- [ ] **Step 1 : Créer `frontend/src/components/locale_switcher.rs`**

```rust
//! Sélecteur de langue FR/EN (deux boutons maison). Pilote la locale via le contexte.

use yew::prelude::*;

use crate::i18n::{use_locale, Locale};

#[function_component(LocaleSwitcher)]
pub fn locale_switcher() -> Html {
    let loc = use_locale();

    let mk = |target: Locale, label: &'static str| {
        let set = loc.set_locale.clone();
        let active = loc.locale == target;
        let onclick = Callback::from(move |_: MouseEvent| set.emit(target));
        let class = if active {
            "locale-btn locale-btn--active"
        } else {
            "locale-btn"
        };
        html! { <button type="button" class={class} {onclick} aria-pressed={active.to_string()}>{ label }</button> }
    };

    html! {
        <span class="locale-switcher" aria-label="Language">
            { mk(Locale::En, "EN") }
            { mk(Locale::Fr, "FR") }
        </span>
    }
}
```

- [ ] **Step 2 : Déclarer le module** dans `frontend/src/components/mod.rs`

```rust
pub mod copy_button;
pub mod locale_switcher;
pub mod pin_field;
pub mod toggle;
```

- [ ] **Step 3 : Réécrire `frontend/src/pages/login.rs`** (i18n + switcher + toast + espacement)

Remplacer les imports en tête par :
```rust
use shadcn_rs::{Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Variant};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::auth::use_auth;
use crate::components::locale_switcher::LocaleSwitcher;
use crate::i18n::use_locale;
use crate::routes::Route;
use crate::toast::use_toast;
```
Au début de `login_page`, après `let auth = use_auth();`, ajouter :
```rust
    let _loc = use_locale();
    let toast = use_toast();
```
Dans `on_submit`, ajouter `toast` aux captures et émettre un toast d'erreur. Remplacer le bloc `Callback::from(move |_: MouseEvent| { ... })` de `on_submit` par :
```rust
        let toast = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let body = latch_dto::LoginReq {
                user: (*user).clone(),
                pass: (*pass).clone(),
            };
            let (error, busy, set_auth, navigator, toast) = (
                error.clone(),
                busy.clone(),
                set_auth.clone(),
                navigator.clone(),
                toast.clone(),
            );
            error.set(None);
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::login(&body).await {
                    Ok(()) => {
                        set_auth.emit(());
                        navigator.push(&Route::Home);
                    }
                    Err(_) => {
                        error.set(Some(t!("login.error_invalid").to_string()));
                        toast.push_error.emit(t!("login.error_invalid").to_string());
                    }
                }
                busy.set(false);
            });
        })
```
Remplacer le `html!` final par (i18n + switcher + classe d'espacement sur le bouton) :
```rust
    html! {
        <div class="auth-screen">
            <Card>
                <CardHeader>
                    <CardTitle>{ t!("login.title") }</CardTitle>
                </CardHeader>
                <CardContent>
                    <Label html_for="user">{ t!("login.user") }</Label>
                    <Input id="user" value={(*user).clone()} oninput={on_user} />
                    <Label html_for="pass">{ t!("login.pass") }</Label>
                    <Input id="pass" r#type="password" value={(*pass).clone()} oninput={on_pass} />
                    if let Some(msg) = (*error).clone() {
                        <p class="error">{ msg }</p>
                    }
                    <Button variant={Variant::Primary} full_width={true}
                            class={classes!("login-submit")}
                            disabled={*busy} onclick={on_submit}>
                        { if *busy { t!("login.submitting") } else { t!("login.submit") } }
                    </Button>
                    <div class="auth-footer"><LocaleSwitcher /></div>
                </CardContent>
            </Card>
        </div>
    }
```

- [ ] **Step 4 : Styles dans `frontend/styles/app.css`** (ajouter en fin de fichier)

```css
/* ---- Espacement login + footer langue ---- */
.login-submit {
  margin-top: 16px;
}
.auth-footer {
  margin-top: 16px;
  display: flex;
  justify-content: center;
}

/* ---- Sélecteur de langue ---- */
.locale-switcher {
  display: inline-flex;
  gap: 2px;
  border: 1px solid hsl(var(--color-border));
  border-radius: var(--radius-md);
  overflow: hidden;
}
.locale-btn {
  border: 0;
  background: transparent;
  padding: 4px 10px;
  font-size: 12px;
  cursor: pointer;
  color: hsl(var(--color-muted-foreground));
}
.locale-btn--active {
  background: hsl(var(--color-primary));
  color: hsl(var(--color-primary-foreground));
}
```

- [ ] **Step 5 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK.

Playwright :
1. `browser_navigate` → `/admin/login`. `browser_evaluate` : la marge entre l'input password et le bouton (`getComputedStyle(loginBtn).marginTop`) ≥ 16px.
2. Cliquer « FR » dans le switcher → `browser_snapshot` : « Sign in » devient « Se connecter », « Username » → « Identifiant ». Cliquer « EN » → revient en anglais.
3. Recharger la page (`browser_navigate` à nouveau) après avoir choisi FR → l'UI reste en FR (persistance localStorage).
Expected: espacement OK, switch FR↔EN réactif, persistance au reload.

- [ ] **Step 6 : Commit**

```bash
rtk git add frontend/src/components/locale_switcher.rs frontend/src/components/mod.rs frontend/src/pages/login.rs frontend/styles/app.css
rtk git commit -m "✨ feat(login): i18n + sélecteur de langue + espacement + toast erreur"
```

---

## Task 6 : Liste (i18n, badges colorés, accessibilité, switcher, toasts)

**Files:**
- Modify: `frontend/styles/variables.css` (ajout `--color-success*` / `--color-warning*`)
- Modify: `frontend/styles/app.css` (`.badge--success`/`.badge--warning`, `.linkish`, `.page-intro`)
- Modify: `frontend/src/pages/list.rs`

**Interfaces:**
- Consumes: `use_locale()`, `use_toast()`, `LocaleSwitcher`.
- Produces: classes CSS `.badge--success` / `.badge--warning` / `.linkish` réutilisées au détail (Task 9).

- [ ] **Step 1 : Ajouter les variables de couleur dans `frontend/styles/variables.css`**

Repérer le bloc `:root { ... }` et le bloc `.dark { ... }`. Dans `:root`, ajouter (couleurs HSL « H S% L% », cohérentes avec la palette) :
```css
  --color-success: 142 71% 38%;
  --color-success-foreground: 0 0% 100%;
  --color-warning: 32 95% 44%;
  --color-warning-foreground: 0 0% 100%;
```
Dans `.dark`, ajouter :
```css
  --color-success: 142 64% 45%;
  --color-success-foreground: 0 0% 100%;
  --color-warning: 32 90% 55%;
  --color-warning-foreground: 0 0% 0%;
```
> Précédent documenté : QUIRKS « variables --color-card*/--color-popover* manquantes » — même type de patch sur la CSS vendorisée.

- [ ] **Step 2 : Styles badges + helpers dans `frontend/styles/app.css`** (fin de fichier)

```css
/* ---- Badges accès (vert = PIN requis, orange = libre) ---- */
.badge--success {
  background: hsl(var(--color-success));
  color: hsl(var(--color-success-foreground));
}
.badge--warning {
  background: hsl(var(--color-warning));
  color: hsl(var(--color-warning-foreground));
}

/* ---- Lien-bouton accessible (remplace <a onclick> sans href) ---- */
.linkish {
  background: none;
  border: 0;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
  text-align: left;
}
.linkish:hover {
  text-decoration: underline;
}

/* ---- Intro de page ---- */
.page-intro {
  color: hsl(var(--color-muted-foreground));
  font-size: 13.5px;
  margin: -8px 0 20px;
}
```

- [ ] **Step 3 : Réécrire `frontend/src/pages/list.rs`** (i18n, badges, accessibilité, switcher, intro, toast création)

Remplacer les imports en tête par :
```rust
use shadcn_rs::{
    Badge, Button, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiError};
use crate::auth::use_auth;
use crate::components::copy_button::CopyButton;
use crate::components::locale_switcher::LocaleSwitcher;
use crate::i18n::use_locale;
use crate::panels::project_form::{FormMode, ProjectForm};
use crate::routes::Route;
use crate::toast::use_toast;
use crate::util::url::public_url;
use latch_dto::ProjectListItem;
```
Au début de `list_page`, après `let auth = use_auth();`, ajouter :
```rust
    let _loc = use_locale();
    let toast = use_toast();
```
Remplacer le calcul `badge` (dans la closure des lignes) par des badges colorés via `class` :
```rust
                let badge = if p.code_enabled {
                    html! { <Badge variant={Variant::Secondary} class={classes!("badge--success")}>{ t!("list.badge_code_on") }</Badge> }
                } else {
                    html! { <Badge variant={Variant::Outline} class={classes!("badge--warning")}>{ t!("list.badge_free") }</Badge> }
                };
```
> Si `Badge` n'accepte pas `class` (vérifier la signature shadcn-rs ; il l'accepte via `#[prop_or_default] class: Classes`), enrober plutôt dans `<span class="badge--success">…</span>`. Choisir la forme qui compile.

Remplacer la version active par i18n :
```rust
                let version = match p.active_version_id {
                    Some(_) => html! { <span>{ t!("list.active") }</span> },
                    None => html! { <span>{ t!("common.dash") }</span> },
                };
```
Remplacer les cellules `<a onclick=... style="cursor:pointer">` par des `<button class="linkish">` (accessibilité). La cellule nom :
```rust
                        <TableCell>
                            <button class="linkish" onclick={onclick.clone()}>{ p.name.clone() }</button>
                        </TableCell>
```
La cellule URL conserve `<code>` + `CopyButton` mais avec aria i18n :
```rust
                        <TableCell>
                            <code>{ url.clone() }</code>
                            <CopyButton value={url} aria_label={AttrValue::from(t!("list.copy_url_aria").to_string())} />
                        </TableCell>
```
Les cellules code et version :
```rust
                        <TableCell>
                            <button class="linkish" onclick={onclick.clone()}>{ badge }</button>
                        </TableCell>
                        <TableCell>
                            <button class="linkish" onclick={onclick}>{ version }</button>
                        </TableCell>
```
Remplacer les en-têtes de colonnes et l'état vide par i18n :
```rust
        Load::Loading => html! { <p>{ t!("common.loading") }</p> },
        Load::Failed(msg) => html! { <p class="error">{ msg.clone() }</p> },
        Load::Ready(items) if items.is_empty() => html! {
            <div class="empty-state">
                <p>{ t!("list.empty") }</p>
                <Button variant={Variant::Primary} onclick={on_new.clone()}>
                    { t!("list.create_first") }
                </Button>
            </div>
        },
```
En-têtes :
```rust
                            <TableHead>{ t!("list.col_name") }</TableHead>
                            <TableHead>{ t!("list.col_url") }</TableHead>
                            <TableHead>{ t!("list.col_code") }</TableHead>
                            <TableHead>{ t!("list.col_version") }</TableHead>
```
Topbar : ajouter intro + switcher + i18n des boutons. Remplacer le `html!` final par :
```rust
    html! {
        <div class="admin-page">
            <header class="topbar">
                <span class="brand">{ "latch" }</span>
                <span class="actions">
                    <LocaleSwitcher />
                    <Button variant={Variant::Primary} onclick={on_new}>{ t!("common.new_project") }</Button>
                    <Button variant={Variant::Ghost} onclick={on_logout}>{ t!("common.logout") }</Button>
                </span>
            </header>
            <p class="page-intro">{ t!("list.intro") }</p>
            { body }
            <ProjectForm
                open={*creating}
                mode={FormMode::Create}
                on_close={{ let c = creating.clone(); Callback::from(move |_| c.set(false)) }}
                on_saved={{
                    let data = data.clone();
                    let toast = toast.clone();
                    Callback::from(move |_| {
                        toast.push_success.emit(t!("toast.project_created").to_string());
                        let data = data.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Ok(items) = api::client::list_projects().await {
                                data.set(Load::Ready(items));
                            }
                        });
                    })
                }}
            />
        </div>
    }
```

- [ ] **Step 4 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK.

Playwright :
1. `/admin` avec ≥1 projet à code activé et ≥1 libre. `browser_evaluate` : le badge « PIN required » a `backgroundColor` vert (≈ `hsl(142 71% 38%)`), le badge « Open » orange.
2. `browser_snapshot` : l'intro de page est présente ; le switcher est dans la topbar ; basculer FR re-rend la liste (colonnes + boutons traduits).
3. Vérifier que les cellules cliquables sont des `<button class="linkish">` (Tab navigue dessus, focus visible).
Expected: badges colorés, i18n réactive, navigation clavier OK.

- [ ] **Step 5 : Commit**

```bash
rtk git add frontend/styles/variables.css frontend/styles/app.css frontend/src/pages/list.rs
rtk git commit -m "✨ feat(list): i18n + badges colorés (vars success/warning) + a11y + switcher + toast"
```

---

## Task 7 : ProjectForm (i18n, PIN disabled, slug disabled, helper text, toast)

**Files:**
- Modify: `frontend/src/panels/project_form.rs`
- Modify: `frontend/styles/app.css` (`.field-help`, état disabled input)

**Interfaces:**
- Consumes: `use_locale()`, `use_toast()`, `Toggle` (Task 4).

- [ ] **Step 1 : Styles helper text dans `frontend/styles/app.css`** (fin de fichier)

```css
/* ---- Aide sous les champs de formulaire ---- */
.field-help {
  display: block;
  color: hsl(var(--color-muted-foreground));
  font-size: 12px;
  margin: 2px 0 6px;
}
/* Input désactivé (PIN quand code off, slug en édition) */
.sheet-content .input:disabled {
  opacity: 0.55;
  cursor: not-allowed;
  background: hsl(var(--color-muted));
}
```

- [ ] **Step 2 : i18n + toast + use_locale dans `project_form.rs`**

Ajouter aux `use` : `use crate::i18n::use_locale;` et `use crate::toast::use_toast;`. Au début de `project_form`, après `let is_edit = ...;`, ajouter :
```rust
    let _loc = use_locale();
    let toast = use_toast();
```
Dans `on_save`, sur le bras `Ok(())`, émettre le bon toast. Capturer `toast` + `is_edit` dans la closure (`is_edit` est déjà calculé ; le re-dériver depuis `mode`). Remplacer le bras `Ok(())` du `match res` par :
```rust
                    Ok(()) => {
                        busy.set(false);
                        let msg = match &mode {
                            FormMode::Create => t!("toast.project_created"),
                            FormMode::Edit(_) => t!("toast.project_updated"),
                        };
                        toast.push_success.emit(msg.to_string());
                        on_saved.emit(());
                        on_close.emit(());
                    }
```
Et ajouter `toast` aux captures en tête de `on_save` (le tuple `let (on_saved, on_close, mode) = (...)` devient `let (on_saved, on_close, mode, toast) = (props.on_saved.clone(), props.on_close.clone(), props.mode.clone(), toast.clone());`), puis le re-cloner dans la closure interne comme les autres.

Remplacer les messages d'erreur de validation par i18n :
```rust
            if name.trim().is_empty() {
                error.set(Some(t!("form.err_name").to_string()));
                return;
            }
            if *code_on && !pin::is_valid_pin(&pin_val) {
                error.set(Some(t!("form.err_pin").to_string()));
                return;
            }
```

- [ ] **Step 3 : Réécrire le `html!` de `project_form.rs`** (i18n, slug disabled, PIN toujours affiché + disabled, helper text)

Remplacer tout le bloc `html! { <SheetContent ...> ... </SheetContent> }` par :
```rust
    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader>
                <SheetTitle>{ if is_edit { t!("form.title_edit") } else { t!("form.title_create") } }</SheetTitle>
            </SheetHeader>

            <Label html_for="pf-name" required={true}>{ t!("form.name") }</Label>
            <Input id="pf-name" value={(*name).clone()} oninput={on_name} />
            <span class="field-help">{ t!("form.name_help") }</span>

            if is_edit {
                <Label html_for="pf-slug">{ t!("form.slug") }</Label>
                <Input id="pf-slug" value={initial.slug.clone()} disabled={true} />
                <span class="field-help">{ t!("form.slug_help") }</span>
            }

            <Label html_for="pf-brand">{ t!("form.brand") }</Label>
            <Input id="pf-brand" value={(*brand).clone()} oninput={on_brand} />
            <span class="field-help">{ t!("form.brand_help") }</span>

            <Label html_for="pf-code">{ t!("form.code") }</Label>
            <div class="toggle-row">
                <Toggle id={AttrValue::from("pf-code")} checked={*code_on} onchange={on_code_toggle.clone()} />
                <span class="hint">{ t!("form.code_help") }</span>
            </div>

            <Label html_for="pf-pin">{ t!("form.pin") }</Label>
            <div class="pin-row">
                <Input id="pf-pin" value={(*pin_val).clone()} oninput={on_pin} disabled={!*code_on} />
                <Button variant={Variant::Outline} onclick={on_regen} disabled={!*code_on}>{ t!("common.regenerate") }</Button>
            </div>
            <span class="field-help">{ t!("form.pin_help") }</span>

            if let Some(msg) = (*error).clone() {
                <p class="error">{ msg }</p>
            }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ t!("common.cancel") }</Button>
                <Button variant={Variant::Primary} onclick={on_save} disabled={*busy}>
                    { if *busy { t!("common.saving") } else { t!("common.save") } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
```
> Changement clé : le champ PIN n'est plus enveloppé dans `if *code_on { ... }` (plus de saut de layout) ; il est **toujours rendu** et `disabled={!*code_on}`. Le slug passe de `readonly={true}` à `disabled={true}`.

- [ ] **Step 4 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK.

Playwright :
1. « New project » : helper text visible sous chaque champ. Toggle code OFF → le champ PIN reste affiché mais grisé (`browser_evaluate` : `pinInput.disabled === true`, opacité réduite), pas de saut de layout. Toggle ON → PIN réactivé.
2. Ouvrir un projet existant → « Edit » : le champ slug est `disabled` (`slugInput.disabled === true`), non éditable au clavier.
3. Enregistrer → toast « Project updated. » / « Project created. ».
Expected: PIN disabled (pas masqué), slug non éditable, helper text présent, toast.

- [ ] **Step 5 : Commit**

```bash
rtk git add frontend/src/panels/project_form.rs frontend/styles/app.css
rtk git commit -m "✨ feat(form): i18n + PIN disabled (au lieu de masqué) + slug disabled + helper text + toast"
```

---

## Task 8 : DeployPanel (i18n, dropzone drag-and-drop, toast)

**Files:**
- Modify: `frontend/Cargo.toml` (web-sys `DragEvent`, `DataTransfer`, `HtmlElement`)
- Modify: `frontend/src/panels/deploy.rs`
- Modify: `frontend/styles/app.css` (`.dropzone`)

**Interfaces:**
- Consumes: `use_locale()`, `use_toast()`, `Toggle`, `gloo_file`.

- [ ] **Step 1 : Features web-sys**

Dans `frontend/Cargo.toml`, ajouter à `[dependencies.web-sys] features = [...]` : `"DragEvent"`, `"DataTransfer"`, `"HtmlElement"`.

- [ ] **Step 2 : Styles dropzone dans `frontend/styles/app.css`** (fin de fichier)

```css
/* ---- Dropzone upload HTML ---- */
.dropzone {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 28px 16px;
  border: 2px dashed hsl(var(--color-border));
  border-radius: var(--radius-md);
  background: hsl(var(--color-muted) / 0.4);
  color: hsl(var(--color-muted-foreground));
  font-size: 13.5px;
  cursor: pointer;
  text-align: center;
  transition: border-color 0.15s, background 0.15s;
}
.dropzone:hover {
  border-color: hsl(var(--color-primary));
}
.dropzone--over {
  border-color: hsl(var(--color-primary));
  background: hsl(var(--color-primary) / 0.08);
  color: hsl(var(--color-foreground));
}
.dropzone__file {
  font-weight: 600;
  color: hsl(var(--color-foreground));
}
```

- [ ] **Step 3 : Réécrire `frontend/src/panels/deploy.rs`** (dropzone + i18n + toast)

```rust
//! Side-panel Déployer une version : dropzone HTML (drag-and-drop + clic) → POST /deploy.

use shadcn_rs::{Button, Label, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlElement};
use yew::prelude::*;

use crate::api;
use crate::components::toggle::Toggle;
use crate::i18n::use_locale;
use crate::toast::use_toast;
use latch_dto::DeployReq;

#[derive(Properties, PartialEq)]
pub struct DeployPanelProps {
    pub open: bool,
    pub project_id: i32,
    pub on_close: Callback<()>,
    pub on_deployed: Callback<()>,
}

/// Formate une taille d'octets en texte court (« 12.3 KB »).
fn human_size(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0} B")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    }
}

#[function_component(DeployPanel)]
pub fn deploy_panel(props: &DeployPanelProps) -> Html {
    let _loc = use_locale();
    let toast = use_toast();
    let html_content = use_state(|| Option::<String>::None);
    let file_label = use_state(|| Option::<String>::None);
    let activate = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);
    let over = use_state(|| false);
    let input_ref = use_node_ref();

    {
        let (html_content, file_label, error, activate, over) = (
            html_content.clone(),
            file_label.clone(),
            error.clone(),
            activate.clone(),
            over.clone(),
        );
        use_effect_with(props.open, move |_| {
            html_content.set(None);
            file_label.set(None);
            error.set(None);
            activate.set(true);
            over.set(false);
            || ()
        });
    }

    // Charge un gloo_file::File : lit le texte + pose le label.
    let load_file = {
        let (html_content, file_label, error) =
            (html_content.clone(), file_label.clone(), error.clone());
        move |file: web_sys::File| {
            let label = format!("{} ({})", file.name(), human_size(file.size()));
            file_label.set(Some(label));
            let gfile = gloo_file::File::from(file);
            let (html_content, error) = (html_content.clone(), error.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match gloo_file::futures::read_as_text(&gfile).await {
                    Ok(text) => html_content.set(Some(text)),
                    Err(_) => error.set(Some(t!("deploy.err_read").to_string())),
                }
            });
        }
    };

    let on_input_change = {
        let load_file = load_file.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    load_file(file);
                }
            }
        })
    };

    let on_zone_click = {
        let input_ref = input_ref.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(input) = input_ref.cast::<HtmlElement>() {
                input.click();
            }
        })
    };

    let on_dragover = {
        let over = over.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            over.set(true);
        })
    };
    let on_dragleave = {
        let over = over.clone();
        Callback::from(move |_: DragEvent| over.set(false))
    };
    let on_drop = {
        let (over, load_file) = (over.clone(), load_file.clone());
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            over.set(false);
            if let Some(dt) = e.data_transfer() {
                if let Some(files) = dt.files() {
                    if let Some(file) = files.get(0) {
                        load_file(file);
                    }
                }
            }
        })
    };

    let on_toggle = {
        let activate = activate.clone();
        Callback::from(move |_: Event| activate.set(!*activate))
    };

    let on_deploy = {
        let (html_content, activate, error, busy, toast) = (
            html_content.clone(),
            activate.clone(),
            error.clone(),
            busy.clone(),
            toast.clone(),
        );
        let (on_close, on_deployed, id) = (
            props.on_close.clone(),
            props.on_deployed.clone(),
            props.project_id,
        );
        Callback::from(move |_: MouseEvent| {
            let Some(html) = (*html_content).clone() else {
                error.set(Some(t!("deploy.err_no_file").to_string()));
                return;
            };
            let req = DeployReq { html, activate: *activate };
            let (on_close, on_deployed, error, busy, toast) = (
                on_close.clone(),
                on_deployed.clone(),
                error.clone(),
                busy.clone(),
                toast.clone(),
            );
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::deploy(id, &req).await {
                    Ok(_) => {
                        toast.push_success.emit(t!("toast.version_deployed").to_string());
                        on_deployed.emit(());
                        on_close.emit(());
                    }
                    Err(e) => {
                        let m = e.user_message();
                        error.set(Some(m.clone()));
                        toast.push_error.emit(m);
                    }
                }
                busy.set(false);
            });
        })
    };

    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_: MouseEvent| on_close.emit(()))
    };

    let zone_class = if *over { "dropzone dropzone--over" } else { "dropzone" };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader><SheetTitle>{ t!("deploy.title") }</SheetTitle></SheetHeader>

            <Label html_for="dp-file">{ t!("deploy.file") }</Label>
            <div class={zone_class} onclick={on_zone_click}
                 ondragover={on_dragover} ondragleave={on_dragleave} ondrop={on_drop}>
                if let Some(label) = (*file_label).clone() {
                    <span class="dropzone__file">{ label }</span>
                } else if *over {
                    <span>{ t!("deploy.dropzone_hover") }</span>
                } else {
                    <span>{ t!("deploy.dropzone_idle") }</span>
                }
            </div>
            <input ref={input_ref} id="dp-file" type="file" accept="text/html,.html"
                   style="display:none" onchange={on_input_change} />

            <div class="toggle-row">
                <Toggle id={AttrValue::from("dp-activate")} checked={*activate} onchange={on_toggle} />
                <span class="hint">{ t!("deploy.activate_help") }</span>
            </div>

            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ t!("common.cancel") }</Button>
                <Button variant={Variant::Primary} disabled={*busy} onclick={on_deploy}>
                    { if *busy { t!("deploy.deploying") } else { t!("deploy.btn") } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
```
> `JsCast` importé pour `input_ref.cast::<HtmlElement>()`. `HtmlElement::click()` nécessite la feature `HtmlElement`. `DragEvent`/`DataTransfer` pour le drop.

- [ ] **Step 4 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK.

Playwright :
1. Détail projet → « Deploy » : la dropzone stylée s'affiche (`browser_evaluate` : `getComputedStyle(zone).borderStyle === "dashed"`).
2. Choisir un fichier HTML via clic (`browser_file_upload` sur l'input caché) → le nom + taille s'affichent dans la zone.
3. Déployer → toast « Version deployed. » ; le détail recharge avec la nouvelle version.
Expected: dropzone visible, fichier chargé via clic, toast de succès.

- [ ] **Step 5 : Commit**

```bash
rtk git add frontend/Cargo.toml frontend/src/panels/deploy.rs frontend/styles/app.css
rtk git commit -m "✨ feat(deploy): dropzone drag-and-drop + i18n + Toggle + toast"
```

---

## Task 9 : Détail + panels danger (i18n, badges, accessibilité, toasts, intros)

**Files:**
- Modify: `frontend/src/pages/detail.rs`
- Modify: `frontend/src/components/pin_field.rs`
- Modify: `frontend/src/panels/delete_project.rs`
- Modify: `frontend/src/panels/delete_version.rs`

**Interfaces:**
- Consumes: `use_locale()`, `use_toast()`, classes `.badge--success/.badge--warning/.linkish` (Task 6).

- [ ] **Step 1 : `pin_field.rs` — i18n des aria-labels**

Ajouter `use crate::i18n::use_locale;`. Au début de `pin_field`, `let _loc = use_locale();`. Remplacer les `aria_label` et le `CopyButton` :
```rust
            <Button variant={Variant::Ghost} size={Size::Sm} onclick={toggle}
                    aria_label={ AttrValue::from(if *revealed { t!("detail.hide_pin").to_string() } else { t!("detail.reveal_pin").to_string() }) }>
                { if *revealed { "🙈" } else { "👁" } }
            </Button>
            <CopyButton value={props.pin.clone()} aria_label={AttrValue::from(t!("detail.copy_pin_aria").to_string())} />
```

- [ ] **Step 2 : `detail.rs` — use_locale + toasts + i18n + accessibilité**

Ajouter aux `use` : `use crate::i18n::use_locale;` et `use crate::toast::use_toast;`. Au début de `detail_page`, après `let auth = use_auth();`, ajouter :
```rust
    let _loc = use_locale();
    let toast = use_toast();
```
Dans la closure `activate` (ligne ~133), émettre un toast après succès. Remplacer la closure `activate` par :
```rust
                let activate = {
                    let reload = reload.clone();
                    let toast = toast.clone();
                    Callback::from(move |_| {
                        let (reload, toast) = (reload.clone(), toast.clone());
                        wasm_bindgen_futures::spawn_local(async move {
                            match api::client::activate_version(id, n).await {
                                Ok(()) => toast.push_success.emit(t!("toast.version_activated").to_string()),
                                Err(e) => toast.push_error.emit(e.user_message()),
                            }
                            reload.emit(());
                        });
                    })
                };
```
Remplacer les textes en dur de `body` et du `html!` final par `t!` :
- `Load::Loading` → `{ t!("common.loading") }`.
- Carte « Accès public » : `t!("detail.access_title")`, label URL `t!("detail.url_label")`, aria copy `t!("detail.copy_url_aria")`, label code `t!("detail.code_label")`, `t!("detail.pin_undefined")` (badge), `t!("detail.free_access")` (badge).
  - Pour le badge « Accès libre », ajouter la classe orange : `<Badge variant={Variant::Outline} class={classes!("badge--warning")}>{ t!("detail.free_access") }</Badge>`. Pour « PIN non défini », garder Outline sans couleur.
- Carte « Configuration » : `t!("detail.config_title")`, `t!("detail.brand_label")`, valeur marque `unwrap_or_else(|| t!("common.dash").to_string())`, label « Code », valeur `if p.code_enabled { t!("detail.code_on") } else { t!("detail.code_off") }`.
- Versions : titre `t!("detail.versions_title")`, en-têtes `t!("detail.col_num")`/`t!("detail.col_date")`/`t!("detail.col_status")`, badge actif `<Badge variant={Variant::Secondary} class={classes!("badge--success")}>{ t!("common.active") }</Badge>`, aria activer/preview/supprimer via `t!("detail.activate_aria")`/`t!("detail.preview_aria")`/`t!("detail.delete_aria")`.
- En-tête : breadcrumb en `<button class="linkish crumb">` (accessibilité, plus de `<a onclick>`) avec `t!("detail.back")` ; boutons actions `t!("common.edit")`/`t!("common.deploy")`/`t!("common.delete")` (conserver les glyphes : ex. `format!("✎ {}", t!("common.edit"))` ou simplement le texte + glyphe via deux nœuds).

Exemple concret pour le breadcrumb (remplacer le `<a class="crumb" onclick={on_back}>` ) :
```rust
                            <button class="linkish crumb" onclick={on_back}>{ t!("detail.back") }</button>
```
Exemple pour les boutons d'action (remplacer le bloc `.head-actions`) :
```rust
                        <div class="head-actions">
                            <Button variant={Variant::Outline} onclick={open_edit}>{ t!("common.edit") }</Button>
                            <Button variant={Variant::Outline} onclick={open_deploy}>{ t!("common.deploy") }</Button>
                            <Button variant={Variant::Destructive} onclick={open_delete}>{ t!("common.delete") }</Button>
                        </div>
```
Ajouter une intro de page après le `<header class="detail-head">…</header>` :
```rust
                    <p class="page-intro">{ t!("detail.intro") }</p>
```
La cellule preview reste un vrai `<a href target="_blank">` (lien réel — ne pas convertir), avec `aria-label={AttrValue::from(t!("detail.preview_aria").to_string())}` :
```rust
                            <a href={preview_href} target="_blank" rel="noopener" class="icon-link"
                               aria-label={AttrValue::from(t!("detail.preview_aria").to_string())}>{ "↗" }</a>
```

- [ ] **Step 3 : `delete_project.rs` — i18n + toast**

Ajouter `use crate::i18n::use_locale;` et `use crate::toast::use_toast;`. Au début, `let _loc = use_locale(); let toast = use_toast();`. Dans `on_confirm`, capturer `toast` et émettre sur succès :
```rust
                match api::client::delete_project(id).await {
                    Ok(()) => {
                        toast.push_success.emit(t!("toast.project_deleted").to_string());
                        on_deleted.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
```
(ajouter `toast` aux tuples de capture, comme les autres champs). Remplacer le `html!` par i18n :
```rust
            <SheetHeader><SheetTitle>{ t!("danger.del_project_title", name = props.project.name.clone()) }</SheetTitle></SheetHeader>
            <p>{ t!("danger.del_project_intro") }</p>
            <ul>
                <li>{ t!("danger.del_project_li1") }</li>
                <li>{ t!("danger.del_project_li2", count = n_versions) }</li>
                <li>{ t!("danger.del_project_li3") }</li>
            </ul>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ t!("common.cancel") }</Button>
                <Button variant={Variant::Destructive} disabled={*busy} onclick={on_confirm}>
                    { t!("danger.del_project_confirm") }
                </Button>
            </SheetFooter>
```

- [ ] **Step 4 : `delete_version.rs` — i18n + toast**

Ajouter `use crate::i18n::use_locale;` et `use crate::toast::use_toast;`. Au début, `let _loc = use_locale(); let toast = use_toast();`. Émettre le toast sur succès (capturer `toast`) :
```rust
                match api::client::delete_version(id, n).await {
                    Ok(()) => {
                        toast.push_success.emit(t!("toast.version_deleted").to_string());
                        on_deleted.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
```
Remplacer le `html!` par i18n :
```rust
            <SheetHeader><SheetTitle>{ t!("danger.del_version_title", n = props.n) }</SheetTitle></SheetHeader>
            <p>{ t!("danger.del_version_intro") }</p>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ t!("common.cancel") }</Button>
                <Button variant={Variant::Destructive} disabled={*busy} onclick={on_confirm}>
                    { if *busy { t!("danger.deleting") } else { t!("danger.del_version_confirm") } }
                </Button>
            </SheetFooter>
```

- [ ] **Step 5 : Build + validation Playwright**

Run: `cd frontend && rtk trunk build` — Expected: OK.

Playwright (parcours complet) :
1. Détail : intro présente ; badge « active » vert ; badge accès « Open » orange si libre ; breadcrumb est un `<button>` focusable.
2. Activer une version → toast « Version activated. ». Supprimer une version inactive → toast « Version deleted. ». Supprimer le projet → toast « Project deleted. » + retour liste.
3. Switch FR : détail entièrement traduit (titres de cartes, colonnes, boutons, panels danger).
Expected: i18n complète, badges colorés, toasts, a11y.

- [ ] **Step 6 : Commit**

```bash
rtk git add frontend/src/pages/detail.rs frontend/src/components/pin_field.rs frontend/src/panels/delete_project.rs frontend/src/panels/delete_version.rs
rtk git commit -m "✨ feat(detail): i18n + badges colorés + a11y + toasts (activate/delete) + intro"
```

---

## Task 10 : Balayage final — résidus FR, qualité, validation e2e, mémoire

**Files:**
- Modify: `frontend/index.html` (lang)
- Modify: `frontend/src/routes.rs` (404 i18n — mineur)
- Modify: docs mémoire (INDEX, HANDOFF, QUIRKS, CONVENTIONS, CLAUDE)

- [ ] **Step 1 : Audit des chaînes FR résiduelles en dur**

Run: `rtk grep -nE "Chargement|Annuler|Supprimer|Déployer|Identifiant|Mot de passe|Connexion|Copié|libre|activé|régénérer" frontend/src`
Expected: ne renvoie que des occurrences déjà passées par `t!` (clés), ou rien. Toute chaîne FR en dur restante dans un `html!`/`error.set(...)` → la remplacer par la clé `t!` adéquate (ajouter la clé aux deux YAML si manquante).

- [ ] **Step 2 : `index.html` lang neutre**

Dans `frontend/index.html`, remplacer `<html lang="fr">` par `<html lang="en">` (défaut EN ; le contenu est piloté par i18n côté wasm).

- [ ] **Step 3 : 404 i18n (mineur)**

Dans `frontend/src/routes.rs`, le bras `Route::NotFound` rend `<h1>{ "404" }</h1>` — laisser tel quel (numérique, neutre). Aucune action requise (documenté ici pour éviter une fausse alerte au Step 1).

- [ ] **Step 4 : Qualité — fmt + clippy + tests + build**

Run: `rtk cargo fmt --all`
Run: `rtk cargo clippy -p latch-ui --target wasm32-unknown-unknown -- -D warnings`
Expected: 0 warning.
Run: `cd frontend && rtk proxy wasm-pack test --headless --firefox`
Expected: tests verts (i18n + pin/url/clipboard).
Run: `cd frontend && rtk trunk build`
Expected: OK.

- [ ] **Step 5 : Validation Playwright de bout en bout (parcours complet)**

Stack live. Dérouler : login (FR↔EN, espacement) → liste (badges, intro, switcher) → créer projet (toggle, PIN disabled, helper, toast) → détail (intro, badges, copie+toast) → déployer (dropzone, toast) → activer (toast) → supprimer version + projet (toasts). Prendre des `browser_take_screenshot` aux étapes clés. Confirmer : aucun texte FR résiduel en mode EN, aucun saut de layout, tous les toggles basculent visuellement.
Expected: parcours complet vert au navigateur.

- [ ] **Step 6 : Mémoire projet (NON-NÉGOCIABLE — définition de « terminé »)**

Mettre à jour :
- `docs/INDEX.md` — ajouter les livrables Phase 3 polish : i18n (LocaleProvider + rust-i18n + sélecteur), couche toasts, Toggle vendorisé, dropzone, badges colorés, helper text/intros, a11y.
- `docs/HANDOFF.md` — entrée datée en haut : dernière chose faite, suspens, prochaine chose (reprendre le choix merge/PR de `feat/phase-3-spa-yew-admin`), notes.
- `docs/QUIRKS.md` — ajouter : (a) `--color-success`/`--color-warning` absents de la CSS vendorisée → ajoutés dans variables.css ; (b) `Switch` shadcn-rs corrigé par vendorisation (`components/toggle.rs`, classe `size-md` load-bearing) ; (c) réactivité i18n = abonnement obligatoire via `use_locale()`.
- `docs/CONVENTIONS.md` — patterns : `LocaleProvider`/`use_locale` + `t!`, `ToastProvider`/`use_toast`, composant vendorisé type (Toggle), règle « vendoriser shadcn-rs cassé ».
- `CLAUDE.md` — si la règle de vendorisation mérite le statut de règle permanente, l'ajouter (sinon laisser dans CONVENTIONS).
- `docs/contrat-deploy.md` — §7 : noter le sélecteur de langue (FR/EN) et l'i18n si cela change un comportement décrit.
- Cocher les items traités dans `docs/superpowers/specs/2026-06-24-phase-3-punchlist-ux.md`.

- [ ] **Step 7 : Commit final**

```bash
rtk git add -A
rtk git commit -m "📝 docs: clôture polish UX + i18n (mémoire à jour, audit chaînes, lang)"
```

---

## Self-Review (effectuée à la rédaction)

**Couverture spec :**
- §3.1 i18n (rust-i18n, LocaleProvider, switcher, persistance, boot) → Tasks 1, 2, 5. ✓
- §3.2 toasts (provider gloo-timers, tous retours d'action) → Task 3 + câblages Tasks 5-9. ✓
- §3.3 Toggle vendorisé → Task 4. ✓
- §5.1 login spacing → Task 5. ✓
- §5.2 badges colorés + finding vars success/warning → Task 6 (Step 1 ajoute les vars). ✓
- §5.3 PIN disabled + slug disabled → Task 7. ✓
- §5.4 dropzone → Task 8. ✓
- §6 helper text + intros + a11y + tout-i18n → Tasks 5-9 + audit Task 10. ✓
- §7 tests (wasm + Playwright) → présents à chaque task + Task 10. ✓
- §2 principe vendorisation → Task 4 + mémoire Task 10. ✓

**Cohérence des types :** `LocaleContext { locale, set_locale }`, `ToastHandle { push_success, push_error }`, `Toggle { checked, disabled, id, onchange, aria_label }` — référencés de façon identique partout. ✓

**Placeholders :** aucun « TBD/TODO ». Les seuls renvois conditionnels (`Badge` accepte-t-il `class` → sinon `<span>`) sont accompagnés de l'alternative concrète. ✓
