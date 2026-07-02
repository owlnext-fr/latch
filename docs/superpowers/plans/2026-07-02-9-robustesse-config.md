# Garde-fou chemins config prod — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rendre impossible par construction la classe d'erreur « chemin FS relatif en prod → couche éphémère → perte de données » (incident 2026-06-29), via un garde-fou fail-secure qui refuse le boot.

**Architecture:** Un helper pur `resolve_abs_path` (calqué sur `resolve_required` de `web/mod.rs`) valide qu'un chemin est absolu hors Dev/Test. Un cœur pur `validate_paths` l'applique aux deux chemins concernés ; un wrapper glue `validate_path_config(ctx)` lit l'env + l'environnement Loco et est appelé en tête de `after_routes` (fail-fast au boot, comme `unlock_secret`/`deploy_token`).

**Tech Stack:** Rust, Loco 0.16 (`loco_rs::Error::Message` pour l'erreur de boot), `cargo nextest`.

## Global Constraints

- **Cœur agnostique** : ce travail vit dans `backend/src/web/` (adaptateur), jamais dans `src/services/` — pas de `use axum`/`loco` dans le cœur (contrat §1, garde `backend/tests/architecture.rs`).
- **Pas d'`unwrap`/`expect`** hors tests (`[workspace.lints]`). Le module de test porte déjà `#[allow(clippy::unwrap_used, clippy::expect_used)]`.
- **`is_prod` = `!matches!(env, Development | Test)`** (fail-secure : tout env inconnu = prod). Réutiliser `cookie_secure(ctx)`, ne pas réimplémenter.
- **Défauts = source unique** : les défauts de chemin remontent en `const` partagées entre résolveur et validateur (évite le piège duplication Sonar, cf. QUIRKS).
- **Validation via `cargo nextest run`** (pas `cargo test` — cf. QUIRKS course inter-process), depuis la racine.
- **Gate** : `cargo fmt` + `cargo clippy --all-targets -- -D warnings` verts ; Sonar new_coverage ≥ 80 %.
- **Commits** : conventionnels + gitmoji (`✨ feat(#9): …`), footer `Co-Authored-By`/`Claude-Session`.

---

## File Structure

- `backend/src/web/mod.rs` — **modifié** : ajoute 2 `const` de défaut, le helper `resolve_abs_path`, le cœur `validate_paths`, le wrapper `validate_path_config` ; refactore `spa_dist_dir`/`storage_from_ctx` pour consommer les `const` ; tests dans le module `#[cfg(test)]` existant.
- `backend/src/app.rs` — **modifié** : 1 ligne d'appel fail-fast en tête de `after_routes`.
- `docs/ENVIRONMENT.md`, `docs/QUIRKS.md`, `.env.example`, `public_docs/` (page deploy) — **modifiés** : documentation du comportement de boot + couplage `.env`↔volume↔`DATABASE_URL`.

---

## Task 1 : Helper `resolve_abs_path` + constantes + refactor résolveurs

**Files:**
- Modify: `backend/src/web/mod.rs` (const en tête ~L19 ; `spa_dist_dir` L21-25 ; `storage_from_ctx` L28-31 ; helper après `resolve_required` ~L91 ; tests dans `mod tests` ~L233+)
- Test: `backend/src/web/mod.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: `loco_rs::Error::Message`, `std::path::PathBuf`, `loco_rs::Result` (déjà importés).
- Produces:
  - `const STORAGE_ROOT_DEFAULT: &str = "data";`
  - `const SPA_DIST_DEFAULT: &str = "../frontend/dist";`
  - `fn resolve_abs_path(env_value: Option<String>, is_prod: bool, default: &str, label: &str) -> Result<PathBuf>` (privé module).

- [ ] **Step 1 : Écrire les tests qui échouent**

Ajouter dans le `mod tests` de `backend/src/web/mod.rs`. D'abord étendre l'import en tête du module :

```rust
    use super::{
        host_authority, resolve_abs_path, resolve_cookie_secret, resolve_required,
        session_cookie_names, SPA_DIST_DEFAULT, STORAGE_ROOT_DEFAULT,
    };
```

Puis ajouter les tests (à la fin du module, avant l'accolade fermante) :

```rust
    // --- resolve_abs_path : garde-fou chemin relatif/absolu (#9) ---

    const PATH_LABEL: &str = "LATCH_STORAGE_ROOT";

    #[test]
    fn abs_path_prod_relative_returns_err() {
        let result = resolve_abs_path(Some("data".to_string()), true, STORAGE_ROOT_DEFAULT, PATH_LABEL);
        assert!(result.is_err(), "prod + chemin relatif doit échouer");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains(PATH_LABEL), "message doit mentionner la var : {msg}");
        assert!(msg.contains("ABSOLU"), "message doit mentionner ABSOLU : {msg}");
    }

    #[test]
    fn abs_path_prod_dot_relative_returns_err() {
        let result = resolve_abs_path(Some("./data".to_string()), true, STORAGE_ROOT_DEFAULT, PATH_LABEL);
        assert!(result.is_err(), "prod + ./relatif doit échouer");
    }

    #[test]
    fn abs_path_prod_absolute_returns_ok() {
        let result = resolve_abs_path(Some("/data".to_string()), true, STORAGE_ROOT_DEFAULT, PATH_LABEL);
        assert!(result.is_ok(), "prod + chemin absolu doit réussir");
        assert_eq!(result.unwrap(), std::path::PathBuf::from("/data"));
    }

    #[test]
    fn abs_path_prod_unset_uses_relative_default_and_errs() {
        // Défaut relatif → en prod, unset échoue (fail-secure voulu).
        let result = resolve_abs_path(None, true, SPA_DIST_DEFAULT, "LATCH_SPA_DIST");
        assert!(result.is_err(), "prod + unset (défaut relatif) doit échouer");
    }

    #[test]
    fn abs_path_dev_relative_returns_ok() {
        let result = resolve_abs_path(Some("data".to_string()), false, STORAGE_ROOT_DEFAULT, PATH_LABEL);
        assert!(result.is_ok(), "dev + chemin relatif doit réussir (comportement dev inchangé)");
    }

    #[test]
    fn abs_path_empty_value_falls_back_to_default() {
        // Une valeur vide est traitée comme unset → défaut ; en dev le défaut relatif passe.
        let result = resolve_abs_path(Some(String::new()), false, STORAGE_ROOT_DEFAULT, PATH_LABEL);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from(STORAGE_ROOT_DEFAULT));
    }
```

- [ ] **Step 2 : Lancer les tests pour vérifier l'échec (compilation)**

Run: `cargo nextest run -p latch web::tests::abs_path`
Expected: FAIL — `cannot find function resolve_abs_path` / `STORAGE_ROOT_DEFAULT` non défini (erreur de compilation).

- [ ] **Step 3 : Ajouter les constantes et le helper**

Dans `backend/src/web/mod.rs`, remonter les défauts en `const` (juste avant `spa_dist_dir`, ~L19) :

```rust
/// Défaut relatif au CWD `backend/` (dev). En prod l'image pose une valeur absolue.
const STORAGE_ROOT_DEFAULT: &str = "data";
/// Défaut relatif au CWD `backend/` (dev). En prod l'image pose `/app/frontend/dist`.
const SPA_DIST_DEFAULT: &str = "../frontend/dist";
```

Refactorer `spa_dist_dir` pour consommer la const :

```rust
pub fn spa_dist_dir() -> PathBuf {
    std::env::var("LATCH_SPA_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(SPA_DIST_DEFAULT))
}
```

Refactorer `storage_from_ctx` pour consommer la const :

```rust
pub fn storage_from_ctx(_ctx: &AppContext) -> Arc<dyn Storage> {
    let root = std::env::var("LATCH_STORAGE_ROOT").unwrap_or_else(|_| STORAGE_ROOT_DEFAULT.to_string());
    Arc::new(FsStorage::new(root.into()))
}
```

Ajouter le helper juste après `resolve_required` (~L91) :

```rust
/// Valide qu'un chemin de configuration est ABSOLU en production (fail-secure).
/// Un chemin relatif hors Dev/Test résout depuis le WORKDIR `/app` du conteneur →
/// couche d'écriture éphémère → perte de données au redéploiement (incident
/// 2026-06-29, cf. `docs/QUIRKS.md`). Une valeur absente ou vide retombe sur le
/// défaut (relatif) → échoue donc aussi en prod, ce qui est voulu.
fn resolve_abs_path(
    env_value: Option<String>,
    is_prod: bool,
    default: &str,
    label: &str,
) -> Result<PathBuf> {
    let raw = env_value
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string());
    let path = PathBuf::from(&raw);
    if is_prod && !path.is_absolute() {
        return Err(loco_rs::Error::Message(format!(
            "{label} doit être un chemin ABSOLU en production (reçu : {raw:?}). \
             Un chemin relatif résout vers la couche éphémère /app/… du conteneur \
             et perd les données au redéploiement (cf. incident 2026-06-29)."
        )));
    }
    Ok(path)
}
```

- [ ] **Step 4 : Lancer les tests pour vérifier le succès**

Run: `cargo nextest run -p latch web::tests::abs_path`
Expected: PASS (6 tests).

- [ ] **Step 5 : Gate fmt/clippy**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: aucune erreur/warning.

- [ ] **Step 6 : Commit**

```bash
git add backend/src/web/mod.rs
git commit -m "$(cat <<'EOF'
✨ feat(#9): helper resolve_abs_path + consts de défaut de chemin

Garde-fou pur (calqué sur resolve_required) : refuse un chemin FS relatif
hors Dev/Test. Défauts remontés en const partagées avec spa_dist_dir /
storage_from_ctx (source unique). Tests table-driven.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01FfvMmpFwbihAHRsW47Pt5f
EOF
)"
```

---

## Task 2 : `validate_paths` (cœur) + `validate_path_config` (glue) + câblage boot

**Files:**
- Modify: `backend/src/web/mod.rs` (fonctions après `resolve_abs_path` ; tests dans `mod tests`)
- Modify: `backend/src/app.rs` (1ʳᵉ ligne de `after_routes`, ~L86)

**Interfaces:**
- Consumes: `resolve_abs_path`, `STORAGE_ROOT_DEFAULT`, `SPA_DIST_DEFAULT`, `cookie_secure` (Task 1 + existant).
- Produces:
  - `fn validate_paths(storage_root: Option<String>, spa_dist: Option<String>, is_prod: bool) -> Result<()>` (privé, pur, testable).
  - `pub fn validate_path_config(ctx: &AppContext) -> Result<()>` (glue : lit l'env + `cookie_secure(ctx)`).

- [ ] **Step 1 : Écrire les tests qui échouent**

Étendre l'import du `mod tests` pour ajouter `validate_paths` :

```rust
    use super::{
        host_authority, resolve_abs_path, resolve_cookie_secret, resolve_required,
        session_cookie_names, validate_paths, SPA_DIST_DEFAULT, STORAGE_ROOT_DEFAULT,
    };
```

Ajouter les tests :

```rust
    // --- validate_paths : applique le garde-fou aux 2 chemins (#9) ---

    #[test]
    fn validate_paths_prod_all_absolute_ok() {
        let result = validate_paths(
            Some("/data".to_string()),
            Some("/app/frontend/dist".to_string()),
            true,
        );
        assert!(result.is_ok(), "prod + 2 chemins absolus doit réussir");
    }

    #[test]
    fn validate_paths_prod_relative_storage_errs() {
        let result = validate_paths(
            Some("data".to_string()),
            Some("/app/frontend/dist".to_string()),
            true,
        );
        assert!(result.is_err(), "storage relatif en prod doit échouer");
        assert!(result.unwrap_err().to_string().contains("LATCH_STORAGE_ROOT"));
    }

    #[test]
    fn validate_paths_prod_relative_spa_errs() {
        let result = validate_paths(
            Some("/data".to_string()),
            Some("../frontend/dist".to_string()),
            true,
        );
        assert!(result.is_err(), "spa_dist relatif en prod doit échouer");
        assert!(result.unwrap_err().to_string().contains("LATCH_SPA_DIST"));
    }

    #[test]
    fn validate_paths_prod_unset_errs() {
        // Les deux unset → défauts relatifs → échec (fail-secure).
        let result = validate_paths(None, None, true);
        assert!(result.is_err(), "prod + unset doit échouer");
    }

    #[test]
    fn validate_paths_dev_relative_ok() {
        let result = validate_paths(Some("data".to_string()), None, false);
        assert!(result.is_ok(), "dev + relatif doit réussir (comportement inchangé)");
    }
```

- [ ] **Step 2 : Lancer les tests pour vérifier l'échec**

Run: `cargo nextest run -p latch web::tests::validate_paths`
Expected: FAIL — `cannot find function validate_paths` (compilation).

- [ ] **Step 3 : Implémenter `validate_paths` + `validate_path_config`**

Dans `backend/src/web/mod.rs`, juste après `resolve_abs_path` :

```rust
/// Applique le garde-fou de chemin aux deux variables filesystem concernées.
/// Cœur pur (paramétré) pour être testable sans `AppContext` ni env.
fn validate_paths(storage_root: Option<String>, spa_dist: Option<String>, is_prod: bool) -> Result<()> {
    resolve_abs_path(storage_root, is_prod, STORAGE_ROOT_DEFAULT, "LATCH_STORAGE_ROOT")?;
    resolve_abs_path(spa_dist, is_prod, SPA_DIST_DEFAULT, "LATCH_SPA_DIST")?;
    Ok(())
}

/// Fail-fast de boot : refuse de démarrer si `LATCH_STORAGE_ROOT` ou `LATCH_SPA_DIST`
/// est relatif (ou absent) en production. À appeler en tête de `after_routes`, comme
/// `unlock_secret`/`deploy_token`. Empêche la reproduction de l'incident 2026-06-29.
pub fn validate_path_config(ctx: &AppContext) -> Result<()> {
    validate_paths(
        std::env::var("LATCH_STORAGE_ROOT").ok(),
        std::env::var("LATCH_SPA_DIST").ok(),
        cookie_secure(ctx),
    )
}
```

- [ ] **Step 4 : Câbler dans `after_routes`**

Dans `backend/src/app.rs`, ajouter en **toute première ligne** de `after_routes` (avant `build_session_store`, ~L86) :

```rust
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Fail-fast : un chemin FS relatif en prod (→ couche éphémère /app) casse le
        // boot, pas les données au redéploiement (incident 2026-06-29, cf. QUIRKS).
        crate::web::validate_path_config(ctx)?;
        let store = crate::web::build_session_store(ctx).await?;
        // ... (reste inchangé)
```

- [ ] **Step 5 : Lancer les tests + build complet**

Run: `cargo nextest run -p latch`
Expected: PASS (tous les tests, dont `validate_paths_*` et l'ensemble de la suite backend inchangée — la validation en `after_routes` tourne en env `Test` = `is_prod=false`, donc n'impacte pas les tests d'intégration existants).

- [ ] **Step 6 : Gate fmt/clippy**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: aucune erreur/warning.

- [ ] **Step 7 : Commit**

```bash
git add backend/src/web/mod.rs backend/src/app.rs
git commit -m "$(cat <<'EOF'
✨ feat(#9): garde-fou de boot — refuse un chemin config relatif en prod

validate_path_config appelé en tête de after_routes (fail-fast comme
unlock_secret/deploy_token). Couvre LATCH_STORAGE_ROOT + LATCH_SPA_DIST.
Cœur pur validate_paths testé ; Test/Dev inchangés (is_prod=false).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01FfvMmpFwbihAHRsW47Pt5f
EOF
)"
```

---

## Task 3 : Documentation (comportement boot + couplage config)

**Files:**
- Modify: `.env.example` (bloc `LATCH_SPA_DIST`)
- Modify: `docs/ENVIRONMENT.md` (comportement boot + couplage)
- Modify: `docs/QUIRKS.md` (compléter l'entrée incident)
- Modify: `public_docs/content/docs/**` (page deploy/configuration — chemin absolu prod)
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md` (mémoire projet)

**Interfaces:** aucune (documentation seule).

- [ ] **Step 1 : `.env.example` — documenter `LATCH_SPA_DIST`**

Le bloc `LATCH_SPA_DIST=` est actuellement vide de contrainte. Le compléter (miroir de `LATCH_STORAGE_ROOT`) :

```
# --- SPA React (interface admin) ---
# Racine des assets buildés de la SPA. DOIT être un chemin ABSOLU en prod (comme
# LATCH_STORAGE_ROOT) : le boot REFUSE de démarrer si relatif ou absent hors Dev/Test
# (fail-secure — évite de servir depuis la couche éphémère /app). L'image Docker la
# pose à /app/frontend/dist. Dev (cargo loco start depuis backend/) : ../frontend/dist.
LATCH_SPA_DIST=
```

- [ ] **Step 2 : `docs/ENVIRONMENT.md` — comportement boot + couplage**

Sous les entrées `LATCH_STORAGE_ROOT` / `LATCH_SPA_DIST`, ajouter une note :

```markdown
> **Garde-fou de boot (#9)** : hors `Development`/`Test`, latch **refuse de démarrer**
> si `LATCH_STORAGE_ROOT` ou `LATCH_SPA_DIST` est un chemin **relatif** (ou absent →
> défaut relatif). Message d'erreur explicite au boot. Fail-secure : empêche la
> reproduction de l'incident 2026-06-29 (chemin relatif → couche éphémère `/app/…` →
> perte des HTML au redéploiement). `DATABASE_URL` n'est **pas** couvert par ce garde-fou
> (URI sqlite, défaut prod déjà absolu) — mais **doit** pointer la même persistance.
>
> **Couplage à respecter en prod** : `docker-compose.yml` monte `./data:/data` ;
> `LATCH_STORAGE_ROOT=/data` ET `DATABASE_URL=sqlite:///data/latch.sqlite?mode=rwc`
> doivent pointer ce **même volume** — sinon base et HTML divergent (l'un persiste,
> l'autre non).
```

- [ ] **Step 3 : `docs/QUIRKS.md` — compléter l'entrée incident**

Dans l'entrée « `LATCH_STORAGE_ROOT` relatif → HTML écrits sur la couche éphémère », ajouter en fin :

```markdown
**Depuis #9 (2026-07-02)** : un **garde-fou de boot** (`web::validate_path_config`, appelé
en tête de `after_routes`) refuse désormais de démarrer en prod si `LATCH_STORAGE_ROOT` ou
`LATCH_SPA_DIST` est relatif/absent — cette mauvaise config casse le boot au lieu de perdre
des données silencieusement. `DATABASE_URL` reste hors garde-fou (URI, défaut absolu).
```

- [ ] **Step 4 : `public_docs/` — exigence chemin absolu**

Repérer la page de configuration/déploiement :

Run: `rtk grep -rln "LATCH_STORAGE_ROOT" public_docs/content`

Dans la page trouvée, ajouter une note (adapter au format MDX de la page) précisant que
`LATCH_STORAGE_ROOT` et `LATCH_SPA_DIST` **doivent être absolus en prod** et que le boot
échoue sinon (fail-secure). **Attention MDX** : `<slug>`/`{…}` en backticks (cf. QUIRKS Phase 8).

- [ ] **Step 5 : Build de vérif public_docs (si la page a changé)**

Run: `cd public_docs && pnpm build`
Expected: build statique vert (pas de lien cassé, MDX valide). Puis `cd ..`.

- [ ] **Step 6 : Mémoire projet**

- `docs/INDEX.md` : ajouter une ligne sous une section #9 :
  `- [x] Garde-fou de boot fail-secure : refuse un chemin FS relatif en prod (LATCH_STORAGE_ROOT + LATCH_SPA_DIST) — #9 — 2026-07-02`
- `docs/HANDOFF.md` : entrée datée en haut (Dernière chose faite / Trucs en suspens / Prochaine chose / Notes pour future Claude).

- [ ] **Step 7 : Commit**

```bash
git add .env.example docs/ENVIRONMENT.md docs/QUIRKS.md docs/INDEX.md docs/HANDOFF.md public_docs
git commit -m "$(cat <<'EOF'
📝 docs(#9): garde-fou chemins prod + couplage .env↔volume↔DATABASE_URL

.env.example (LATCH_SPA_DIST), ENVIRONMENT (comportement boot + couplage),
QUIRKS (entrée incident complétée), public_docs (chemin absolu prod),
INDEX/HANDOFF (mémoire).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01FfvMmpFwbihAHRsW47Pt5f
EOF
)"
```

---

## Self-Review (auteur du plan)

**Spec coverage :**
- §4.1 helper `resolve_abs_path` + consts → Task 1. ✅
- §4.2 `validate_path_config` + câblage `after_routes` → Task 2. ✅
- §4.3 tests table-driven → Task 1 (resolve_abs_path) + Task 2 (validate_paths). ✅
- §4.4 doc (ENVIRONMENT, QUIRKS, public_docs, .env.example, INDEX/HANDOFF) → Task 3. ✅
- §3 `DATABASE_URL` audit+doc seulement → documenté en Task 3 Step 2/3 (exclusion explicite). ✅
- §5 definition of done (fmt/clippy/nextest/Sonar/mémoire) → répartie dans les steps de gate. ✅

**Placeholder scan :** aucun TODO/TBD ; tout le code est fourni. ✅

**Type consistency :** `resolve_abs_path(Option<String>, bool, &str, &str) -> Result<PathBuf>` cohérent Task 1↔2 ; `validate_paths(Option<String>, Option<String>, bool) -> Result<()>` cohérent Task 2 ; `validate_path_config(&AppContext) -> Result<()>` appelé identiquement en Task 2 Step 4. Consts `STORAGE_ROOT_DEFAULT`/`SPA_DIST_DEFAULT` identiques partout. ✅

**Note CONVENTIONS** : le spec évoquait d'ajouter le pattern « garde-fou chemin fail-secure au boot » à `docs/CONVENTIONS.md` *si jugé réutilisable*. Optionnel — à décider à la fin (le pattern `resolve_*` fail-secure y est déjà implicitement documenté via le code). Non bloquant pour la DoD.
