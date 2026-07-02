# Plan — Revalidation doc : commentaires + MCP (issue #4)

Branche : `feat/4-doc-revalidation`. Réf. spec : `docs/superpowers/specs/4-doc-revalidation-comments-mcp.md`.

## Étape 1 — Corrections texte (rapides, sans dépendance)
1. `index.mdx` — carte MCP : « Two tools » → 3 tools, ajouter `pull_prototype`.
2. `configuration.mdx` — (a) `DEPLOY_TOKEN` : ajouter `pull_prototype` à la liste ;
   (b) section rate-limit : ajouter les 4 vars `LATCH_COMMENT_RL_*` (défauts 10/1/60/1).
3. `quickstart.mdx` — ajouter une étape/callout « Itérer avec les retours » : après
   commentaires des reviewers, `pull_prototype` ramène HTML + fils dans Claude, on itère,
   on redéploie. Lien vers tools-reference#pull_prototype.

## Étape 2 — Instance seedée pour les captures
Précondition QA : `:5150` sert le build à jour.
1. Build : `cd frontend && pnpm build` (dist) ; backend `cargo loco start` depuis `backend/`.
2. Seed (placeholders génériques, zéro nom client) :
   - Projet `Mon Projet` (slug auto), `comments_enabled = true`, sans code (pour accès visiteur direct) OU avec code puis déverrouillage.
   - Déployer une version HTML simple mono-fichier avec des éléments ancrables (titre, bouton).
   - Poster 1-2 commentaires visiteur (via la barre flottante, navigateur).
   - Depuis la Review page admin : répondre au fil (badge Admin) + démarrer une note privée.

## Étape 3 — Captures Playwright
Harness : tools `mcp__plugin_playwright_playwright__*` (navigateur réel).
- `comments-visitor-bar.png` — barre flottante + compose popup (`/c/<slug>`).
- `comments-thread-admin-reply.png` — fil avec réponse admin + badge Admin.
- `comments-review-page.png` — Review page admin, pins positionnés (`/admin/projects/$id/versions/$n/review`).
- `comments-version-panel.png` — panel Comments par version (page détail).
- (option) `project-comments-toggle.png` — toggle `comments_enabled`.
Sortie : `public_docs/public/img/`. Viewport large, thème par défaut, données factices.

## Étape 4 — Câblage dans les MDX
- `admin/comments.mdx` — insérer visitor bar (§Reviewer experience), thread+badge (§Identity/Admin), Review page (§Review page).
- `admin/versions.mdx` — Review page + panel Comments (sections existantes).
- `admin/projects.mdx#comments` — toggle (optionnel).
Alt text descriptif, style aligné sur les `![...]` existants.

## Étape 5 — Build + QA humaine
1. `cd public_docs && pnpm build` + `pnpm types:check` (ou équivalent) → vert.
2. Lancer le dev-server fuma, remettre à l'utilisateur pour QA visuelle.
3. Itérer selon retours (recadrage captures, wording).

## Étape 6 — Finition (gate)
1. QUIRKS.md : gap `.env.example` des vars `LATCH_COMMENT_RL_*` + recette seed captures.
2. INDEX.md + HANDOFF.md à jour ; follow-up issue pour `.env.example` si retenu.
3. PR `Closes #4`, carte → In review ; CI + Sonar verts → merge, suppression branche.

## Risques / points d'attention
- Seed des commentaires : nécessite de piloter l'UI visiteur (cookie `latch_comment`) et la Review page admin — chemin le plus fragile. Fallback : POST direct sur les endpoints commentaires si l'UI résiste.
- Captures : éviter tout texte client réel dans le HTML de démo.
- Ne pas toucher au backend (scope doc).
