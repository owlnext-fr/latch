import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { api } from '@/api/client'
import type { components } from '@/api/schema'

// ────────────────────────────────────────────────────────
// Queries
// ────────────────────────────────────────────────────────

export function useProjects() {
  return useQuery({
    queryKey: ['projects'],
    queryFn: async () => {
      const { data, error } = await api.GET('/api/projects')
      if (error) throw new Error('list')
      return data
    },
  })
}

export function useProject(id: number) {
  return useQuery({
    queryKey: ['project', id],
    queryFn: async () => {
      const { data, error } = await api.GET('/api/projects/{id}', {
        params: { path: { id } },
      })
      if (error) throw new Error('detail')
      return data
    },
  })
}

// ────────────────────────────────────────────────────────
// Mutations
// ────────────────────────────────────────────────────────

export function useCreateProject() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateProjectReq']) => {
      const { data, error } = await api.POST('/api/projects', { body })
      if (error) throw new Error('create')
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      toast.success(t('toast.project_created'))
    },
    onError: () => toast.error(t('error.network')),
  })
}

export function useUpdateProject() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async ({
      id,
      body,
    }: {
      id: number
      body: components['schemas']['UpdateProjectReq']
    }) => {
      const { data, error } = await api.PUT('/api/projects/{id}', {
        params: { path: { id } },
        body,
      })
      if (error) throw new Error('update')
      return data
    },
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      toast.success(t('toast.project_updated'))
    },
    onError: () => toast.error(t('error.network')),
  })
}

export function useDeleteProject() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async (id: number) => {
      const { data, error } = await api.DELETE('/api/projects/{id}', {
        params: { path: { id } },
      })
      if (error) throw new Error('delete')
      return data
    },
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      toast.success(t('toast.project_deleted'))
    },
    onError: () => toast.error(t('error.network')),
  })
}

// useSetCode and useClearCode are intentionally SILENT on success (no toast).
// They are sub-mutations of the ProjectForm save flow: `updateProject` always
// fires first and emits the single "project updated" toast for the whole save.
// Adding a toast here would produce a double-toast on every edit that touches
// the access code. The onError path still shows an error so failures are visible.
export function useSetCode() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async ({
      id,
      pin,
    }: {
      id: number
      pin: string
    }) => {
      const { data, error } = await api.POST('/api/projects/{id}/code', {
        params: { path: { id } },
        body: { pin },
      })
      if (error) throw new Error('set_code')
      return data
    },
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      // No toast: the parent form save (updateProject) already toasted once.
    },
    onError: () => toast.error(t('error.network')),
  })
}

export function useClearCode() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async (id: number) => {
      const { data, error } = await api.DELETE('/api/projects/{id}/code', {
        params: { path: { id } },
      })
      if (error) throw new Error('clear_code')
      return data
    },
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      // No toast: the parent form save (updateProject) already toasted once.
    },
    onError: () => toast.error(t('error.network')),
  })
}

export function useDeploy() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async ({
      id,
      body,
    }: {
      id: number
      body: components['schemas']['DeployReq']
    }) => {
      const { data, error } = await api.POST('/api/projects/{id}/deploy', {
        params: { path: { id } },
        body,
      })
      if (error) throw new Error('deploy')
      return data
    },
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      toast.success(t('toast.version_deployed'))
    },
    onError: () => toast.error(t('error.network')),
  })
}

export function useActivateVersion() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async ({ id, n }: { id: number; n: number }) => {
      const { data, error } = await api.POST(
        '/api/projects/{id}/versions/{n}/activate',
        { params: { path: { id, n } } },
      )
      if (error) throw new Error('activate')
      return data
    },
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      toast.success(t('toast.version_activated'))
    },
    onError: () => toast.error(t('error.network')),
  })
}

export function useDeleteVersion() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async ({ id, n }: { id: number; n: number }) => {
      const { data, error } = await api.DELETE(
        '/api/projects/{id}/versions/{n}',
        { params: { path: { id, n } } },
      )
      if (error) throw new Error('delete_version')
      return data
    },
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      qc.invalidateQueries({ queryKey: ['project', id] })
      toast.success(t('toast.version_deleted'))
    },
    onError: () => toast.error(t('error.network')),
  })
}
