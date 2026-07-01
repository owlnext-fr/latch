import { test, expect, type APIRequestContext } from '@playwright/test'
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

// --- Captures des commentaires (feature commentaires, issue #4) ---------------
//
// Proto de démo un peu plus « habillé » que celui des specs, pour des captures
// présentables. `#cta` reste présent et unique → ancrage stable ("anchored").
// Données 100% fictives (placeholders génériques) — jamais de nom client.
const commentProto = `<!doctype html><html lang="fr"><head><meta charset="utf-8">
<style>
  body{margin:0;font-family:system-ui,-apple-system,sans-serif;background:#f5f5f4;color:#1c1917}
  .wrap{max-width:640px;margin:0 auto;padding:72px 32px}
  .hero{background:#fff;border:1px solid #e7e5e4;border-radius:16px;padding:44px;box-shadow:0 1px 3px rgba(0,0,0,.06)}
  h1{font-size:30px;margin:0 0 12px}
  p{font-size:16px;line-height:1.65;color:#57534e;margin:0 0 28px}
  #cta{padding:14px 24px;font-size:16px;border:0;border-radius:8px;background:#1c1917;color:#fff;cursor:pointer}
</style></head><body><div class="wrap"><div class="hero">
  <h1>Mon Projet</h1>
  <p>Prototype de démonstration servi par latch. Épinglez un commentaire sur n'importe quel élément de la page.</p>
  <button id="cta">En savoir plus</button>
</div></div></body></html>`

// Crée un projet avec commentaires activés (accès libre pour une capture visiteur directe).
async function createCommentProject(
  request: APIRequestContext,
  baseURL: string,
  name: string,
) {
  const res = await request.post('/api/projects', {
    headers: { Origin: baseURL },
    data: { name, code_enabled: false, comments_enabled: true },
  })
  const project = (await res.json()) as { id: number; slug: string }
  const dep = await request.post(`/api/projects/${project.id}/deploy`, {
    headers: { Origin: baseURL },
    data: { html: commentProto, activate: true },
  })
  const version = (await dep.json()) as { n: number }
  return { id: project.id, slug: project.slug, n: version.n }
}

// Poste un commentaire visiteur via l'API publique (identité côté contexte `request`).
async function seedVisitorComment(
  request: APIRequestContext,
  baseURL: string,
  slug: string,
  author: string,
  body: string,
) {
  const anchor = JSON.stringify({
    v: 1,
    selector: '#cta',
    fingerprint: { tag: 'button', text: 'En savoir plus', role: 'button', ordinal: 0 },
    textQuote: { exact: 'En savoir plus', prefix: '', suffix: '' },
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0.1, y: 0.1 },
  })
  const res = await request.post(`/c/${slug}/comments`, {
    headers: { 'X-Comment-Client': '1', Origin: baseURL },
    data: { anchor, author_name: author, body },
  })
  return res.json() as Promise<{ pin: number }>
}

// Ouvre le mode commentaire côté visiteur et clique #cta pour ancrer un commentaire.
async function enterCommentModeOnCta(page: import('@playwright/test').Page) {
  await page.getByRole('button', { name: /^(Comment|Commenter)$/ }).click()
  const ctaBox = await page
    .frameLocator('iframe[title="prototype"]')
    .locator('#cta')
    .boundingBox()
  const cx = ctaBox!.x + ctaBox!.width / 2
  const cy = ctaBox!.y + ctaBox!.height / 2
  await page.mouse.move(cx, cy)
  await page.mouse.click(cx, cy)
}

// (1) Surface visiteur : barre d'action flottante + popup de composition ancré.
test('capture commentaire visiteur — composition', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createCommentProject(request, baseURL!, 'Mon Projet')

  await page.goto(`/c/${project.slug}`)
  await expect(page.getByTestId('comments-mount')).toBeAttached()
  await enterCommentModeOnCta(page)

  // Le popup de composition est ouvert : on pré-remplit pour une capture parlante.
  await page.getByLabel(/Your name|Votre nom/).fill('Léa')
  await page.getByLabel(/^(Comment|Commentaire)$/).fill('Ce bouton gagnerait à être plus visible sur mobile.')
  await page.screenshot({ path: `${assetsDir}/comments-visitor-compose.png` })
})

// (2) Surface visiteur : fil ouvert avec une réponse admin portant le badge « Admin ».
test('capture commentaire visiteur — fil avec réponse admin', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createCommentProject(request, baseURL!, 'Mon Projet')

  // Le visiteur poste via l'UI → le cookie d'identité `latch_comment` vit dans le contexte page.
  await page.goto(`/c/${project.slug}`)
  await expect(page.getByTestId('comments-mount')).toBeAttached()
  await enterCommentModeOnCta(page)
  await page.getByLabel(/Your name|Votre nom/).fill('Léa')
  await page.getByLabel(/^(Comment|Commentaire)$/).fill('Ce bouton gagnerait à être plus visible sur mobile.')
  const posted = page.waitForResponse(
    (r) => r.url().includes(`/c/${project.slug}/comments`) && r.request().method() === 'POST',
  )
  await page.getByRole('button', { name: /^(Post|Publier)$/ }).click()
  await posted

  // L'admin (contexte `request`, session déjà ouverte) répond au fil du visiteur via l'API.
  const list = await request.get(`/api/projects/${project.id}/versions/${project.n}/comments`)
  const pins = (await list.json()) as { pins: { id: number }[] }
  const pinId = pins.pins[0].id
  await request.post(`/api/projects/${project.id}/comments/pins/${pinId}/replies`, {
    headers: { Origin: baseURL! },
    data: { body: 'Bien vu, on passe le bouton en pleine largeur sur petit écran.' },
  })

  // Le visiteur recharge et rouvre son fil : la réponse admin y apparaît avec le badge.
  await page.reload()
  await expect(page.locator('[data-status="anchored"]').first()).toBeVisible()
  await page.getByRole('button', { name: /My comments|Mes commentaires/ }).click()
  await expect(page.getByTestId('comments-drawer')).toBeVisible()
  await page.getByTestId('drawer-row').first().click()
  await expect(page.getByText('Admin').first()).toBeVisible()
  await page.screenshot({ path: `${assetsDir}/comments-thread-admin-reply.png` })
})

// (3) Surface admin : page Review avec les pins positionnés sur le prototype.
test('capture page Review admin — pins', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createCommentProject(request, baseURL!, 'Mon Projet')
  await seedVisitorComment(request, baseURL!, project.slug, 'Léa', 'Ce bouton gagnerait à être plus visible sur mobile.')

  // Session navigateur admin (indépendante du contexte `request`).
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()
  await expect(page.getByText('+ New project')).toBeVisible()

  const loaded = page.waitForResponse(
    (r) =>
      r.url().includes(`/api/projects/${project.id}/versions/${project.n}/comments`) &&
      r.status() === 200,
  )
  await page.goto(`/admin/projects/${project.id}/versions/${project.n}/review`)
  await expect(
    page.frameLocator('iframe[title="Prototype preview"]').locator('#cta'),
  ).toBeVisible()
  await loaded
  await expect(page.locator('[data-testid="pin-badge"]').first()).toBeVisible()
  await page.screenshot({ path: `${assetsDir}/comments-review-page.png`, fullPage: true })
})

// (4) Surface admin : panel « Comments » par version (liste des fils) sur la page détail.
test('capture panel Comments par version', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createCommentProject(request, baseURL!, 'Mon Projet')
  await seedVisitorComment(request, baseURL!, project.slug, 'Léa', 'Ce bouton gagnerait à être plus visible sur mobile.')

  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()
  await expect(page.getByText('+ New project')).toBeVisible()

  await page.goto(`/admin/projects/${project.id}`)
  // Le bouton « Comments » de la ligne de version n'est actif que si comment_count > 0.
  const commentsBtn = page.getByRole('button', { name: 'View comments for this version' })
  await expect(commentsBtn).toBeEnabled()
  await commentsBtn.click()

  // Le panel est un Sheet Radix (overlay fixed) : on capture l'élément `dialog`
  // lui-même (crop propre) après la fin de l'animation d'ouverture.
  const sheet = page.getByRole('dialog')
  await expect(sheet.getByText(/Comments — v/)).toBeVisible()
  await expect(sheet.getByText(/Ce bouton gagnerait/)).toBeVisible()
  await page.waitForTimeout(500)
  await sheet.screenshot({ path: `${assetsDir}/comments-version-panel.png` })
})

// (5) Surface admin : toggle `comments_enabled` du formulaire projet.
test('capture toggle commentaires (form projet)', async ({ page, request }) => {
  await apiLogin(request)
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()
  await expect(page.getByText('+ New project')).toBeVisible()

  await page.getByRole('button', { name: '+ New project' }).click()

  // Formulaire en Sheet Radix : capture de l'élément `dialog` après animation.
  const sheet = page.getByRole('dialog')
  await expect(sheet.getByRole('heading', { name: 'New project' })).toBeVisible()
  await expect(sheet.locator('#project-comments')).toBeVisible()
  await page.waitForTimeout(500)
  await sheet.screenshot({ path: `${assetsDir}/project-comments-toggle.png` })
})
