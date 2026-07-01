import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { ComposePopup } from './compose-popup'

const point = { x: 20, y: 20 }

function renderPopup(props: Partial<Parameters<typeof ComposePopup>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ComposePopup
        point={point}
        submitting={false}
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
        {...props}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => {
  localStorage.clear()
  return i18n.changeLanguage('en')
})

describe('ComposePopup', () => {
  it('blocks submit when name or body is empty', async () => {
    const onSubmit = vi.fn()
    renderPopup({ onSubmit })
    await userEvent.click(screen.getByRole('button', { name: 'Post' }))
    expect(onSubmit).not.toHaveBeenCalled()
    expect(screen.getByText('Please enter your name.')).toBeInTheDocument()
  })

  it('submits name + body and stores the name', async () => {
    const onSubmit = vi.fn()
    renderPopup({ onSubmit })
    await userEvent.type(screen.getByLabelText('Your name'), 'Léa')
    await userEvent.type(screen.getByLabelText('Comment'), 'Looks good')
    await userEvent.click(screen.getByRole('button', { name: 'Post' }))
    expect(onSubmit).toHaveBeenCalledWith({ author_name: 'Léa', body: 'Looks good' })
    expect(localStorage.getItem('latch:comment-name')).toBe('Léa')
  })

  it('pre-fills the name from localStorage', () => {
    localStorage.setItem('latch:comment-name', 'Léa')
    renderPopup()
    expect(screen.getByLabelText('Your name')).toHaveValue('Léa')
  })

  it('calls onCancel', async () => {
    const onCancel = vi.fn()
    renderPopup({ onCancel })
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalledOnce()
  })

  it('blocks submit when body exceeds 2000 characters', async () => {
    const onSubmit = vi.fn()
    renderPopup({ onSubmit })
    await userEvent.type(screen.getByLabelText('Your name'), 'Léa')
    fireEvent.change(screen.getByLabelText('Comment'), { target: { value: 'x'.repeat(2001) } })
    await userEvent.click(screen.getByRole('button', { name: 'Post' }))
    expect(onSubmit).not.toHaveBeenCalled()
    expect(screen.getByText('Comment is too long (max 2000 characters).')).toBeInTheDocument()
  })
})
