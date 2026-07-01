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
  canAuthor: true,
  canEditOwn: true,
  canModerate: true,
})

function toMessage(m: AdminCommentMessage): CommentMessage {
  return {
    id: m.id,
    author_name: m.author_name,
    body: m.body,
    created_at: m.created_at,
    updated_at: m.updated_at,
    editable: m.is_admin,
    is_admin: m.is_admin,
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

export function createAdminAdapter(
  projectId: number,
  n: number,
  authorLabel: string,
): CommentsAdapter {
  return {
    capabilities: ADMIN_CAPS,
    fixedAuthorName: authorLabel,

    async list(): Promise<CommentList> {
      const { data, error } = await api.GET('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
      })
      if (error || !data) throw new Error('comments:admin:list')
      return { version: data.version, pins: data.pins.map(toPin) }
    },

    async createPin(input): Promise<CommentPin> {
      const { data, error } = await api.POST('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
        body: { anchor: input.anchor, body: input.body },
      })
      if (error || !data) throw new Error('comments:admin:createPin')
      return toPin(data)
    },

    async addReply(pinId, input): Promise<CommentMessage> {
      const { data, error } = await api.POST('/api/projects/{id}/comments/pins/{pin}/replies', {
        params: { path: { id: projectId, pin: pinId } },
        body: { body: input.body },
      })
      if (error || !data) throw new Error('comments:admin:addReply')
      return toMessage(data)
    },

    async editMessage(messageId, body): Promise<CommentMessage> {
      const { data, error } = await api.PUT('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
        body: { body },
      })
      if (error || !data) throw new Error('comments:admin:editMessage')
      return toMessage(data)
    },

    async deleteMessage(messageId: number): Promise<void> {
      const { error } = await api.DELETE('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
      })
      if (error) throw new Error('comments:admin:deleteMessage')
    },

    async deletePin(pinId: number): Promise<void> {
      const { error } = await api.DELETE('/api/projects/{id}/comments/pins/{pin}', {
        params: { path: { id: projectId, pin: pinId } },
      })
      if (error) throw new Error('comments:admin:deletePin')
    },
  }
}
