import api from './client'
import type { LoginRequest, LoginResponse, UserInfo } from '@/types'

export const authApi = {
  login: async (data: LoginRequest) => {
    const res = await api.post<LoginResponse>('/auth/login', data)
    return res.data
  },
  logout: async () => {
    await api.post('/auth/logout')
  },
  me: async () => {
    const res = await api.get<UserInfo>('/auth/me')
    return res.data
  },
}
