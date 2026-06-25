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

// jsdom lacks document.elementFromPoint, which input-otp uses for caret positioning.
if (!document.elementFromPoint) {
  document.elementFromPoint = () => null
}

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }))
afterEach(() => server.resetHandlers())
afterAll(() => server.close())
