import { it, expect, vi } from 'vitest'
import { createAdminAdapter } from './admin-adapter'
import type { Mock } from 'vitest'

vi.mock('@/api/client', () => ({
  api: { GET: vi.fn(), POST: vi.fn(), PUT: vi.fn(), DELETE: vi.fn() },
}))
import { api } from '@/api/client'

it('list() mappe is_admin -> editable', async () => {
  ;(api.GET as Mock).mockResolvedValue({
    data: {
      version: 2,
      pins: [
        {
          id: 7,
          anchor: '{}',
          created_at: 'x',
          messages: [
            { id: 11, author_name: 'admin', body: 'hi', created_at: 'a', updated_at: 'b', is_admin: true },
            { id: 12, author_name: 'Lea', body: 'yo', created_at: 'a', updated_at: 'b', is_admin: false },
          ],
        },
      ],
    },
    error: undefined,
  })
  const out = await createAdminAdapter(3, 2, 'Admin').list()
  expect(out.pins[0].messages[0].editable).toBe(true) // message admin
  expect(out.pins[0].messages[0].is_admin).toBe(true)
  expect(out.pins[0].messages[1].editable).toBe(false) // message visiteur
})

it('createPin POSTs anchor+body (pas de author_name)', async () => {
  ;(api.POST as Mock).mockResolvedValue({
    data: { id: 9, anchor: '{}', created_at: 'x', messages: [] },
    error: undefined,
  })
  await createAdminAdapter(3, 2, 'Admin').createPin({ anchor: '{"v":1}', author_name: 'ignoré', body: 'note' })
  expect(api.POST).toHaveBeenCalledWith('/api/projects/{id}/versions/{n}/comments', {
    params: { path: { id: 3, n: 2 } },
    body: { anchor: '{"v":1}', body: 'note' },
  })
})

it('addReply POSTs body au pin', async () => {
  ;(api.POST as Mock).mockResolvedValue({
    data: { id: 15, author_name: 'admin', body: 'r', created_at: 'a', updated_at: 'b', is_admin: true },
    error: undefined,
  })
  await createAdminAdapter(3, 2, 'Admin').addReply(7, { author_name: 'ignoré', body: 'r' })
  expect(api.POST).toHaveBeenCalledWith('/api/projects/{id}/comments/pins/{pin}/replies', {
    params: { path: { id: 3, pin: 7 } },
    body: { body: 'r' },
  })
})

it('editMessage PUTs body', async () => {
  ;(api.PUT as Mock).mockResolvedValue({
    data: { id: 11, author_name: 'admin', body: 'edited', created_at: 'a', updated_at: 'b', is_admin: true },
    error: undefined,
  })
  await createAdminAdapter(3, 2, 'Admin').editMessage(11, 'edited')
  expect(api.PUT).toHaveBeenCalledWith('/api/projects/{id}/comments/messages/{cid}', {
    params: { path: { id: 3, cid: 11 } },
    body: { body: 'edited' },
  })
})

it('deletePin DELETEs le pin propre', async () => {
  ;(api.DELETE as Mock).mockResolvedValue({ error: undefined })
  await createAdminAdapter(3, 2, 'Admin').deletePin(7)
  expect(api.DELETE).toHaveBeenCalledWith('/api/projects/{id}/comments/pins/{pin}', {
    params: { path: { id: 3, pin: 7 } },
  })
})

it('capabilities = authoring complet + moderation, fixedAuthorName = label', () => {
  const a = createAdminAdapter(1, 1, 'Admin')
  expect(a.capabilities).toEqual({ canAuthor: true, canEditOwn: true, canModerate: true })
  expect(a.fixedAuthorName).toBe('Admin')
})
