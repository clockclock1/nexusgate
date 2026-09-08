import api from './client'
import type { UserInfo } from '@/types'

export const usersApi = {
  list: async () => {
    const res = await api.get<UserInfo[]>('/users')
    return res.data
  },
  create: async (data: Partial<UserInfo> & { password?: string }) => {
    const res = await api.post<UserInfo>('/users', data)
    return res.data
  },
  update: async (id: string, data: Partial<UserInfo> & { password?: string }) => {
    const res = await api.put<UserInfo>(`/users/${id}`, data)
    return res.data
  },
  remove: async (id: string) => {
    await api.delete(`/users/${id}`)
  },
}
