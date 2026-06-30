import { it, expect, vi } from 'vitest'
import { createAdminAdapter } from './admin-adapter'
import type { Mock } from 'vitest'

vi.mock('@/api/client', () => ({
  api: { GET: vi.fn(), DELETE: vi.fn() },
}))
import { api } from '@/api/client'

it('list() mappe AdminCommentMessage to CommentMessage editable:false', async () => {
  ;(api.GET as Mock).mockResolvedValue({
    data: { version: 2, pins: [
      { id: 7, anchor: '{}', created_at: 'x', messages: [
        { id: 11, author_name: 'Lea', body: 'hi', created_at: 'a', updated_at: 'b' },
      ] },
    ] },
    error: undefined,
  })
  const a = createAdminAdapter(3, 2)
  const out = await a.list()
  expect(api.GET).toHaveBeenCalledWith('/api/projects/{id}/versions/{n}/comments', {
    params: { path: { id: 3, n: 2 } },
  })
  expect(out.pins[0].messages[0].editable).toBe(false)
  expect(out.version).toBe(2)
})

it('deleteMessage() calls moderation endpoint', async () => {
  ;(api.DELETE as Mock).mockResolvedValue({ error: undefined })
  await createAdminAdapter(3, 2).deleteMessage(11)
  expect(api.DELETE).toHaveBeenCalledWith('/api/projects/{id}/comments/messages/{cid}', {
    params: { path: { id: 3, cid: 11 } },
  })
})

it('capabilities = canModerate only', () => {
  expect(createAdminAdapter(1, 1).capabilities).toEqual({
    canAuthor: false, canEditOwn: false, canModerate: true,
  })
})

it('createPin/editMessage throw (unsupported in admin)', async () => {
  const a = createAdminAdapter(1, 1)
  await expect(a.createPin({ anchor: '', author_name: '', body: '' })).rejects.toThrow()
  await expect(a.editMessage(1, 'x')).rejects.toThrow()
})
