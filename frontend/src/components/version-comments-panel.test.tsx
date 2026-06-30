import { describe, it, expect, beforeEach, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { renderWithProviders } from '@/test/utils'
import { VersionCommentsPanel } from './version-comments-panel'

const ORIGIN = globalThis.location.origin

describe('VersionCommentsPanel', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('liste les pins et permet la modération', async () => {
    let deleted = false
    server.use(
      http.get(`${ORIGIN}/api/projects/3/versions/2/comments`, () => {
        if (deleted) {
          return HttpResponse.json({ version: 2, pins: [] })
        }
        return HttpResponse.json({
          version: 2,
          pins: [
            {
              id: 7,
              anchor: JSON.stringify({
                fingerprint: { tag: 'button', text: 'En savoir plus' },
              }),
              created_at: '2026-06-30T10:00:00Z',
              messages: [
                {
                  id: 11,
                  author_name: 'Léa',
                  body: 'à revoir',
                  created_at: '2026-06-30T10:00:00Z',
                  updated_at: '2026-06-30T10:00:00Z',
                },
              ],
            },
          ],
        })
      }),
      http.delete(`${ORIGIN}/api/projects/3/comments/messages/11`, () => {
        deleted = true
        return HttpResponse.json({ ok: true })
      }),
    )
    renderWithProviders(
      <VersionCommentsPanel
        projectId={3}
        version={2}
        open
        onOpenChange={vi.fn()}
      />,
    )
    expect(await screen.findByText(/En savoir plus/)).toBeInTheDocument()
    expect(screen.getByText('à revoir')).toBeInTheDocument()
    await userEvent.click(
      screen.getByRole('button', { name: /delete this message|supprimer ce message/i }),
    )
    await userEvent.click(
      screen.getByRole('button', { name: /^delete$|^supprimer$/i }),
    )
    await waitFor(() => expect(screen.queryByText('à revoir')).toBeNull())
  })

  it("affiche l'état vide", async () => {
    server.use(
      http.get(`${ORIGIN}/api/projects/3/versions/9/comments`, () =>
        HttpResponse.json({ version: 9, pins: [] }),
      ),
    )
    renderWithProviders(
      <VersionCommentsPanel
        projectId={3}
        version={9}
        open
        onOpenChange={vi.fn()}
      />,
    )
    expect(
      await screen.findByText(/no comments on this version|aucun commentaire sur cette version/i),
    ).toBeInTheDocument()
  })
})
