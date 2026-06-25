# Phase 7 — Lot 4 : Page d'erreur stylée pour le serving `/c/<slug>`

> Design doc. Quatrième et dernier lot de la Phase 7 (« Peaufinage graphique / web »).
> Statut : design validé (brainstorming 2026-06-26). Backend Rust + 3ᵉ entrée Vite.

## Contexte & motivation

Aujourd'hui, les branches d'erreur de `controllers/serve.rs` (slug inconnu, projet sans
version active, erreur interne) renvoient l'erreur Loco par défaut — **du JSON brut** (ex.
`{"error":"not_found"}` + 404) — sur une **surface publique vue par le client final**. C'est
moche et incohérent avec la page de déverrouillage stylée. Ce lot sert à la place une **page
HTML stylée**, cohérente avec unlock, `no-store`.

### État actuel (constaté)

- `controllers/serve.rs::serve` renvoie `Err(loco_rs::Error::NotFound)` / `.map_err(into_response)?`
  sur les branches d'erreur → rendu JSON par le renderer Loco global.
- Branches concernées :
  - **slug inconnu** : `svc.get_by_slug(&slug).await.map_err(into_response)?` (NotFound→404, Db→500) ;
  - **pas de version active** : `let Some(active_id) = … else { return Err(loco_rs::Error::NotFound) }` ;
  - **version introuvable** : `.ok_or(loco_rs::Error::NotFound)?` (incohérence interne) ;
  - **erreur storage** : `storage.read(...).await.map_err(into_response)?` (→ 500).
- `unlock.html` est servi par `serve.rs::unlock_page_response()` qui **lit le fichier du disque**
  (`crate::web::unlock_index()` = `spa_dist_dir().join("unlock.html")`) et le renvoie via
  `html_response` (`no-store`, `text/html`, status 200).
- `controllers/error.rs::into_response` mappe `CoreError` → `loco_rs::Error`. Le renderer Loco
  global doit rester JSON (admin/MCP).
- `public_meta` (`/api/public/{slug}`) renvoie du JSON (API) — hors périmètre.
- Backlog Phase 4 ouvert : « erreur opaque + sans log de `storage.read` dans serve.rs » — ce lot
  le résout (log serveur + message client générique).

## Décisions de design (tranchées au brainstorming)

| # | Décision | Choix retenu |
|---|---|---|
| D1 | Rendu | **3ᵉ entrée Vite `error.html`** (calquée sur unlock) servie par `serve.rs` (lecture disque). Cohérence Tailwind + logo Lot 3 + i18n client-side. |
| D2 | Granularité des messages | **Générique unique** : un seul message pour tous les cas (slug inconnu, sans version, erreur interne). Seul le **status HTTP** varie (404/500). `serve.rs` sert `error.html` tel quel → **zéro injection**, **pas de leak** d'existence de slug. |
| D3 | i18n | **Client-side bilingue** (instance i18next dédiée + `LanguageDetector`, pattern unlock + auto-découverte Lot 1). |
| D4 | Brand | **Aucun** (page générique, aucune donnée projet, aucun fetch). |

## Objectifs

1. `/c/<slug>` ne renvoie **plus de JSON brut** sur slug inconnu / sans version / erreur interne →
   page HTML stylée, `no-store`, status correct (404/500).
2. Page cohérente avec unlock (logo, palette claire, carte centrée), bilingue client-side.
3. Les 500 (DB/storage) **loggent côté serveur** (`tracing::error!`) ; message client générique.

### Non-objectifs

- Distinguer les cas d'erreur (générique assumé, cf. D2).
- Toucher `public_meta` (reste JSON) ou le renderer Loco global.
- Thème dark sur la page d'erreur (clair-only comme unlock).

## Architecture

### Frontend (3ᵉ entrée Vite)

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/error.html` | Entrée Vite (title, robots noindex, favicon, `#error-root`, script) | **Créer** |
| `frontend/src/error/main.tsx` | Monte `ErrorPage` dans `I18nextProvider` | **Créer** |
| `frontend/src/error/error-page.tsx` | Carte centrée : Logo + titre + message + `useDocumentTitle` | **Créer** |
| `frontend/src/error/i18n.ts` | Instance i18next dédiée, glob `locales/error/*.json` | **Créer** |
| `frontend/src/i18n/locales/error/{en,fr}.json` | `_meta` + 3 clés | **Créer** |
| `frontend/vite.config.ts` | `error` ajouté à `rollupOptions.input` | **Modifier** |

`error.html` (calqué sur `unlock.html`) : `lang`, `<meta name="robots" content="noindex, nofollow">`,
`<link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg">`, `<div id="error-root">`,
`<script type="module" src="/src/error/main.tsx">`.

`error-page.tsx` : `<div flex min-h-svh items-center justify-center bg-background p-4>` →
`<div flex w-full max-w-sm flex-col items-center gap-6>` → `<Logo className="size-12" />` +
`<Card className="w-full">` (`CardHeader` > `CardTitle` = `t('error.title')` ; `CardContent`/
`CardDescription` = `t('error.message')`). `useDocumentTitle(t('error.page_title'))`. Aucun fetch,
aucun état, aucun `LocaleSwitcher`.

`error/i18n.ts` : `i18next.createInstance()` + `LanguageDetector` + `parseLocales(import.meta.glob(
'../i18n/locales/error/*.json', { eager: true }))`, `escapeValue: false`, `lookupLocalStorage:
'latch.locale'` (cohérent unlock).

Clés (`locales/error/{en,fr}.json`, `_meta` + 3) :
| Clé | EN | FR |
|---|---|---|
| `error.title` | Prototype unavailable | Prototype indisponible |
| `error.message` | This prototype is not available or has been removed. | Ce prototype n'est pas disponible ou a été retiré. |
| `error.page_title` | Unavailable — latch | Indisponible — latch |

### Backend

`web/mod.rs` : `pub fn error_index() -> PathBuf { spa_dist_dir().join("error.html") }` (miroir de
`unlock_index()`).

`controllers/serve.rs` :
```rust
/// Sert la page d'erreur stylée (error.html buildé) avec le status donné, no-store.
/// Fallback texte inline si le fichier manque — jamais de JSON brut sur /c.
async fn serve_error_page(status: StatusCode) -> Response {
    let path = crate::web::error_index();
    let html = tokio::fs::read_to_string(&path).await.unwrap_or_else(|_| {
        "<!doctype html><meta charset=utf-8><title>latch</title>\
         <p>Ce prototype n'est pas disponible.</p>".to_string()
    });
    (
        status,
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8")),
        ],
        html,
    ).into_response()
}
```

Branches de `serve` modifiées (le reste — unlock/cookie/HTML actif — inchangé) :
- **slug inconnu / erreur DB** : `get_by_slug` passe d'un `?` à un `match` :
  `Ok(p) => p` ;
  `Err(CoreError::NotFound) => return Ok(serve_error_page(StatusCode::NOT_FOUND).await)` ;
  `Err(e) => { tracing::error!(error = %e, slug = %slug, "serve: db error"); return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await) }`.
- **pas de version active** : `else { return Ok(serve_error_page(StatusCode::NOT_FOUND).await) }`.
- **version introuvable** : `None => { tracing::error!(...); return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await) }`.
- **erreur storage** : `Err(e) => { tracing::error!(error = %e, version = active_id, "serve: storage read"); return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await) }`.

`use crate::services::errors::CoreError;` ajouté. `public_meta` inchangé (JSON).

## Plan de tests

**Backend (intégration, harness loco + fake dist)** — pattern `fake_dist` existant (tempdir avec
un `error.html` factice posé + `LATCH_SPA_DIST` exporté avant `request::<App>`) :
- GET `/c/<slug-inconnu>` → **404**, `content-type: text/html`, `cache-control: no-store`, corps
  contient `id="error-root"` (marqueur d'`error.html`).
- GET `/c/<slug-sans-version-active>` → **404** html (projet créé sans version active).
- (Helper/fallback : `error.html` absent → réponse texte inline, pas de JSON, status correct.)
- Invariant maintenu : `/c` ne porte ni hash ni PIN (la page générique n'expose rien).

**Frontend (Vitest)** :
- `error-page.test.tsx` : Logo (`alt="latch"`) + `error.message` rendus ; `document.title` posé.

**Build** : `error.html` produit dans `dist/` ; favicon + logo sous `/assets` ; bundle error sans
code admin.

**e2e (optionnel, léger)** : `serve-unlock.spec.ts` — « GET /c/bogus → page stylée 404 ».

## Critères de sortie du Lot 4

1. `/c/<slug>` ne renvoie plus de JSON brut (slug inconnu / sans version / erreur interne) → page
   HTML stylée, `no-store`, status 404/500.
2. Page cohérente avec unlock (logo, palette, centrée), bilingue client-side.
3. 500 loggés côté serveur (`tracing::error!`), message client générique.
4. `cargo fmt` + `cargo clippy -D warnings` + `cargo nextest` + `pnpm lint/typecheck/test` + build
   verts ; **SonarCloud new_coverage ≥ 80 %**.
5. Mémoire à jour : INDEX, HANDOFF, CONVENTIONS (3ᵉ entrée Vite + serve_error_page), QUIRKS,
   BACKLOG (backlog storage-log Phase 4 → RÉSOLU), **ROADMAP : Phase 7 LIVRÉE**.

## Risques & points de vigilance

- **Tests intégration & fake dist** : le harness doit poser un `error.html` factice dans le dist de
  test (comme pour `unlock.html`) sinon `serve_error_page` tombe sur le fallback inline (test du
  fallback distinct).
- **`tracing::error!`** : ne jamais logguer de secret (slug = non sensible ; ne pas logguer le PIN).
- **Isolation bundle** : `error.html` est une entrée publique — pas d'import de code admin.
- **Build/Docker** : `error.html` à la racine du `dist/` (comme `unlock.html`), servi par lecture
  disque (pas le mount `/assets`). Vérifier que le Dockerfile copie tout `dist/` (déjà le cas).

## Dépendances

- Consomme du Lot 1 : `parseLocales` (auto-découverte locales). Du Lot 3 : `<Logo>`, `useDocumentTitle`.
- Clôt la Phase 7.
