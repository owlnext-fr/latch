import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api/client'
import type { components } from '@/api/schema'

type AdminCommentList = components['schemas']['AdminCommentList']

export function versionCommentsKey(projectId: number, n: number): unknown[] {
  return ['admin-version-comments', projectId, n]
}

export function useVersionComments(
  projectId: number,
  n: number,
): UseQueryResult<AdminCommentList> {
  return useQuery({
    queryKey: versionCommentsKey(projectId, n),
    queryFn: async () => {
      const { data, error } = await api.GET(
        '/api/projects/{id}/versions/{n}/comments',
        {
          params: { path: { id: projectId, n } },
        },
      )
      if (error || !data) throw new Error('admin:version-comments')
      return data
    },
  })
}

export function useModerateComment(projectId: number, n: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (messageId: number) => {
      const { error } = await api.DELETE(
        '/api/projects/{id}/comments/messages/{cid}',
        {
          params: { path: { id: projectId, cid: messageId } },
        },
      )
      if (error) throw new Error('admin:moderate')
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: versionCommentsKey(projectId, n) })
      void qc.invalidateQueries({ queryKey: ['project', projectId] })
    },
  })
}
