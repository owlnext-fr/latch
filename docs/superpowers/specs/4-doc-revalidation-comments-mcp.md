# Spec — Revalidation doc : commentaires + nouvelles méthodes MCP (issue #4)

## Contexte

Trois features livrées récemment doivent être reflétées fidèlement dans la doc
publique fumadocs (`public_docs/`) :

1. Commentaires ancrés côté **visiteur** (`/c/<slug>`).
2. Commentaires côté **admin** (Review page : création de fil, réponse, badge Admin, modération).
3. Tool MCP **`pull_prototype`** (pull HTML + fils de commentaires pour itérer depuis Claude).

L'issue demande de scanner **toute** la doc, corriger les écarts, et **reprendre des
captures** (Playwright) là où il en manque.

## État des lieux (audit vérifié dans le code)

### Fond déjà correct
- `admin/comments.mdx` — décrit fidèlement visiteur + admin (Review page, badge, modération). Colle au code (`routes/review.tsx`, `comments/data/admin-adapter.ts`, `services/comments.rs`).
- `publish-from-claude/tools-reference.mdx` — les 3 tools documentés, `pull_prototype` avec signature + réponse + invariants sécurité corrects.
- `admin/versions.mdx`, `admin/projects.mdx` — sections Comments/Review correctes.

### Écarts texte confirmés (à corriger)
| Fichier | Écart |
|---|---|
| `index.mdx` | Carte MCP « Two tools — deploy_prototype and list_projects » → il y a **3** tools ; ajouter `pull_prototype`. |
| `deploy/configuration.mdx` | `DEPLOY_TOKEN` décrit comme validé par « every MCP tool (deploy_prototype, list_projects) » → ajouter `pull_prototype`. |
| `deploy/configuration.mdx` | 4 variables de rate-limit commentaires non documentées : `LATCH_COMMENT_RL_IP_BURST` (10), `LATCH_COMMENT_RL_IP_PER_SECOND` (1), `LATCH_COMMENT_RL_SLUG_BURST` (60), `LATCH_COMMENT_RL_SLUG_PERIOD_SECS` (1). Source de vérité : `backend/src/controllers/serve.rs:703-706`. |
| `quickstart.mdx` | Aucune mention de la boucle « pull le proto + les commentaires pour itérer » — c'est pourtant la valeur produit de #4. |

### Manque de captures (feature commentaires = 0 visuel aujourd'hui)
Assets actuels : `admin-list.png`, `admin-versions.png`, `unlock.png` (aucun sur les commentaires).

Captures à produire (Playwright, instance seedée) :
1. **Visiteur** — barre d'action flottante + popup de composition d'un commentaire ancré.
2. **Visiteur** — fil ouvert avec une réponse **admin** portant le badge « Admin ».
3. **Admin** — Review page : prototype en frame avec les **pins positionnés**.
4. **Admin** — panel « Comments » par version (liste des fils) sur la page détail.
5. (optionnel) **Admin** — toggle `comments_enabled` du formulaire projet.

## Hors-scope / découvertes
- Les 4 vars `LATCH_COMMENT_RL_*` manquent aussi dans `.env.example` (le code les lit mais le template ne les liste pas). → à consigner dans QUIRKS + décider d'un follow-up (issue) ; ne pas élargir #4 au backend sans raison.

## Invariants à ne pas violer (contrat §9)
La doc ne doit jamais suggérer qu'une réponse expose un hash, un PIN hors détail projet,
ou un `owner_token`. Les exemples de réponse MCP restent conformes (`is_admin` booléen).

## Confidentialité
Aucune donnée de seed ne doit contenir de nom de client réel : placeholders génériques
(`Mon Projet` / `mon-projet`, `ACME`, `demo`) pour les projets, versions et commentaires
qui apparaîtront dans les captures.

## Critères d'acceptation
- Les 4 écarts texte corrigés.
- ≥ 4 captures des commentaires produites, intégrées aux MDX pertinents avec alt text.
- Build fumadocs vert (`pnpm build` + `types:check`).
- QA visuelle humaine OK sur le dev-server fuma.
- Mémoires projet à jour (INDEX, HANDOFF, QUIRKS).
