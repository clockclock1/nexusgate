import { Navigate, Route, Routes } from 'react-router-dom'
import { Spin } from 'antd'
import { useEffect, useState, type ReactNode } from 'react'
import { AppLayout } from '@/components'
import { useAuthStore } from '@/stores/authStore'
import LoginPage from '@/pages/Login'
import DashboardPage from '@/pages/Dashboard'
import ServerPage from '@/pages/Server'
import NodesPage from '@/pages/Nodes'
import NodeDetailPage from '@/pages/Nodes/Detail'
import ServicesPage from '@/pages/Services'
import RoutesPage from '@/pages/Routes'
import P2PPage from '@/pages/P2P'
import ConnectionsPage from '@/pages/Connections'
import TrafficPage from '@/pages/Traffic'
import LogsPage from '@/pages/Logs'
import UsersPage from '@/pages/Users'
import SettingsPage from '@/pages/Settings'

function ProtectedRoute({ children }: { children: ReactNode }) {
  const token = useAuthStore((s) => s.token)
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  const hydrateUser = useAuthStore((s) => s.hydrateUser)
  const [ready, setReady] = useState(false)

  useEffect(() => {
    let alive = true
    const init = async () => {
      if (token) await hydrateUser()
      if (alive) setReady(true)
    }
    init()
    return () => {
      alive = false
    }
  }, [token, hydrateUser])

  if (!ready) {
    return (
      <div style={{ height: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg-app)' }}>
        <Spin size="large" tip="验证登录状态..." />
      </div>
    )
  }

  if (!isAuthenticated()) {
    return <Navigate to="/login" replace />
  }

  return <>{children}</>
}

export default function AppRouter() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  return (
    <Routes>
      <Route
        path="/login"
        element={isAuthenticated() ? <Navigate to="/dashboard" replace /> : <LoginPage />}
      />
      <Route
        element={
          <ProtectedRoute>
            <AppLayout />
          </ProtectedRoute>
        }
      >
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<DashboardPage />} />
        <Route path="server" element={<ServerPage />} />
        <Route path="nodes" element={<NodesPage />} />
        <Route path="nodes/:id" element={<NodeDetailPage />} />
        <Route path="services" element={<ServicesPage />} />
        <Route path="routes" element={<RoutesPage />} />
        <Route path="p2p" element={<P2PPage />} />
        <Route path="connections" element={<ConnectionsPage />} />
        <Route path="traffic" element={<TrafficPage />} />
        <Route path="logs" element={<LogsPage />} />
        <Route path="users" element={<UsersPage />} />
        <Route path="settings" element={<SettingsPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </Routes>
  )
}
