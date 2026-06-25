import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { PinField } from './pin-field'

function renderPin(pin: string | null, editable = false) {
  return render(
    <I18nextProvider i18n={i18n}>
      <PinField pin={pin} editable={editable} />
    </I18nextProvider>,
  )
}

describe('PinField (read mode)', () => {
  it('masks the pin until revealed', async () => {
    renderPin('123456')
    expect(screen.getByText('••••••')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /reveal|révéler/i }))
    expect(screen.getByText('123456')).toBeInTheDocument()
  })

  it('hides pin again after second click', async () => {
    renderPin('123456')
    const btn = screen.getByRole('button', { name: /reveal|révéler/i })
    await userEvent.click(btn)
    expect(screen.getByText('123456')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /hide|masquer/i }))
    expect(screen.getByText('••••••')).toBeInTheDocument()
  })

  it('shows copy button when pin is present', () => {
    renderPin('123456')
    expect(screen.getByRole('button', { name: /copy.*pin|copier.*pin/i })).toBeInTheDocument()
  })

  it('renders nothing when pin is null', () => {
    const { container } = renderPin(null)
    expect(container).toBeEmptyDOMElement()
  })
})

describe('PinField (edit mode)', () => {
  it('renders an input in editable mode', () => {
    renderPin('123456', true)
    expect(screen.getByRole('textbox')).toBeInTheDocument()
  })

  it('input is disabled when disabled prop is set', () => {
    render(
      <I18nextProvider i18n={i18n}>
        <PinField pin="123456" editable disabled />
      </I18nextProvider>,
    )
    expect(screen.getByRole('textbox')).toBeDisabled()
  })
})
