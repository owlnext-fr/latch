import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'node:path'

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  test: {
    // Vitest ne ramasse QUE les tests unitaires/composants sous src/ — les specs
    // Playwright (e2e/*.spec.ts) sont exécutées par Playwright, pas Vitest.
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    environment: 'jsdom',
    environmentOptions: { jsdom: { url: 'http://localhost' } },
    globals: true,
    setupFiles: ['./vitest.setup.ts'],
  },
})
