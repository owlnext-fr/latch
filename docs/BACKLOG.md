# Backlog — icebox d'idées brutes

> **Tampon de capture rapide, PAS le tracker.** Le suivi des tâches vit sur le
> **board GitHub** (projet #1, `github.com/owlnext-fr/latch`). Ici : les idées pas
> encore assez mûres pour devenir une issue, les caveats assumés, et la trace des
> choix déjà tranchés. **Règle** : dès qu'un item est concret/prêt, on le promeut en
> issue (colonne *Ready*/*Backlog*) et on le RETIRE d'ici. Triage à chaque fin de cycle.
>
> _Migration 2026-07-01 → board : les items actionnables ont été portés en issues
> **#9** (Phase 9 polish), **#10** (commentaires follow-ups), **#11** (dette CI),
> **#12** (durcissements sécurité), **#13** (nettoyages). Retirés d'ici pour tuer le doublon._
>
> _Triage 2026-07-02 → board : « Revue UX d'ensemble pour distribution » promue en issue
> **#21** (a11y clavier, loading states, mobile ; milestone non-commercialisable). Retirée d'ici._

## Idées ouvertes — pas encore mûres / hors périmètre build

### `/admin` restreint en IP / Tailscale
Durcissement « hide » supplémentaire : `/admin` n'a pas besoin d'être public (accès
navigateur des designers). `/mcp`, lui, doit rester public (cloud Anthropic). Non
retenu en v1 pour ne pas complexifier le branchement. _(Volontairement laissé hors
du lot durcissement #12.)_

### Provisioning du connecteur MCP aux designers
Dépend de la formule OWLNEXT (Owner provisionne en Team/Ent vs chacun ajoute l'URL
en Pro/Max). Hors périmètre build — à traiter au branchement, pas au code.

### Base de slug éditable
En v1, le slug est en lecture seule (base lisible auto-générée + suffixe fixe). Rouvrir
l'édition de la base du slug nécessite de retoucher le cœur (`slug.rs`), l'API
(`PUT /api/projects/{id}`), le DTO et le side-panel `ProjectForm`. Reporté : faible besoin
identifié, risque de collisions à gérer si l'admin change la base d'un projet déjà partagé.

### Override `PUBLIC_BASE_URL`
En v1, la SPA construit l'URL publique via `window.location.origin` (admin et serving
`/c` sur la même origine). Si l'admin et le serving `/c/<slug>` étaient un jour sur des
hosts distincts (ex. CDN ou sous-domaine dédié), il faudrait un `PUBLIC_BASE_URL` injecté
au build ou à l'exécution. Non nécessaire aujourd'hui : même binaire, même origin.

### `same_host` — port par défaut et IPv6 sans crochets
`same_host` accepte `("example.com:80", "example.com")` car l'un n'a pas de port explicite
— sans connaître le schéma (http/https), on ne peut pas résoudre le port par défaut. Caveat
acceptable en v1 (le proxy Caddy normalise le Host avant de transmettre). IPv6 sans crochets
(`::1` au lieu de `[::1]`) serait mal découpé par `rsplit_once(':')` — mais les navigateurs
émettent toujours `[::1]` dans Origin/Host. Les deux cas sont documentés dans QUIRKS.

## Trace historique — choix tranchés / résolus (ne pas redécouvrir)

- ~~Suffixe de slug plus long~~ — **TRANCHÉ v1** (2026-06-24) : 8 chars base62 (≈ 47 bits) par défaut.
- ~~`serverInfo.name` MCP advertise `"rmcp"`~~ — **RÉSOLU** 2026-06-25 (`with_server_info(Implementation::new("latch", …))`).
- ~~Cache de build Docker (cargo-chef)~~ — **LIVRÉ** 2026-06-25.
- ~~Conteneur en utilisateur non-root~~ — **LIVRÉ** 2026-06-25 (distroless `nonroot`, uid 65532).
- ~~Erreur opaque + sans log de `storage.read` dans `serve.rs`~~ — **RÉSOLU** (Phase 7 Lot 4 : logs `tracing::error!` + page d'erreur HTML générique).
- ~~Couche de toast globale SPA~~ — **Résolu** par React (sonner sur toutes les mutations).
- ~~Remontée d'erreur sur `activate_version`~~ — **Résolu** par React (`onError` → toast).
- ~~Polish UI login.rs / `activate_version` / dropzone flicker~~ — **Clos** (SPA Yew retirée).
- ~~Enrichir `ProjectListItem` (`active_version_n` + `version_count`)~~ — **FAIT** 2026-06-25 (`797e56b`).
- ~~Panneau « mes commentaires » avec saut-au-pin (spec §8.6)~~ — **FAIT** 2026-07-01 (`comments-drawer.tsx` + `focusPinFromList`).
