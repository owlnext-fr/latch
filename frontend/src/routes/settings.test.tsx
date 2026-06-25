import { describe, it, expect, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { renderWithRouter } from '@/test/utils'

function mockSettings() {
  server.use(
    http.get(`${window.location.origin}/api/settings`, () =>
      HttpResponse.json({
        deploy_token: 'tok-abc-123',
        mcp_url: 'https://demo.test/mcp',
        public_base_url: 'https://demo.test',
      }),
    ),
  )
}

describe('SettingsPage', () => {
  beforeEach(() => server.resetHandlers())

  it('masks the deploy token by default and reveals it on click', async () => {
    mockSettings()
    renderWithRouter('/settings')

    // mcp_url visible directement
    await waitFor(() => expect(screen.getByText('https://demo.test/mcp')).toBeInTheDocument())
    // token masqué : la valeur en clair n'est pas affichée tant qu'on ne révèle pas
    expect(screen.queryByText('tok-abc-123')).not.toBeInTheDocument()

    const reveal = screen.getByRole('button', { name: /reveal|révéler/i })
    await userEvent.click(reveal)
    expect(screen.getByText('tok-abc-123')).toBeInTheDocument()
  })
})
