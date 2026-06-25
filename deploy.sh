#!/usr/bin/env bash
# Déploiement manuel sur la box. GHCR public → pas de `docker login` requis.
# L'image ne contient aucun secret : tout est injecté par .env au runtime.
set -euo pipefail

# Le runtime tourne en non-root (uid 65532) ; le bind-mount ./data doit lui appartenir.
mkdir -p data
if ! chown -R 65532:65532 data 2>/dev/null; then
  # Pas les droits (deploy non-root) : avertir si ./data n'est pas déjà à 65532,
  # sinon le container non-root échouera à écrire (SQLite + HTML des versions).
  owner=$(stat -c %u data 2>/dev/null || echo '?')
  if [ "$owner" != "65532" ]; then
    echo "⚠️  ./data appartient à uid $owner, pas 65532 — le runtime non-root ne pourra pas y écrire." >&2
    echo "    Corrige une fois en root :  sudo chown -R 65532:65532 ./data" >&2
  fi
fi

docker compose pull        # pull de l'image GHCR publique
docker compose up -d       # relance avec le .env
docker image prune -f      # nettoie les vieilles images
