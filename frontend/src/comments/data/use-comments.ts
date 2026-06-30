import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from '@tanstack/react-query'
import type { CommentList, CommentsAdapter } from './adapter'

export function commentsKey(slug: string): unknown[] {
  return ['comments', slug]
}

export function useCommentList(
  slug: string,
  adapter: CommentsAdapter,
): UseQueryResult<CommentList> {
  return useQuery({
    queryKey: commentsKey(slug),
    queryFn: () => adapter.list(),
  })
}

export function useCreatePin(slug: string, adapter: CommentsAdapter) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (input: { anchor: string; author_name: string; body: string }) =>
      adapter.createPin(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export function useAddReply(slug: string, adapter: CommentsAdapter) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { pinId: number; author_name: string; body: string }) =>
      adapter.addReply(v.pinId, { author_name: v.author_name, body: v.body }),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export function useEditMessage(slug: string, adapter: CommentsAdapter) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { messageId: number; body: string }) =>
      adapter.editMessage(v.messageId, v.body),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export function useDeleteMessage(slug: string, adapter: CommentsAdapter) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (messageId: number) => adapter.deleteMessage(messageId),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export function useDeletePin(slug: string, adapter: CommentsAdapter) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (pinId: number) => adapter.deletePin(pinId),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}
