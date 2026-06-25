import { useQuery } from '@tanstack/react-query'
import { api } from '@/api/client'

export function useSettings(enabled = true) {
  return useQuery({
    queryKey: ['settings'],
    enabled,
    queryFn: async () => {
      const { data, error } = await api.GET('/api/settings')
      if (error) throw new Error('settings')
      return data
    },
  })
}
