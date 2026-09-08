import api from './client'
import type { ServerConfig, ServerInfo } from '@/types'

export const serverApi = {
  getInfo: async () => {
    const res = await api.get<ServerInfo>('/server/info')
    return res.data
  },
  getConfig: async () => {
    const res = await api.get<ServerConfig>('/server/config')
    return res.data
  },
  updateConfig: async (data: Partial<ServerConfig>) => {
    const res = await api.put<ServerConfig>('/server/config', data)
    return res.data
  },
  restart: async () => {
    const res = await api.post('/server/restart')
    return res.data
  },
}
