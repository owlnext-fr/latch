import { describe, expect, it, vi } from 'vitest'
import { type ReactNode } from 'react'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useCommentList, useCreatePin, commentsKey } from './use-comments'
import type { CommentsAdapter } from './adapter'

function fakeAdapter(over: Partial<CommentsAdapter> = {}): CommentsAdapter {
  return {
    capabilities: { canAuthor: true, canEditOwn: true, canModerate: false },
    fixedAuthorName: null,
    list: vi.fn().mockResolvedValue({ version: 1, pins: [] }),
    createPin: vi.fn().mockResolvedValue({ id: 1, anchor: '{}', created_at: 'n', messages: [] }),
    addReply: vi.fn(),
    editMessage: vi.fn(),
    deleteMessage: vi.fn(),
    deletePin: vi.fn(),
    ...over,
  }
}

function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  const invalidate = vi.spyOn(qc, 'invalidateQueries')
  function Wrapper({ children }: Readonly<{ children: ReactNode }>) {
    return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  }
  return { Wrapper, invalidate }
}

describe('use-comments hooks', () => {
  it('useCommentList loads via the adapter', async () => {
    const adapter = fakeAdapter()
    const { Wrapper } = makeWrapper()
    const { result } = renderHook(() => useCommentList('demo', adapter), { wrapper: Wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.version).toBe(1)
  })

  it('useCreatePin invalidates the comment list on success', async () => {
    const adapter = fakeAdapter()
    const { Wrapper, invalidate } = makeWrapper()
    const { result } = renderHook(() => useCreatePin('demo', adapter), { wrapper: Wrapper })
    result.current.mutate({ anchor: '{}', author_name: 'Léa', body: 'Hi' })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidate).toHaveBeenCalledWith({ queryKey: commentsKey('demo') })
  })
})
