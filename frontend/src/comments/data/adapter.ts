import type { components } from '@/api/schema'

export type CommentList = components['schemas']['CommentList']
export type CommentPin = components['schemas']['CommentPin']
export type CommentMessage = components['schemas']['CommentMessage']

/** Capacités de l'appelant — pilotent l'UI (l'autorisation réelle vit au backend). */
export interface Capabilities {
  canAuthor: boolean
  canEditOwn: boolean
  canModerate: boolean
}

/** Façade de données partagée par le visiteur (et plus tard l'admin, Plan 3). */
export interface CommentsAdapter {
  readonly capabilities: Capabilities
  /** Nom d'auteur imposé (admin) ; `null` = l'appelant saisit son nom (visiteur). */
  readonly fixedAuthorName: string | null
  list(): Promise<CommentList>
  createPin(input: { anchor: string; author_name: string; body: string }): Promise<CommentPin>
  addReply(pinId: number, input: { author_name: string; body: string }): Promise<CommentMessage>
  editMessage(messageId: number, body: string): Promise<CommentMessage>
  deleteMessage(messageId: number): Promise<void>
  deletePin(pinId: number): Promise<void>
}
