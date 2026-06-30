import { test, expect, type APIRequestContext } from '@playwright/test'

const PROTO_HTML =
  '<!doctype html><html><body style="margin:0"><div style="padding:60px"><button id="cta" style="padding:14px 22px;font-size:16px">En savoir plus</button></div></body></html>'

// Connexion admin via l'API (le cookie de session reste dans le contexte `request`).
async function apiLogin(request: APIRequestContext) {
  const res = await request.post('/api/login', { data: { user: 'admin', pass: 'secret' } })
  expect(res.ok()).toBeTruthy()
}

// Crée un projet via l'API. `Origin` requis (garde same-origin sur les mutations).
async function createProject(
  request: APIRequestContext,
  baseURL: string,
  opts: { name: string; code_enabled: boolean; pin?: string; comments_enabled?: boolean },
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

test('visiteur : cibler un élément, écrire et persister un commentaire', async ({
  page,
  request,
  baseURL,
}) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, {
    name: 'ACME',
    code_enabled: false,
    comments_enabled: true,
  })
  await deploy(request, baseURL!, project.id, PROTO_HTML)

  await page.goto('/c/' + project.slug)

  // La couche comments doit être attachée (lazy module chargé).
  // Le div comments-mount a des enfants positionnés en absolute/fixed → taille nulle.
  // On vérifie l'attache ET que le bouton de l'ActionBar est visible (module opérationnel).
  await expect(page.getByTestId('comments-mount')).toBeAttached()
  const commentBtn = page.getByRole('button', { name: /^(Comment|Commenter)$/ })
  await expect(commentBtn).toBeVisible()

  // Entrer en mode sélection d'élément.
  await commentBtn.click()

  // Cibler le bouton #cta dans l'iframe.
  // Le shell rend le proto dans une iframe avec title="prototype" (src `/c/{slug}/raw`).
  // frameLocator.boundingBox() retourne les coordonnées en page-space.
  const ctaBox = await page
    .frameLocator('iframe[title="prototype"]')
    .locator('#cta')
    .boundingBox()
  expect(ctaBox).not.toBeNull()
  const cx = ctaBox!.x + ctaBox!.width / 2
  const cy = ctaBox!.y + ctaBox!.height / 2

  // La pick-surface overlay intercepte le clic et le délègue au picker same-origin.
  await page.mouse.move(cx, cy)
  await page.mouse.click(cx, cy)

  // Remplir le formulaire de composition.
  await page.getByLabel(/Your name|Votre nom/).fill('Léa')
  await page.getByLabel(/^(Comment|Commentaire)$/).fill('À revoir')

  // Soumettre et attendre la réponse POST du serveur (anti-flaky).
  const postResponse = page.waitForResponse(
    (r) =>
      r.url().includes(`/c/${project.slug}/comments`) && r.request().method() === 'POST',
  )
  await page.getByRole('button', { name: /^(Post|Publier)$/ }).click()
  await postResponse

  // Un badge de pin ancré doit apparaître sur l'overlay.
  await expect(page.locator('[data-status="anchored"]').first()).toBeVisible()

  // Reload : le cookie d'identité visiteur + GET /comments reconstruit les pins.
  // Le proto HTML est identique → le pin doit se ré-ancrer sur `anchored` (pas `approximate` ni `orphaned`).
  await page.reload()
  await expect(page.locator('[data-status="anchored"]').first()).toBeVisible()
})
