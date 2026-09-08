import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { UserInfo } from '@/types'
import { authApi } from '@/api'

interface AuthState {
  token: string | null
  user: UserInfo | null
  setAuth: (token: string, user: UserInfo) => void
  logout: () => void
  hydrateUser: () => Promise<void>
  isAuthenticated: () => boolean
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      token: null,
      user: null,
      setAuth: (token, user) => {
        localStorage.setItem('token', token)
        set({ token, user })
      },
      logout: () => {
        localStorage.removeItem('token')
        set({ token: null, user: null })
      },
      hydrateUser: async () => {
        if (!get().token) return
        try {
          const user = await authApi.me()
          set({ user })
        } catch {
          get().logout()
        }
      },
      isAuthenticated: () => Boolean(get().token),
    }),
    {
      name: 'p2p-auth',
      partialize: (s) => ({ token: s.token, user: s.user }),
    },
  ),
)
