# syntax=docker/dockerfile:1

###############################################################################
# Stage 1 — build de la SPA Yew (Trunk → wasm32)
###############################################################################
FROM rust:1-bookworm AS frontend
RUN rustup target add wasm32-unknown-unknown \
 && cargo install trunk --version ^0.21 --locked
WORKDIR /src
COPY . .
# Trunk télécharge wasm-bindgen-cli tout seul (version alignée sur la crate).
RUN cd frontend && trunk build --release

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
