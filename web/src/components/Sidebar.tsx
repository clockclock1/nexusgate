import { useMemo } from 'react'
import { Layout, Menu } from 'antd'
import { useLocation, useNavigate } from 'react-router-dom'
import {
  ApiOutlined,
  ApartmentOutlined,
  CloudServerOutlined,
  ClusterOutlined,
  DashboardOutlined,
  DeploymentUnitOutlined,
  FileTextOutlined,
  NodeIndexOutlined,
  SettingOutlined,
  ShareAltOutlined,
  TeamOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons'

const { Sider } = Layout

const items = [
  { key: '/dashboard', icon: <DashboardOutlined />, label: '仪表盘' },
  { key: '/servers', icon: <CloudServerOutlined />, label: '服务端节点' },
  { key: '/server', icon: <DeploymentUnitOutlined />, label: '当前服务端' },
  { key: '/nodes', icon: <ClusterOutlined />, label: '客户端节点' },
  { key: '/services', icon: <ApiOutlined />, label: '服务' },
  { key: '/routes', icon: <NodeIndexOutlined />, label: '路由' },
  { key: '/p2p', icon: <ShareAltOutlined />, label: 'P2P 网络' },
  { key: '/connections', icon: <ApartmentOutlined />, label: '连接' },
  { key: '/traffic', icon: <ThunderboltOutlined />, label: '流量' },
  { key: '/logs', icon: <FileTextOutlined />, label: '日志' },
  { key: '/users', icon: <TeamOutlined />, label: '用户' },
  { key: '/settings', icon: <SettingOutlined />, label: '设置' },
]

interface SidebarProps {
  collapsed: boolean
}

export default function Sidebar({ collapsed }: SidebarProps) {
  const navigate = useNavigate()
  const location = useLocation()

  const selected = useMemo(() => {
    const hit = items.find((i) => location.pathname === i.key || location.pathname.startsWith(`${i.key}/`))
    return [hit?.key || '/dashboard']
  }, [location.pathname])

  return (
    <Sider
      className="app-sider"
      collapsible
      collapsed={collapsed}
      trigger={null}
      width={220}
      breakpoint="lg"
    >
      <div className="logo">
        <div className="logo-mark">P</div>
        {!collapsed && <span>P2P Network</span>}
      </div>
      <Menu
        theme="dark"
        mode="inline"
        selectedKeys={selected}
        items={items}
        onClick={({ key }) => navigate(key)}
        style={{ background: 'transparent', borderInlineEnd: 'none', marginTop: 8 }}
      />
    </Sider>
  )
}
