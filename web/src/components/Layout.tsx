import { useEffect, useState } from 'react'
import { Layout, Spin } from 'antd'
import { Outlet } from 'react-router-dom'
import Sidebar from './Sidebar'
import HeaderBar from './Header'
import { serverApi } from '@/api'
import { applyServerSession } from '@/stores/authStore'
import { useServerStore } from '@/stores/serverStore'

const { Content } = Layout

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false)
  const [serverOnline, setServerOnline] = useState(true)
  const [booting, setBooting] = useState(true)
  const activeServerId = useServerStore((s) => s.activeServerId)
  const loadServers = useServerStore((s) => s.loadServers)

  useEffect(() => {
    let alive = true
    ;(async () => {
      try {
        await loadServers()
        applyServerSession()
      } catch {
        /* panel local API may be down */
      } finally {
        if (alive) setBooting(false)
      }
    })()
    return () => {
      alive = false
    }
  }, [loadServers])

  useEffect(() => {
    let alive = true
    const check = async () => {
      try {
        const info = await serverApi.getInfo()
        if (alive) setServerOnline(info.status !== 'offline')
      } catch {
        if (alive) setServerOnline(false)
      }
    }
    if (!booting && activeServerId) {
      check()
      const t = window.setInterval(check, 30000)
      const onChanged = () => {
        applyServerSession()
        check()
      }
      window.addEventListener('nexus-server-changed', onChanged)
      return () => {
        alive = false
        window.clearInterval(t)
        window.removeEventListener('nexus-server-changed', onChanged)
      }
    }
    return () => {
      alive = false
    }
  }, [activeServerId, booting])

  if (booting) {
    return (
      <div style={{ height: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg-app)' }}>
        <Spin size="large" tip="加载控制台..." />
      </div>
    )
  }

  return (
    <Layout className="app-layout" style={{ minHeight: '100vh' }}>
      <Sidebar collapsed={collapsed} />
      <Layout>
        <HeaderBar
          collapsed={collapsed}
          onToggle={() => setCollapsed((v) => !v)}
          serverOnline={serverOnline}
        />
        <Content className="app-content">
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  )
}
