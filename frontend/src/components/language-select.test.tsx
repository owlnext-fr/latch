import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { LanguageSelect } from './language-select'

function renderLS() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <LanguageSelect />
      </I18nextProvider>,
    )
  })
}

describe('LanguageSelect', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('shows the current language in the trigger', () => {
    renderLS()
    expect(screen.getByRole('combobox')).toHaveTextContent('English')
  })

  it('lists discovered locales and switches language on selection', async () => {
    const user = userEvent.setup()
    renderLS()
    await user.click(screen.getByRole('combobox'))
    // Options are rendered in a portal once open.
    const frenchOption = await screen.findByRole('option', { name: /Français/ })
    expect(screen.getByRole('option', { name: /English/ })).toBeInTheDocument()
    await user.click(frenchOption)
    expect(i18n.language).toBe('fr')
  })
})
