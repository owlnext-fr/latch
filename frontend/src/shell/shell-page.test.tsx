import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { http, HttpResponse } from 'msw'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { server } from '@/test/msw'
import { ShellPage } from './shell-page'

function renderShell() {
  return render(
    <I18nextProvider i18n={i18n}>
      <ShellPage />
    </I18nextProvider>,
  )
}

describe('ShellPage', () => {
  beforeEach(() => {
    localStorage.clear()
    window.history.pushState({}, '', '/c/demo-abc123')
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('always renders the prototype iframe pointing at /raw', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 204 }))
    const { container } = renderShell()
    const iframe = container.querySelector('iframe')
    expect(iframe?.getAttribute('src')).toBe('/c/demo-abc123/raw')
  })

  it('shows the overlay when notes are unseen, hides after dismiss', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        status: 200,
        json: async () => ({ n: 2, notes_md: '# New' }),
      }),
    )
    renderShell()
    const dismiss = await screen.findByTestId('notes-dismiss')
    expect(screen.getByRole('heading', { name: 'New' })).toBeInTheDocument()
    dismiss.click()
    await waitFor(() =>
      expect(screen.queryByTestId('notes-dismiss')).toBeNull(),
    )
    expect(localStorage.getItem('latch:seen:demo-abc123')).toBe('2')
  })

  it('does not show the overlay when the version was already seen', async () => {
    localStorage.setItem('latch:seen:demo-abc123', '2')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        status: 200,
        json: async () => ({ n: 2, notes_md: '# New' }),
      }),
    )
    renderShell()
    await waitFor(() => {})
    expect(screen.queryByTestId('notes-dismiss')).toBeNull()
  })
})

const ORIGIN = globalThis.location.origin

describe('ShellPage — comments gating', () => {
  beforeEach(() => {
    i18n.changeLanguage('en')
    // slug courant dérivé du pathname : on force /c/<slug>
    window.history.pushState({}, '', '/c/demo-aB3dEf9z')
    server.use(
      http.get(`${ORIGIN}/c/demo-aB3dEf9z/notes`, () => new HttpResponse(null, { status: 204 })),
      http.get(`${ORIGIN}/c/demo-aB3dEf9z/comments`, () =>
        HttpResponse.json({ version: 1, pins: [] }, { status: 200 }),
      ),
    )
  })

  it('mounts the comments layer when comments_enabled is true', async () => {
    server.use(
      http.get(`${ORIGIN}/api/public/demo-aB3dEf9z`, () =>
        HttpResponse.json({ code_enabled: false, comments_enabled: true }, { status: 200 }),
      ),
    )
    render(
      <I18nextProvider i18n={i18n}>
        <ShellPage />
      </I18nextProvider>,
    )
    expect(await screen.findByTestId('comments-mount')).toBeInTheDocument()
  })

  it('does NOT mount the comments layer when comments_enabled is false', async () => {
    server.use(
      http.get(`${ORIGIN}/api/public/demo-aB3dEf9z`, () =>
        HttpResponse.json({ code_enabled: false, comments_enabled: false }, { status: 200 }),
      ),
    )
    render(
      <I18nextProvider i18n={i18n}>
        <ShellPage />
      </I18nextProvider>,
    )
    // laisser les effets se résoudre
    await waitFor(() => expect(screen.getByTitle('prototype')).toBeInTheDocument())
    expect(screen.queryByTestId('comments-mount')).not.toBeInTheDocument()
  })
})
