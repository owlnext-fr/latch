# Fiche — site de documentation public de `latch`

> **Document autonome.** Brief de construction du site de doc public, **découplé du
> kit principal** (`CLAUDE.md` / `docs/`). Se prend en main séparément : à dérouler
> une fois l'admin réelle disponible (cf. séquençage §6). Le seul lien avec le kit est
> le *sourcing de contenu* (§5) — certaines pages se dérivent du contrat.

---

## 1. Ce que c'est

La vitrine publique de `latch` : une home soignée + une documentation structurée,
pour présenter le projet comme un livrable FOSS sérieux. Artefact **séparé** de
l'app Rust — sa propre build, sa propre CI, son propre déploiement.

## 2. Décisions figées

- **Outil : Fumadocs** (framework de doc React/Next, MDX). Choisi pour la finition
  turnkey — thème soigné, composants, home présentable — qui prime ici sur la pureté
  Rust. (Zola a été envisagé pour rester 100 % Rust mais écarté : pas turnkey, finition
  à la charge du dev. mdBook écarté : format « livre », pas de vraie landing.)
- **Export statique** (`output: 'export'` côté Next) → site HTML/CSS/JS inerte, **aucun
  serveur Node au runtime**. Node vit **uniquement au build/CI**. Le runtime de `latch`
  n'est pas touché : la cohérence Rust vaut pour ce qui tourne en prod, pas pour l'outil
  qui génère un site statique une fois.
- **Hébergement : GitHub Pages.** Déploiement par GitHub Actions.
- **Emplacement : `public_docs/`** dans le monorepo `owlnext-fr/latch` (docs versionnées
  avec le code, PR de doc à côté des PR de feature).
- **Recherche** : statique, via index Orama pré-rendu au build (pas de serveur).

## 3. Décision encore ouverte — l'URL d'hébergement

Une seule chose à trancher avant le premier déploiement, parce qu'elle change la config
Next :

- **Domaine custom `docs.latch.owlnext.fr`** (CNAME → GitHub Pages). Site servi à la
  **racine** → pas de `basePath`, pas d'`assetPrefix`, finition Fumadocs pleine et
  sans piège. **C'est le lean.** Coût : un enregistrement DNS de plus.
- **Sous-chemin `owlnext-fr.github.io/latch`** : pas de DNS à gérer, mais Next en export
  statique exige alors `basePath: '/latch'` + `assetPrefix` (URL complète) + un
  `.nojekyll`. C'est *la* source classique de « le site se déploie mais styles/scripts
  en 404 ». Ça marche, mais c'est un cran de friction et une config à ne pas rater.

> Tant que ce n'est pas tranché, partir du domaine custom dans la config et garder le
> sous-chemin documenté en repli.

## 4. Piège à éviter — collision avec le `docs/` interne

Le repo a déjà un `docs/` : c'est le **kit de contrôle interne** (contrat, BOOTSTRAP,
HANDOFF…), **pas** de la doc utilisateur. Fumadocs doit sourcer son contenu **uniquement
depuis `public_docs/content/`**, jamais depuis le `docs/` du repo. Sinon on publie le
HANDOFF et le contrat comme « documentation produit ».

## 5. Arborescence de contenu (par audience)

`latch` a trois publics qui ne lisent pas les mêmes pages — la structure les sépare.

```
public_docs/content/
  index.mdx                  # home : pitch une phrase, capture, liens Quickstart + Deploy
  quickstart.mdx             # chemin doré bout-en-bout (déployer → 1er projet → brancher
                             #   le MCP → 1re publi depuis Claude → ouvrir le lien client)
  deploy/                    # public OPÉRATEUR
    docker.mdx               #   conteneur simple
    docker-compose.mdx       #   compose + volume + Caddy + .env (recommandé)
    from-source.mdx          #   build backend + trunk build front
    configuration.mdx        #   réf. env (ADMIN_*, DEPLOY_TOKEN, UNLOCK_COOKIE_SECRET…)
    backup-upgrade.mdx       #   volume data/ (sqlite + html ensemble), migration au boot
  admin/                     # public DESIGNER (pilote l'admin)
    projects.mdx             #   créer (side-panel), slug + suffixe
    access-codes.mdx         #   PIN auto-généré, les deux états de /c/<slug>
    versions.mdx             #   déployer, prévisualiser, basculer l'active
    co-branding.mdx          #   nom de marque sur la page de déverrouillage
  publish-from-claude/       # public DESIGNER (publie depuis Claude)
    connect-mcp.mdx          #   brancher le connecteur, le deploy_token, le consentement
    deploy-prototype.mdx     #   le tool, l'argument activate
    why-token-not-oauth.mdx  #   le Modèle 1 expliqué + la note sécu
  how-it-works/              # public CONTRIBUTEUR / curieux
    architecture.mdx         #   l'archi en couches (réécrite du contrat, pour public)
    security-model.mdx       #   deux cookies, rate-limit load-bearing, CVE/allowed_hosts
    contributing.mdx         #   build, tests par couche, CI
```

## 6. Sourcing du contenu & séquençage

- **Dérivable maintenant**, sans UI finie — tout vient du contrat (`docs/contrat-deploy.md`)
  et du BOOTSTRAP : `how-it-works/` (archi, modèle sécu, contributing) et `deploy/`
  (commandes, env, Caddy, backup). Ces pages peuvent être rédigées en parallèle de
  l'implémentation.
- **À écrire après la Phase 3** du kit (admin réelle) : `admin/` et le `quickstart`,
  parce qu'ils dépendent de l'UI réelle (captures + commandes/écrans exacts).

## 7. Captures — réutiliser le harnais Playwright

Les captures d'écran se font via **le harnais e2e Playwright déjà dans la stack** (même
automation navigateur, même stack montée avec données de seed) → captures
**reproductibles et rafraîchissables** quand l'UI change, pas des PNG manuels qui
pourrissent. Un script dédié boote la stack avec un projet de seed et capture les écrans
admin + la page de déverrouillage.

**Caveat honnête :** l'étape « 1re publi depuis Claude » se passe dans l'UI de claude.ai.
Playwright ne doit pas automatiser de captures de l'interface d'Anthropic (auth, ToS,
fragilité). Cette partie s'illustre par un **schéma annoté** ou par le **résultat** (la
nouvelle version qui apparaît dans l'admin, le lien client qui s'ouvre), pas par une
capture automatisée de claude.ai.

## 8. Build & déploiement

- Scaffold : `pnpm create fumadocs-app` (choisir Next.js). Node ≥ version courante,
  gestionnaire de paquets selon le template (souvent pnpm) — vérifier au scaffold.
- Config Next : `output: 'export'`, `images: { unoptimized: true }`, et selon §3 soit
  racine (domaine custom) soit `basePath`/`assetPrefix` (sous-chemin) + `.nojekyll`.
- **Workflow CI séparé `deploy-docs.yml`**, distinct de la CI Rust : déclenché sur push
  `main` filtré sur `public_docs/**`, installe Node + deps, build statique (avec index
  Orama), déploie sur GitHub Pages (`actions/deploy-pages` ou branche `gh-pages`).
- La CI Rust du kit n'est pas touchée — deux pipelines, deux déclencheurs.

## 9. Définition de « terminé » (pour le site)

- Build statique OK, déployé sur GitHub Pages à l'URL retenue (§3), styles/scripts servis
  (pas de 404 d'assets).
- Toutes les pages de l'arborescence §5 présentes ; `how-it-works/` et `deploy/` complètes,
  `admin/` + `quickstart` complétées après Phase 3.
- Captures à jour (harnais Playwright), schéma pour le flux Claude.
- Liens internes vérifiés ; recherche fonctionnelle.
- Sources de contenu issues de `public_docs/content/` uniquement (jamais le `docs/` interne).
