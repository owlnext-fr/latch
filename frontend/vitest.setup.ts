import '@testing-library/jest-dom/vitest'
import { afterAll, afterEach, beforeAll } from 'vitest'
import { server } from './src/test/msw'

// jsdom lacks ResizeObserver, which Radix UI primitives (Switch, etc.) require.
if (!('ResizeObserver' in globalThis)) {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
}

// jsdom ne fournit pas IntersectionObserver (utilisé par le contrôleur de suivi des pins).
if (!('IntersectionObserver' in globalThis)) {
  globalThis.IntersectionObserver = class {
    readonly root = null
    readonly rootMargin = ''
    readonly thresholds = []
    observe() {}
    unobserve() {}
    disconnect() {}
    takeRecords() {
      return []
    }
  } as unknown as typeof IntersectionObserver
}

// jsdom lacks document.elementFromPoint, which input-otp uses for caret positioning.
if (!document.elementFromPoint) {
  document.elementFromPoint = () => null
}

// input-otp planifie des setTimeout longue durée (jusqu'à 5 s) pour synchroniser le
// caret. Si l'un d'eux se déclenche APRÈS le teardown de l'environnement jsdom, react-dom
// tente une mise à jour d'état et touche `window` (disparu) → « ReferenceError: window is
// not defined » qui fait échouer Vitest (exit 1) de façon flaky selon le timing.
// Parade : tracer les timers et annuler ceux encore pendants à la fin de chaque test —
// aucun ne survit à l'environnement. (Aucun test n'utilise de fake timers, patch global sûr.)
const realSetTimeout = globalThis.setTimeout.bind(globalThis)
const pendingTimers = new Set<ReturnType<typeof setTimeout>>()
globalThis.setTimeout = ((handler: TimerHandler, timeout?: number, ...args: unknown[]) => {
  const id = realSetTimeout(
    (...callbackArgs: unknown[]) => {
      pendingTimers.delete(id)
      if (typeof handler === 'function') {
        ;(handler as (...a: unknown[]) => void)(...callbackArgs)
      }
    },
    timeout,
    ...args,
  )
  pendingTimers.add(id)
  return id
}) as typeof setTimeout

// jsdom lacks these Element methods that Radix Select relies on for positioning.
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {}
}
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false
}
if (!Element.prototype.releasePointerCapture) {
  Element.prototype.releasePointerCapture = () => {}
}

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }))
afterEach(() => {
  server.resetHandlers()
  // Annule les timers input-otp encore pendants (cf. note plus haut) avant que
  // l'environnement ne soit démonté.
  for (const id of pendingTimers) clearTimeout(id)
  pendingTimers.clear()
})
afterAll(() => server.close())
