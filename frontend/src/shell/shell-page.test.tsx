import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
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
