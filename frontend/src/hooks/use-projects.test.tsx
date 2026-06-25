import { type ReactNode } from 'react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import i18n from '@/i18n'
import {
  useCreateProject,
  useUpdateProject,
  useDeleteProject,
  useSetCode,
  useClearCode,
  useDeploy,
  useActivateVersion,
  useDeleteVersion,
} from './use-projects'

// Hooks exercised here are the 8 mutations of the admin. Each one, on success,
// invalidates the relevant TanStack Query caches and (most) fire a sonner toast.
// These tests drive the SUCCESS path of every mutation so the `onSuccess`
// callbacks (qc.invalidateQueries + toast) are actually executed — the part the
// existing route/component tests never reach.

const ORIGIN = globalThis.location.origin

// We assert real behaviour, not just the mock: a spy on QueryClient.invalidateQueries
// proves the cache invalidation contract, and the resolved mutation data proves the
// request actually round-tripped through the typed client.

function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  const invalidateSpy = vi.spyOn(qc, 'invalidateQueries')
  function Wrapper({ children }: Readonly<{ children: ReactNode }>) {
    return (
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={qc}>{children}</QueryClientProvider>
      </I18nextProvider>
    )
  }
  return { Wrapper, qc, invalidateSpy }
}

describe('use-projects mutations — success path', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('useCreateProject invalidates the projects list on success', async () => {
    server.use(
      http.post(`${ORIGIN}/api/projects`, () =>
        HttpResponse.json(
          { id: 7, name: 'Mon Projet', slug: 'mon-projet-aB3dEf9z' },
          { status: 200 },
        ),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useCreateProject(), { wrapper: Wrapper })

    result.current.mutate({ name: 'Mon Projet' })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(result.current.data).toMatchObject({ id: 7 })
  })

  it('useUpdateProject invalidates both list and the project detail', async () => {
    server.use(
      http.put(`${ORIGIN}/api/projects/42`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useUpdateProject(), { wrapper: Wrapper })

    result.current.mutate({ id: 42, body: { name: 'Renamed' } })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
  })

  it('useDeleteProject invalidates list and detail for the deleted id', async () => {
    server.use(
      http.delete(`${ORIGIN}/api/projects/42`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useDeleteProject(), { wrapper: Wrapper })

    result.current.mutate(42)

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
  })

  it('useSetCode invalidates list and detail (silent: no toast)', async () => {
    server.use(
      http.post(`${ORIGIN}/api/projects/42/code`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useSetCode(), { wrapper: Wrapper })

    result.current.mutate({ id: 42, pin: '123456' })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
  })

  it('useClearCode invalidates list and detail (silent: no toast)', async () => {
    server.use(
      http.delete(`${ORIGIN}/api/projects/42/code`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useClearCode(), { wrapper: Wrapper })

    result.current.mutate(42)

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
  })

  it('useDeploy invalidates list and detail and returns the new version', async () => {
    server.use(
      http.post(`${ORIGIN}/api/projects/42/deploy`, () =>
        HttpResponse.json({ id: 100, n: 3 }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useDeploy(), { wrapper: Wrapper })

    result.current.mutate({ id: 42, body: { html: '<p>hi</p>', activate: true } })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
    expect(result.current.data).toMatchObject({ n: 3 })
  })

  it('useActivateVersion invalidates list and detail', async () => {
    server.use(
      http.post(`${ORIGIN}/api/projects/42/versions/3/activate`, () =>
        HttpResponse.json({ ok: true, active_version_id: 100 }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useActivateVersion(), {
      wrapper: Wrapper,
    })

    result.current.mutate({ id: 42, n: 3 })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
  })

  it('useDeleteVersion invalidates list and detail', async () => {
    server.use(
      http.delete(`${ORIGIN}/api/projects/42/versions/3`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useDeleteVersion(), { wrapper: Wrapper })

    result.current.mutate({ id: 42, n: 3 })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['projects'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 42] })
  })
})
