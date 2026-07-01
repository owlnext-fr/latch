/**
 * Smoke Vite dev-server (:5173) — couvre deux angles morts de la suite e2e principale
 * (qui ne passe que par le build :5150 et ne voit jamais Vite).
 *
 * Bug 1 — CSRF (commit 550560c) : en dev, le proxy Vite forwarde Origin: :5173
 *   mais le backend attend Host et Origin sur le même port (:5150). Sans la correction
 *   `changeOrigin + setHeader origin`, toute mutation admin renvoie 403.
 *
 * Bug 2 — MIME (commit 550560c) : le shell visiteur (/c/<slug>) référence
 *   /assets/unlock-*.js. Sans le proxy `/assets` → :5150, Vite renvoie son
 *   index.html de fallback (text/html) → le navigateur rejette le module JS
 *   (MIME mismatch) → page blanche.
 *
 * Ce test pilote un vrai navigateur CONTRE Vite :5173 :
 *   - login + création projet via le formulaire admin → traversent le proxy → CSRF
 *   - déploiement via page.request (même session cookie, même proxy) → CSRF
 *   - page visiteur /c/<slug> → charge unlock-*.js via /assets proxy → MIME
 *
 * Note : page.request partage le contexte navigateur (cookies inclus) et résout les
 * URL relatives par rapport à baseURL (:5173) → les appels API traversent le proxy
 * Vite, contrairement au contexte `request` du test (APIRequestContext isolé).
 */

import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from '@playwright/test'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const PROTO_HTML = readFileSync(path.resolve(__dirname, '../e2e/fixtures/proto.html'), 'utf8')
const VITE_ORIGIN = 'http://127.0.0.1:5173'

test('vite-smoke: admin via proxy Vite (CSRF) + assets visiteur (MIME)', async ({ page }) => {
  // Collecte des erreurs MIME avant toute navigation
  const mimeErrors: string[] = []
  page.on('console', (msg) => {
    const text = msg.text()
    if (
      msg.type() === 'error' &&
      (text.includes('MIME') ||
        text.includes('module script') ||
        text.includes('text/html') ||
        text.includes('Failed to load module'))
    ) {
      mimeErrors.push(text)
    }
  })
  page.on('pageerror', (err) => {
    const m = err.message
    if (m.includes('MIME') || m.includes('module script') || m.includes('text/html')) {
      mimeErrors.push(m)
    }
  })

  // ── 1. Login via formulaire UI ──────────────────────────────────────────────
  // Le formulaire POST /api/login transite par Vite → proxy → backend.
  // Vite réécrit Origin: :5173 → :5150 (via changeOrigin + proxyReq.setHeader).
  // Sans ce fix : 403 dès le login.
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()

  const newProjectBtn = page.getByRole('button', { name: /new project/i })
  await expect(newProjectBtn).toBeVisible()

  // ── 2. Création de projet via formulaire (exercice de la garde CSRF) ────────
  // POST /api/projects depuis l'origine :5173 → Vite proxie vers :5150 en
  // réécrivant l'Origin. Sans le fix : le backend voit Origin :5173 ≠ Host :5150
  // → 403 → la mutation échoue silencieusement → "Smoke Vite" n'apparaît jamais.
  //
  // On intercepte la réponse avant le clic pour en extraire id + slug directement.
  const postResponsePromise = page.waitForResponse(
    (r) => r.url().includes('/api/projects') && r.request().method() === 'POST',
  )

  await newProjectBtn.click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toBeVisible()
  await dialog.getByLabel('Name', { exact: true }).fill('Smoke Vite')
  // Laisser code ON (code_enabled: true par défaut du formulaire) pour que la page
  // visiteur serve l'unlock form (nécessaire pour l'assertion #pin ci-dessous).
  await dialog.getByRole('button', { name: 'Save' }).click()

  // Attend la réponse (CSRF assertion : si le proxy cassait l'Origin, le backend
  // retournerait 403 et le toast "Project created." n'apparaîtrait jamais).
  const postResponse = await postResponsePromise
  expect(
    postResponse.ok(),
    `POST /api/projects via Vite proxy a renvoyé ${postResponse.status()} — la garde CSRF a peut-être tiré`,
  ).toBeTruthy()

  // Toast = signal que la mutation a réussi côté backend (unique à ce run)
  await expect(page.getByText('Project created.')).toBeVisible()
  // Le projet apparaît dans la liste (.first() : en dev réutilisé, des runs
  // précédents peuvent avoir laissé des projets homonymes dans le backend partagé)
  await expect(page.getByText('Smoke Vite').first()).toBeVisible()
  const project = (await postResponse.json()) as { id: number; slug: string }

  // ── 3. Déploiement d'une version via page.request ───────────────────────────
  // page.request partage les cookies de session posés par pageLogin ci-dessus et
  // résout les URL relatives par rapport à baseURL (:5173) → la requête traverse
  // le proxy Vite (exercice CSRF supplémentaire : même garde, même fix).
  // Sans version active, /c/<slug> renvoie l'error.html → #pin absent.
  const deployRes = await page.request.post(`/api/projects/${project.id}/deploy`, {
    headers: { Origin: VITE_ORIGIN },
    data: { html: PROTO_HTML, activate: true },
  })
  expect(
    deployRes.ok(),
    `POST /api/projects/${project.id}/deploy via Vite proxy a renvoyé ${deployRes.status()}`,
  ).toBeTruthy()

  // ── 4. Page visiteur charge ses assets JS via le proxy /assets ──────────────
  // /c/<slug> → backend sert shell.html (depuis dist/) qui référence
  // /assets/unlock-*.js. Le navigateur charge /assets/unlock-*.js depuis :5173.
  //   • Sans proxy /assets : Vite renvoie text/html (SPA fallback) → module rejeté
  //     → #pin jamais rendu → assertion ci-dessous échoue.
  //   • Avec proxy /assets → :5150 : backend renvoie le JS réel (application/javascript)
  //     → module chargé → unlock-page montée → #pin visible.
  await page.goto(`/c/${project.slug}`)

  // #pin = l'input OTP de la page de déverrouillage — il n'existe que si unlock-*.js
  // a été chargé avec le bon Content-Type (application/javascript, pas text/html).
  await expect(page.locator('#pin')).toBeVisible()

  // Aucune erreur MIME ne doit avoir été captée
  expect(mimeErrors, `Erreurs MIME capturées : ${JSON.stringify(mimeErrors)}`).toHaveLength(0)
})
