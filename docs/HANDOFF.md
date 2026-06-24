# Handoff — état courant

> Notes informelles pour la prochaine session (humaine ou Claude). Format libre,
> chronologique inverse (le plus récent en haut). À mettre à jour en fin de session
> significative — l'idée est de se resituer en 30 secondes.

## 2026-06-24 — Phase 0 livrée (scaffold & squelette CI/Docker)

### Dernière chose faite
- **Phase 0 du ROADMAP terminée, tous critères de sortie verts** (vérifiés réellement,
  pas sur parole) :
  - Workspace 2 membres : `backend/` (Loco 0.16.4, crate `latch`, bin `latch-cli`) +
    `frontend/` (crate `latch-ui`, Yew 0.21) + sous-crate `backend/migration`.
  - Scaffold généré via `loco new --db sqlite --bg none --assets none` → starter minimal
    **sans users/JWT** (rien à retirer), **sans worker/Redis**.
  - `libsqlite3-sys` en `bundled` (unifié avec sqlx 0.8 → `libsqlite3-sys 0.30.1`).
  - `cargo loco start` boote (depuis `backend/`), `trunk build` produit le bundle wasm.
  - fmt + clippy `-D warnings` verts (backend ET frontend wasm) ; `cargo test` vert.
  - Image Docker multi-stage construite (~85 Mo) + **smoke test conteneur** : `/_health`
    = `{"ok":true}`, auto-migrate au boot, `latch.sqlite` créé dans le volume.
  - Écrits : Dockerfile, `docker-compose.yml`, `deploy.sh`, `.env.example`, deny.toml,
    CI `.github/workflows/ci.yml`, dual-license MIT/Apache, README + badge.

### Versions épinglées (résolues via Context7 + crates.io)
- loco `0.16` (lock 0.16.4) · rmcp **pin 1.8.0** (≥1.4 CVE, pas encore dep → Phase 5) ·
  yew **0.21** (imposé par `shadcn-rs 0.1.0` qui requiert `yew ^0.21`) · shadcn-rs 0.1.0
  (compile en wasm, OK) · sea-orm 1.1 (aligné Loco).

### Trucs en suspens / à savoir
- **Lancer le serveur depuis `backend/`** (Loco lit `./config` au CWD) — cf. QUIRKS.
- `default-members = [backend, backend/migration]` : le frontend wasm est exclu des
  commandes natives (sinon `cargo build` tente de le compiler pour l'hôte) — cf. QUIRKS.
- CI **non exécutée** dans cet environnement (pas de runner) : YAML à confirmer au 1ᵉʳ push.
  Job `cargo-deny` en `continue-on-error` (à durcir Phase 6).
- `Cargo.lock` est commité (pin réel). `.vscode/` toujours hors commit.

### Prochaine chose à creuser
- **Phase 1** : cœur `services/` (projects, deploy tx, slug, Storage, CoreError) +
  migrations `projects`/`versions`/`sessions` + tests unit. Agnostique HTTP.

### Notes pour future Claude
- Avant de coder une API Loco/sea-orm/rmcp/yew : **Context7** (versions épinglées).
- Le smoke test conteneur est reproductible : `docker run -p 5151:5150 -v <data>:/data ghcr.io/owlnext-fr/latch:dev`.

## 2026-06-24 — Bootstrap mémoire projet livré

### Dernière chose faite
- Rangé les docs normatifs sous `docs/` (ils traînaient à la racine, alors que
  `CLAUDE.md` les référençait déjà sous `docs/` — les liens sont maintenant corrects).
- Mis en place le système de mémoire persistante : bloc « Mémoire projet » dans
  `CLAUDE.md` (decision tree + règle de fin d'implémentation non-négociable), hook
  `SessionStart` (`.claude/hooks/load-memory.sh`) qui injecte le head de `HANDOFF.md`
  + `INDEX.md` au démarrage, `.gitignore` pour `.claude/settings.local.json`.
- Créé `docs/superpowers/{specs,plans}/` (specs & plans détaillés par feature
  non-triviale, fichiers `YYYY-MM-DD-<slug>.md`).

### Règle actée cette session
- **Convention de commit = gitmoji + conventionnel** (`<gitmoji> <type>: <desc>`,
  ex. `✨ feat:`, `🐛 fix:`). Consignée dans `docs/BOOTSTRAP.md §4`. Obligatoire.

### Trucs en suspens
- Bootstrap commité sur la branche **`chore/bootstrap-memoire`** (on était sur `main`).
- `.claude/settings.json` + `.claude/hooks/` + `.rtk/filters.toml` sont **commités**
  (setup partagé équipe). `.vscode/` laissé hors commit (spécifique éditeur).
- Contenu existant **préservé** (non écrasé par les templates vides du prompt) :
  `INDEX.md`, `ENVIRONMENT.md`, `CONVENTIONS.md`, `QUIRKS.md`, `BACKLOG.md` gardent
  leur contenu projet riche issu du cadrage.

### Prochaine chose à creuser
- Dérouler la **Phase 0** du ROADMAP (scaffold & squelette CI/Docker).

### Notes pour future Claude
- En début de session, le hook t'aura déjà injecté le head de `HANDOFF.md`. Lis-le,
  puis `docs/INDEX.md`, puis les normatifs (`contrat-deploy` → `BOOTSTRAP` → `ROADMAP`).
- Le hook ne montre que 80 lignes de `HANDOFF.md` (append-only, il grossit) ; si tu
  veux plus de contexte, lis le fichier entier.

## 2026-06-24 — Kit dérivé, avant tout code

Le cadrage archi est **clos**. Le kit (`CLAUDE.md`, `docs/contrat-deploy.md`,
`docs/BOOTSTRAP.md`, `docs/ROADMAP.md`) est la source de vérité. Rien n'est encore
codé : on entre en **Phase 0** (scaffold).

Décisions structurantes verrouillées : Loco/axum + SeaORM/SQLite (`bundled`) ;
frontend **Yew + shadcn-rs** servi en statique (choix assumé « PoC technique, fun >
simplicité », pas le plus simple — le plus simple aurait été du server-rendered) ;
admin **cookie-session** (pas le JWT Loco) ; `/c/<slug>` à **deux états** avec page de
déverrouillage stylée + PIN 6 chiffres + rate-limit *load-bearing* ; MCP **Modèle 1**
(`deploy_token` en argument) ; GHCR **public**, déploiement **manuel** via `deploy.sh`.

Prochaine action : dérouler la Phase 0 du ROADMAP. Avant de coder une API d'une crate
listée dans le tableau Context7 du `CLAUDE.md`, **résoudre la doc via Context7**.

À trancher quand ça deviendra concret (non bloquant) : longueur exacte du suffixe de
slug (cf. QUIRKS). Acté : nom du projet **`latch`** (repo `owlnext-fr/latch`), domaine
de serving **`latch.owlnext.fr`**.
