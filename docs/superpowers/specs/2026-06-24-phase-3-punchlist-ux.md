# Phase 3 — Punch-list post-test live (retours UX) + chantier polish/i18n

> Doc intermédiaire. Issue d'un test manuel de la SPA avec l'humain (2026-06-24,
> via Playwright). Capture (1) les **patchs UX** à faire en priorité prochaine
> session, (2) les **bugs déjà corrigés** ce jour, (3) le **chantier plus large**
> de polissage produit / passage en anglais avant distribution.
>
> Process prochaine session : **appliquer les patchs → tout valider avec Playwright
> → self-review** (cf. §3).

## 1. Patchs UX à faire (prochaine session) — PRIORITAIRE

### Login
- [x] **Espacement** : ajouter un espace entre le champ « Mot de passe » et le bouton
  « Se connecter » (ils sont collés). (Vérifier le spacing dans `pages/login.rs` /
  `app.css`.)

### Liste
- [x] **Couleurs des badges code** : « code activé » → **vert**, « libre/désactivé »
  → **orange**. (Aujourd'hui `Variant::Secondary` / `Variant::Outline` dans
  `pages/list.rs` ; idem cohérence sur le détail. Les variables `--color-success`
  /`--color-warning` existent dans la CSS vendorisée — soit un `Variant` adéquat,
  soit des classes custom dans `app.css`.)

### Création / Modification (`panels/project_form.rs`)
- [x] **Le toggle « Code d'accès » ne bascule pas visuellement** (reste coché même
  quand l'état applicatif change correctement). C'est le *quirk* du `Switch`
  shadcn-rs (l'état « contrôlé » retombe sur l'état interne tant que `checked`
  passe par `false` — cf. QUIRKS). À régler : forcer le rendu contrôlé (ex. `key`
  qui change avec l'état, ou piloter autrement le composant, ou remplacer par un
  switch maison stylé).
- [x] **Désactivation du code = champ PIN disabled, pas masqué** : aujourd'hui le
  champ PIN est *retiré du DOM* quand le code est off (`if *code_on`), ce qui fait
  sauter le layout. À la place : **toujours afficher** le champ PIN, le passer en
  **`disabled`** (grisé) quand le code est off (et vider / neutraliser sa valeur).
  Plus UX-friendly (pas de saut de mise en page).

### Modification uniquement
- [x] **Slug éditable alors qu'il doit être en lecture seule** : l'input slug est
  modifiable. `readonly` ne suffit visiblement pas — le passer en **`disabled`**
  (grisé, non focusable) en mode édition. (`panels/project_form.rs`.)

### Déploiement (`panels/deploy.rs`)
- [x] **Dropzone** : remplacer l'`<input type="file">` brut (moche) par une vraie
  **zone de drop** (drag-and-drop + clic pour parcourir), stylée.
- [x] **Même bug de toggle** que la création (« Activer immédiatement ») → même
  correctif que le Switch ci-dessus.

### Général
- [x] **Snackbars / toasts** pour le retour des actions (succès / échec) :
  création, édition, déploiement, activation, suppression, copie. Aujourd'hui le
  feedback est inline/partiel et `activate_version` est silencieux. shadcn-rs
  `Toast`/`Sonner` sont **déclaratifs et sans auto-dismiss** (cf. QUIRKS) → il faut
  construire une **petite couche de toasts maison** (contexte Yew : `Vec<Toast>` +
  `gloo-timers` pour l'auto-dismiss + un provider monté à la racine), rendue via
  `SonnerToast` ou un composant maison.

## 2. Bugs déjà corrigés ce jour (2026-06-24) — NE PAS refaire

Découverts au test live (invisibles aux reviews SDD et au smoke curl, qui
n'exercent pas le wasm rendu — d'où l'importance de l'e2e Playwright, Phase 6) :

- ✅ **Routing 404** : `BrowserRouter basename="/admin"` cassait tout. yew-router
  0.18 a un bug dans `strip_basename` qui transforme l'URL racine exacte `/admin`
  en `//admin` (jamais matchée). **Fix** : retrait du `basename`, `#[at("/admin/...")]`
  **absolus**, pas de basename. (`frontend/src/routes.rs`, `main.rs`.)
- ✅ **CSS de layout absente** : seule la CSS des *composants* shadcn était
  vendorisée ; toutes les classes de *mise en page* de l'app (`.admin-page`,
  `.topbar`, `.kv`, `.toggle-row`, `.auth-screen`, `.detail-head`, `.pin-row`,
  `.empty-state`…) n'avaient aucune règle → login non centré, cartes pleine
  largeur. **Fix** : `frontend/styles/app.css` (liée après `shadcn-rs.css`,
  copiée par Trunk via `copy-dir`).
- ✅ **Animation Sheet buggée** : les keyframes `slide-in-*` de shadcn-rs 0.1
  laissent un `transform` résiduel (~`translateY(-50%)`) qui pousse le drawer hors
  écran → contenu invisible. **Fix** : `app.css` force `.sheet-content { animation:
  none !important; transform: none !important; display:flex; flex-direction:column }`
  (drawer statique, footer en bas via `margin-top:auto`).

## 3. Chantier plus large — polish produit + i18n (après les patchs §1)

L'humain estime (à raison) qu'il manque des choses pour un produit distribuable.
À traiter en self-review/itération dédiée :

- [x] **Explications sur les champs de formulaire** : helper text / descriptions
  sous chaque champ (au-delà du seul toggle code), pour guider l'utilisateur
  non-technique.
- [x] **Explications sur les pages** : courts textes d'intro / de contexte par
  écran (liste, détail, panels) — ce que fait la page, à quoi sert chaque bloc.
- [x] **UX-friendly pour distribution** : revue d'ensemble de l'ergonomie pour un
  livrable « propre » (états de chargement soignés, messages d'erreur clairs,
  cohérence visuelle, accessibilité — `<a>` sans href → `<button>`, focus, labels).
- [x] **Passer TOUS les textes en anglais (EN)** : l'UI est actuellement en
  français. Tout traduire (labels, boutons, messages, explications). Envisager une
  petite couche i18n ou au minimum centraliser les chaînes.
- [x] **Self-review** : relire l'ensemble produit (pas seulement le code) après les
  patchs, traquer ce qui manque encore pour une vraie distribution.

## 4. Notes de réalisation (pour la prochaine session)

- Lancer la SPA en dev : build `cd frontend && trunk build`, puis backend depuis
  `backend/` avec env (`ADMIN_USER`/`ADMIN_PASS`/`SESSION_SECRET`/`LATCH_SPA_DIST=
  ../frontend/dist`/`DATABASE_URL`…). Servie sous `http://127.0.0.1:5150/admin`.
- Pour une itération CSS pure : pas besoin de redémarrer le backend (ServeDir lit
  `dist/` à chaque requête) — juste `trunk build` + hard refresh (Ctrl-Shift-R).
- Valider chaque patch avec **Playwright** (navigate + screenshot + snapshot +
  `browser_evaluate` pour les styles calculés). C'est ce qui a permis de diagnostiquer
  les 3 bugs ci-dessus.
- Le `Switch` shadcn et l'animation `Sheet` sont les deux pièges connus de la lib —
  cf. `docs/QUIRKS.md`.
