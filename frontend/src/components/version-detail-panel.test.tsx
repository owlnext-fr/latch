import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { VersionDetailPanel } from './version-detail-panel'
import type { components } from '@/api/schema'

type VersionItem = components['schemas']['VersionItem']

function renderPanel(version: VersionItem) {
  return render(
    <I18nextProvider i18n={i18n}>
      <VersionDetailPanel version={version} open onOpenChange={vi.fn()} />
    </I18nextProvider>,
  )
}

const base: VersionItem = {
  id: 10,
  n: 2,
  created_at: '2024-01-15T10:00:00Z',
  is_active: true,
}

describe('VersionDetailPanel', () => {
  it('renders rendered release notes when present', () => {
    renderPanel({ ...base, release_notes: '# Hello\n\n- a' })
    expect(screen.getByRole('heading', { name: 'Hello' })).toBeInTheDocument()
    expect(screen.getByRole('list')).toBeInTheDocument()
  })

  it('shows the empty state when there are no notes', () => {
    renderPanel({ ...base, release_notes: null })
    expect(
      screen.getByText(/no release notes for this version/i),
    ).toBeInTheDocument()
  })
})
