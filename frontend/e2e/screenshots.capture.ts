import { test, type APIRequestContext } from '@playwright/test'
import path from 'node:path'
import { readFileSync, mkdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Skippé par défaut : ne tourne que lancé explicitement avec CAPTURE=1.
test.skip(!process.env.CAPTURE, 'capture manuelle uniquement (CAPTURE=1)')

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const assetsDir = path.resolve(__dirname, '../../docs/assets')
const protoV1 = readFileSync(path.resolve(__dirname, 'fixtures/proto.html'), 'utf8')
mkdirSync(assetsDir, { recursive: true })

async function apiLogin(request: APIRequestContext) {
  await request.post('/api/login', { data: { user: 'admin', pass: 'secret' } })
}
async function createDeployed(
  request: APIRequestContext,
  baseURL: string,
  name: string,
  code_enabled: boolean,
  pin?: string,
) {
  const res = await request.post('/api/projects', {
    headers: { Origin: baseURL },
    data: { name, code_enabled, pin },
  })
  const project = await res.json()
  await request.post(`/api/projects/${project.id}/deploy`, {
    headers: { Origin: baseURL },
    data: { html: protoV1, activate: true },
  })
  return project as { id: number; slug: string }
}

test('capture liste admin', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  // 2 projets de démo FACTICES : "Mon Projet" (protégé) + "ACME" (libre).
  await createDeployed(request, baseURL!, 'Mon Projet', true, '135790')
  await createDeployed(request, baseURL!, 'ACME', false)
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()
  await page.waitForURL(/\/admin\/?$/)
  await page.screenshot({ path: `${assetsDir}/admin-list.png`, fullPage: true })
})

test('capture page unlock', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createDeployed(request, baseURL!, 'Mon Projet', true, '135790')
  await page.goto(`/c/${project.slug}`)
  await page.locator('#pin').waitFor()
  await page.screenshot({ path: `${assetsDir}/unlock.png`, fullPage: true })
})
