#!/usr/bin/env bash
# Déploiement manuel sur la box. GHCR public → pas de `docker login` requis.
# L'image ne contient aucun secret : tout est injecté par .env au runtime.
set -euo pipefail

docker compose pull        # pull de l'image GHCR publique
docker compose up -d       # relance avec le .env
docker image prune -f      # nettoie les vieilles images
