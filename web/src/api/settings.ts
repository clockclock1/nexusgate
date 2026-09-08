import api from './client'
import type { AppSettings } from '@/types'

export const settingsApi = {
  get: async () => {
    const res = await api.get<AppSettings>('/settings')
    return res.data
  },
  update: async (data: Partial<AppSettings>) => {
    const res = await api.put<AppSettings>('/settings', data)
    return res.data
  },
}
