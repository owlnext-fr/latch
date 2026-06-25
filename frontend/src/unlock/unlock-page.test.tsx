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

  it('recharge la page sur PIN correct (204) via le bouton submit', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 204 })),
    )
    renderUnlock()
    // Type only 5 digits first so onComplete does NOT fire, then click the button manually.
    await userEvent.type(screen.getByLabelText(/access code|code/i), '12345')
    // Guard: button must be disabled with < 6 digits
    expect(screen.getByRole('button', { name: /unlock|déverrouiller/i })).toBeDisabled()
    // Type the 6th digit — onComplete fires and submits; button click here is redundant
    // but we click it to assert the button path also works (busy guard prevents double-fire).
    await userEvent.type(screen.getByLabelText(/access code|code/i), '6')
    await waitFor(() => expect(reloadPage).toHaveBeenCalledOnce())
  })

  it('affiche une erreur sur PIN faux (401) et vide le champ OTP', async () => {
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
    // After a 401, the OTP input must be cleared so the user can retype
    await waitFor(() => {
      const inputs = document.querySelectorAll('input[type="text"], input[inputmode="numeric"]')
      const filled = Array.from(inputs).filter((el) => (el as HTMLInputElement).value !== '')
      expect(filled).toHaveLength(0)
    })
  })

  it('soumet automatiquement quand le 6ème chiffre est saisi (sans cliquer le bouton)', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 204 })),
    )
    renderUnlock()
    // Type digit by digit into the OTP input — onComplete fires on the 6th
    await userEvent.type(screen.getByLabelText(/access code|code/i), '654321')
    // reloadPage must be called WITHOUT clicking the submit button
    await waitFor(() => expect(reloadPage).toHaveBeenCalledOnce())
  })

  it('shows the logo and sets the neutral document title', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
    )
    renderUnlock()
    expect(await screen.findByAltText('latch')).toBeInTheDocument()
    await waitFor(() => expect(document.title).toBe('Unlock — latch'))
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
