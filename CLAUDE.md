# CLAUDE.md — latch

> Orchestrateur. Ce fichier ne contient **pas** les décisions : il dit dans quel
> ordre les lire et comment travailler. Le contenu normatif vit dans `docs/`.
>
> **Nom : `latch`.** Repo : `github.com/owlnext-fr/latch`. Crate backend : `latch` ;
> app React : `frontend/` (Vite). Package : `ghcr.io/owlnext-fr/latch`.
> Domaine de serving : `latch.owlnext.fr`.

## Ce qu'est ce projet, en deux phrases

Une petite app Rust qui sert des prototypes HTML mono-fichier derrière un host
contrôlé, avec versioning et code d'accès optionnel par projet. Trois surfaces
sur un seul binaire Loco : serving client (`/c/<slug>`), admin (`/admin`),
endpoint MCP (`/mcp`) appelé par Claude pour déployer.

## Protocole obligatoire avant tout travail

1. **Se resituer via la mémoire projet** : lire `docs/HANDOFF.md` (état courant,
   normalement injecté par le hook SessionStart) puis `docs/INDEX.md` (ce qui est
   déjà livré). Ne jamais dire « je n'ai pas le contexte » sans avoir lu ces fichiers.

2. **Lire les documents normatifs**, dans cet ordre :
   - `docs/contrat-deploy.md` — archi en couches, modèle de données, les trois
     surfaces, **les invariants de sécurité**. **Le contrat fait loi.**
   - `docs/BOOTSTRAP.md` — stack, versions épinglées, structure du repo, outillage,
     règles de test, CI, Docker, déploiement.
   - `docs/ROADMAP.md` — phases, dépendances, critères de sortie. Identifier la
     phase courante avant de coder.

3. **Doc d'abord, code ensuite.** Si une décision manque ou semble se contredire
   entre deux docs, on tranche dans la doc *avant* d'écrire du code. Un spec flou
   produit du code flou.

## Workflow Context7 (obligatoire)

Avant d'utiliser une API d'une de ces librairies, **résoudre la doc via Context7**
plutôt que de coder de mémoire — ces crates bougent vite et la mémoire du modèle
est périmable. La référence est la **version épinglée** du `Cargo.toml`/lockfile,
pas la dernière publiée.

| Sujet | Crate / outil | Pourquoi vérifier |
|---|---|---|
| Framework web, routing, `after_routes`, sessions | `loco-rs` | Pré-1.0, breaking changes fréquents |
| ORM, entités, migrations, transactions | `sea-orm` | API de query/transaction précise |
| Endpoint MCP, transport Streamable HTTP, `allowed_hosts` | `rmcp` | A sauté en 1.x ; CVE Host-header < 1.4.0 |
| Routing SPA admin | `@tanstack/react-router` | Code-based, basepath `/admin` |
| Data-fetching + cache SPA | `@tanstack/react-query` | Invalidation + stale-while-revalidate |
| Composants UI admin | `shadcn/ui` (Radix) | Base stone oklch, preset bJfDPe2y |
| Formulaires SPA | `react-hook-form` / `zod` | Schémas de validation |
| i18n SPA | `react-i18next` | FR + EN, défaut EN |
| Client API typé | `openapi-fetch` / `openapi-typescript` | Généré depuis `openapi.json` → `schema.d.ts` |
| Cookie signé (déverrouillage client) | `axum-extra` (SignedCookieJar) / `cookie` | Détails de signature/scoping |

## Carte des chantiers — où vit quoi

- **Règle d'architecture, modèle de données, invariant de sécurité** → `docs/contrat-deploy.md`.
- **Standard d'outillage, version épinglée, règle de test, étape CI/Docker** → `docs/BOOTSTRAP.md`.
- **Ordre des phases, critère « la phase X est finie »** → `docs/ROADMAP.md`.
- **État courant, ce qui vient d'être fait** → `docs/HANDOFF.md`.
- **Ce qui est livré et marche** → `docs/INDEX.md`.
- **Piège rencontré, contournement** → `docs/QUIRKS.md`.
- **Squelette de code récurrent découvert en route** → `docs/CONVENTIONS.md`.
- **Idée reportée, hors-périmètre v1** → `docs/BACKLOG.md`.
- **Orchestration seule** (cet ordre de lecture, le workflow) → ce fichier.

> Routage en cas de doute : une règle *d'archi ou de sécu* va dans le **contrat**,
> jamais ici. `CLAUDE.md` ne porte que l'orchestration.

## Réflexe de sécurité (à chaque endpoint, à chaque réponse)

Avant de renvoyer quoi que ce soit depuis un adaptateur (web, MCP), vérifier :
**aucune réponse ne contient de hash**, et le **PIN en clair n'apparaît que sur le
détail d'un projet**, jamais dans une liste, jamais via MCP. C'est un invariant du
contrat, pas une option. Il est couvert par un test qui casse le build s'il est violé.

## Confidentialité — jamais de nom de client (NON-NÉGOCIABLE)

Aucun **nom de client réel** (ni marque, ni projet client identifiable) ne doit
apparaître **où que ce soit** dans le repo : code, tests, fixtures, exemples, commentaires,
docs normatifs, docs mémoire (`docs/`), specs/plans, briefs, messages de commit. Pour tout
exemple, utiliser des placeholders génériques manifestement fictifs (`Mon Projet` /
`mon-projet`, `ACME`, `demo`, etc.). En cas de doute sur un nom, le traiter comme client
et le remplacer. Si un nom client est repéré, le purger du working-tree **immédiatement**
(et signaler s'il subsiste dans l'historique git — la purge d'historique est une décision
humaine, surtout si la branche/`main` est déjà poussée).

## Définition de « terminé »

Une tâche n'est terminée que si **tout** ce qui suit est vrai :
- `cargo fmt` et `cargo clippy` (warnings = erreurs) passent ;
- les tests verts à chaque couche concernée : unit (cœur), intégration (Loco +
  SQLite de test), MCP (gate token), frontend (Vitest + Testing Library / MSW), e2e (Playwright) ;
- les critères de sortie de la phase ROADMAP sont remplis ;
- la doc reste cohérente avec le code (si une décision a changé, le contrat est mis à jour) ;
- `docs/HANDOFF.md` reçoit une entrée datée, et `docs/INDEX.md` est mis à jour si un
  livrable est passé au vert.

## Mémoire projet — où chercher quoi

Le projet maintient une base de connaissances opérationnelle sous `docs/`. **En début de session, scanner ces fichiers pour se resituer** :

- **`docs/HANDOFF.md`** — état courant, dernière chose faite, trucs à savoir tout de suite. **À lire en premier.**
- **`docs/INDEX.md`** — catalogue des features livrées avec liens vers spec/plan.
- **`docs/ENVIRONMENT.md`** — paths, services, env vars, accès. À consulter avant de lancer toute commande non-triviale.

À consulter au cas par cas :
- **`docs/QUIRKS.md`** — pièges et comportements non-évidents.
- **`docs/BACKLOG.md`** — idées et améliorations identifiées mais non urgentes.
- **`docs/CONVENTIONS.md`** — skeletons de code et règles tacites.
- **`docs/superpowers/specs/`** — design docs détaillés par feature.
- **`docs/superpowers/plans/`** — plans d'implémentation détaillés par feature.

### À mettre à jour DURANT la session (decision tree — une question = un fichier)

| Tu découvres ou décides… | Fichier |
|---|---|
| Une règle qui s'applique TOUJOURS au projet | `CLAUDE.md` |
| Un squelette de code récurrent | `docs/CONVENTIONS.md` |
| Une feature livrée | ajouter une ligne dans `docs/INDEX.md` + spec/plan dans `docs/superpowers/` si non-trivial |
| Où vit un container, un path, un port, un accès | `docs/ENVIRONMENT.md` |
| Un comportement non-évident, un piège | `docs/QUIRKS.md` (ajouter dès la découverte, pas plus tard) |
| Une idée future / nice-to-have | `docs/BACKLOG.md` |
| L'état mental d'une session significative | `docs/HANDOFF.md` (en fin de session) |

### Règle de fin d'implémentation (NON-NÉGOCIABLE)

À la fin de toute implémentation significative (feature livrée, refactor majeur, bug fix non-trivial, nouvelle commande/script), **avant de signaler la fin du travail**, tu DOIS :

1. **Mettre à jour `docs/INDEX.md`** — ajouter une ligne dans la table correspondante (feature, commande, etc.).
2. **Mettre à jour `docs/HANDOFF.md`** — ajouter une entrée datée en haut (sous le titre H1) avec : `Dernière chose faite`, `Trucs en suspens`, `Prochaine chose à creuser`, `Notes pour future Claude`.
3. **Mettre à jour `docs/QUIRKS.md`** si tu as découvert un piège non-évident pendant l'implémentation.
4. **Mettre à jour `docs/BACKLOG.md`** si tu as identifié des améliorations futures que tu n'as pas implémentées.
5. **Mettre à jour `docs/CONVENTIONS.md`** si tu as introduit un nouveau pattern qui doit être reproduit.
6. **Mettre à jour `docs/ENVIRONMENT.md`** si tu as ajouté/découvert un service, path, port, env var.
7. **Mettre à jour `CLAUDE.md`** si tu as établi une règle qui s'applique toujours au projet.

Ces mises à jour font partie de la définition de "terminé". Une feature livrée sans mise à jour de la mémoire est une feature à moitié livrée.

<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (60-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk go test             # Go test failures only (90%)
rtk jest                # Jest failures only (99.5%)
rtk vitest              # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk pytest              # Python test failures only (90%)
rtk rake test           # Ruby test failures only (90%)
rtk rspec               # RSpec test failures only (60%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%). Format flags (-c, -l, -L, -o, -Z) run raw.
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->