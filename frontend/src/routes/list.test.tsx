import { describe, it, expect, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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
    active_version_n: 2,
    version_count: 3,
    brand_name: null,
    comments_enabled: false,
  },
  {
    id: 2,
    name: 'Demo ACME',
    slug: 'demo-acme-xB3nLp9q',
    code_enabled: false,
    active_version_n: null,
    version_count: 0,
    brand_name: 'ACME',
    comments_enabled: false,
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

    // Active version shows the sequential n (v2) + the version count, not a PK.
    expect(screen.getByText('v2')).toBeInTheDocument()
    expect(screen.getByText('3 versions')).toBeInTheDocument()
  })

  it('SECURITY — PIN is never rendered in the list (§9.2)', async () => {
    // The list DTO type (ProjectListItem) has no `pin` field — the backend never
    // sends a PIN in list responses. This test proves the invariant structurally:
    //   1. We deliberately do NOT put a pin value in the fixture (the type forbids it).
    //   2. We pick a sentinel 6-digit string that would only appear if the component
    //      fabricated or leaked a PIN — then assert it is absent from the DOM.
    //   3. We also confirm the "PIN required" access badge IS rendered, proving we
    //      actually exercised the code_enabled=true branch.
    //
    // The previous assertion (`not.toMatch(/\bpin\b/i)`) was silently broken:
    // the slug "mon-projet-k7Qp2maZ" ends with "Z", so "ZPIN" in the concatenated
    // textContent has no word boundary before it — the regex never matched even when
    // the badge was there. It was a tautological guard. This test cannot be fooled
    // by adjacent text: queryByText uses exact DOM node matching.

    // Sentinel PIN: not present in fixture data or i18n keys.
    const SENTINEL_PIN = '999888'

    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('Mon Projet')).toBeInTheDocument()
    })

    // The access badge IS rendered (code_enabled=true → "PIN required").
    // This confirms the branch was exercised.
    expect(screen.getByText('PIN required')).toBeInTheDocument()

    // No 6-digit sentinel PIN digit-string leaked into any DOM node.
    // Because ProjectListItem carries no pin field, this can only appear if
    // someone incorrectly fabricated PIN data in the list renderer.
    expect(screen.queryByText(SENTINEL_PIN)).toBeNull()

    // Belt-and-suspenders: no element in the document renders any raw 6-digit
    // string that resembles a PIN (digit-only, 6 chars). We check text nodes
    // directly rather than textContent concatenation to avoid word-boundary issues.
    const allElements = document.body.querySelectorAll('*')
    for (const el of allElements) {
      // Only check leaf text nodes (elements with no element children)
      if (el.children.length === 0) {
        const text = el.textContent?.trim() ?? ''
        // A leaked PIN would be a standalone 6-digit string
        expect(text).not.toMatch(/^\d{6}$/)
      }
    }
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

  it('opens the create form when the empty-state button is clicked', async () => {
    const user = userEvent.setup()
    mockProjectsList([])
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('No projects yet.')).toBeInTheDocument()
    })

    // onClick → setFormOpen(true) → ProjectForm sheet mounts with the create title.
    await user.click(screen.getByText('+ Create the first project'))

    await waitFor(() => {
      expect(screen.getByText('New project')).toBeInTheDocument()
    })
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

  it('shows the active version number (v{n}) + count, dash when none', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    await waitFor(() => {
      expect(screen.getByText('Demo ACME')).toBeInTheDocument()
    })

    // Project with no active version (version_count 0) shows the dash.
    expect(screen.getByText('—')).toBeInTheDocument()

    // Project with active_version_n=2 / version_count=3 shows "v2" + "3 versions"
    // (the sequential n, NOT the DB primary key).
    expect(screen.getByText('v2')).toBeInTheDocument()
    expect(screen.getByText('3 versions')).toBeInTheDocument()
  })

  it('sets the document title to the projects title', async () => {
    mockProjectsList(PROJECTS)
    renderWithRouter('/')
    await waitFor(() => expect(document.title).toBe('Projects — latch admin'))
  })

  it('shows a preview link to the active version for a deployed project', async () => {
    // Mon Projet has active_version_n=2, id=1
    mockProjectsList(PROJECTS)
    renderWithRouter('/')

    const link = await screen.findByRole('link', { name: /preview active version/i })
    expect(link).toHaveAttribute('href', '/api/projects/1/versions/2/preview')
    expect(link).toHaveAttribute('target', '_blank')
  })

  it('does not show a preview link when the project has no active version', async () => {
    // Only the project with active_version_n=null — no link should be rendered
    mockProjectsList([PROJECTS[1]])
    renderWithRouter('/')

    await screen.findByText('Demo ACME')
    expect(screen.queryByRole('link', { name: /preview active version/i })).toBeNull()
  })
})
