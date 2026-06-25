import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { ErrorPage } from './error-page'

function renderError() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <ErrorPage />
      </I18nextProvider>,
    )
  })
}

describe('ErrorPage', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('renders the logo and the generic unavailable message', () => {
    renderError()
    expect(screen.getByAltText('latch')).toBeInTheDocument()
    expect(
      screen.getByText('This prototype is not available or has been removed.'),
    ).toBeInTheDocument()
  })

  it('sets the document title', () => {
    renderError()
    expect(document.title).toBe('Unavailable — latch')
  })
})
