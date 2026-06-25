#!/usr/bin/env bash
# Déploiement manuel sur la box. GHCR public → pas de `docker login` requis.
# L'image ne contient aucun secret : tout est injecté par .env au runtime.
set -euo pipefail

# Le runtime tourne en non-root (uid 65532) ; le bind-mount ./data doit lui appartenir.
mkdir -p data
chown -R 65532:65532 data 2>/dev/null || true   # best-effort (nécessite root la 1re fois)

docker compose pull        # pull de l'image GHCR publique
docker compose up -d       # relance avec le .env
docker image prune -f      # nettoie les vieilles images
