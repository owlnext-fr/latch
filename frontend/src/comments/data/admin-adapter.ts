import { api } from '@/api/client'
import type {
  Capabilities,
  CommentList,
  CommentMessage,
  CommentPin,
  CommentsAdapter,
} from './adapter'
import type { components } from '@/api/schema'

type AdminCommentMessage = components['schemas']['AdminCommentMessage']
type AdminCommentPin = components['schemas']['AdminCommentPin']

const ADMIN_CAPS: Readonly<Capabilities> = Object.freeze({
  canAuthor: false,
  canEditOwn: false,
  canModerate: true,
})

function toMessage(m: AdminCommentMessage): CommentMessage {
  return {
    id: m.id,
    author_name: m.author_name,
    body: m.body,
    created_at: m.created_at,
    updated_at: m.updated_at,
    editable: false,
  }
}

function toPin(p: AdminCommentPin): CommentPin {
  return {
    id: p.id,
    anchor: p.anchor,
    created_at: p.created_at,
    messages: p.messages.map(toMessage),
  }
}

const UNSUPPORTED = 'admin:unsupported'

export function createAdminAdapter(projectId: number, n: number): CommentsAdapter {
  return {
    capabilities: ADMIN_CAPS,

    async list(): Promise<CommentList> {
      const { data, error } = await api.GET('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
      })
      if (error || !data) throw new Error('comments:admin:list')
      return { version: data.version, pins: data.pins.map(toPin) }
    },

    async createPin() { throw new Error(UNSUPPORTED) },
    async addReply() { throw new Error(UNSUPPORTED) },
    async editMessage() { throw new Error(UNSUPPORTED) },
    async deletePin() { throw new Error(UNSUPPORTED) },

    async deleteMessage(messageId: number): Promise<void> {
      const { error } = await api.DELETE('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
      })
      if (error) throw new Error('comments:admin:deleteMessage')
    },
  }
}
