import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { LocaleSwitcher } from './locale-switcher'

function renderLS() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <LocaleSwitcher />
      </I18nextProvider>,
    )
  })
}

describe('LocaleSwitcher', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('renders one button per discovered locale', () => {
    renderLS()
    expect(screen.getByRole('button', { name: 'EN' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'FR' })).toBeInTheDocument()
  })

  it('switches language on click', async () => {
    const user = userEvent.setup()
    renderLS()
    await user.click(screen.getByRole('button', { name: 'FR' }))
    expect(i18n.language).toBe('fr')
  })
})
