import { api } from '@/api/client'
import type {
  Capabilities,
  CommentList,
  CommentMessage,
  CommentPin,
  CommentsAdapter,
} from './adapter'

/** En-tête anti-CSRF exigé par le backend sur tous les writes commentaires. */
const WRITE_HEADERS = { 'X-Comment-Client': '1' }

const VISITOR_CAPS: Capabilities = Object.freeze({
  canAuthor: true,
  canEditOwn: true,
  canModerate: false,
})

export function createVisitorAdapter(slug: string): CommentsAdapter {
  return {
    capabilities: VISITOR_CAPS,

    async list(): Promise<CommentList> {
      const { data, error } = await api.GET('/c/{slug}/comments', {
        params: { path: { slug } },
      })
      if (error || !data) throw new Error('comments:list')
      return data
    },

    async createPin(input): Promise<CommentPin> {
      const { data, error } = await api.POST('/c/{slug}/comments', {
        params: { path: { slug } },
        body: input,
        headers: { ...WRITE_HEADERS } as Record<string, string>,
      })
      if (error || !data) throw new Error('comments:createPin')
      return data
    },

    async addReply(pinId, input): Promise<CommentMessage> {
      const { data, error } = await api.POST('/c/{slug}/comments/pins/{pin}/replies', {
        params: { path: { slug, pin: pinId } },
        body: input,
        headers: { ...WRITE_HEADERS } as Record<string, string>,
      })
      if (error || !data) throw new Error('comments:addReply')
      return data
    },

    async editMessage(messageId, body): Promise<CommentMessage> {
      const { data, error } = await api.PUT('/c/{slug}/comments/messages/{id}', {
        params: { path: { slug, id: messageId } },
        body: { body },
        headers: { ...WRITE_HEADERS } as Record<string, string>,
      })
      if (error || !data) throw new Error('comments:editMessage')
      return data
    },

    async deleteMessage(messageId): Promise<void> {
      const { error } = await api.DELETE('/c/{slug}/comments/messages/{id}', {
        params: { path: { slug, id: messageId } },
        headers: { ...WRITE_HEADERS } as Record<string, string>,
      })
      if (error) throw new Error('comments:deleteMessage')
    },

    async deletePin(pinId): Promise<void> {
      const { error } = await api.DELETE('/c/{slug}/comments/pins/{pin}', {
        params: { path: { slug, pin: pinId } },
        headers: { ...WRITE_HEADERS } as Record<string, string>,
      })
      if (error) throw new Error('comments:deletePin')
    },
  }
}
