import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, it, vi } from 'vitest'
import { server } from '@/test/msw'
import { renderWithRouter } from '@/test/utils'

// openapi-fetch constructs relative URLs ("/api/login") that Node's fetch
// rejects. Patch the useLogin hook to POST with an absolute URL instead.
vi.mock('@/hooks/use-auth', async () => {
  const { useMutation } = await import('@tanstack/react-query')
  return {
    useLogin: () =>
      useMutation({
        mutationFn: async (body: { user: string; pass: string }) => {
          const res = await fetch('http://localhost/api/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
          })
          if (!res.ok) throw new Error(String(res.status))
          return res.json()
        },
      }),
    useLogout: () =>
      useMutation({
        mutationFn: async () => {
          await fetch('http://localhost/api/logout', { method: 'POST' })
        },
      }),
  }
})

// Wait for the router to mount LoginPage, then fill and submit the form.
async function fillAndSubmit(user: string, pass: string) {
  await userEvent.type(await screen.findByLabelText(/username/i), user)
  await userEvent.type(screen.getByLabelText(/password/i), pass)
  await userEvent.click(screen.getByRole('button', { name: /sign in/i }))
}

describe('LoginPage', () => {
  it('shows error message on 401', async () => {
    server.use(
      http.post('http://localhost/api/login', () => new HttpResponse(null, { status: 401 })),
    )

    renderWithRouter('/login')

    await fillAndSubmit('admin', 'wrong')

    expect(await screen.findByText('Invalid credentials.')).toBeInTheDocument()
    // Form is still visible — the user has not been redirected
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument()
  })

  it('navigates to / on successful login', async () => {
    server.use(
      http.post('http://localhost/api/login', () => HttpResponse.json({ ok: true })),
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
