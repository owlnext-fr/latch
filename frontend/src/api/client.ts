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

// baseUrl '' = même origine. credentials include = cookie session same-origin.
export const api = createClient<paths>({ baseUrl: '', credentials: 'include' })
api.use(authMiddleware)
