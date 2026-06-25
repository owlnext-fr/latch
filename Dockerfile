# syntax=docker/dockerfile:1

###############################################################################
# Stage 1 — build de la SPA React (Vite + pnpm)
###############################################################################
FROM node:24-bookworm-slim AS frontend
RUN corepack enable
WORKDIR /src/frontend
# Couche cache : deps seules (lock copié avant la source)
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
# Source + schéma OpenAPI commité (pour gen:api si lancé ; sinon schema.d.ts est commité)
COPY frontend/ ./
COPY openapi.json /src/openapi.json
RUN pnpm build      # vite build → /src/frontend/dist

###############################################################################
# Stage 2 — build du backend (SQLite « bundled » → binaire autonome)
###############################################################################
FROM rust:1-bookworm AS backend
WORKDIR /src
COPY . .
# -p latch ne compile que le backend natif (le frontend wasm est hors scope ici,
# mais sa présence est requise pour charger le workspace).
RUN cargo build -p latch --release

###############################################################################
# Stage 3 — runtime minimal (distroless : pas de shell, pas de lib SQLite système)
###############################################################################
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /app
COPY --from=backend  /src/target/release/latch-cli  /app/latch-cli
COPY --from=backend  /src/backend/config            /app/config
COPY --from=frontend /src/frontend/dist             /app/frontend/dist
ENV LOCO_ENV=production
ENV LATCH_SPA_DIST=/app/frontend/dist
EXPOSE 5150
# auto_migrate=true dans config/production.yaml → migrations jouées au boot.
# Pas de `migrate && start` (distroless n'a pas de shell pour chaîner).
ENTRYPOINT ["/app/latch-cli"]
CMD ["start"]
