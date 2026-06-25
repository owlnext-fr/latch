# syntax=docker/dockerfile:1

###############################################################################
# Stage 1 — build de la SPA React (Vite + pnpm)
###############################################################################
FROM node:24-bookworm-slim AS frontend
RUN corepack enable
WORKDIR /src/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
# --ignore-scripts : pas de lifecycle scripts à l'install (S6505). Le build SPA
# n'a besoin d'aucun postinstall (esbuild/rollup via optionalDependencies).
RUN pnpm install --frozen-lockfile --ignore-scripts
COPY frontend/ ./
COPY openapi.json /src/openapi.json
RUN pnpm build

###############################################################################
# Stage 2 — build du backend via cargo-chef (couche deps cachée)
###############################################################################
FROM rust:1.96-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
# Build des deps seules → couche cachée tant que Cargo.toml/lock ne changent pas.
RUN cargo chef cook --release -p latch --locked --recipe-path recipe.json
COPY . .
# --locked : respecte Cargo.lock, pas de résolution flottante (S8549).
RUN cargo build -p latch --release --locked

###############################################################################
# Stage 2.5 — préparer /data possédé par nonroot (distroless n'a pas de shell)
###############################################################################
FROM debian:bookworm-slim AS dataprep
RUN mkdir -p /data && chown 65532:65532 /data

###############################################################################
# Stage 3 — runtime minimal NON-ROOT (distroless, tag figé :nonroot)
###############################################################################
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=builder  /src/target/release/latch-cli  /app/latch-cli
COPY --from=builder  /src/backend/config            /app/config
COPY --from=frontend /src/frontend/dist             /app/frontend/dist
# /data possédé par nonroot (65532) → volume inscriptible au premier boot.
COPY --from=dataprep --chown=65532:65532 /data /data
ENV LOCO_ENV=production
ENV LATCH_SPA_DIST=/app/frontend/dist
EXPOSE 5150
USER nonroot
ENTRYPOINT ["/app/latch-cli"]
CMD ["start"]
