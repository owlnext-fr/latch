import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeProvider } from 'next-themes'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { ThemeToggle } from './theme-toggle'

function renderTT() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem storageKey="latch.theme">
          <ThemeToggle />
        </ThemeProvider>
      </I18nextProvider>,
    )
  })
}

describe('ThemeToggle', () => {
  beforeEach(() => {
    localStorage.clear()
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        addEventListener: () => {},
        removeEventListener: () => {},
        addListener: () => {},
        removeListener: () => {},
        dispatchEvent: () => false,
        onchange: null,
      }),
    })
  })

  it('renders the three theme options', async () => {
    renderTT()
    expect(await screen.findByRole('button', { name: /System/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Light/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Dark/ })).toBeInTheDocument()
  })

  it('marks System as pressed by default and switches on click', async () => {
    const user = userEvent.setup()
    renderTT()
    const system = await screen.findByRole('button', { name: /System/ })
    expect(system).toHaveAttribute('aria-pressed', 'true')

    await user.click(screen.getByRole('button', { name: /Dark/ }))
    expect(screen.getByRole('button', { name: /Dark/ })).toHaveAttribute('aria-pressed', 'true')
  })
})
