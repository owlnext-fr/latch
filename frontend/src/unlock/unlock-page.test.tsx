import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import i18n from './i18n'
import { UnlockPage } from './unlock-page'

vi.mock('./reload', () => ({ reloadPage: vi.fn() }))
import { reloadPage } from './reload'

function renderUnlock() {
  return render(
    <I18nextProvider i18n={i18n}>
      <UnlockPage />
    </I18nextProvider>,
  )
}

beforeEach(() => {
  window.history.replaceState({}, '', '/c/demo-abc')
  vi.mocked(reloadPage).mockClear()
})

describe('UnlockPage', () => {
  it('affiche le brand_name récupéré', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: 'ACME', code_enabled: true }),
      ),
    )
    renderUnlock()
    await waitFor(() => expect(screen.getByText(/ACME/)).toBeInTheDocument())
  })

  it('recharge la page sur PIN correct (204)', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 204 })),
    )
    renderUnlock()
    await userEvent.type(screen.getByLabelText(/access code|code/i), '123456')
    await userEvent.click(screen.getByRole('button', { name: /unlock|déverrouiller/i }))
    await waitFor(() => expect(reloadPage).toHaveBeenCalledOnce())
  })

  it('affiche une erreur sur PIN faux (401)', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 401 })),
    )
    renderUnlock()
    await userEvent.type(screen.getByLabelText(/access code|code/i), '000000')
    await userEvent.click(screen.getByRole('button', { name: /unlock|déverrouiller/i }))
    await waitFor(() => expect(screen.getByText(/incorrect/i)).toBeInTheDocument())
    expect(reloadPage).not.toHaveBeenCalled()
  })

  it('affiche un message de throttle sur 429', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 429 })),
    )
    renderUnlock()
    await userEvent.type(screen.getByLabelText(/access code|code/i), '111111')
    await userEvent.click(screen.getByRole('button', { name: /unlock|déverrouiller/i }))
    await waitFor(() =>
      expect(screen.getByText(/too many attempts|trop de tentatives/i)).toBeInTheDocument(),
    )
  })
})
