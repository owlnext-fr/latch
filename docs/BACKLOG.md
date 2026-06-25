# Backlog — reporté, hors périmètre v1

> Idées et durcissements écartés *consciemment* de la v1, gardés pour ne pas les
> redécouvrir. Rien ici n'est un manque : ce sont des choix de périmètre.

## git-cliff en CI (release automatisée) (Phase 6 – 2026-06-25)
`CHANGELOG.md` est aujourd'hui généré manuellement (`git cliff --output CHANGELOG.md`). Pour
automatiser la génération à chaque release, ajouter un job CI déclenché sur un push de tag `v*`
qui : (1) lance `git cliff --tag $TAG --output CHANGELOG.md`, (2) commite + pousse le CHANGELOG
mis à jour (ou l'intègre à la GitHub Release). Hors périmètre Phase 6 (génération manuelle suffit
pour la v1) ; à ajouter au moment où la cadence de releases le justifie.

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

## Nettoyage du fichier HTML sur `delete_version` (2026-06-24)
`DELETE /admin/projects/{id}/versions/{n}` supprime la ligne DB mais laisse le fichier
HTML dans le storage. Orphelin inoffensif pour la v1 (petits volumes). Amélioration
future : appeler `storage.delete(&version.html_path)` après la suppression DB. À ne
faire qu'après que le storage expose une méthode `delete` (non encore déclarée dans le
trait `Storage`).

## `/admin` restreint en IP / Tailscale
Durcissement « hide » supplémentaire : `/admin` n'a pas besoin d'être public (accès
navigateur des designers). `/mcp`, lui, doit rester public (cloud Anthropic). Non
retenu en v1 pour ne pas complexifier le branchement.

## Provisioning du connecteur MCP aux designers
Dépend de la formule OWLNEXT (Owner provisionne en Team/Ent vs chacune ajoute l'URL
en Pro/Max). Hors périmètre build — à traiter au branchement, pas au code.

## ~~`serverInfo.name` MCP advertise `"rmcp"` au lieu de `"latch"`~~ — **RÉSOLU 2026-06-25**
`get_info()` MCP appelait `Implementation::from_build_env()` (défaut `ServerInfo`) qui capturait le
`CARGO_CRATE_NAME` de la crate rmcp. Fix livré : `with_server_info(Implementation::new("latch", env!("CARGO_PKG_VERSION")))`
dans `get_info()` (`backend/src/mcp/mod.rs`). Le test `mcp_initialize_handshake` asserte désormais
`serverInfo.name == "latch"` directement.

## ~~Cache de build Docker (cargo-chef)~~ — **LIVRÉ** (Toolchain Task 5 – 2026-06-25)
Dockerfile réécrit avec `cargo-chef` (couche `cook` cachée par `type=gha`). Build rapide sur rebuild. Cf. `docs/INDEX.md`.

## ~~Conteneur en utilisateur non-root~~ — **LIVRÉ** (Toolchain Task 5 – 2026-06-25)
Runtime `gcr.io/distroless/cc-debian12:nonroot` (uid 65532) + stage `dataprep` pour ownership `/data`. `deploy.sh` contient la garde `chown -R 65532:65532 data`. Cf. `docs/INDEX.md` + QUIRKS.

## same_host — port par défaut et IPv6 sans crochets (Phase 2 – 2026-06-24)
`same_host` accepte `("example.com:80", "example.com")` car l'un n'a pas de port explicite — sans connaître le schéma (http/https), on ne peut pas résoudre le port par défaut. Caveat acceptable en v1 (le proxy Caddy normalise le Host avant de transmettre). IPv6 sans crochets (`::1` au lieu de `[::1]`) serait mal découpé par `rsplit_once(':')` — mais les navigateurs émettent toujours `[::1]` dans Origin/Host. Les deux cas sont documentés dans QUIRKS.

## Opacification des erreurs 500 dans `controllers/error.rs` (Phase 2 – 2026-06-24)
`controllers/error.rs::into_response` interpole le texte de `sea_orm::DbErr` / `io::Error`
directement dans le corps de la réponse 500 — aucun secret n'y transite aujourd'hui (les
filtres de requête ne portent pas le PIN), mais un durcissement défensif consisterait à
logger le détail côté serveur et renvoyer un message opaque générique.

## Validation de longueur sur `name` et `brand_name` (Phase 1 – 2026-06-24)
Aujourd'hui, `name` et `brand_name` n'ont aucune contrainte de longueur ni en DB (SQLite `TEXT` = illimité) ni dans le service (`ProjectsService::create` valide uniquement la présence de `name`). Une valeur absurdement longue passerait sans erreur. À ajouter : validation applicative (ex. `name.len() <= 128`) + contrainte DB `VARCHAR(128)` via migration, pour éviter les surprises à l'affichage en Phase 3 (SPA Yew).

## Base de slug éditable (Phase 3 – 2026-06-24)
En v1, le slug est en lecture seule (base lisible auto-générée + suffixe fixe). Rouvrir l'édition de la base du slug nécessite de retoucher le cœur (`slug.rs`), l'API (`PUT /api/projects/{id}`), le DTO et le side-panel `ProjectForm`. Reporté : faible besoin identifié, risque de collisions à gérer si l'admin change la base d'un projet déjà partagé.

## Override `PUBLIC_BASE_URL` (Phase 3 – 2026-06-24)
En v1, la SPA construit l'URL publique via `window.location.origin` (admin et serving `/c` sur la même origine). Si l'admin et le serving `/c/<slug>` étaient un jour sur des hosts distincts (ex. CDN ou sous-domaine dédié), il faudrait un `PUBLIC_BASE_URL` injecté au build ou à l'exécution. Non nécessaire aujourd'hui : même binaire, même origin.

## Couche de toast globale SPA ~~(Phase 3 – 2026-06-24)~~ — **Résolu par React (sonner)**
~~shadcn-rs~~ La migration React a adopté **sonner** comme provider de toasts (auto-dismiss,
programmatique). Toutes les mutations TanStack Query déclenchent `toast.success` / `toast.error`
dans `onSuccess` / `onError`. Ce backlog est **clos**.

## Remontée d'erreur sur `activate_version` ~~(Phase 3 – Yew)~~ — **Résolu par React**
La SPA React affiche les erreurs via `onError` du hook `useActivateVersion` (toast sonner).
Ce backlog est **clos**.

## Polish UI login.rs / `activate_version` / dropzone flicker ~~(Yew punchlist)~~ — **Clos (Yew retiré)**
Ces items concernaient la SPA Yew (`pages/login.rs`, `activate_version`, `ondragleave`).
Avec la migration React, la SPA est réécrite — ces bugs Yew ne se posent plus. **Clos.**

## Chantier polish produit + i18n avant distribution — partiellement résolu par React
- ~~Passer l'UI en EN~~ → **Résolu** : react-i18next FR/EN, défaut EN.
- ~~Explications sur les champs~~ → **Résolu** : helper text dans `ProjectForm` React.
- ~~Dropzone~~ → **Résolu** : dropzone dans `DeployPanel` React.
- ~~Toasts~~ → **Résolu** : sonner sur toutes les mutations.
- **Restant** : revue UX d'ensemble pour distribution (accessibilité clavier, états de chargement
  sur l'activation, cohérence sur mobiles).

## ~~Enrichir `ProjectListItem` : `active_version_n` + `version_count`~~ — FAIT (2026-06-25, commit `797e56b`)
Livré : `ProjectListItem` expose `active_version_n` (n° de version active) + `version_count` et n'expose
plus `active_version_id`. Service `list_with_versions` (2 requêtes, regroupement mémoire, pas de N+1).
`openapi.json` + `schema.d.ts` régénérés ; `routes/list.tsx` affiche « v{n} · {count} versions » (pluriel i18next).

## Bouton « Activer » : état pending (Plan 2 review – 2026-06-25)
Le bouton « Activer » dans la liste des versions (page détail) ne désactive pas immédiatement
pendant la mutation `useActivateVersion`. L'utilisateur peut cliquer plusieurs fois. À corriger :
passer l'`isPending` du hook en `disabled` sur le bouton. Simple, faible risque.

## Code-splitting — bundle frontend 604 kB (Plan 2 review – 2026-06-25)
Le bundle Vite de production est ~604 kB (non gzippé). Le routing TanStack Router permet
du lazy-loading par route (`lazy(() => import('./routes/detail'))`). À évaluer après stabilisation
des routes : le découpage par route (list / detail / login) réduirait le First Load JS.

## Reusable workflows CI (Plan 3 review – 2026-06-25)
Les jobs GitHub Actions back/front/e2e sont définis inline dans `ci.yml`. Refactorer en
`workflow_call` réutilisables dans `.github/workflows/` améliorerait la lisibilité et
permettrait de les déclencher séparément. Non-bloquant pour la v1.

## Actions CI épinglées sur Node 20 déprécié (Toolchain CI – 2026-06-25)
La CI épingle les actions à des SHA v4 (checkout, setup-node, pnpm/action-setup, cache,
upload/download-artifact, docker-*) qui ciblent **Node 20** (déprécié, forcé sur Node 24 par
les runners → annotations non bloquantes sur chaque run). Bump des majors vers les versions
Node-24 (checkout v5, setup-node v5, etc.) — re-résoudre les SHA. Purement hygiène, aucun
impact fonctionnel ; à faire au prochain passage sur `ci.yml`.

## `deny.toml` — transitives de `utoipa-swagger-ui 9` (Zlib) (revue finale Plan 1 – 2026-06-25)
`rust-embed`, `zip`, `zlib-rs` (licence **Zlib**), `zopfli`, `arbitrary` sont entrées au
lockfile avec `utoipa-swagger-ui 9`. L'allowlist stricte de `deny.toml` peut rejeter `Zlib`
au prochain `cargo deny check licenses`. À compléter lors de la remise au vert supply-chain.

## Cookie `SecurityScheme` dans l'OpenAPI (revue finale Plan 1 – 2026-06-25)
L'admin est protégé par cookie de session same-origin mais `ApiDoc` ne déclare pas de
`securityScheme`. Sans impact sur le client `openapi-fetch` (cookies envoyés via
`credentials: 'include'`), mais améliorerait la doc Swagger + l'auto-doc des 401.
À ajouter via un `Modify`/`modifiers` utoipa (scheme `apiKey` cookie nommé `latch_admin`).
Non bloquant.

## Phase 4 — écartés de périmètre (2026-06-25)

### Table `unlock_attempts` + statistiques admin (Phase 4 – écarté §9)
Stocker les tentatives d'unlock (slug, IP, timestamp, succès/échec) pour exposer des
stats à l'admin (nombre de tentatives, taux d'échec) et permettre un audit. Écarté v1 :
le rate-limit in-memory suffit pour la barrière opérationnelle ; les stats sont cosmétiques.
Dépendance : migration + entité SeaORM + section admin détail.

### Scheduler / purge des cookies expirés (Phase 4 – écarté §9)
Un scheduler cron (Loco worker ou tâche async) qui purgerait périodiquement les lignes
de session/cookie expirées. Écarté v1 : SQLite reste petit, pas de pression de croissance
immédiate. À envisager si la volumétrie monte.

### Backoff durable au reboot (Phase 4 – écarté §9.5)
Les compteurs de rate-limit governor sont **in-memory** : un reboot remet les compteurs à
zéro, permettant à un attaquant de relancer une vague après un redémarrage du serveur.
Un backoff durable (Redis, SQLite, ou `RUSTSEC`-safe external store) mitigerait ce vecteur.
Écarté v1 : la limite est assumée et documentée (contrat §9.5, QUIRKS) ; restart est une
opération monitored.

### `unlock.html` `lang="en"` statique — i18n du shell HTML (Phase 4 – revue 2026-06-25)
L'attribut `lang="html"` du shell `unlock.html` est figé en `"en"` (généré par Vite).
Pour une page de déverrouillage multilingue, le shell HTML devrait refléter la langue
de l'utilisateur. Faible priorité : le contenu dynamique de la page est rendu par React
et peut déjà être i18n côté client sans que `lang` corresponde exactement.

### Clarifier / aligner la sémantique de `LATCH_UNLOCK_RL_IP_PER_SECOND` (Phase 4 – revue 2026-06-25)
tower_governor utilise un modèle « token bucket » : `per_second` contrôle le taux de
remplissage du bucket (tokens/seconde), pas une limite de fenêtre glissante. Vérifier que
le nommage `LATCH_UNLOCK_RL_IP_PER_SECOND` est suffisamment clair pour les opérateurs ou
s'il vaut mieux le renommer en `LATCH_UNLOCK_RL_IP_REPLENISH_PER_SEC`. Non-breaking si
renommé avant la Phase 6 (packaging publiable).

### Test isolé du plafond slug-global (Phase 4 – revue 2026-06-25)
Le governor slug-global (`LATCH_UNLOCK_RL_SLUG_BURST`/`PERIOD_SECS`) n'a pas de test
d'intégration qui vérifie le rejet quand le plafond est atteint sur un seul slug (contrairement
au per-IP qui est testé). À ajouter pour garantir la régression de la barrière §9.5.

### Erreur opaque + sans log de `storage.read` dans `serve.rs` (Phase 4 – revue 2026-06-25)
Le handler `serve` mappe une erreur `storage.read` via `.map_err(into_response)` **sans
aucun log côté serveur** (pas de `tracing::error!`). La réponse 500 passe par
`loco_rs::Error::Message` (inclut le texte de l'erreur IO ; pas de fuite du chemin de
fichier via `io::Error` ici, mais pas opaque non plus). Durcir : logger le détail côté
serveur (observabilité) ET renvoyer un message générique `"internal error"`. Même pattern
que le backlog `controllers/error.rs`.

### Broutilles UI unlock (revue itération 2026-06-25)
- Clés i18n admin mortes après le retrait des swaps de texte des boutons (`login.submitting`,
  `deploy.deploying`, `danger.deleting`) — inoffensives, à supprimer un jour.
- `InputOTPSeparator` vendorisé dans `components/ui/input-otp.tsx` mais inutilisé (boilerplate
  shadcn gardé pour parité). À trimmer si on ne s'en sert jamais.
- Bordure des slots OTP en valeur arbitraire `oklch(0.85 0.003 48.717)` (même teinte que `--input`,
  plus foncée) **sans variante dark** : invisible aujourd'hui (page unlock en fond clair only). Si
  le dark mode arrive sur cette surface, lier à un token plutôt qu'une valeur en dur.
- ~~Favicon `/vite.svg` 404 sur `/admin`~~ — **corrigé** (lien favicon retiré d'`index.html` ;
  l'asset Vite par défaut n'était qu'un placeholder de scaffold).

