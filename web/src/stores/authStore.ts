import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { UserInfo } from '@/types'
import { authApi } from '@/api'
import { useServerStore } from '@/stores/serverStore'

interface Session {
  token: string
  user: UserInfo | null
}

interface AuthState {
  /** JWT sessions keyed by managed server id */
  sessions: Record<string, Session>
  setAuth: (token: string, user: UserInfo, serverId?: string) => void
  logout: (serverId?: string) => void
  hydrateUser: () => Promise<void>
  isAuthenticated: () => boolean
  token: string | null
  user: UserInfo | null
  syncActive: () => void
}

function activeId(): string | null {
  return useServerStore.getState().activeServerId
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      sessions: {},
      token: null,
      user: null,
      syncActive: () => {
        const id = activeId()
        const session = id ? get().sessions[id] : undefined
        set({
          token: session?.token ?? null,
          user: session?.user ?? null,
        })
        if (session?.token) {
          localStorage.setItem('token', session.token)
        } else {
          localStorage.removeItem('token')
        }
      },
      setAuth: (token, user, serverId) => {
        const id = serverId || activeId()
        if (!id) return
        const sessions = { ...get().sessions, [id]: { token, user } }
        set({ sessions })
        if (id === activeId()) {
          localStorage.setItem('token', token)
          set({ token, user })
        }
      },
      logout: (serverId) => {
        const id = serverId || activeId()
        const sessions = { ...get().sessions }
        if (id) delete sessions[id]
        set({ sessions })
        if (!id || id === activeId()) {
          localStorage.removeItem('token')
          set({ token: null, user: null })
        }
      },
      hydrateUser: async () => {
        get().syncActive()
        if (!get().token) return
        try {
          const user = await authApi.me()
          const id = activeId()
          if (!id || !get().token) return
          const sessions = {
            ...get().sessions,
            [id]: { token: get().token!, user },
          }
          set({ sessions, user })
        } catch {
          get().logout()
        }
      },
      isAuthenticated: () => {
        get().syncActive()
        return Boolean(get().token)
      },
    }),
    {
      name: 'p2p-auth',
      partialize: (s) => ({ sessions: s.sessions }),
      onRehydrateStorage: () => (state) => {
        state?.syncActive()
      },
    },
  ),
)

/** Call after switching active server. */
export function applyServerSession() {
  useAuthStore.getState().syncActive()
}
