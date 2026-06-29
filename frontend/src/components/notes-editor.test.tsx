import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { NotesEditor } from './notes-editor'

describe('NotesEditor', () => {
  it('renders the editor and a preview tab', () => {
    render(
      <I18nextProvider i18n={i18n}>
        <NotesEditor value={'# Hi'} onChange={vi.fn()} />
      </I18nextProvider>,
    )
    // Onglet aperçu présent (libellé i18n ou data-testid)
    expect(screen.getByTestId('notes-editor')).toBeInTheDocument()
    expect(screen.getByTestId('notes-preview-tab')).toBeInTheDocument()
  })
})
