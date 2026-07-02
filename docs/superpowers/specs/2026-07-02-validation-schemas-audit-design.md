# Design — Audit & consolidation de la validation des entrées (#23)

> Statut : **spec validée en brainstorming** (2026-07-02). Prochaine étape : plan d'implémentation.
> Branche : `feat/23-validation-audit`. Issue : #23.

## 1. Objectif

Garantir que **toute entrée qui franchit une frontière** (payload web, argument de tool
MCP) est **validée à la frontière**, de façon **centralisée, déclarative et testée**. Le
backend est la **seule source de vérité** ; le front n'est qu'un confort UX (non garanti).

La validation `name`/`brand_name` livrée en #13 (fonctions impératives dans le service)
est un cas particulier de ce que cette passe généralise et systématise.

## 2. Décision d'architecture (validée)

### 2.1 Validation de **forme** à la frontière, invariants **métier** au cœur

Deux natures distinctes, deux emplacements :

| Nature | Exemples | Où | Comment |
|---|---|---|---|
| **Forme d'input** | longueur, format (PIN 6 chiffres), requis, taille | **frontière** (adaptateurs) | attributs `#[validate]` sur les DTOs, `.validate()` appelé à la réception |
| **Invariant métier** | « pas de suppression de version active », existence du slug, unicité | **cœur** (`services/`) | code impératif, inchangé |

Conséquence sur le **contrat §1** : le cœur passe de « suppose l'appelant déjà **autorisé** »
à « suppose l'appelant déjà **autorisé et l'input déjà validé à la frontière** ». La
validation de forme **migre** des services vers les DTOs. Les *invariants métier* restent.

### 2.2 Crate `validator` (déjà dans le lockfile en 0.20.0)

`#[derive(Validate)]` + attributs par champ. `length` sur `String` compte les **caractères
Unicode** (= convention `.chars().count()` de #13). `.validate() → Result<(), ValidationErrors>`.
Ajout en **dépendance directe** (déjà transitive → coût supply-chain nul, cargo-deny déjà vert).

### 2.3 Deux points d'invocation (une seule définition de bornes)

- **Frontière web (axum 0.8)** : extracteur maison `ValidatedJson<T>` (`impl FromRequest`) qui
  désérialise le JSON **puis** appelle `.validate()`. Échec → `400` (`invalid_params`), mappé
  via `CoreError::Validation` (déjà `→ 400`, cf. `controllers/error.rs`).
- **Frontière MCP (rmcp 1.8)** : les tools reçoivent `Parameters<T>` (`params.0`). On ajoute
  `Validate` sur `T` et on appelle `params.0.validate()?` en **tête de chaque tool**, avant tout
  appel cœur. Échec → tool error. (MCP ne passe **pas** par l'extracteur axum → invocation explicite obligatoire.)

## 3. Registre central — `backend/src/services/validation.rs` (nouveau)

Source de vérité unique des bornes + fonctions de validation réutilisables.

### 3.1 Bornes **const** (compile-time, attribut déclaratif)

Regroupe les constantes aujourd'hui éparpillées (`projects.rs`, `comments.rs`, `deploy.rs`) :

```
pub const MAX_NAME_LEN: u64 = 128;          // ex-MAX_PROJECT_NAME_LEN (projects.rs)
pub const MAX_BODY_LEN: u64 = 2000;         // ex-comments.rs
pub const MAX_AUTHOR_NAME_LEN: u64 = 80;    // ex-comments.rs
pub const MAX_RELEASE_NOTES_LEN: u64 = 10_000; // ex-deploy.rs
```

Usage déclaratif : `#[validate(length(min = 1, max = MAX_NAME_LEN))]`.
> ⚠️ Détail à confirmer au plan : `validator 0.20` accepte-t-il un **chemin de const** dans
> `length(max = …)` ? Si non, on garde le littéral **avec un commentaire pointant la const**,
> ou on passe par `custom`. À trancher à l'implémentation (non bloquant pour le design).

### 3.2 Bornes **env-configurables** (runtime → `custom`)

`html` et `anchor` sont des limites **opérationnelles de taille**. Un attribut déclaratif ne
peut PAS porter une valeur runtime → **`custom(function = …)`** qui lit l'env à l'appel :

```
LATCH_MAX_HTML_BYTES    (défaut 5_242_880 = 5 Mo)
LATCH_MAX_ANCHOR_BYTES  (défaut 8_192 = 8 Ko)
```

Fonctions custom (dans `validation.rs`), lecture env via un `OnceLock` (éviter de relire l'env
à chaque requête) :

```
fn validate_html(v: &str)   -> Result<(), ValidationError>  // non-vide + bytes ≤ LATCH_MAX_HTML_BYTES
fn validate_anchor(v: &str) -> Result<(), ValidationError>  // non-vide + bytes ≤ LATCH_MAX_ANCHOR_BYTES
```

### 3.3 Fonctions `custom` de format (réutilisables)

- `validate_pin` — 6 chiffres ASCII (réutilise `pin::is_valid_pin`).
- Cross-field `CreateProjectReq` : « `pin` valide **si** `code_enabled` » → `#[validate(schema(function = …))]` au niveau struct.
- **Sanitisation** de `author_name` (strip control chars) = une **transformation**, pas une
  validation → reste côté service (avant stockage). La **borne** (≤ 80 + non-vide) migre en attribut.

## 4. Annotations DTO — carte complète (d'après l'inventaire)

### 4.1 Admin API — `backend/src/dto/mod.rs`

| DTO | Champ | Attribut |
|---|---|---|
| `CreateProjectReq` | `name` | `length(min = 1, max = MAX_NAME_LEN)` |
| | `brand_name` | `length(max = MAX_NAME_LEN)` (Option) |
| | `pin`/`code_enabled` | `schema(function = validate_pin_if_code)` (cross-field) |
| `UpdateProjectReq` | `name` | `length(min = 1, max = MAX_NAME_LEN)` (Option) |
| | `brand_name` | `length(max = MAX_NAME_LEN)` (Option<Option>) |
| `SetCodeReq` | `pin` | `custom(validate_pin)` |
| `DeployReq` | `html` | `custom(validate_html)` **← trou comblé** |
| | `notes` | `length(max = MAX_RELEASE_NOTES_LEN)` |
| `LoginReq` | `user`, `pass` | `length(min = 1)` |
| `CreatePinReq` | `author_name` | `length(min = 1, max = MAX_AUTHOR_NAME_LEN)` |
| | `body` | `length(min = 1, max = MAX_BODY_LEN)` |
| | `anchor` | `custom(validate_anchor)` **← trou comblé** |
| `ReplyReq` | `author_name` | `length(min = 1, max = MAX_AUTHOR_NAME_LEN)` |
| | `body` | `length(min = 1, max = MAX_BODY_LEN)` |
| `EditMessageReq` | `body` | `length(min = 1, max = MAX_BODY_LEN)` |
| `AdminCreatePinReq` | `anchor` | `custom(validate_anchor)` **← trou comblé** |
| | `body` | `length(min = 1, max = MAX_BODY_LEN)` |
| `AdminReplyReq` | `body` | `length(min = 1, max = MAX_BODY_LEN)` |
| `UnlockReq` | `pin` | `custom(validate_pin)` **← format validé avant secure_compare (400 au lieu de 401 si malformé)** |

`deploy_token` : **exempté** (secret comparé en temps constant, aucune borne de forme).

### 4.2 MCP — `backend/src/mcp/mod.rs`

| Struct | Champ | Attribut |
|---|---|---|
| `DeployArgs` | `html` | `custom(validate_html)` **← trou comblé (MCP aussi)** |
| | `release_notes` | `length(max = MAX_RELEASE_NOTES_LEN)` (Option) |
| | `slug` | `length(min = 1)` |
| | `deploy_token` | exempté (secret) |
| `PullArgs` | `slug` | `length(min = 1)` |
| | `deploy_token` | exempté |
| `ListArgs` | `deploy_token` | exempté |

## 5. Invariant testé (le « test bloquant »)

Deux niveaux, complémentaires :

1. **Type-level (mécanique, compile-time)** : l'extracteur `ValidatedJson<T>` exige `T: Validate`,
   et chaque tool MCP appelle `params.0.validate()`. Un **nouveau DTO branché sur une frontière
   sans `impl Validate` ne compile pas**. C'est la garantie « chaque DTO de frontière a une validation ».
2. **Comportemental (registre table-driven)** : un test (dans `backend/tests/` ou à côté de
   `architecture.rs`) parcourt un tableau `(constructeur DTO over-limit) → assert Err(validate)`
   couvrant chaque `*Req`/`*Args`. Une **régression** (borne retirée/relâchée) casse le build.

> Limite assumée & documentée : Rust ne peut pas détecter mécaniquement un **champ** oublié
> *à l'intérieur* d'un DTO (pas de réflexion). Le niveau 1 garantit « le DTO passe par `.validate()` » ;
> la complétude par champ reste couverte par le registre (niveau 2) + review. C'est ~10× l'état actuel.

## 6. Front (option B — UX-only, hors invariant)

Ajout d'un attribut HTML **`maxLength`** (valeurs par défaut statiques, indicatives) sur les 3
inputs sans borne aujourd'hui : `notes` (éditeur), `author_name` (`compose-popup`), `body` en
**reply/edit** (`thread-popup`). **Aucune** logique zod dupliquée, **aucun** test de parité.
Si un opérateur change la borne back via env, le front reste conservateur — le back tranche.

Le zod existant (`project-form`, `login`) est laissé tel quel (déjà cohérent). **Pas** de
fichier de constantes front partagé, **pas** de source-unique front↔back (écartés en brainstorming).

## 7. Configuration & documentation (livrables obligatoires)

Nouvelles vars, cohérentes avec le bloc `LATCH_*` existant :

| Var | Défaut | Rôle |
|---|---|---|
| `LATCH_MAX_HTML_BYTES` | `5242880` (5 Mo) | Taille max du HTML déployé (web + MCP) |
| `LATCH_MAX_ANCHOR_BYTES` | `8192` (8 Ko) | Taille max du descripteur d'ancrage d'un commentaire |

À documenter (partie intégrante du « terminé ») :
- **`.env.example`** — le template committé qui documente toutes les vars (⚠️ pas `.env.local`,
  gitignoré/SONAR uniquement). Bloc cohérent avec les autres `LATCH_*`.
- **fumadocs** — `public_docs/content/docs/deploy/configuration.mdx` (table des clés).
- **`docs/ENVIRONMENT.md`** — section vars.

## 8. Plan de test

- **Unit** (`validation.rs`) : chaque `custom` (pin, html borne env, anchor borne env, cross-field pin/code).
- **Registre table-driven** : rejet au-delà de chaque borne, sur tous les `*Req`/`*Args` (niveau 2 §5).
- **Intégration web** : un `POST` over-limit sur chaque endpoint concerné → `400` ; un valide → OK.
- **MCP** : `deploy_prototype` avec `html` over-limit → tool error ; `release_notes` > 10 000 → error.
- **Env** : `LATCH_MAX_HTML_BYTES` bas → rejet à la nouvelle borne (prouve la lecture env).
- Suites existantes vertes (name:"P" etc. — bornes MAX-only, pas de plancher nouveau).

## 9. Contrat (mises à jour normatives)

- **§1** : préciser que la **validation de forme** vit à la frontière (extracteur web + tools MCP),
  le cœur la suppose faite. Les invariants métier restent au cœur.
- **§9** : ajouter l'invariant « **toute entrée de frontière est validée via `Validate`** (source de
  vérité back), couverte par un test bloquant » — au même rang que « aucun hash sérialisé »,
  « PIN jamais en liste », « deploy_token sur tous les tools ».

## 10. Périmètre — hors sujet (non-goals)

- **Pas** de parité zod front testée (front = UX indicatif).
- **Pas** de source-unique front↔back via OpenAPI.
- Bornes de **longueur de champ** (name/body/author/release_notes) restent **const** (pas d'env — YAGNI).
- **Pas** de refonte des invariants métier existants (delete active version, etc.).

## 11. Risques / points ouverts (non bloquants)

- `validator 0.20` + chemin de const dans `length(max = …)` : à confirmer au plan (fallback : littéral commenté ou `custom`).
- Lecture env dans `validation.rs` (cœur) : acceptable (config, pas HTTP) ; cacher via `OnceLock`.
- Migration cœur→frontière : bien retirer les checks devenus redondants dans les services sans
  casser les tests unit de service existants (certains testent la validation via le service).
- `sanitize_author_name` (strip control chars) reste une transformation côté service.
