import { describe, expect, it, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ThemeProvider, useTheme } from 'next-themes'

function Probe() {
  const { theme } = useTheme()
  return <span data-testid="theme">{theme ?? 'pending'}</span>
}

describe('ThemeProvider (config)', () => {
  beforeEach(() => {
    // Mock window.matchMedia for theme detection
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: (query: string) => ({
        matches: query === '(prefers-color-scheme: dark)',
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => true,
      }),
    })
  })

  it('provides a resolved theme value to consumers', async () => {
    render(
      <ThemeProvider attribute="class" defaultTheme="system" enableSystem storageKey="latch.theme">
        <Probe />
      </ThemeProvider>,
    )
    // Wait for the theme to resolve from the effect
    const themeElement = await screen.findByTestId('theme')
    expect(themeElement).toHaveTextContent(/^(system|light|dark)$/)
  })
})
