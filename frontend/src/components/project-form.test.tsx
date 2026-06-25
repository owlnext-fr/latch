import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { renderWithProviders } from '@/test/utils'
import { ProjectForm } from './project-form'

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
    await user.click(screen.getByRole('switch'))

    await waitFor(() => {
      expect(screen.getByLabelText('PIN (6 digits)')).toBeDisabled()
    })
  })
})
