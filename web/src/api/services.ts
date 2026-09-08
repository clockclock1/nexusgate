import api from './client'
import type { ServiceInfo } from '@/types'

export const servicesApi = {
  list: async (params?: { node_id?: string; protocol?: string }) => {
    const res = await api.get<ServiceInfo[]>('/services', { params })
    return res.data
  },
  get: async (id: string) => {
    const res = await api.get<ServiceInfo>(`/services/${id}`)
    return res.data
  },
  create: async (data: Partial<ServiceInfo>) => {
    const res = await api.post<ServiceInfo>('/services', data)
    return res.data
  },
  update: async (id: string, data: Partial<ServiceInfo>) => {
    const res = await api.put<ServiceInfo>(`/services/${id}`, data)
    return res.data
  },
  remove: async (id: string) => {
    await api.delete(`/services/${id}`)
  },
}
