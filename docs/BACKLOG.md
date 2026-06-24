# Backlog — reporté, hors périmètre v1

> Idées et durcissements écartés *consciemment* de la v1, gardés pour ne pas les
> redécouvrir. Rien ici n'est un manque : ce sont des choix de périmètre.

## Auth MCP — Modèle 2 (vrai OAuth 2.1)
401 + `WWW-Authenticate`, métadonnées `.well-known/oauth-protected-resource`,
authorization-code + PKCE + consentement, client pré-enregistré ou DCR. La voie
« propre » officiellement supportée par claude.ai. Reportée : chantier séparé,
bascule possible sans toucher au reste. v1 = Modèle 1 (`deploy_token` en argument).

## PIN chiffré au repos
v1 stocke le PIN en clair (choix (b), pour le copier en admin). Pour un repo de
référence, un chiffrement-au-repos (clé en env, déchiffré à la lecture admin) serait
plus exemplaire. Non-breaking : seule la colonne change, l'algo de vérif est identique.

## Passphrase configurable par projet (au lieu du PIN seul)
v1 = PIN 6 chiffres (friction minimale pour client non-tech). Comme on stocke de
toute façon une valeur récupérable, offrir « PIN ou passphrase » par projet est un
changement non-breaking : seul l'input de la page de déverrouillage change.

## ~~Suffixe de slug plus long~~ — TRANCHÉ en v1 (2026-06-24)
Retenu directement en v1 : **8 chars base62** (≈ 47 bits) comme défaut, cf. QUIRKS.
Plus dans le backlog.

## `/admin` restreint en IP / Tailscale
Durcissement « hide » supplémentaire : `/admin` n'a pas besoin d'être public (accès
navigateur des designers). `/mcp`, lui, doit rester public (cloud Anthropic). Non
retenu en v1 pour ne pas complexifier le branchement.

## Provisioning du connecteur MCP aux designers
Dépend de la formule OWLNEXT (Owner provisionne en Team/Ent vs chacune ajoute l'URL
en Pro/Max). Hors périmètre build — à traiter au branchement, pas au code.

## Cache de build Docker (cargo-chef)
Le Dockerfile Phase 0 fait un `COPY . . && cargo build` simple : chaque build recompile
toutes les deps. Passer à `cargo-chef` (recipe deps en couche cachée) accélérerait
fortement la CI/les rebuilds. Non-breaking, purement perf de build.

## Conteneur en utilisateur non-root
L'image distroless tourne en `root` (le `latch.sqlite` du volume est créé root). Passer
à `gcr.io/distroless/cc-debian12:nonroot` + ownership du volume `/data` durcirait le
runtime. Reporté : friction d'ownership du volume à régler, faible enjeu derrière Caddy.

## Validation de longueur sur `name` et `brand_name` (Phase 1 – 2026-06-24)
Aujourd'hui, `name` et `brand_name` n'ont aucune contrainte de longueur ni en DB (SQLite `TEXT` = illimité) ni dans le service (`ProjectsService::create` valide uniquement la présence de `name`). Une valeur absurdement longue passerait sans erreur. À ajouter : validation applicative (ex. `name.len() <= 128`) + contrainte DB `VARCHAR(128)` via migration, pour éviter les surprises à l'affichage en Phase 3 (SPA Yew).
