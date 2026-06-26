import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

// Sous-chemin projet GitHub Pages (owlnext-fr.github.io/latch) par défaut.
// Bascule racine/domaine custom ultérieure : poser DOCS_BASE_PATH=''.
const basePath = process.env.DOCS_BASE_PATH ?? '/latch';

/** @type {import('next').NextConfig} */
const config = {
  output: 'export',
  reactStrictMode: true,
  // Pas de serveur d'optimisation d'images en export statique.
  images: { unoptimized: true },
  // basePath + assetPrefix pour servir sous /latch sans 404 d'assets.
  basePath,
  assetPrefix: basePath || undefined,
  // /me -> /me/ + /me/index.html : URLs stables sous Pages.
  trailingSlash: true,
};

export default withMDX(config);
