import api from './client'
import type {
  ConnectionInfo,
  DashboardStats,
  LogEntry,
  NetworkTopology,
  P2pStats,
  TrafficSeries,
} from '@/types'

export const metricsApi = {
  dashboard: async () => {
    const res = await api.get<DashboardStats>('/metrics/dashboard')
    return res.data
  },
  traffic: async (params?: { interval?: 'hourly' | 'daily' }) => {
    const res = await api.get<TrafficSeries>('/metrics/traffic', { params })
    return res.data
  },
  connections: async (params?: { mode?: string; status?: string; node_id?: string }) => {
    const res = await api.get<ConnectionInfo[]>('/metrics/connections', { params })
    return res.data
  },
  logs: async (params?: { level?: string; limit?: number }) => {
    const res = await api.get<LogEntry[]>('/metrics/logs', { params })
    return res.data
  },
  topology: async () => {
    const res = await api.get<NetworkTopology>('/metrics/topology')
    return res.data
  },
  p2pStats: async () => {
    const res = await api.get<P2pStats>('/metrics/p2p')
    return res.data
  },
}
