import { fileURLToPath } from 'node:url'
import { defineConfig, type ProxyOptions } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

const BACKEND = 'http://127.0.0.1:5150'

// Proxy dev vers le backend en réécrivant l'en-tête `Origin` sur la cible.
// Le backend porte une garde CSRF same-origin (`require_same_origin` : l'hôte de
// `Origin` doit matcher celui du `Host`). En dev, l'admin tourne sur :5173 (Vite)
// et proxy vers :5150 : le proxy pose `Host: …:5150` (cible) mais le navigateur
// envoie `Origin: …:5173` → ports différents → 403 sur toute mutation
// (create/update/delete projet, écritures commentaires). On réaligne donc l'`Origin`
// proxifié sur la cible. Dev-only, aucun impact prod (en prod l'admin est servi par
// le backend → same-origin natif).
const proxyToBackend: ProxyOptions = {
  target: BACKEND,
  changeOrigin: true,
  configure: (proxy) => {
    proxy.on('proxyReq', (proxyReq) => {
      proxyReq.setHeader('origin', BACKEND)
    })
  },
}

export default defineConfig({
  base: '/',
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        unlock: fileURLToPath(new URL('./unlock.html', import.meta.url)),
        error: fileURLToPath(new URL('./error.html', import.meta.url)),
        shell: fileURLToPath(new URL('./shell.html', import.meta.url)),
      },
    },
  },
  server: {
    proxy: {
      '/api': proxyToBackend,
      '/_health': BACKEND,
      '/c': proxyToBackend,
      // Les surfaces visiteur (unlock/shell/error) sont servies par le backend depuis
      // le `dist/` buildé et référencent des assets hashés `/assets/*`. Sans ce proxy,
      // Vite renvoie son index.html de fallback (text/html) pour `/assets/*` → le
      // navigateur rejette le module JS (MIME) → page cassée. L'admin en dev ne passe
      // jamais par `/assets` (il charge via `/src`,`/@vite`), donc aucun impact.
      '/assets': BACKEND,
    },
  },
})
