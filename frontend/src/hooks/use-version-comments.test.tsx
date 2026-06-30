import { type ReactNode } from 'react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import {
  useVersionComments,
  useModerateComment,
  versionCommentsKey,
} from './use-version-comments'

const ORIGIN = globalThis.location.origin

function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  const invalidateSpy = vi.spyOn(qc, 'invalidateQueries')
  function Wrapper({ children }: Readonly<{ children: ReactNode }>) {
    return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  }
  return { Wrapper, qc, invalidateSpy }
}

describe('use-version-comments', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  it('useVersionComments fetch la liste admin', async () => {
    server.use(
      http.get(`${ORIGIN}/api/projects/3/versions/2/comments`, () =>
        HttpResponse.json({ version: 2, pins: [] }),
      ),
    )
    const { Wrapper } = makeWrapper()
    const { result } = renderHook(() => useVersionComments(3, 2), {
      wrapper: Wrapper,
    })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.version).toBe(2)
  })

  it('useModerateComment DELETE le message', async () => {
    let called = false
    server.use(
      http.delete(`${ORIGIN}/api/projects/3/comments/messages/11`, () => {
        called = true
        return HttpResponse.json({ ok: true })
      }),
    )
    const { Wrapper } = makeWrapper()
    const { result } = renderHook(() => useModerateComment(3, 2), {
      wrapper: Wrapper,
    })
    await result.current.mutateAsync(11)
    expect(called).toBe(true)
  })

  it('useModerateComment invalide versionComments et le projet', async () => {
    server.use(
      http.delete(`${ORIGIN}/api/projects/3/comments/messages/11`, () =>
        HttpResponse.json({ ok: true }),
      ),
    )
    const { Wrapper, invalidateSpy } = makeWrapper()
    const { result } = renderHook(() => useModerateComment(3, 2), {
      wrapper: Wrapper,
    })
    await result.current.mutateAsync(11)
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: versionCommentsKey(3, 2),
    })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['project', 3] })
  })
})
