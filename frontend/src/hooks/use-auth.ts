import { useMutation } from '@tanstack/react-query'
import { api } from '@/api/client'

export function useLogin() {
  return useMutation({
    mutationFn: async (body: { user: string; pass: string }) => {
      const { data, error, response } = await api.POST('/api/login', { body })
      if (error || !response.ok) throw new Error(String(response.status))
      return data
    },
  })
}

export function useLogout() {
  return useMutation({
    mutationFn: async () => {
      await api.POST('/api/logout')
    },
  })
}
