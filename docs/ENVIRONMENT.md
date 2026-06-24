# Environment — spécifique à l'instance

> Ce qui est propre à *ta* machine / *ton* déploiement : paths réels, ports, contenu
> du `.env`, secrets (jamais les valeurs ici — juste les clés attendues). Pour les
> commandes génériques de build/test, voir `docs/BOOTSTRAP.md §3` plutôt que dupliquer.

## Variables d'environnement attendues (`.env`)
- `ADMIN_USER` — identifiant admin.
- `ADMIN_PASS` — mot de passe admin (comparé à temps constant, non hashé).
- `DEPLOY_TOKEN` — secret applicatif validé par les tools MCP.
- `UNLOCK_COOKIE_SECRET` — clé HMAC de signature du cookie de déverrouillage client.
- _(à compléter : `DATABASE_URL` SQLite, chemin du volume `data/`, etc.)_

## Serving
- Domaine : `latch.owlnext.fr` (Caddy en façade, TLS + reverse proxy).
- Path MCP : `/mcp` _(option : path non devinable — à figer si retenu)._

## Box de déploiement
- _(host, chemin du repo/compose, emplacement du volume `data/` — à remplir)._

## GHCR
- Package : `ghcr.io/owlnext-fr/latch` — **public** (pas de `docker login` au pull).

## Connexion du connecteur MCP côté Claude web
- _(procédure de branchement aux designers — dépend de la formule OWLNEXT,
  laissée hors périmètre build ; à documenter au moment du branchement)._
