import { defineConfig } from '@playwright/test'

const PORT = 5150
const DB = '/tmp/latch-e2e.sqlite'

// Le backend Loco sert le dist/ React buildé sous /admin (même mécanisme qu'en prod).
// webServer : build le front puis lance le backend sur une DB e2e fraîche (auto_migrate au boot).
// LOCO_ENV non défini → development → cookie session non-Secure (OK en http localhost).
export default defineConfig({
  testDir: './e2e',
  // Découverte étendue : *.spec.ts (specs CI) + *.capture.ts (captures manuelles, skip sauf CAPTURE=1).
  testMatch: /.*\.(spec|capture)\.ts$/,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    // Origine seule — les specs naviguent vers /admin/... (basepath du routeur).
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: 'on-first-retry',
  },
  webServer: {
    // LATCH_BINDING=127.0.0.1 : force le bind IPv4 explicite, cohérent avec le poll
    // de `url` ci-dessous. Sinon `binding: localhost` peut résoudre vers ::1 (IPv6)
    // sur les runners CI → ECONNREFUSED sur 127.0.0.1 → timeout webServer flaky.
    command: `pnpm build && rm -f ${DB} && cd ../backend && LATCH_BINDING=127.0.0.1 LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret LATCH_STORAGE_ROOT=/tmp/latch-e2e-data LATCH_LOGIN_RL_BURST=100000 DATABASE_URL='sqlite://${DB}?mode=rwc' cargo loco start`,
    url: `http://127.0.0.1:${PORT}/_health`,
    timeout: 180_000,
    reuseExistingServer: !process.env.CI,
    stdout: 'pipe',
    stderr: 'pipe',
  },
})
