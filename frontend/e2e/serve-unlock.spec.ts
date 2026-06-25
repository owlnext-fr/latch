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
) {
  const res = await request.post(`/api/projects/${id}/deploy`, {
    headers: { Origin: baseURL },
    data: { html, activate },
  })
  expect(res.ok()).toBeTruthy()
  return res.json() as Promise<{ id: number; n: number }>
}

test('projet libre : /c sert le proto en no-store', async ({ request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, { name: 'ACME', code_enabled: false })
  await deploy(request, baseURL!, project.id, protoV1)

  const res = await request.get(`/c/${project.slug}`)
  expect(res.status()).toBe(200)
  expect(res.headers()['cache-control']).toContain('no-store')
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
  await page.locator('#pin').click()
  await page.locator('#pin').pressSequentially('000000')
  await expect(page.locator('#pin')).toBeVisible()
  await expect(page.getByText('Demo proto')).toHaveCount(0)

  // 3) Bon PIN → auto-submit (onComplete) → cookie posé → reload → proto servi.
  await page.reload()
  await page.locator('#pin').click()
  await page.locator('#pin').pressSequentially('135790')
  await expect(page.getByText('Demo proto')).toBeVisible()
})

test('bascule de version : /c reflète la v2 activée', async ({ request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, { name: 'demo', code_enabled: false })
  await deploy(request, baseURL!, project.id, protoV1) // v1 active

  let res = await request.get(`/c/${project.slug}`)
  expect(await res.text()).toContain('Demo proto')

  const v2 = await deploy(request, baseURL!, project.id, protoV2) // v2 active
  expect(v2.n).toBe(2)

  res = await request.get(`/c/${project.slug}`)
  const body = await res.text()
  expect(body).toContain('PROTO-V2')
  expect(body).not.toContain('Demo proto')
})
