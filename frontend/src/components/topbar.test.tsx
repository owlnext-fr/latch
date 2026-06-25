import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { server } from '@/test/msw'
import i18n from '@/i18n'
import { Topbar } from './topbar'

const ORIGIN = globalThis.location.origin

function renderTopbar() {
  const history = createMemoryHistory({ initialEntries: ['/projects/1'] })
  const rootRoute = createRootRoute({ component: Outlet })
  const homeRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: () => <div>Home list</div>,
  })
  const loginRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/login',
    component: () => <div>Login screen</div>,
  })
  const detailRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/projects/$id',
    component: () => <Topbar />,
  })
  const routeTree = rootRoute.addChildren([homeRoute, loginRoute, detailRoute])
  const router = createRouter({ routeTree, history })
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={qc}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </I18nextProvider>,
    )
  })
  return router
}

describe('Topbar', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('navigates to the project list when the title is clicked', async () => {
    const user = userEvent.setup()
    renderTopbar()

    // The route component mounts asynchronously (router hydration).
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'latch' })).toBeInTheDocument(),
    )

    // onClick → router.navigate({ to: '/' }).
    await user.click(screen.getByRole('button', { name: 'latch' }))

    await waitFor(() => {
      expect(screen.getByText('Home list')).toBeInTheDocument()
    })
  })

  it('logs out then navigates to /login', async () => {
    const user = userEvent.setup()
    server.use(
      http.post(`${ORIGIN}/api/logout`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    renderTopbar()

    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Log out' }),
      ).toBeInTheDocument(),
    )

    // handleLogout → logout.mutate(...) → onSettled → router.navigate({ to: '/login' }).
    await user.click(screen.getByRole('button', { name: 'Log out' }))

    await waitFor(() => {
      expect(screen.getByText('Login screen')).toBeInTheDocument()
    })
  })
})
