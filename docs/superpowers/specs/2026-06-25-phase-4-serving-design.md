# Design — Phase 4 : serving `/c/<slug>` + déverrouillage

> Spec validée le 2026-06-25 (brainstorming). Source de vérité pour le plan
> d'implémentation. Le **contrat `docs/contrat-deploy.md` fait loi** — ce design en
> est la déclinaison Phase 4 (§6 serving, §9.5/§9.6 invariants). Toute divergence se
> résout en faveur du contrat, ou le contrat est amendé *en premier*.

## 0. Objectif & cadrage

Servir les prototypes sous `/c/<slug>` derrière un host contrôlé, avec deux états
(libre / protégé par code) et une **page de déverrouillage stylée** plutôt qu'un
Basic Auth. Le déverrouillage pose un **cookie signé** qui lie le PIN courant, et
`POST /c/<slug>/unlock` est protégé par un **rate-limit *load-bearing*** (la vraie
barrière derrière un PIN 6 chiffres).

**Décisions structurantes prises au brainstorming :**
- Rate-limit **100 % `governor`** (in-memory), deux clés : `IP+slug` (backoff) et
  `slug` seul (plafond global). **Pas de table** de tentatives, pas de scheduler,
  pas de purge, pas de stat admin (tous écartés sciemment — cf. §9).
- Cookie via **`SignedCookieJar` (axum-extra)** + **empreinte PIN** dans la valeur
  signée → révocation par rotation du code (§6) conservée.
- Page de déverrouillage = **entrée Vite dédiée React + composants shadcn réels**,
  bundle isolé (zéro code admin), alimentée par un endpoint meta public + fetch.

## 1. Routes & surfaces

Nouvel adaptateur entrant `backend/src/controllers/serve.rs`, monté dans `app.rs`.

| Méthode | Route | Auth | Rôle |
|---|---|---|---|
| `GET` | `/c/{slug}` | cookie unlock si projet protégé | sert le proto actif **ou** la page de déverrouillage |
| `POST` | `/c/{slug}/unlock` | rate-limité (pas de session) | vérifie le PIN, pose le cookie |
| `GET` | `/api/public/{slug}` | **aucune** | `{ brand_name, code_enabled }` pour la page unlock |

`/api/public/*` est public et structurellement incapable de fuiter le PIN (DTO dédié
sans champ `pin`). Les trois routes coexistent avec `/admin` et `/api` (admin) sur le
même binaire Loco.

## 2. `GET /c/{slug}` — arbre de décision (côté serveur)

Le serveur décide ; le HTML servi sur le chemin heureux est l'**artefact stocké**
(octets bruts), **jamais** du React.

```
slug inconnu                          → 404
projet sans version active            → 404   (rien à servir)
code désactivé                        → HTML stocké actif (Storage::read), no-store
code activé + cookie valide           → HTML stocké actif, no-store
code activé + cookie absent/invalide  → unlock.html buildé, HTTP 200, no-store
```

- Lecture du HTML : `Storage::read(&version.html_path)` (trait existant Phase 1).
- Pattern réponse brute : `([(CACHE_CONTROL, "no-store"), (CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()` — déjà employé par `preview_version` (Phase 2).
- `unlock.html` localisé via `web::unlock_dist()` (calqué sur `spa_dist_dir()`), servi
  tel quel — **aucune injection de placeholder** (les données dynamiques arrivent par
  le fetch meta, cf. §4).

## 3. Page de déverrouillage — entrée Vite dédiée

- **`frontend/unlock.html` + `frontend/src/unlock/main.tsx`** : 2ᵉ entrée Vite
  (`build.rollupOptions.input` multi-page → `dist/unlock.html`). Monte un petit arbre
  React avec les **vrais** `<Card>`/`<Input>`/`<Button>` shadcn et le **thème partagé**
  (mêmes tokens oklch). Bundle **isolé** : pas de router, pas de TanStack Query, pas
  d'openapi-fetch, aucun code admin.
- Flux client :
  1. au mount, `fetch GET /api/public/{slug}` → titre « Prototype préparé pour
     {brand_name} » (ou titre neutre si `brand_name` absent) ;
  2. saisie PIN (6 chiffres) → `fetch POST /c/{slug}/unlock`.
- Réponses gérées côté client :
  - **200/204** → `window.location.reload()` (le GET sert alors le proto) ;
  - **401/422** → message « code incorrect » inline ;
  - **429** → message « trop de tentatives, réessaie dans un moment ».
- i18n : `react-i18next` (FR/EN, défaut EN) avec un **catalogue minimal** propre à
  l'unlock (pas le catalogue admin complet).
- Le slug est lu depuis l'URL courante (`window.location.pathname`).

## 4. `GET /api/public/{slug}` — meta publique

- DTO `PublicMeta { brand_name: Option<String>, code_enabled: bool }` (dans `crate::dto`,
  dérive `Serialize` + `utoipa::ToSchema`).
- Slug existant → 200 + meta ; slug inconnu → 404.
- **Pas de PIN, pas de version, pas de date, pas de hash** — invariant §9.1/§9.2
  garanti par structure (le type n'a pas ces champs).
- `#[utoipa::path]` ajouté → régénérer `openapi.json` (`UPDATE_OPENAPI=1 cargo test
  --test openapi_drift`) **et** `schema.d.ts` (`pnpm gen:api`). Seule retouche au
  contrat OpenAPI ; elle n'expose rien de sensible (`brand_name` est fait pour être
  affiché publiquement sur la page unlock).

## 5. `POST /c/{slug}/unlock` + cookie

Ordre du handler :

1. **Rate-limit** (cf. §6) — en amont, via layers de route.
2. `ProjectsService::verify_code(slug, pin)` — comparaison **temps constant** (cœur,
   Phase 1). Slug inconnu → 404.
3. **Échec PIN** → **401** (corps minimal), **aucun cookie posé**.
4. **Succès** → pose le cookie via `SignedCookieJar` :
   - valeur signée = `exp ‖ fp`, où `fp = HMAC(UNLOCK_COOKIE_SECRET, slug ‖ pin_courant)`
     (fonction pure du cœur, §7) ;
   - attributs : `HttpOnly` (toujours), `Secure` (prod uniquement — fail-secure comme
     le cookie admin), `SameSite=Lax`, `Path=/c/{slug}`, `Max-Age = LATCH_UNLOCK_TTL_DAYS` ;
   - réponse **204** (corps vide) + `Set-Cookie`. Le client en **fetch** recharge
     lui-même (`window.location.reload()`) et le GET sert alors le proto. *(Pas un 303 :
     un `fetch` suivrait la redirection et récupérerait le HTML du proto au lieu de
     rendre la main au JS. Le 303 ne vaudrait que pour un `<form>` natif sans JS.)*

Validation du cookie au GET (§2) : le jar vérifie l'**intégrité** (clé globale), **puis**
le handler recalcule `fp` avec le **PIN actuel** et contrôle `exp` non dépassé.
PIN roté → `fp` ne matche plus → cookie rejeté ⇒ **révocation par rotation** (§6).
Le `Path=/c/{slug}` scope le cookie au projet.

## 6. Rate-limit `/unlock` — deux layers `governor` (in-memory)

Deux `GovernorLayer` empilés sur `POST /c/{slug}/unlock`, chacun avec un `KeyExtractor`
custom. Pattern établi (CONVENTIONS « Rate-limit tower_governor ») — on ajoute les deux
extracteurs.

| Layer | Clé | Défaut (réglable env) | Rôle §9.5 |
|---|---|---|---|
| **IP+slug** | `(SmartIp, slug)` | `burst 5`, `per_second 1` | backoff par client |
| **slug global** | `slug` seul | `burst 20`, refill ~1/3 s | plafond global, rattrape la rotation d'IP |

- `SmartIpKeyExtractor` lit `X-Forwarded-For`/`X-Real-IP` (derrière Caddy), comme le login.
- Modèle GCRA (fuite continue) : burst épuisé → throttle progressif ≈ cooldown.
  Suffisant contre le brute-force d'un PIN 6 chiffres.
- Dépassement → **429**.
- **Limite assumée** : compteurs **en RAM**, reset au redémarrage du process (la
  rotation d'IP n'est rattrapée que pendant la vie du process). Acceptable derrière un
  slug quasi non-énumérable (≈47 bits) ; à **documenter en QUIRKS**. Satisfait le
  *comportement* du §9.5 (backoff IP+slug + plafond global par slug) sans durabilité
  → **pas d'amendement du contrat**.

## 7. Cœur (`src/services/`) — ajouts agnostiques HTTP

- Module `services/unlock_cookie.rs` (ou fonctions dans `security.rs`) :
  - `fingerprint(secret: &[u8], slug: &str, pin: &str) -> String` — HMAC ;
  - `verify(secret, slug, pin, exp, fp, now) -> bool` — recompute + `secure_compare`
    (existant) + contrôle d'expiration.
  - **Fonctions pures**, zéro `use axum` / `use loco_rs`. Couvrent tamper + rotation +
    expiration.
- `verify_code` (Phase 1) réutilisé tel quel.
- Garde d'archi `backend/tests/architecture.rs` : reste verte.
- Choix de crate HMAC : à résoudre via **Context7** au plan (aligné sur ce qui est déjà
  au lockfile si possible ; sinon `hmac` + `sha2`). `cargo deny` à re-vérifier si
  nouvelle dépendance.

## 8. En-têtes & sécurité

- `Cache-Control: no-store` sur **toute** la surface `/c` (proto **et** page unlock) — §6.
- `Content-Type: text/html; charset=utf-8` pour le proto.
- `/api/public/{slug}` : JSON.
- Invariants §9 : pas de hash, PIN hors de toute réponse de cette surface ; `PublicMeta`
  les garantit par structure. Garde Origin **non** requise sur `/unlock` (surface
  publique cross-site par nature, comme le login ; la barrière est le PIN + rate-limit).

## 9. Hors périmètre (acté au brainstorming)

Écartés **sciemment** — à consigner en BACKLOG, pas des manques :
- Table `unlock_attempts` (persistance des tentatives).
- Scheduler Loco + purge lazy.
- Stat de déverrouillages affichée en admin (dépendait de la table).
- Backoff durable au reboot (la version in-memory est retenue).

E2E Playwright du flux complet `/c` : **Phase 6** (pas Phase 4). Le smoke admin
existant reste inchangé.

## 10. Config (env, défauts)

| Var | Défaut | Rôle |
|---|---|---|
| `UNLOCK_COOKIE_SECRET` | obligatoire en prod (panique si absent) | clé HMAC empreinte **et** jar signé |
| `LATCH_UNLOCK_TTL_DAYS` | `30` | durée de vie du cookie unlock |
| `LATCH_UNLOCK_RL_IP_BURST` | `5` | burst layer IP+slug |
| `LATCH_UNLOCK_RL_IP_REPLENISH_PER_SEC` | `1` | refill layer IP+slug |
| `LATCH_UNLOCK_RL_SLUG_BURST` | `20` | burst layer slug global |
| `LATCH_UNLOCK_RL_SLUG_PER_SECOND` | `0.33` (≈1/3 s) | refill layer slug global |

(`UNLOCK_COOKIE_SECRET` est déjà listé dans `docs/ENVIRONMENT.md` / `.env.example` ;
les autres sont nouveaux.)

## 11. Critères de sortie (ROADMAP Phase 4)

Tests verts à chaque couche concernée :

**Unit (cœur)** : `fingerprint`/`verify` — tamper, rotation, expiration, temps constant.

**Intégration (Loco + SQLite test)** :
- projet libre → GET sert le HTML actif (`no-store`) ;
- projet protégé sans cookie → GET rend `unlock.html` (**200**, pas 401) ;
- `POST /unlock` bon PIN → 204 + `Set-Cookie` ; mauvais PIN → 401 sans cookie ;
- cookie valide → GET sert l'actif ; **rotation du PIN invalide le cookie** ;
- rate-limit effectif (burst dépassé → 429), déclenché via `X-Forwarded-For` ;
- slug inconnu / sans version active → 404 ;
- `GET /api/public/{slug}` → `brand_name`/`code_enabled`, **jamais** de PIN (invariant).

**Front (Vitest + MSW)** : composant unlock — états brand / erreur 401 / 429.

**Qualité** : `cargo fmt` + `cargo clippy -D warnings`, `cargo nextest`, drift OpenAPI
+ schema, `pnpm lint`/`typecheck`/`test` verts ; `cargo deny` si nouvelle dépendance.
