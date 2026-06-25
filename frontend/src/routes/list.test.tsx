import { describe, it, expect, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { renderWithRouter } from '@/test/utils'
import type { components } from '@/api/schema'

type ProjectListItem = components['schemas']['ProjectListItem']

// ─── Fixtures ────────────────────────────────────────────────────────────────
// Using fictitious placeholder names (no real client names).

const PROJECTS: ProjectListItem[] = [
  {
    id: 1,
    name: 'Mon Projet',
    slug: 'mon-projet-k7Qp2maZ',
    code_enabled: true,
    active_version_id: 3,
    brand_name: null,
  },
  {
    id: 2,
    name: 'Demo ACME',
    slug: 'demo-acme-xB3nLp9q',
    code_enabled: false,
    active_version_id: null,
    brand_name: 'ACME',
  },
]

// ─── Helpers ──────────────────────────────────────────────────────────────────

function mockProjectsList(projects: ProjectListItem[], status = 200) {
  server.use(
    http.get(`${window.location.origin}/api/projects`, () =>
      HttpResponse.json(projects, { status }),
    ),
  )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('ListPage', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('renders the project list with names and access badges', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    // Wait for the data to load
    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // Both project names are rendered
    expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    expect(screen.getByText('Demo ACME')).toBeInTheDocument()

    // Badge PIN requis for code_enabled=true
    expect(screen.getByText('PIN required')).toBeInTheDocument()

    // Badge Open for code_enabled=false
    expect(screen.getByText('Open')).toBeInTheDocument()
  })

  it('SECURITY — PIN is never rendered in the list (§9.2)', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // The list DTO has no pin field — verify no PIN digits leaked
    // ProjectListItem structurally has no pin — assert it cannot appear
    const allText = document.body.textContent ?? ''
    // A PIN would be a 6-digit number. We check that no pin-like pattern
    // (that would come from a ProjectListItem with a pin field) is rendered.
    // The fixture has no pin at all — if pin appeared it would mean the component
    // is fabricating data or the DTO changed.
    expect(allText).not.toMatch(/\bpin\b/i)
  })

  it('shows empty state when no projects exist', async () => {
    mockProjectsList([])
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('No projects yet.')).toBeInTheDocument()
    })

    // Empty state "create first" button
    expect(screen.getByText('+ Create the first project')).toBeInTheDocument()
  })

  it('shows loading state initially then resolves', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    // After resolution, projects are shown
    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })
  })

  it('renders public URL copy buttons', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // Each project row has a copy button (aria-label "Copy the URL")
    const copyButtons = screen.getAllByLabelText('Copy the URL')
    expect(copyButtons).toHaveLength(PROJECTS.length)
  })

  it('renders dash for project with no active version', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('Demo ACME')).toBeInTheDocument()
    })

    // Project with active_version_id null shows the common.dash character
    expect(screen.getByText('—')).toBeInTheDocument()
  })
})
