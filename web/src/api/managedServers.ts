import axios from 'axios'
import type { ManagedServer } from '@/types'

/** Local admin-panel API (not proxied to Super Node). */
const adminApi = axios.create({
  baseURL: '/admin/api',
  timeout: 15000,
  headers: { 'Content-Type': 'application/json' },
})

export interface ServersPayload {
  servers: ManagedServer[]
  default_server?: string | null
}

export const managedServersApi = {
  list: async () => {
    const res = await adminApi.get<ServersPayload>('/servers')
    return res.data
  },
  upsert: async (data: {
    id: string
    name: string
    api_upstream: string
    make_default?: boolean
  }) => {
    const res = await adminApi.post<ServersPayload>('/servers', data)
    return res.data
  },
  remove: async (id: string) => {
    const res = await adminApi.delete<ServersPayload>(`/servers/${id}`)
    return res.data
  },
  setDefault: async (id: string) => {
    const res = await adminApi.put<ServersPayload>('/servers/default', { id })
    return res.data
  },
  probe: async (id: string) => {
    const res = await adminApi.get<{ id: string; ok: boolean; status?: number; error?: string }>(
      `/servers/${id}/probe`,
    )
    return res.data
  },
  hubStatus: async () => {
    const res = await adminApi.get<{
      enabled: boolean
      control_port?: number
      data_port?: number
      listen?: string
      peers_total?: number
      servers_online?: number
      edges_online?: number
    }>('/hub/status')
    return res.data
  },
  hubPeers: async () => {
    const res = await adminApi.get<{
      enabled: boolean
      peers: Array<{
        node_id: string
        role: string
        name?: string
        version?: string
        online: boolean
      }>
    }>('/hub/peers')
    return res.data
  },
}
