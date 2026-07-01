import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { ThreadPopup } from './thread-popup'
import type { CommentPin } from '../data/adapter'
import type { PinPosition } from '../follow/controller'

const pin: CommentPin = {
  id: 7,
  anchor: '{}',
  created_at: 'now',
  messages: [
    { id: 1, author_name: 'Léa', body: 'First', created_at: 'n', updated_at: 'n', editable: true, is_admin: false },
    { id: 2, author_name: 'Max', body: 'Reply', created_at: 'n', updated_at: 'n', editable: false, is_admin: false },
  ],
}
const position: PinPosition = {
  id: 7,
  status: 'anchored',
  rect: { x: 0, y: 0, width: 10, height: 10 },
  offset: { x: 0.5, y: 0.5 },
}
const caps = { canAuthor: true, canEditOwn: true, canModerate: false }

function renderThread(over: Partial<Parameters<typeof ThreadPopup>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ThreadPopup
        pin={pin}
        position={position}
        capabilities={caps}
        busy={false}
        onReply={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDeletePin={vi.fn()}
        onClose={vi.fn()}
        {...over}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => i18n.changeLanguage('en'))

describe('ThreadPopup', () => {
  it('renders every message body as plain text', () => {
    renderThread()
    expect(screen.getByText('First')).toBeInTheDocument()
    // 'Reply' also appears as the submit-button label — narrow to <p> to avoid ambiguity
    expect(screen.getByText('Reply', { selector: 'p' })).toBeInTheDocument()
    expect(screen.getByText('Léa')).toBeInTheDocument()
  })

  it('shows edit/delete only on editable messages', () => {
    renderThread()
    // 1 message editable -> 1 bouton Edit, 1 bouton Delete (hors delete-pin)
    expect(screen.getAllByRole('button', { name: 'Edit' })).toHaveLength(1)
  })

  it('submits a reply', async () => {
    const onReply = vi.fn()
    renderThread({ onReply })
    await userEvent.type(screen.getByPlaceholderText('Reply…'), 'Nice')
    await userEvent.click(screen.getByRole('button', { name: 'Reply' }))
    expect(onReply).toHaveBeenCalledWith('Nice')
  })

  it('flags a moved pin', () => {
    renderThread({ position: { ...position, status: 'approximate' } })
    expect(screen.getByText('This element may have moved')).toBeInTheDocument()
  })

  it('fires onClose from the close button', async () => {
    const onClose = vi.fn()
    renderThread({ onClose })
    await userEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('fires onDeletePin from the delete-thread button', async () => {
    const onDeletePin = vi.fn()
    renderThread({ onDeletePin })
    await userEvent.click(screen.getByRole('button', { name: 'Delete thread' }))
    expect(onDeletePin).toHaveBeenCalledOnce()
  })

  it('shows exactly one per-message Delete on the editable message', () => {
    renderThread()
    expect(screen.getAllByRole('button', { name: 'Delete' })).toHaveLength(1)
  })

  it('affiche la corbeille de modération sur un message non-editable quand canModerate', () => {
    const moderatorCaps = { canAuthor: false, canEditOwn: false, canModerate: true }
    const singleMessagePin: CommentPin = {
      id: 1,
      anchor: '{}',
      created_at: '',
      messages: [
        { id: 9, author_name: 'Léa', body: 'salut', created_at: '', updated_at: '', editable: false, is_admin: false },
      ],
    }
    renderThread({ pin: singleMessagePin, capabilities: moderatorCaps })
    expect(screen.getByRole('button', { name: /delete/i })).toBeEnabled()
    // pas de bouton "supprimer le fil" en modération
    expect(screen.queryByRole('button', { name: /delete thread/i })).toBeNull()
  })

  it('ne montre PAS la suppression quand visiteur non-auteur (editable=false, canEditOwn)', () => {
    const visitorCaps = { canAuthor: true, canEditOwn: true, canModerate: false }
    const nonEditablePin: CommentPin = {
      id: 1,
      anchor: '{}',
      created_at: '',
      messages: [
        { id: 9, author_name: 'Léa', body: 'salut', created_at: '', updated_at: '', editable: false, is_admin: false },
      ],
    }
    renderThread({ pin: nonEditablePin, capabilities: visitorCaps })
    expect(screen.queryByRole('button', { name: /delete/i })).toBeNull()
  })

  it('rend modifier/supprimer en boutons-icône (aria, sans texte visible)', () => {
    renderThread()
    // icônes seulement → aucun libellé texte visible
    expect(screen.queryByText('Edit')).toBeNull()
    expect(screen.queryByText('Delete')).toBeNull()
    // mais toujours accessibles par leur aria-label
    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument()
  })

  it('rend supprimer (message) et supprimer-le-fil en variante danger', () => {
    renderThread()
    expect(screen.getByRole('button', { name: 'Delete' })).toHaveAttribute(
      'data-variant',
      'destructive',
    )
    expect(
      screen.getByRole('button', { name: 'Delete thread' }),
    ).toHaveAttribute('data-variant', 'destructive')
  })

  it('affiche le libellé Admin + badge sur un message is_admin', () => {
    const adminPin: CommentPin = {
      id: 1, anchor: '{}', created_at: 'n',
      messages: [
        { id: 9, author_name: 'admin', body: 'note', created_at: '', updated_at: '', editable: true, is_admin: true },
      ],
    }
    renderThread({ pin: adminPin, capabilities: { canAuthor: true, canEditOwn: true, canModerate: true } })
    // 'Admin' apparaît deux fois : le libellé auteur ET le badge (même wording i18n).
    expect(screen.getAllByText('Admin')).toHaveLength(2)
    // Le nom brut 'admin' (stocké) ne doit PAS s'afficher tel quel comme auteur.
    expect(screen.queryByText('admin')).not.toBeInTheDocument()
  })
})
