import { type ReactNode } from 'react'
import { render, type RenderResult, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router'
import i18n from '@/i18n'
import { LoginPage } from '@/routes/login'
import { ListPage } from '@/routes/list'

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function AllProviders({ children }: { children: ReactNode }) {
  const queryClient = makeQueryClient()
  return (
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </I18nextProvider>
  )
}

export function renderWithProviders(ui: ReactNode): RenderResult {
  return render(ui, { wrapper: AllProviders })
}

type TestPath = '/login' | '/'

type TestRouter = ReturnType<typeof buildTestRouter>

export type RenderWithRouterResult = RenderResult & { router: TestRouter }

/**
 * Build a fresh TanStack memory-router for tests.
 * Each call creates an isolated router instance — safe to use in parallel tests.
 */
function buildTestRouter(initialPath: TestPath) {
  const history = createMemoryHistory({ initialEntries: [initialPath] })
  const rootRoute = createRootRoute({ component: Outlet })
  const loginRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/login',
    component: LoginPage,
  })
  const listRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: ListPage,
  })
  const routeTree = rootRoute.addChildren([loginRoute, listRoute])
  return createRouter({ routeTree, history })
}

/**
 * Render the app in a TanStack memory router starting at `initialPath`.
 * The router is pre-loaded so the correct route is already mounted when the
 * function returns — no extra `await` needed in tests before the first query.
 *
 * Returns the RTL render result plus the `router` instance so tests can
 * assert on `router.state.location.pathname`.
 */
export function renderWithRouter(initialPath: TestPath): RenderWithRouterResult {
  const router = buildTestRouter(initialPath)
  const queryClient = makeQueryClient()

  let result!: RenderResult
  // Wrap in act so React flushes all effects (including router hydration)
  act(() => {
    result = render(
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </I18nextProvider>,
    )
  })
  return { ...result, router }
}
