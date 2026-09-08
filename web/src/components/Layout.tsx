import { useEffect, useState } from 'react'
import { Layout, Spin } from 'antd'
import { Outlet } from 'react-router-dom'
import Sidebar from './Sidebar'
import HeaderBar from './Header'
import { serverApi } from '@/api'

const { Content } = Layout

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false)
  const [serverOnline, setServerOnline] = useState(true)
  const [booting, setBooting] = useState(true)

  useEffect(() => {
    let alive = true
    const check = async () => {
      try {
        const info = await serverApi.getInfo()
        if (alive) setServerOnline(info.status !== 'offline')
      } catch {
        if (alive) setServerOnline(false)
      } finally {
        if (alive) setBooting(false)
      }
    }
    check()
    const t = window.setInterval(check, 30000)
    return () => {
      alive = false
      window.clearInterval(t)
    }
  }, [])

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
