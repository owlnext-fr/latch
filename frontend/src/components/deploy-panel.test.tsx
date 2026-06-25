import { describe, it, expect, beforeEach, vi } from 'vitest'
import { screen, waitFor, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { renderWithProviders } from '@/test/utils'
import { DeployPanel } from './deploy-panel'

const ORIGIN = globalThis.location.origin

function htmlFile(name = 'proto.html', content = '<html><body>hi</body></html>') {
  return new File([content], name, { type: 'text/html' })
}

describe('DeployPanel', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('shows the idle dropzone text when no file is chosen', () => {
    renderWithProviders(
      <DeployPanel projectId={1} open onOpenChange={() => {}} />,
    )
    // computeDropzoneText() → idle branch.
    expect(
      screen.getByText('Drag an HTML file here, or click to browse'),
    ).toBeInTheDocument()
  })

  it('blocks submit and shows an error when no file is selected', async () => {
    const user = userEvent.setup()
    const deployed = vi.fn()
    server.use(
      http.post(`${ORIGIN}/api/projects/1/deploy`, () => {
        deployed()
        return HttpResponse.json({ id: 1, n: 1 }, { status: 200 })
      }),
    )

    renderWithProviders(
      <DeployPanel projectId={1} open onOpenChange={() => {}} />,
    )

    await user.click(screen.getByRole('button', { name: 'Deploy' }))

    // handleSubmit early-returns on missing file → error message, no network call.
    expect(await screen.findByText('Choose an HTML file.')).toBeInTheDocument()
    expect(deployed).not.toHaveBeenCalled()
  })

  it('reflects a chosen file in the dropzone and deploys, closing the panel', async () => {
    const deployed = vi.fn()
    server.use(
      http.post(`${ORIGIN}/api/projects/1/deploy`, async ({ request }) => {
        const body = (await request.json()) as { html: string; activate: boolean }
        deployed(body)
        return HttpResponse.json({ id: 5, n: 2 }, { status: 200 })
      }),
    )

    const onOpenChange = vi.fn()
    renderWithProviders(
      <DeployPanel projectId={1} open onOpenChange={onOpenChange} />,
    )

    // The hidden file input — pick a file (handleInputChange → acceptFile).
    const input = document.querySelector(
      'input[type="file"]',
    ) as HTMLInputElement
    fireEvent.change(input, { target: { files: [htmlFile()] } })

    // computeDropzoneText() → file-chosen branch shows the file name.
    await waitFor(() =>
      expect(screen.getByText(/proto\.html/)).toBeInTheDocument(),
    )

    // Submit the form → handleSubmit reads file.text() and calls deploy.mutate.
    fireEvent.submit(screen.getByText(/proto\.html/).closest('form')!)

    await waitFor(() => expect(deployed).toHaveBeenCalledTimes(1))
    expect(deployed.mock.calls[0][0]).toMatchObject({
      html: '<html><body>hi</body></html>',
      activate: true,
    })

    // onSuccess closes the panel.
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
  })

  it('shows the hover dropzone text on drag-over', async () => {
    renderWithProviders(
      <DeployPanel projectId={1} open onOpenChange={() => {}} />,
    )

    const dropzone = screen.getByRole('button', {
      name: 'Drag an HTML file here, or click to browse',
    })
    // handleDragOver → isDragOver=true → computeDropzoneText() hover branch +
    // dropzoneBorder primary branch.
    fireEvent.dragOver(dropzone)
    await waitFor(() =>
      expect(screen.getByText('Drop the file to load it')).toBeInTheDocument(),
    )

    // handleDragLeave → back to idle.
    fireEvent.dragLeave(dropzone)
    await waitFor(() =>
      expect(
        screen.getByText('Drag an HTML file here, or click to browse'),
      ).toBeInTheDocument(),
    )
  })

  it('accepts a dropped file via the drop handler', async () => {
    renderWithProviders(
      <DeployPanel projectId={1} open onOpenChange={() => {}} />,
    )

    const dropzone = screen.getByRole('button', {
      name: 'Drag an HTML file here, or click to browse',
    })
    // handleDrop → acceptFile(dropped).
    fireEvent.drop(dropzone, {
      dataTransfer: { files: [htmlFile('dropped.html')] },
    })

    await waitFor(() =>
      expect(screen.getByText(/dropped\.html/)).toBeInTheDocument(),
    )
  })
})
