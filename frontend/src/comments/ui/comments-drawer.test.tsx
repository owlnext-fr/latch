import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { CommentsDrawer, sortPins } from './comments-drawer'
import type { CommentPin } from '../data/adapter'
import type { AnchorStatus } from '../anchor/resolve'

function pin(id: number, author: string, created: string): CommentPin {
  return {
    id,
    anchor: '{}',
    created_at: created,
    messages: [
      {
        id: id * 10,
        author_name: author,
        body: `Body ${id}`,
        created_at: created,
        updated_at: created,
        editable: false,
        is_admin: false,
      },
    ],
  }
}

const pins = [
  pin(1, 'Alice', '2026-07-01T09:00:00Z'),
  pin(2, 'Max', '2026-07-01T11:00:00Z'),
  pin(3, 'Jo', '2026-07-01T08:00:00Z'),
  pin(4, 'Sam', '2026-07-01T07:00:00Z'),
]
const statusOf = (id: number): AnchorStatus | undefined => {
  if (id === 3) return 'orphaned'
  if (id === 4) return 'approximate'
  return 'anchored'
}

function renderDrawer(
  over: Partial<Parameters<typeof CommentsDrawer>[0]> = {},
) {
  return render(
    <I18nextProvider i18n={i18n}>
      <CommentsDrawer
        open
        pins={pins}
        statusOf={statusOf}
        hiddenOf={() => false}
        onClose={vi.fn()}
        onSelect={vi.fn()}
        {...over}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => i18n.changeLanguage('en'))

describe('sortPins', () => {
  it('met orphelins et déplacés en bas, sains par récence desc', () => {
    const ids = sortPins(pins, statusOf).map((p) => p.id)
    // 2 (11h) > 1 (9h) sains ; puis 3 orphelin (8h) > 4 déplacé (7h), récence desc dans chaque groupe
    expect(ids).toEqual([2, 1, 3, 4])
  })
})

describe('CommentsDrawer', () => {
  it('rend une ligne par pin', () => {
    renderDrawer()
    expect(screen.getAllByTestId('drawer-row')).toHaveLength(4)
    expect(screen.getByText('orphaned')).toBeInTheDocument()
  })

  it('affiche une date absolue AVEC heure (h:mm) sur les lignes', () => {
    renderDrawer()
    const rows = screen.getAllByTestId('drawer-row')
    expect(
      rows.some((r) => /\d{1,2}:\d{2}/.test(r.textContent ?? '')),
    ).toBe(true)
  })

  it('appelle onSelect avec l’id au clic', async () => {
    const onSelect = vi.fn()
    renderDrawer({ onSelect })
    await userEvent.click(screen.getAllByTestId('drawer-row')[0])
    expect(onSelect).toHaveBeenCalledWith(2) // première ligne = plus récente
  })

  it('affiche le badge "moved" (pas "orphaned") et un avatar ambre pour un pin approximate', () => {
    renderDrawer()
    const movedBadge = screen.getByText('moved')
    expect(movedBadge).toBeInTheDocument()
    const row = movedBadge.closest('button[data-testid="drawer-row"]')
    expect(row).not.toBeNull()
    expect(within(row as HTMLElement).queryByText('orphaned')).toBeNull()
    const avatar = row?.querySelector('span.rounded-full.border-2')
    expect(avatar).toHaveStyle({ background: 'rgb(245, 158, 11)' })
  })

  it('marque « hors écran » un pin hidden et affiche une note au clic (sans ouvrir le fil)', async () => {
    const onSelect = vi.fn()
    renderDrawer({ hiddenOf: (id) => id === 1, onSelect })
    const badge = screen.getByTestId('offscreen-badge')
    expect(badge).toBeInTheDocument()
    const row = badge.closest(
      'button[data-testid="drawer-row"]',
    ) as HTMLElement
    await userEvent.click(row)
    // clic sur une ligne hors écran : pas d'ouverture de fil, juste la note inline
    expect(onSelect).not.toHaveBeenCalled()
    expect(screen.getByTestId('offscreen-notice')).toBeInTheDocument()
  })

  it('efface la note « hors écran » quand on sélectionne ensuite une ligne visible', async () => {
    const onSelect = vi.fn()
    renderDrawer({ hiddenOf: (id) => id === 1, onSelect })
    const rows = screen.getAllByTestId('drawer-row')
    const aliceRow = rows.find((r) => r.textContent?.includes('Alice'))!
    const maxRow = rows.find((r) => r.textContent?.includes('Max'))!
    await userEvent.click(aliceRow) // ligne hors écran → note affichée
    expect(screen.getByTestId('offscreen-notice')).toBeInTheDocument()
    await userEvent.click(maxRow) // ligne visible → ouvre le fil + efface la note
    expect(onSelect).toHaveBeenCalledWith(2)
    expect(screen.queryByTestId('offscreen-notice')).toBeNull()
  })

  it('affiche l’état vide', () => {
    renderDrawer({ pins: [] })
    expect(screen.getByText('No comments yet')).toBeInTheDocument()
    expect(screen.queryByTestId('drawer-row')).toBeNull()
  })

  it('ne rend rien si fermé', () => {
    const { container } = renderDrawer({ open: false })
    expect(
      container.querySelector('[data-testid="comments-drawer"]'),
    ).toBeNull()
  })
})
