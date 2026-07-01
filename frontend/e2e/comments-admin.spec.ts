import { test, expect, type APIRequestContext, type Page } from '@playwright/test'

/**
 * Seed strategy : Option B (API directe) — POST /c/{slug}/comments avec un
 * AnchorDescriptor valide pointant vers #cta (présent dans PROTO_HTML).
 *
 * Pourquoi B plutôt que A (UI visiteur) :
 *  - Plus déterministe : pas de mouseMove ni de clic dans l'iframe.
 *  - Le sélecteur #cta est stable et unique dans le proto → le pin se résout
 *    en "anchored" dans la page Review (même DOM, même origin).
 *  - Si l'ancrage échouait (e.g. proto vide), le pin passerait en "orphaned" avec
 *    un fallbackPoint, rendrait quand même sa pastille et le test passerait.
 *
 * Login : les tests qui naviguent dans le SPA admin (page object) utilisent
 * `pageLogin(page)` (formulaire /admin/login) car le contexte `request` et le
 * contexte navigateur ne partagent pas la session axum_session dans ce setup e2e.
 * Les helpers de création de ressources utilisent toujours `apiLogin(request)`.
 */

const PROTO_HTML =
  '<!doctype html><html><body style="margin:0"><div style="padding:60px"><button id="cta" style="padding:14px 22px;font-size:16px">En savoir plus</button></div></body></html>'

// --- Helpers (repris de serve-unlock.spec.ts / comments.spec.ts) --------------

// Le webServer e2e pose LATCH_LOGIN_RL_BURST=100000 → jamais de 429 en tests.
async function apiLogin(request: APIRequestContext): Promise<void> {
  const res = await request.post('/api/login', { data: { user: 'admin', pass: 'secret' } })
  expect(res.ok()).toBeTruthy()
}

/** Login via le formulaire de la SPA admin (pour les tests qui naviguent dans le browser). */
async function pageLogin(page: Page): Promise<void> {
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()
  // Attendre que la page de liste se charge (preuve que l'auth a réussi)
  await expect(page.getByText('+ New project')).toBeVisible()
}

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
) {
  const res = await request.post(`/api/projects/${id}/deploy`, {
    headers: { Origin: baseURL },
    data: { html, activate },
  })
  expect(res.ok()).toBeTruthy()
  return res.json() as Promise<{ id: number; n: number }>
}

/**
 * Seed un commentaire via l'API publique visiteur (Option B).
 * L'ancre cible le bouton #cta qui est physiquement présent dans PROTO_HTML,
 * ce qui garantit un statut "anchored" dans la page Review.
 */
async function seedComment(
  request: APIRequestContext,
  baseURL: string,
  slug: string,
): Promise<void> {
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
    data: { anchor, author_name: 'Léa', body: 'À revoir ce bouton' },
  })
  expect(res.ok()).toBeTruthy()
}

/**
 * Seed complet réutilisable : projet + version déployée + un commentaire
 * visiteur ancré sur #cta (via apiLogin(request) + createProject + deploy +
 * seedComment). Extrait des tests de modération pour éviter la duplication
 * (DRY) — chaque test appelant ce helper doit avoir sa propre session
 * `request` déjà authentifiée (apiLogin) car createProject/deploy en ont besoin.
 */
async function seedProjectWithVisitorComment(
  request: APIRequestContext,
  baseURL: string,
): Promise<{ id: number; slug: string; n: number }> {
  const project = await createProject(request, baseURL, {
    name: 'ACME',
    code_enabled: false,
    comments_enabled: true,
  })
  const version = await deploy(request, baseURL, project.id, PROTO_HTML)
  await seedComment(request, baseURL, project.slug)
  return { id: project.id, slug: project.slug, n: version.n }
}

// --- Tests -------------------------------------------------------------------

test('admin : page Review affiche le pin, la modération depuis le fil le supprime', async ({
  page,
  request,
  baseURL,
}) => {
  // 1. Setup : projet + version + commentaire seedé via API (session admin request)
  await apiLogin(request)
  const { id: projectId, n: versionN } = await seedProjectWithVisitorComment(request, baseURL!)

  // Login admin via le formulaire de la SPA (session browser indépendante de request)
  await pageLogin(page)

  // 2. Préparer la surveillance de la réponse GET commentaires admin
  //    (à poser AVANT la navigation pour ne pas manquer la requête).
  const commentsLoaded = page.waitForResponse(
    (r) =>
      r.url().includes(`/api/projects/${projectId}/versions/${versionN}/comments`) &&
      r.status() === 200,
    { timeout: 15_000 },
  )

  // 3. Naviguer sur la page Review admin
  await page.goto(`/admin/projects/${projectId}/versions/${versionN}/review`)

  // 4. Attendre que l'iframe charge le proto (preuve que le picker a un DOM à traverser)
  await expect(
    page.frameLocator('iframe[title="Prototype preview"]').locator('#cta'),
  ).toBeVisible({ timeout: 15_000 })

  // 5. Attendre la réponse GET commentaires (CommentsApp a monté et chargé la liste)
  await commentsLoaded

  // 6. La pastille du pin doit apparaître sur l'overlay (pinsVisible = true par défaut)
  const pinBadge = page.locator('[data-testid="pin-badge"]').first()
  await expect(pinBadge).toBeVisible({ timeout: 10_000 })

  // Régression : le pin doit s'aligner verticalement sur #cta (offset 0.5,0.5 → centre),
  // et NON être décalé vers le bas de la hauteur de la topbar (bug corrigé par l'overlay fixed).
  const ctaBox = await page
    .frameLocator('iframe[title="Prototype preview"]')
    .locator('#cta')
    .boundingBox()
  const pinBox = await pinBadge.boundingBox()
  const ctaCenterY = ctaBox!.y + ctaBox!.height / 2
  const pinCenterY = pinBox!.y + pinBox!.height / 2
  expect(Math.abs(pinCenterY - ctaCenterY)).toBeLessThan(20)

  // 7. Ouvrir le fil en cliquant sur la pastille
  await pinBadge.click()

  // 8. Le ThreadPopup s'ouvre avec un bouton de suppression (canModerate = true).
  //    Les clés comment.* sont maintenant fusionnées dans l'admin via mergeFragmentGlob →
  //    le bouton affiche le texte traduit "Delete" (EN), jamais la clé brute.
  const deleteBtn = page.getByRole('button', { name: 'Delete' })
  await expect(deleteBtn).toBeVisible()

  // 9. Modérer : poser l'écouteur DELETE avant le clic
  const deleteResponse = page.waitForResponse(
    (r) =>
      r.url().includes('/comments/messages/') && r.request().method() === 'DELETE',
    { timeout: 10_000 },
  )
  await deleteBtn.click()
  await deleteResponse

  // 10. Après suppression du dernier message, le pin est soft-deleté côté backend.
  //     La liste est refetchée → 0 pins → 0 pastilles + popup fermée.
  await expect(page.locator('[data-testid="pin-badge"]')).toHaveCount(0, { timeout: 10_000 })
})

test('admin répond à un fil visiteur depuis la Review', async ({ page, request, baseURL }) => {
  // 1. Setup déterministe : projet + version + commentaire visiteur seedé via API.
  await apiLogin(request)
  const { id: projectId, n: versionN } = await seedProjectWithVisitorComment(request, baseURL!)

  // 2. Login admin via le formulaire de la SPA (session browser indépendante de request)
  await pageLogin(page)

  const commentsLoaded = page.waitForResponse(
    (r) =>
      r.url().includes(`/api/projects/${projectId}/versions/${versionN}/comments`) &&
      r.status() === 200,
    { timeout: 15_000 },
  )

  // 3. Naviguer sur la page Review admin
  await page.goto(`/admin/projects/${projectId}/versions/${versionN}/review`)

  // 4. Attendre que l'iframe charge le proto, puis que le fil ait chargé.
  await expect(
    page.frameLocator('iframe[title="Prototype preview"]').locator('#cta'),
  ).toBeVisible({ timeout: 15_000 })
  await commentsLoaded

  // 5. Ouvrir le fil visiteur en cliquant sur la pastille du pin.
  const pinBadge = page.locator('[data-testid="pin-badge"]').first()
  await expect(pinBadge).toBeVisible({ timeout: 10_000 })
  await pinBadge.click()

  // 6. Composer du ThreadPopup : pas de champ nom (identité "Admin" forcée côté
  //    serveur, cf. comments-app.tsx). Textarea reconnue par son placeholder
  //    traduit ("Reply…", clé comment.thread.reply_placeholder), bouton par son
  //    libellé traduit ("Reply", clé comment.thread.reply_submit).
  const replyBox = page.getByPlaceholder('Reply…')
  await expect(replyBox).toBeVisible()
  await replyBox.fill('Réponse de l’équipe')

  const replyPosted = page.waitForResponse(
    (r) => r.url().includes('/replies') && r.request().method() === 'POST',
    { timeout: 10_000 },
  )
  await page.getByRole('button', { name: 'Reply' }).click()
  await replyPosted

  // 7. La réponse apparaît dans le fil, attribuée à "Admin" (nom d'auteur +
  //    badge, tous deux littéralement "Admin" — cf. clés comment.admin_author /
  //    comment.admin_badge). On scope au message qui contient notre texte pour
  //    éviter toute ambiguïté avec l'existant commentaire visiteur de Léa.
  const replyMessage = page.locator('li', { hasText: 'Réponse de l’équipe' })
  await expect(replyMessage).toBeVisible()
  await expect(replyMessage.getByText('Admin').first()).toBeVisible()
})

test('ProjectForm : toggle commentaires suit code_enabled puis se découple', async ({
  page,
  request,
}) => {
  // apiLogin pour la cohérence (même si ce test n'appelle aucune API directement)
  await apiLogin(request)
  await pageLogin(page)

  // Ouvrir le formulaire de création de projet
  await page.getByRole('button', { name: '+ New project' }).click()

  // Attendre que le panneau latéral s'ouvre
  await expect(page.getByRole('heading', { name: 'New project' })).toBeVisible()

  const codeSwitch = page.locator('#project-code')
  const commentsSwitch = page.locator('#project-comments')

  // Par défaut en mode create : code_enabled = true, comments_enabled = true (miroir)
  await expect(codeSwitch).toHaveAttribute('aria-checked', 'true')
  await expect(commentsSwitch).toHaveAttribute('aria-checked', 'true')

  // Désactiver le code d'accès → comments_enabled doit se désactiver aussi (miroir auto)
  await codeSwitch.click()
  await expect(codeSwitch).toHaveAttribute('aria-checked', 'false')
  await expect(commentsSwitch).toHaveAttribute('aria-checked', 'false')

  // Activer manuellement les commentaires (touche le toggle → découple du miroir)
  await commentsSwitch.click()
  await expect(commentsSwitch).toHaveAttribute('aria-checked', 'true')

  // Réactiver le code d'accès → comments_enabled doit rester ON (découplé)
  await codeSwitch.click()
  await expect(codeSwitch).toHaveAttribute('aria-checked', 'true')
  await expect(commentsSwitch).toHaveAttribute('aria-checked', 'true')
})
