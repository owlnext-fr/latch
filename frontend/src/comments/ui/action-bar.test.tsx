import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { ActionBar } from './action-bar'

const caps = { canAuthor: true, canEditOwn: true, canModerate: false }

function renderBar(over: Partial<Parameters<typeof ActionBar>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ActionBar
        capabilities={caps}
        pinCount={2}
        pickActive={false}
        pinsVisible
        onTogglePick={vi.fn()}
        onToggleVisible={vi.fn()}
        onOpenList={vi.fn()}
        {...over}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => i18n.changeLanguage('en'))

describe('ActionBar', () => {
  it('shows the comment count', () => {
    renderBar()
    expect(screen.getByText('2 comments')).toBeInTheDocument()
  })

  it('triggers pick mode toggle', async () => {
    const onTogglePick = vi.fn()
    renderBar({ onTogglePick })
    await userEvent.click(screen.getByRole('button', { name: 'Comment' }))
    expect(onTogglePick).toHaveBeenCalledOnce()
  })

  it('hides the pick button when canAuthor is false', () => {
    renderBar({ capabilities: { ...caps, canAuthor: false } })
    expect(screen.queryByRole('button', { name: 'Comment' })).not.toBeInTheDocument()
  })
})
