import createClient, { type Middleware } from 'openapi-fetch'
import type { paths } from './schema'

let onUnauthorized: (() => void) | null = null
export function setUnauthorizedHandler(fn: () => void) {
  onUnauthorized = fn
}

const authMiddleware: Middleware = {
  async onResponse({ response }) {
    if (response.status === 401) onUnauthorized?.()
    return response
  },
}

// baseUrl = origine courante (absolue). En prod, l'admin et l'API partagent la
// même origine → cookies session envoyés. En test (jsdom) l'URL absolue permet à
// undici/MSW d'intercepter (un baseUrl '' produit une URL relative que Node rejette).
// credentials include = cookie session same-origin.
export const api = createClient<paths>({
  baseUrl: globalThis.location.origin,
  credentials: 'include',
  // Résoudre `fetch` à l'appel (pas à la création du client) : sinon openapi-fetch
  // capture la référence globale au load du module, ce qui empêche MSW de l'intercepter
  // en test (MSW remplace `globalThis.fetch` après l'import). No-op en prod.
  fetch: (input) => globalThis.fetch(input),
})
api.use(authMiddleware)
