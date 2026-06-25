import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render } from '@testing-library/react'
import { screen, waitFor, act } from '@testing-library/react'
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
import type { components } from '@/api/schema'
import { DetailPage } from '@/routes/detail'

type ProjectDetail = components['schemas']['ProjectDetail']

// ─── Fixtures ─────────────────────────────────────────────────────────────────
// Using fictitious placeholder names only (no real client names).

const PROJECT_DETAIL: ProjectDetail = {
  id: 1,
  name: 'Mon Projet',
  slug: 'mon-projet-k7Qp2maZ',
  code_enabled: true,
  pin: '123456',
  active_version_id: 2,
  brand_name: 'ACME',
  versions: [
    {
      id: 10,
      n: 1,
      created_at: '2024-01-15T10:00:00Z',
      is_active: false,
    },
    {
      id: 11,
      n: 2,
      created_at: '2024-01-20T12:00:00Z',
      is_active: true,
    },
  ],
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function mockProjectDetail(project: ProjectDetail, status = 200) {
  server.use(
    http.get(`${window.location.origin}/api/projects/${project.id}`, () =>
      HttpResponse.json(project, { status }),
    ),
  )
}

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function renderDetailPage(projectId: number) {
  const history = createMemoryHistory({
    initialEntries: [`/projects/${projectId}`],
  })
  const rootRoute = createRootRoute({ component: Outlet })
  const listRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: () => <div>List</div>,
  })
  const detailRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/projects/$id',
    component: DetailPage,
  })
  const routeTree = rootRoute.addChildren([listRoute, detailRoute])
  const router = createRouter({ routeTree, history })
  const queryClient = makeQueryClient()

  let result!: ReturnType<typeof render>
  act(() => {
    result = render(
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </I18nextProvider>,
    )
  })
  return result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('DetailPage', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('renders public access card with URL and copy button', async () => {
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Public access')).toBeInTheDocument()
    })

    // Public URL is visible
    expect(
      screen.getByText(new RegExp(`/c/${PROJECT_DETAIL.slug}`)),
    ).toBeInTheDocument()

    // Copy URL button present
    expect(screen.getByLabelText('Copy the URL')).toBeInTheDocument()
  })

  it('renders PIN masked (••••••) and revealable when code_enabled=true', async () => {
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('••••••')).toBeInTheDocument()
    })

    // PIN reveal button present
    expect(screen.getByLabelText('Reveal PIN')).toBeInTheDocument()
  })

  it('renders 2 version rows with active badge on the active one', async () => {
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Versions')).toBeInTheDocument()
    })

    // Row for version n=1 (not active)
    expect(screen.getByText('1')).toBeInTheDocument()
    // Row for version n=2 (active)
    expect(screen.getByText('2')).toBeInTheDocument()

    // 'active' badge appears (for the active version)
    expect(screen.getByText('active')).toBeInTheDocument()
  })

  it('renders Edit, Deploy, Delete action buttons', async () => {
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Deploy' })).toBeInTheDocument()
    // There may be multiple "Delete" buttons (header + per-version rows)
    const deleteButtons = screen.getAllByRole('button', { name: 'Delete' })
    expect(deleteButtons.length).toBeGreaterThanOrEqual(1)
  })

  it('renders the error state when the detail query fails', async () => {
    // 500 → useProject throws → isError branch (mainContent error message).
    server.use(
      http.get(`${window.location.origin}/api/projects/1`, () =>
        HttpResponse.json({}, { status: 500 }),
      ),
    )
    renderDetailPage(1)

    await waitFor(() => {
      expect(
        screen.getByText('Network error — please retry.'),
      ).toBeInTheDocument()
    })
  })

  it('opens the deploy panel when the Deploy action is clicked', async () => {
    const user = userEvent.setup()
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // setDeployOpen(true) → DeployPanel mounts its sheet content.
    await user.click(screen.getByRole('button', { name: 'Deploy' }))

    await waitFor(() => {
      expect(screen.getByText('Deploy a version')).toBeInTheDocument()
    })
  })

  it('opens the delete-project panel when the header Delete is clicked', async () => {
    const user = userEvent.setup()
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // The first "Delete" button is the header danger action (setDeleteProjectOpen).
    const headerDelete = screen.getAllByRole('button', { name: 'Delete' })[0]
    await user.click(headerDelete)

    await waitFor(() => {
      expect(screen.getByText('Delete "Mon Projet"')).toBeInTheDocument()
    })
  })

  it('activates a non-active version via the Activate action', async () => {
    const user = userEvent.setup()
    const activated = vi.fn()
    mockProjectDetail(PROJECT_DETAIL)
    server.use(
      http.post(
        `${window.location.origin}/api/projects/1/versions/1/activate`,
        () => {
          activated()
          return HttpResponse.json(
            { ok: true, active_version_id: 10 },
            { status: 200 },
          )
        },
      ),
    )
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Versions')).toBeInTheDocument()
    })

    // Only version n=1 is non-active → exactly one "Activate" button.
    await user.click(screen.getByRole('button', { name: 'Activate' }))

    // activateVersion.mutate({ id: 1, n: 1 }) → POST fired.
    await waitFor(() => expect(activated).toHaveBeenCalledTimes(1))
  })

  it('opens the delete-version panel for a non-active version', async () => {
    const user = userEvent.setup()
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Versions')).toBeInTheDocument()
    })

    // Per-version Delete (n=1, the non-active row) → setDeleteVersion(v) →
    // DeleteVersionPanel mounts.
    const deleteButtons = screen.getAllByRole('button', { name: 'Delete' })
    // Last "Delete" is the per-row one (header Delete is first).
    await user.click(deleteButtons[deleteButtons.length - 1])

    await waitFor(() => {
      expect(screen.getByText('Delete version v1')).toBeInTheDocument()
    })
  })

  it('navigates back to the list via the breadcrumb', async () => {
    const user = userEvent.setup()
    mockProjectDetail(PROJECT_DETAIL)
    renderDetailPage(1)

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // router.navigate({ to: '/' }) → the test list route renders "List".
    await user.click(screen.getByText('‹ Projects'))

    await waitFor(() => {
      expect(screen.getByText('List')).toBeInTheDocument()
    })
  })
})
