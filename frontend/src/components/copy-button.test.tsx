import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import { Toaster } from 'sonner'
import i18n from '@/i18n'
import { CopyButton } from './copy-button'

beforeEach(() => {
  Object.assign(navigator, {
    clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
  })
})

function renderCopy(text: string) {
  return render(
    <I18nextProvider i18n={i18n}>
      <Toaster />
      <CopyButton text={text} ariaLabel="Copy the PIN" />
    </I18nextProvider>,
  )
}

describe('CopyButton', () => {
  it('calls clipboard.writeText with the provided text', async () => {
    renderCopy('123456')
    await userEvent.click(screen.getByRole('button', { name: /copy the pin/i }))
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('123456')
  })

  it('renders a button with the given aria-label', () => {
    renderCopy('abc')
    expect(screen.getByRole('button', { name: /copy the pin/i })).toBeInTheDocument()
  })
})
