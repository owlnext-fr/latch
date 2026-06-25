import { test, expect } from '@playwright/test'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const PROTO = path.resolve(__dirname, 'fixtures/proto.html')

// Smoke admin contre la stack réelle (backend Loco sert le dist/ React sous /admin).
// Parcours : login → créer un projet → ouvrir le détail → déployer une version.
test('admin smoke: login → create project → deploy version', async ({ page }) => {
  // 1. Login
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()

  // Landed on the list (the "+ New project" action is visible).
  const newProjectBtn = page.getByRole('button', { name: /new project/i })
  await expect(newProjectBtn).toBeVisible()

  // 2. Create a project (code enabled by default, PIN auto-generated)
  await newProjectBtn.click()
  const createSheet = page.getByRole('dialog')
  await expect(createSheet).toBeVisible()
  await createSheet.getByLabel('Name', { exact: true }).fill('Mon Projet')
  await createSheet.getByRole('button', { name: 'Save' }).click()

  // Toast confirms creation + the row appears in the table.
  await expect(page.getByText('Project created.')).toBeVisible()
  const row = page.getByText('Mon Projet', { exact: true })
  await expect(row).toBeVisible()

  // 3. Open detail → deploy a version
  await row.click()
  // Detail page: the "Deploy" action opens the DeployPanel.
  await page.getByRole('button', { name: 'Deploy', exact: true }).first().click()
  const deploySheet = page.getByRole('dialog')
  await expect(deploySheet).toBeVisible()

  // Upload the prototype HTML into the hidden file input, activate immediately, deploy.
  await page.setInputFiles('input[type="file"]', PROTO)
  await deploySheet.getByRole('checkbox').check()
  await deploySheet.getByRole('button', { name: 'Deploy', exact: true }).click()

  // Toast confirms deployment + a version row appears.
  await expect(page.getByText('Version deployed.')).toBeVisible()
})
