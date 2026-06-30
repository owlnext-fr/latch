import { describe, expect, it } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { createVisitorAdapter } from './visitor-adapter'

const ORIGIN = globalThis.location.origin
const SLUG = 'mon-projet-aB3dEf9z'

describe('visitor adapter', () => {
  it('list() fetches the visitor comment list', async () => {
    server.use(
      http.get(`${ORIGIN}/c/${SLUG}/comments`, () =>
        HttpResponse.json({ version: 3, pins: [] }, { status: 200 }),
      ),
    )
    const adapter = createVisitorAdapter(SLUG)
    const list = await adapter.list()
    expect(list.version).toBe(3)
    expect(list.pins).toEqual([])
  })

  it('createPin() POSTs with the X-Comment-Client header', async () => {
    let seenHeader: string | null = null
    server.use(
      http.post(`${ORIGIN}/c/${SLUG}/comments`, ({ request }) => {
        seenHeader = request.headers.get('X-Comment-Client')
        return HttpResponse.json(
          { id: 12, anchor: '{}', created_at: 'now', messages: [] },
          { status: 200 },
        )
      }),
    )
    const adapter = createVisitorAdapter(SLUG)
    const pin = await adapter.createPin({ anchor: '{}', author_name: 'Léa', body: 'Hi' })
    expect(pin.id).toBe(12)
    expect(seenHeader).toBe('1')
  })

  it('deleteMessage() resolves on ok response', async () => {
    server.use(
      http.delete(`${ORIGIN}/c/${SLUG}/comments/messages/31`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    await expect(createVisitorAdapter(SLUG).deleteMessage(31)).resolves.toBeUndefined()
  })

  it('list() rejects on a 403 (locked project)', async () => {
    server.use(
      http.get(`${ORIGIN}/c/${SLUG}/comments`, () => new HttpResponse(null, { status: 403 })),
    )
    await expect(createVisitorAdapter(SLUG).list()).rejects.toThrow()
  })

  it('addReply() POSTs to the pin replies route with the header', async () => {
    let seen: string | null = null
    server.use(
      http.post(`${ORIGIN}/c/${SLUG}/comments/pins/7/replies`, ({ request }) => {
        seen = request.headers.get('X-Comment-Client')
        return HttpResponse.json(
          { id: 99, author_name: 'Léa', body: 'Re', created_at: 'n', updated_at: 'n', editable: true },
          { status: 200 },
        )
      }),
    )
    const msg = await createVisitorAdapter(SLUG).addReply(7, { author_name: 'Léa', body: 'Re' })
    expect(msg.id).toBe(99)
    expect(seen).toBe('1')
  })

  it('editMessage() PUTs to the message route with the header', async () => {
    let seen: string | null = null
    server.use(
      http.put(`${ORIGIN}/c/${SLUG}/comments/messages/31`, ({ request }) => {
        seen = request.headers.get('X-Comment-Client')
        return HttpResponse.json(
          { id: 31, author_name: 'Léa', body: 'Edited', created_at: 'n', updated_at: 'n', editable: true },
          { status: 200 },
        )
      }),
    )
    const msg = await createVisitorAdapter(SLUG).editMessage(31, 'Edited')
    expect(msg.body).toBe('Edited')
    expect(seen).toBe('1')
  })

  it('deletePin() DELETEs the pin route with the header', async () => {
    let seen: string | null = null
    server.use(
      http.delete(`${ORIGIN}/c/${SLUG}/comments/pins/7`, ({ request }) => {
        seen = request.headers.get('X-Comment-Client')
        return HttpResponse.json({ ok: true }, { status: 200 })
      }),
    )
    await expect(createVisitorAdapter(SLUG).deletePin(7)).resolves.toBeUndefined()
    expect(seen).toBe('1')
  })

  it('exposes visitor capabilities', () => {
    expect(createVisitorAdapter(SLUG).capabilities).toEqual({
      canAuthor: true,
      canEditOwn: true,
      canModerate: false,
    })
  })
})
