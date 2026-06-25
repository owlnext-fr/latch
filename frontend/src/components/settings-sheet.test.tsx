import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { ThemeProvider } from 'next-themes'
import { server } from '@/test/msw'
import i18n from '@/i18n'
import { SettingsSheet } from './settings-sheet'

const ORIGIN = globalThis.location.origin

function renderSheet(open: boolean) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem storageKey="latch.theme">
          <QueryClientProvider client={qc}>
            <SettingsSheet open={open} onOpenChange={() => {}} />
          </QueryClientProvider>
        </ThemeProvider>
      </I18nextProvider>,
    )
  })
}

describe('SettingsSheet', () => {
  beforeEach(async () => {
    server.resetHandlers()
    localStorage.clear()
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: (query: string) => ({
        matches: false, media: query,
        addEventListener: () => {}, removeEventListener: () => {},
        addListener: () => {}, removeListener: () => {},
        dispatchEvent: () => false, onchange: null,
      }),
    })
    await i18n.changeLanguage('en')
    server.use(
      http.get(`${ORIGIN}/api/settings`, () =>
        HttpResponse.json({
          mcp_url: 'https://latch.example/mcp',
          deploy_token: 'tok-123456',
          public_base_url: 'https://latch.example',
        }),
      ),
    )
  })

  it('renders MCP infos with a help text per field when open', async () => {
    renderSheet(true)
    expect(await screen.findByText('https://latch.example/mcp')).toBeInTheDocument()
    expect(screen.getByText('Set this in Claude\'s MCP connector.')).toBeInTheDocument()
    expect(screen.getByText('Secret validated by all MCP tools.')).toBeInTheDocument()
    expect(screen.getByText('Public root of this instance.')).toBeInTheDocument()
    // deploy_token masqué par défaut
    expect(screen.getByText('••••••')).toBeInTheDocument()
  })

  it('renders the preferences controls (language + theme)', async () => {
    renderSheet(true)
    await screen.findByText('https://latch.example/mcp')
    expect(screen.getByRole('combobox')).toBeInTheDocument() // LanguageSelect
    expect(screen.getByRole('button', { name: /System/ })).toBeInTheDocument() // ThemeToggle
  })
})
