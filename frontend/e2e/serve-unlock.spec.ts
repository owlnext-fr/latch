import { test, expect, type APIRequestContext } from '@playwright/test'
import path from 'node:path'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const protoV1 = readFileSync(path.resolve(__dirname, 'fixtures/proto.html'), 'utf8')
const protoV2 = readFileSync(path.resolve(__dirname, 'fixtures/proto-v2.html'), 'utf8')

// Connexion admin via l'API (le cookie de session reste dans le contexte `request`).
async function apiLogin(request: APIRequestContext) {
  const res = await request.post('/api/login', { data: { user: 'admin', pass: 'secret' } })
  expect(res.ok()).toBeTruthy()
}

// Crée un projet via l'API. `Origin` requis (garde same-origin sur les mutations).
async function createProject(
  request: APIRequestContext,
  baseURL: string,
  opts: { name: string; code_enabled: boolean; pin?: string },
) {
  const res = await request.post('/api/projects', {
    headers: { Origin: baseURL },
    data: opts,
  })
  expect(res.ok()).toBeTruthy()
  return res.json() as Promise<{ id: number; slug: string; pin: string | null }>
}

async function deploy(
  request: APIRequestContext,
  baseURL: string,
  id: number,
  html: string,
  activate = true,
  notes?: string,
) {
  const res = await request.post(`/api/projects/${id}/deploy`, {
    headers: { Origin: baseURL },
    data: { html, activate, ...(notes !== undefined ? { notes } : {}) },
  })
  expect(res.ok()).toBeTruthy()
  return res.json() as Promise<{ id: number; n: number }>
}

test('projet libre : /c sert le proto en no-store', async ({ request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, { name: 'ACME', code_enabled: false })
  await deploy(request, baseURL!, project.id, protoV1)

  // Le HTML brut du proto est maintenant sur /raw (l'iframe du shell le charge).
  const res = await request.get(`/c/${project.slug}/raw`)
  expect(res.status()).toBe(200)
  expect(res.headers()['cache-control']).toContain('no-store')
  expect(res.headers()['content-security-policy']).toContain("frame-ancestors 'self'")
  expect(await res.text()).toContain('Demo proto')
})

test('projet protégé : unlock par PIN puis proto servi', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, {
    name: 'Mon Projet',
    code_enabled: true,
    pin: '135790', // PIN explicite → déterministe pour la saisie
  })
  await deploy(request, baseURL!, project.id, protoV1)

  // 1) Sans cookie → page d'unlock (l'input OTP #pin n'existe QUE sur l'unlock).
  await page.goto(`/c/${project.slug}`)
  await expect(page.locator('#pin')).toBeVisible()
  await expect(page.getByText('Demo proto')).toHaveCount(0)

  // 2) Mauvais PIN → reste sur l'unlock, proto non servi.
  //    Synchro sur la réponse /unlock (anti-flaky : l'URL ne change pas).
  await page.locator('#pin').click()
  const wrongResp = page.waitForResponse((r) => r.url().includes('/unlock'))
  await page.locator('#pin').pressSequentially('000000')
  await wrongResp
  await expect(page.locator('#pin')).toBeVisible()
  await expect(page.getByText('Demo proto')).toHaveCount(0)

  // 3) Bon PIN → auto-submit (onComplete) → cookie posé → reload → proto servi dans l'iframe.
  //    Synchro sur la réponse /unlock avant d'asserter le proto.
  await page.reload()
  await page.locator('#pin').click()
  const unlockResp = page.waitForResponse((r) => r.url().includes('/unlock'))
  await page.locator('#pin').pressSequentially('135790')
  await unlockResp
  // Le proto est chargé dans l'iframe /c/${slug}/raw par le shell.
  await expect(page.frameLocator('iframe').getByText('Demo proto')).toBeVisible()
})

test('bascule de version : /c reflète la v2 activée', async ({ request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, { name: 'demo', code_enabled: false })
  await deploy(request, baseURL!, project.id, protoV1) // v1 active

  // Le contenu du proto se vérifie sur /raw (l'iframe du shell).
  let res = await request.get(`/c/${project.slug}/raw`)
  expect(await res.text()).toContain('Demo proto')

  const v2 = await deploy(request, baseURL!, project.id, protoV2) // v2 active
  expect(v2.n).toBe(2)

  res = await request.get(`/c/${project.slug}/raw`)
  const body = await res.text()
  expect(body).toContain('PROTO-V2')
  expect(body).not.toContain('Demo proto')
})

test('overlay de notes : visible puis mémorisé après dismiss', async ({
  page,
  request,
  baseURL,
}) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, {
    name: 'Mon Projet',
    code_enabled: false,
  })

  // Déploie avec des notes Markdown.
  const notesContent = '# Nouveautés\n\n- point A\n- point B'
  await deploy(request, baseURL!, project.id, protoV1, true, notesContent)

  // Visite le shell — l'overlay de notes doit apparaître.
  await page.goto(`/c/${project.slug}`)

  // Synchro : attend la réponse /notes avant d'asserter le DOM.
  await page.waitForResponse((r) => r.url().includes('/notes') && r.status() === 200)

  // L'overlay doit être visible avec le bouton dismiss.
  await expect(page.getByTestId('notes-dismiss')).toBeVisible({ timeout: 8000 })

  // Le contenu Markdown rendu doit contenir les éléments des notes.
  await expect(page.getByText('Nouveautés')).toBeVisible()
  await expect(page.getByText('point A')).toBeVisible()

  // Le proto reste accessible dans l'iframe.
  await expect(page.frameLocator('iframe').getByText('Demo proto')).toBeVisible()

  // Dismiss → l'overlay disparaît.
  await page.getByTestId('notes-dismiss').click()
  await expect(page.getByTestId('notes-dismiss')).toHaveCount(0)

  // Reload → l'overlay NE réapparaît PAS (mémorisé dans localStorage `latch:seen:<slug>`).
  // Synchro déterministe : on attend que l'iframe charge /raw (preuve que le shell est
  // monté et que localStorage a été consulté), puis on asserte l'absence de l'overlay.
  await page.reload()
  await page.waitForResponse((r) => r.url().includes('/raw') && r.status() === 200)
  await expect(page.frameLocator('iframe').getByText('Demo proto')).toBeVisible()
  await expect(page.getByTestId('notes-dismiss')).toHaveCount(0)
})
