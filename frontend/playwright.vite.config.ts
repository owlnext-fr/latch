import { defineConfig } from '@playwright/test'

const PORT = 5150
const VITE_PORT = 5173
// DB distincte du config par défaut (/tmp/latch-e2e.sqlite) pour éviter toute
// collision si les deux suites tournent simultanément (reuseExistingServer local).
const DB = '/tmp/latch-vite-e2e.sqlite'

// Config dédiée au smoke Vite dev-server (:5173).
// Deux webServers :
//   [0] backend Loco — sert le dist/ buildé (assets + routes /c /api /mcp)
//   [1] Vite dev — sert l'admin SPA avec HMR, proxie /api, /c, /assets vers :5150
// baseURL = :5173 → tous les navigations de la suite traversent Vite.
// reuseExistingServer: !CI → en dev, réutilise les serveurs déjà démarrés par l'utilisateur.
export default defineConfig({
  testDir: './e2e-vite',
  testMatch: /.*\.spec\.ts$/,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: `http://127.0.0.1:${VITE_PORT}`,
    trace: 'on-first-retry',
  },
  webServer: [
    {
      // Même commande que playwright.config.ts mais avec une DB isolée.
      // LATCH_BINDING=127.0.0.1 : cohérent avec le poll ci-dessous (IPv4 explicite, anti-flaky CI).
      command: `pnpm build && rm -f ${DB} && cd ../backend && LATCH_BINDING=127.0.0.1 LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret LATCH_STORAGE_ROOT=/tmp/latch-vite-e2e-data LATCH_LOGIN_RL_BURST=100000 DATABASE_URL='sqlite://${DB}?mode=rwc' cargo loco start`,
      url: `http://127.0.0.1:${PORT}/_health`,
      timeout: 180_000,
      reuseExistingServer: !process.env.CI,
      stdout: 'pipe',
      stderr: 'pipe',
    },
    {
      // --host 127.0.0.1 : force le bind IPv4 explicite, cohérent avec le poll
      // de `url` ci-dessous. Sinon Vite bind `localhost` qui peut résoudre vers
      // ::1 (IPv6) sur les runners CI → ECONNREFUSED sur 127.0.0.1 → timeout
      // webServer flaky (même piège que LATCH_BINDING pour le backend).
      command: `pnpm dev --port ${VITE_PORT} --strictPort --host 127.0.0.1`,
      url: `http://127.0.0.1:${VITE_PORT}/admin`,
      timeout: 120_000,
      reuseExistingServer: !process.env.CI,
      stdout: 'pipe',
      stderr: 'pipe',
    },
  ],
})
