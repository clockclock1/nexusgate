import api from './client'
import type {
  ConnectionInfo,
  LogEntry,
  NodeInfo,
  PageResult,
  RouteInfo,
  ServiceInfo,
  TrafficSeries,
} from '@/types'

export const nodesApi = {
  list: async (params?: { status?: string; q?: string }) => {
    const res = await api.get<NodeInfo[] | PageResult<NodeInfo>>('/nodes', { params })
    return Array.isArray(res.data) ? res.data : res.data.items ?? []
  },
  get: async (id: string) => {
    const res = await api.get<NodeInfo>(`/nodes/${id}`)
    return res.data
  },
  create: async (data: { name: string; node_id?: string }) => {
    const res = await api.post<NodeInfo>('/nodes', data)
    return res.data
  },
  update: async (id: string, data: Partial<{ name: string; enabled: boolean }>) => {
    const res = await api.put<NodeInfo>(`/nodes/${id}`, data)
    return res.data
  },
  remove: async (id: string) => {
    await api.delete(`/nodes/${id}`)
  },
  regenToken: async (id: string) => {
    const res = await api.post<{ node_id: string; token: string }>(`/nodes/${id}/token`)
    return res.data
  },
  getServices: async (id: string) => {
    const res = await api.get<ServiceInfo[]>(`/nodes/${id}/services`)
    return res.data
  },
  getRoutes: async (id: string) => {
    const res = await api.get<RouteInfo[]>(`/nodes/${id}/routes`)
    return res.data
  },
  getConnections: async (id: string) => {
    const res = await api.get<ConnectionInfo[]>(`/nodes/${id}/connections`)
    return res.data
  },
  getTraffic: async (id: string, params?: { interval?: string }) => {
    const res = await api.get<TrafficSeries>(`/nodes/${id}/traffic`, { params })
    return res.data
  },
  getLogs: async (id: string, params?: { level?: string; limit?: number }) => {
    const res = await api.get<LogEntry[]>(`/nodes/${id}/logs`, { params })
    return res.data
  },
}
