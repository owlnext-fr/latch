import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, it } from 'vitest'
import { server } from '@/test/msw'
import { renderWithRouter } from '@/test/utils'

// No vi.mock — the real useLogin hook is exercised through the production
// api client (openapi-fetch with baseUrl = window.location.origin + lazy
// globalThis.fetch), which MSW intercepts in jsdom exactly as it does for
// the list/detail tests. This ensures regressions in use-auth.ts are caught.

// Wait for the router to mount LoginPage, then fill and submit the form.
async function fillAndSubmit(user: string, pass: string) {
  await userEvent.type(await screen.findByLabelText(/username/i), user)
  await userEvent.type(screen.getByLabelText(/password/i), pass)
  await userEvent.click(screen.getByRole('button', { name: /sign in/i }))
}

describe('LoginPage', () => {
  it('shows error message on 401', async () => {
    server.use(
      http.post(`${window.location.origin}/api/login`, () =>
        new HttpResponse(null, { status: 401 }),
      ),
    )

    renderWithRouter('/login')

    await fillAndSubmit('admin', 'wrong')

    expect(await screen.findByText('Invalid credentials.')).toBeInTheDocument()
    // Form is still visible — the user has not been redirected
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
  })

  it('navigates to / on successful login', async () => {
    server.use(
      http.post(`${window.location.origin}/api/login`, () =>
        HttpResponse.json({ ok: true }),
      ),
    )

    const { router } = renderWithRouter('/login')

    await fillAndSubmit('admin', 'secret')

    // After success the login form should unmount (router moved to /)
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /sign in/i })).not.toBeInTheDocument()
    })
    // Router state reflects the new pathname
    expect(router.state.location.pathname).toBe('/')
  })
})
