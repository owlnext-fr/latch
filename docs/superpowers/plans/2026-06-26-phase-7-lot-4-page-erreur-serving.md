# Phase 7 — Lot 4 : Page d'erreur stylée serving `/c` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer le JSON brut des branches d'erreur de `/c/<slug>` par une page HTML stylée (3ᵉ entrée Vite `error.html`), message générique unique, `no-store`, status 404/500, + log serveur des 500.

**Architecture:** Une 3ᵉ entrée Vite `error.html` (React minimal, calquée sur unlock : Logo + carte centrée + i18n client-side). `serve.rs` la lit du disque (`web::error_index()`) et la sert avec le status voulu via un helper `serve_error_page`. Les branches `Err` terminales de `serve` deviennent des `Ok(serve_error_page(...))`.

**Tech Stack:** Rust (Loco/axum, SeaORM, tracing), React 19 + Vite 8 + react-i18next, Vitest + Testing Library, cargo nextest (intégration loco + SQLite de test).

## Global Constraints

- **Surface publique** : aucune réponse `/c` ne porte de hash ni de PIN. La page d'erreur est **générique** (aucune donnée projet) → pas de leak.
- **`no-store`** sur toute réponse `/c` (page d'erreur comprise), `content-type: text/html; charset=utf-8`.
- **Message générique unique** : même corps pour tous les cas (slug inconnu / sans version / interne). Seul le **status** varie (404 not-found, 500 interne). `serve.rs` sert `error.html` tel quel (zéro injection).
- **500 loggés** côté serveur via `tracing::error!` (jamais de secret ; slug OK, pas le PIN). Message client générique.
- **`public_meta` (`/api/public/{slug}`) reste JSON** — ne pas toucher. Le renderer Loco global reste JSON (admin/MCP).
- **Isolation bundle** : `error.html` est une entrée publique — pas d'import de code admin.
- **i18n** : instance i18next dédiée + `parseLocales` (auto-découverte Lot 1), `lookupLocalStorage: 'latch.locale'`.
- **Confidentialité** : aucun nom de client réel (fixtures = placeholders fictifs `Mon Projet`/`ACME`).
- **Couverture** : SonarCloud `new_coverage ≥ 80 %`.
- **Commandes** : backend depuis la racine (`cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run`) ; frontend depuis `frontend/` (`rtk vitest run`, `pnpm typecheck`, `rtk lint`, `pnpm build`). Loco se lance depuis `backend/`.
- **Subagents** : IGNORER le protocole load-memory du CLAUDE.md, ne pas répondre « Mémoire chargée ».

---

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/error.html` | 3ᵉ entrée Vite | **Créer** |
| `frontend/src/error/main.tsx` | Mount ErrorPage + I18nextProvider | **Créer** |
| `frontend/src/error/error-page.tsx` | Carte centrée Logo + titre + message | **Créer** |
| `frontend/src/error/error-page.test.tsx` | Test | **Créer** |
| `frontend/src/error/i18n.ts` | Instance i18next dédiée (glob error locales) | **Créer** |
| `frontend/src/i18n/locales/error/en.json`, `.../fr.json` | `_meta` + 3 clés | **Créer** |
| `frontend/vite.config.ts` | `error` dans `rollupOptions.input` | **Modifier** |
| `backend/src/web/mod.rs` | `error_index()` | **Modifier** |
| `backend/src/controllers/serve.rs` | `serve_error_page` + branches d'erreur | **Modifier** |
| `backend/tests/serve.rs` | fake_dist + tests 404 stylés + fallback | **Modifier** |
| `docs/INDEX.md`, `HANDOFF.md`, `CONVENTIONS.md`, `QUIRKS.md`, `BACKLOG.md`, `ROADMAP.md` | mémoire | **Modifier** |

---

## Task 1 : Frontend — 3ᵉ entrée Vite `error.html` + page React

**Files:**
- Create: `frontend/error.html`, `frontend/src/error/main.tsx`, `frontend/src/error/error-page.tsx`, `frontend/src/error/error-page.test.tsx`, `frontend/src/error/i18n.ts`, `frontend/src/i18n/locales/error/en.json`, `frontend/src/i18n/locales/error/fr.json`
- Modify: `frontend/vite.config.ts`

**Interfaces:**
- Consumes: `parseLocales` (`@/i18n/available-locales`, Lot 1), `<Logo>` (`@/components/logo`, Lot 3), `useDocumentTitle` (`@/hooks/use-document-title`, Lot 3).
- Produces: built `dist/error.html` (servi par le backend en Task 2).

- [ ] **Step 1 : Créer les catalogues de locale error**

Create `frontend/src/i18n/locales/error/en.json` :
```json
{
  "_meta": { "name": "English", "flag": "GB" },
  "error.title": "Prototype unavailable",
  "error.message": "This prototype is not available or has been removed.",
  "error.page_title": "Unavailable — latch"
}
```
Create `frontend/src/i18n/locales/error/fr.json` :
```json
{
  "_meta": { "name": "Français", "flag": "FR" },
  "error.title": "Prototype indisponible",
  "error.message": "Ce prototype n'est pas disponible ou a été retiré.",
  "error.page_title": "Indisponible — latch"
}
```

- [ ] **Step 2 : Créer `src/error/i18n.ts`** (calqué sur `src/unlock/i18n.ts`)

```ts
import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { parseLocales } from '@/i18n/available-locales'

const { resources, locales } = parseLocales(
  import.meta.glob('../i18n/locales/error/*.json', { eager: true }),
)

const instance = i18next.createInstance()
instance
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: locales.map((l) => l.code),
    keySeparator: false,
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: { order: ['localStorage', 'navigator'], lookupLocalStorage: 'latch.locale' },
  })

export default instance
```

- [ ] **Step 3 : Écrire le test (échoue)**

Create `frontend/src/error/error-page.test.tsx` :
```tsx
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { ErrorPage } from './error-page'

function renderError() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <ErrorPage />
      </I18nextProvider>,
    )
  })
}

describe('ErrorPage', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('renders the logo and the generic unavailable message', () => {
    renderError()
    expect(screen.getByAltText('latch')).toBeInTheDocument()
    expect(
      screen.getByText('This prototype is not available or has been removed.'),
    ).toBeInTheDocument()
  })

  it('sets the document title', () => {
    renderError()
    expect(document.title).toBe('Unavailable — latch')
  })
})
```

- [ ] **Step 4 : Lancer, vérifier l'échec**

Run: `rtk vitest run src/error/error-page.test.tsx`
Expected: FAIL — `./error-page` introuvable.

- [ ] **Step 5 : Implémenter `src/error/error-page.tsx`**

```tsx
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Logo } from '@/components/logo'
import { useDocumentTitle } from '@/hooks/use-document-title'

export function ErrorPage() {
  const { t } = useTranslation()
  useDocumentTitle(t('error.page_title'))

  return (
    <div className="flex min-h-svh items-center justify-center bg-background p-4">
      <div className="flex w-full max-w-sm flex-col items-center gap-6">
        <Logo className="size-12" />
        <Card className="w-full">
          <CardHeader>
            <CardTitle>{t('error.title')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground text-sm">{t('error.message')}</p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
```

- [ ] **Step 6 : Créer `src/error/main.tsx`** (calqué sur `src/unlock/main.tsx`)

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { ErrorPage } from './error-page'
import '@/index.css'

createRoot(document.getElementById('error-root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <ErrorPage />
    </I18nextProvider>
  </StrictMode>,
)
```

- [ ] **Step 7 : Créer `frontend/error.html`** (calqué sur `unlock.html`)

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="robots" content="noindex, nofollow" />
    <link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg" />
    <title>latch</title>
  </head>
  <body>
    <div id="error-root"></div>
    <script type="module" src="/src/error/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 8 : Ajouter l'entrée `error` à Vite**

Edit `frontend/vite.config.ts` — dans `build.rollupOptions.input`, ajouter après la ligne `unlock` :
```ts
        error: fileURLToPath(new URL('./error.html', import.meta.url)),
```

- [ ] **Step 9 : Lancer le test + build**

Run: `rtk vitest run src/error/error-page.test.tsx` puis `pnpm build`
Expected: PASS (2 tests) ; build produit `dist/error.html` (vérifier sa présence : `ls dist/error.html`).

- [ ] **Step 10 : Commit**

```bash
git add frontend/error.html frontend/src/error/ frontend/src/i18n/locales/error/ frontend/vite.config.ts
git commit -m "✨ feat(serve): page d'erreur stylée /c (3e entrée Vite error.html)"
```

---

## Task 2 : Backend — `serve_error_page` + branches d'erreur de `serve`

**Files:**
- Modify: `backend/src/web/mod.rs`
- Modify: `backend/src/controllers/serve.rs`
- Modify: `backend/tests/serve.rs`

**Interfaces:**
- Consumes: `dist/error.html` (Task 1) via `web::error_index()`.
- Produces: `web::error_index() -> PathBuf` ; `/c/<slug>` renvoie une page HTML stylée sur erreur.

- [ ] **Step 1 : Ajouter `error_index()` dans `web/mod.rs`**

Edit `backend/src/web/mod.rs` — juste après la fonction `unlock_index()` :
```rust
/// Chemin du `error.html` buildé (page d'erreur stylée du serving `/c`).
pub fn error_index() -> PathBuf {
    spa_dist_dir().join("error.html")
}
```

- [ ] **Step 2 : Mettre à jour `fake_dist()` + les tests 404 (RED)**

Edit `backend/tests/serve.rs` :
- Dans `fake_dist()`, après l'écriture de `unlock.html`, ajouter l'écriture d'`error.html` :
```rust
    std::fs::write(
        dir.path().join("error.html"),
        "<!doctype html><title>latch</title><div id=\"error-root\">latch-error</div>",
    )
    .expect("write error.html");
```
- Remplacer le corps du test `unknown_slug_is_404` par :
```rust
#[tokio::test]
#[serial]
async fn unknown_slug_serves_styled_error_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/c/nope-xxxxxxxx").await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8",
            "page d'erreur HTML, pas du JSON"
        );
        assert_eq!(res.headers().get("cache-control").unwrap(), "no-store");
        assert!(res.text().contains("error-root"), "rend error.html");
    })
    .await;
}
```
- Remplacer le corps du test `project_without_active_version_is_404` par :
```rust
#[tokio::test]
#[serial]
async fn project_without_active_version_serves_styled_error_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        make_project(&ctx.db, "vide-aaaaaaaa", false, None, None).await;
        let res = request.get("/c/vide-aaaaaaaa").await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(res.text().contains("error-root"));
    })
    .await;
}
```

- [ ] **Step 3 : Lancer ces tests, vérifier l'échec**

Run (depuis la racine) : `cargo nextest run -p latch --test serve unknown_slug_serves_styled_error_404 project_without_active_version_serves_styled_error_404`
Expected: FAIL — aujourd'hui `serve` renvoie du JSON (content-type `application/json`), donc l'assert content-type/`error-root` casse.

- [ ] **Step 4 : Ajouter le helper `serve_error_page` dans `serve.rs`**

Edit `backend/src/controllers/serve.rs` — ajouter l'import en tête (avec les autres `use crate::...`) :
```rust
use crate::services::errors::CoreError;
```
puis ajouter le helper près de `html_response` :
```rust
/// Sert la page d'erreur stylée (`error.html` buildé) avec le status donné, `no-store`.
/// Fallback texte inline si le fichier manque — jamais de JSON brut sur `/c`.
async fn serve_error_page(status: StatusCode) -> Response {
    let path = crate::web::error_index();
    let html = tokio::fs::read_to_string(&path).await.unwrap_or_else(|_| {
        "<!doctype html><meta charset=utf-8><title>latch</title>\
         <p>Ce prototype n'est pas disponible.</p>"
            .to_string()
    });
    (
        status,
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
        ],
        html,
    )
        .into_response()
}
```

- [ ] **Step 5 : Réécrire les branches d'erreur de `serve`**

Edit `backend/src/controllers/serve.rs` — dans `serve`, remplacer le bloc actuel (de `let project = svc.get_by_slug...` jusqu'au `Ok(html_response(html))` final) par :
```rust
    // Slug inconnu → page d'erreur 404 ; erreur DB → 500 (loggée).
    let project = match svc.get_by_slug(&slug).await {
        Ok(p) => p,
        Err(CoreError::NotFound) => return Ok(serve_error_page(StatusCode::NOT_FOUND).await),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: get_by_slug failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };

    // Pas de version active → rien à servir → page d'erreur 404.
    let Some(active_id) = project.active_version_id else {
        return Ok(serve_error_page(StatusCode::NOT_FOUND).await);
    };

    // Projet protégé sans cookie valide → page de déverrouillage (avant de lire le HTML).
    if project.code_enabled {
        let pin = project.pin.clone().unwrap_or_default();
        let key = crate::web::unlock_key(&ctx)?;
        let jar = SignedCookieJar::from_headers(&headers, key);
        let now = chrono::Utc::now().timestamp();
        let secret = crate::web::unlock_secret(&ctx)?;
        let ok = match jar.get(UNLOCK_COOKIE_NAME) {
            Some(c) => verify_token(secret.as_bytes(), &slug, &pin, c.value(), now),
            None => false,
        };
        if !ok {
            return unlock_page_response().await;
        }
    }

    // Libre, ou protégé + cookie valide → servir le HTML de la version active.
    use crate::models::_entities::versions;
    let version = match versions::Entity::find_by_id(active_id).one(&ctx.db).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            tracing::error!(version = active_id, slug = %slug, "serve: active version row missing");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: version lookup failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };
    let storage = crate::web::storage_from_ctx(&ctx);
    let html = match storage.read(&version.html_path).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: storage read failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };
    Ok(html_response(html))
```
**Note** : `into_response` reste importé et utilisé par `public_meta` + `unlock` (ne pas retirer l'import). `StatusCode` est déjà importé (ligne `use axum::http::{HeaderMap, HeaderValue, StatusCode};`).

- [ ] **Step 6 : Lancer les tests serve ciblés, vérifier le succès**

Run: `cargo nextest run -p latch --test serve`
Expected: PASS — les 2 tests réécrits + tous les tests serve existants (libre, unlock, rotation, rate-limit) restent verts.

- [ ] **Step 7 : Ajouter le test du fallback (error.html absent)**

Edit `backend/tests/serve.rs` — ajouter :
```rust
#[tokio::test]
#[serial]
async fn missing_error_html_falls_back_to_inline_text() {
    // dist sans error.html → fallback inline (pas de JSON brut), toujours no-store.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("unlock.html"), "<title>u</title>").unwrap();
    std::env::set_var("LATCH_SPA_DIST", dir.path());
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/c/nope-yyyyyyyy").await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(res.headers().get("cache-control").unwrap(), "no-store");
        assert!(
            res.text().contains("pas disponible"),
            "fallback inline HTML, pas du JSON"
        );
        assert!(!res.text().contains("{"), "pas de JSON brut");
    })
    .await;
}
```

- [ ] **Step 8 : fmt + clippy + suite serve complète**

Run (depuis la racine) :
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run -p latch --test serve
```
Expected: fmt clean ; clippy 0 warning ; tous les tests serve verts (dont le fallback).

- [ ] **Step 9 : Commit**

```bash
git add backend/src/web/mod.rs backend/src/controllers/serve.rs backend/tests/serve.rs
git commit -m "✨ feat(serve): branches d'erreur /c en pages HTML stylées + log 500"
```

---

## Task 3 : Vérification finale + mémoire (Phase 7 LIVRÉE)

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md`, `docs/BACKLOG.md`, `docs/ROADMAP.md`

- [ ] **Step 1 : Gate complète**

Run :
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
cd frontend && rtk lint && pnpm typecheck && rtk vitest run --coverage && pnpm build
```
Expected: tout vert ; couverture du code neuf (`error-page`) ≥ 80 % ; `dist/error.html` présent.

- [ ] **Step 2 : Vérifier l'isolation du bundle error**

Run (depuis `frontend/`) :
```bash
grep -rl "ProjectForm\|deploy_token\|use-projects" dist/assets/*.js | grep -i error || echo "OK: bundle error sans code admin"
```
Expected: `OK: ...` — le chunk de l'entrée `error` ne contient pas de code admin.

- [ ] **Step 3 : `docs/CONVENTIONS.md`**

Ajouter :
```markdown
## Page d'erreur serving /c (Phase 7 Lot 4)
3ᵉ entrée Vite `error.html` (calquée sur unlock : `src/error/{main,error-page,i18n}.tsx` +
`locales/error/*.json` auto-découverts). Servie par `serve.rs::serve_error_page(status)` qui lit
`web::error_index()` (= `dist/error.html`) et renvoie HTML + `no-store` + status, avec un fallback
texte inline si le fichier manque. Les branches `Err` terminales de `serve` deviennent des
`Ok(serve_error_page(...))` (décision locale à l'adaptateur public ; le renderer Loco global reste
JSON pour admin/MCP). Message **générique unique** (zéro injection, pas de leak d'existence de slug).
```

- [ ] **Step 4 : `docs/QUIRKS.md`**

Ajouter :
```markdown
## fake_dist écrit unlock.html ET error.html (Phase 7 Lot 4)
Les tests d'intégration `serve` posent un faux `dist/` via `fake_dist()` : il écrit MAINTENANT
`unlock.html` ET `error.html` (marqueur `id="error-root"`). Un test dédié vérifie le fallback inline
quand `error.html` manque. Toute réponse `/c` (page d'erreur comprise) reste `no-store`.
```

- [ ] **Step 5 : `docs/BACKLOG.md`** — marquer RÉSOLU le backlog Phase 4

Trouver l'entrée « Erreur opaque + sans log de `storage.read` dans `serve.rs` » et la préfixer
`~~...~~ — RÉSOLU (Phase 7 Lot 4)` avec une ligne : « `serve.rs` logge désormais `tracing::error!`
sur les 500 (DB/storage/version) et renvoie une page générique au client. »

- [ ] **Step 6 : `docs/INDEX.md`** — ajouter une ligne

```markdown
| Phase 7 Lot 4 — Page d'erreur serving /c | Page HTML stylée (3e entrée Vite error.html) sur slug inconnu/sans version/erreur interne, no-store, status 404/500, log 500 serveur | `docs/superpowers/specs/2026-06-26-phase-7-lot-4-page-erreur-serving-design.md` · plan associé |
```

- [ ] **Step 7 : `docs/ROADMAP.md`** — clore la Phase 7

Marquer la **Phase 7 LIVRÉE** (les 4 lots faits) : ajouter `✅ LIVRÉE (2026-06-26)` au titre de la
Phase 7 et une courte note listant les 4 lots (fondations i18n/thème ; panneau Settings side-panel ;
identité visuelle ; page d'erreur serving).

- [ ] **Step 8 : `docs/HANDOFF.md`** — entrée datée

Sous le H1 : `Dernière chose faite` (Phase 7 LIVRÉE — Lot 4 page d'erreur stylée /c ; les 4 lots sur
`feat/phase-7-lot-1-fondations`, prêts au merge groupé), `Trucs en suspens` (merge des 4 lots = choix
humain ; e2e page d'erreur = optionnel non fait → BACKLOG), `Prochaine chose à creuser` (Phase 8
Fumadocs), `Notes pour future Claude` (3ᵉ entrée Vite + serve_error_page + fallback inline).

- [ ] **Step 9 : Commit**

```bash
git add docs/
git commit -m "📝 docs(phase-7): Lot 4 livré — Phase 7 LIVRÉE (mémoire + ROADMAP)"
```

---

## Self-Review (effectuée à l'écriture)

- **Couverture du spec** : 3ᵉ entrée Vite error.html (T1) ✓ ; ErrorPage Logo+message générique+titre (T1) ✓ ; i18n dédiée glob (T1) ✓ ; `error_index()` (T2) ✓ ; `serve_error_page` + fallback (T2) ✓ ; branches serve (slug inconnu/DB/sans version/version manquante/storage) → Ok(html) avec 404/500 + log (T2) ✓ ; `public_meta` intouché (non modifié) ✓ ; tests intégration 404 stylés + fallback (T2) ✓ ; test frontend (T1) ✓ ; gate + isolation + mémoire + ROADMAP LIVRÉE (T3) ✓.
- **Placeholders** : aucun ; code complet.
- **Cohérence des types** : `serve_error_page(StatusCode) -> Response` (T2) ; `error_index() -> PathBuf` (T2) ; `parseLocales`/`Logo`/`useDocumentTitle` réutilisés (Lots 1/3). `CoreError` a `Display` (thiserror) → `%e` compile.
- **Risque connu** : les 2 tests 404 existants sont réécrits (renommés) — c'est voulu (changement de comportement : JSON→HTML). `fake_dist` modifié écrit error.html → les autres tests serve restent verts (ils ne dépendent pas de l'absence d'error.html). Le test fallback utilise son propre tempdir sans error.html.
