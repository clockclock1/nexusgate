import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type ThemeMode = 'dark' | 'light'

interface ThemeState {
  mode: ThemeMode
  toggle: () => void
  setMode: (mode: ThemeMode) => void
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      mode: 'dark',
      toggle: () => {
        const next = get().mode === 'dark' ? 'light' : 'dark'
        document.documentElement.setAttribute('data-theme', next)
        set({ mode: next })
      },
      setMode: (mode) => {
        document.documentElement.setAttribute('data-theme', mode)
        set({ mode })
      },
    }),
    { name: 'p2p-theme' },
  ),
)
