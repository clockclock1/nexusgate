import api from './client'
import type { RouteInfo } from '@/types'

export const routesApi = {
  list: async (params?: { node_id?: string }) => {
    const res = await api.get<RouteInfo[]>('/routes', { params })
    return res.data
  },
  get: async (id: string) => {
    const res = await api.get<RouteInfo>(`/routes/${id}`)
    return res.data
  },
  create: async (data: Partial<RouteInfo>) => {
    const res = await api.post<RouteInfo>('/routes', data)
    return res.data
  },
  update: async (id: string, data: Partial<RouteInfo>) => {
    const res = await api.put<RouteInfo>(`/routes/${id}`, data)
    return res.data
  },
  remove: async (id: string) => {
    await api.delete(`/routes/${id}`)
  },
}
