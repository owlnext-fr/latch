import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { renderWithProviders } from '@/test/utils'
import type { components } from '@/api/schema'
import { ProjectForm } from './project-form'

type ProjectDetail = components['schemas']['ProjectDetail']

const baseProject: ProjectDetail = {
  id: 1,
  name: 'Mon Projet',
  slug: 'mon-projet',
  code_enabled: true,
  comments_enabled: true,
  pin: '000000',
  brand_name: null,
  versions: [],
}

function renderForm(
  opts: { mode: 'create' } | { mode: 'edit'; project: ProjectDetail },
) {
  const project = opts.mode === 'edit' ? opts.project : undefined
  renderWithProviders(
    <ProjectForm open mode={opts.mode} project={project} onOpenChange={() => {}} />,
  )
}

function mockCreate() {
  const spy = vi.fn()
  server.use(
    http.post('/api/projects', async ({ request }) => {
      const body = await request.json()
      spy(body)
      return HttpResponse.json(
        { ...baseProject, ...(body as object) },
        { status: 200 },
      )
    }),
  )
  return spy
}

describe('ProjectForm — validation', () => {
  it('blocks submit on empty name with form.err_name and no network call', async () => {
    const user = userEvent.setup()
    const created = vi.fn()
    server.use(
      http.post('/api/projects', () => {
        created()
        return HttpResponse.json({}, { status: 200 })
      }),
    )

    renderWithProviders(
      <ProjectForm open mode="create" onOpenChange={() => {}} />,
    )

    // Clear the (empty by default) name field to be explicit, then submit.
    const name = screen.getByLabelText('Name')
    await user.clear(name)
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('Name is required.')).toBeInTheDocument()
    expect(created).not.toHaveBeenCalled()
  })

  it('rejects a 5-digit PIN when code is enabled with form.err_pin', async () => {
    const user = userEvent.setup()
    const created = vi.fn()
    server.use(
      http.post('/api/projects', () => {
        created()
        return HttpResponse.json({}, { status: 200 })
      }),
    )

    renderWithProviders(
      <ProjectForm open mode="create" onOpenChange={() => {}} />,
    )

    await user.type(screen.getByLabelText('Name'), 'My Project')

    // Code is ON by default in create mode; force the PIN to 5 digits.
    const pin = screen.getByLabelText('PIN (6 digits)')
    await user.clear(pin)
    await user.type(pin, '12345')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(
      await screen.findByText('The PIN must be 6 digits.'),
    ).toBeInTheDocument()
    expect(created).not.toHaveBeenCalled()
  })

  it('keeps the PIN field rendered but disabled when code is toggled off', async () => {
    const user = userEvent.setup()

    renderWithProviders(
      <ProjectForm open mode="create" onOpenChange={() => {}} />,
    )

    const pin = screen.getByLabelText('PIN (6 digits)')
    expect(pin).toBeEnabled()

    // Toggle the access-code switch OFF.
    await user.click(screen.getByRole('switch', { name: /access code/i }))

    await waitFor(() => {
      expect(screen.getByLabelText('PIN (6 digits)')).toBeDisabled()
    })
  })
})

describe('ProjectForm — comments_enabled', () => {
  it('création : comments suit le code tant que non touché', async () => {
    const user = userEvent.setup()
    renderForm({ mode: 'create' })
    const code = screen.getByRole('switch', { name: /access code/i })
    const comments = screen.getByRole('switch', { name: /comment/i })
    expect(comments).toBeChecked() // code ON par défaut → comments ON
    await user.click(code) // code OFF
    await waitFor(() => {
      expect(screen.getByRole('switch', { name: /comment/i })).not.toBeChecked()
    })
  })

  it("création : une fois touché, comments n'est plus piloté par le code", async () => {
    const user = userEvent.setup()
    renderForm({ mode: 'create' })
    const code = screen.getByRole('switch', { name: /access code/i })
    const comments = screen.getByRole('switch', { name: /comment/i })
    await user.click(comments) // touché → false
    await user.click(code) // code OFF
    await waitFor(() => {
      expect(screen.getByRole('switch', { name: /comment/i })).not.toBeChecked()
    })
    await user.click(code) // code ON
    await waitFor(() => {
      expect(screen.getByRole('switch', { name: /comment/i })).not.toBeChecked() // ne re-suit plus
    })
  })

  it('édition : warning si commentaires ON et code passé OFF', async () => {
    const user = userEvent.setup()
    renderForm({
      mode: 'edit',
      project: { ...baseProject, code_enabled: true, comments_enabled: true },
    })
    await user.click(screen.getByRole('switch', { name: /access code/i })) // code → OFF
    expect(await screen.findByText(/spam-protected|publicly writable/i)).toBeInTheDocument()
  })

  it('soumission création envoie comments_enabled', async () => {
    const user = userEvent.setup()
    const create = mockCreate()
    renderForm({ mode: 'create' })
    await user.type(screen.getByLabelText(/^name$/i), 'Mon Projet')
    await user.click(screen.getByRole('button', { name: /save/i }))
    await waitFor(() => {
      expect(create).toHaveBeenCalledWith(
        expect.objectContaining({ comments_enabled: true }),
      )
    })
  })
})
