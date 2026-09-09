import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { managedServersApi } from '@/api/managedServers'
import type { ManagedServer } from '@/types'

interface ServerState {
  servers: ManagedServer[]
  defaultServerId: string | null
  activeServerId: string | null
  loaded: boolean
  loadServers: () => Promise<void>
  setActiveServerId: (id: string) => void
  upsertServer: (data: {
    id: string
    name: string
    api_upstream: string
    make_default?: boolean
  }) => Promise<void>
  removeServer: (id: string) => Promise<void>
  setDefaultServer: (id: string) => Promise<void>
  activeServer: () => ManagedServer | null
}

export const useServerStore = create<ServerState>()(
  persist(
    (set, get) => ({
      servers: [],
      defaultServerId: null,
      activeServerId: null,
      loaded: false,
      loadServers: async () => {
        const data = await managedServersApi.list()
        const active =
          get().activeServerId && data.servers.some((s) => s.id === get().activeServerId)
            ? get().activeServerId
            : data.default_server || data.servers[0]?.id || null
        set({
          servers: data.servers,
          defaultServerId: data.default_server ?? null,
          activeServerId: active,
          loaded: true,
        })
      },
      setActiveServerId: (id) => {
        if (!get().servers.some((s) => s.id === id)) return
        set({ activeServerId: id })
      },
      upsertServer: async (data) => {
        const res = await managedServersApi.upsert(data)
        set({
          servers: res.servers,
          defaultServerId: res.default_server ?? null,
          activeServerId:
            get().activeServerId && res.servers.some((s) => s.id === get().activeServerId)
              ? get().activeServerId
              : res.default_server || res.servers[0]?.id || null,
        })
      },
      removeServer: async (id) => {
        const res = await managedServersApi.remove(id)
        const nextActive =
          get().activeServerId === id
            ? res.default_server || res.servers[0]?.id || null
            : get().activeServerId && res.servers.some((s) => s.id === get().activeServerId)
              ? get().activeServerId
              : res.default_server || res.servers[0]?.id || null
        set({
          servers: res.servers,
          defaultServerId: res.default_server ?? null,
          activeServerId: nextActive,
        })
      },
      setDefaultServer: async (id) => {
        const res = await managedServersApi.setDefault(id)
        set({
          servers: res.servers,
          defaultServerId: res.default_server ?? null,
        })
      },
      activeServer: () => {
        const { servers, activeServerId } = get()
        return servers.find((s) => s.id === activeServerId) || null
      },
    }),
    {
      name: 'p2p-active-server',
      partialize: (s) => ({ activeServerId: s.activeServerId }),
    },
  ),
)
