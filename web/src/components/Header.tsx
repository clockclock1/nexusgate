import { Badge, Button, Dropdown, Select, Space, Typography, message } from 'antd'
import {
  LogoutOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  MoonOutlined,
  SettingOutlined,
  SunOutlined,
  UserOutlined,
} from '@ant-design/icons'
import { useNavigate } from 'react-router-dom'
import { applyServerSession, useAuthStore } from '@/stores/authStore'
import { useServerStore } from '@/stores/serverStore'
import { useThemeStore } from '@/stores/themeStore'

const { Text } = Typography

interface HeaderBarProps {
  collapsed: boolean
  onToggle: () => void
  serverOnline?: boolean
}

export default function HeaderBar({ collapsed, onToggle, serverOnline = true }: HeaderBarProps) {
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const logout = useAuthStore((s) => s.logout)
  const mode = useThemeStore((s) => s.mode)
  const toggleTheme = useThemeStore((s) => s.toggle)
  const servers = useServerStore((s) => s.servers)
  const activeServerId = useServerStore((s) => s.activeServerId)
  const setActiveServerId = useServerStore((s) => s.setActiveServerId)
  const activeServer = useServerStore((s) => s.activeServer)

  const onSwitchServer = (id: string) => {
    if (id === activeServerId) return
    setActiveServerId(id)
    applyServerSession()
    window.dispatchEvent(new CustomEvent('nexus-server-changed'))
    const hasSession = Boolean(useAuthStore.getState().sessions[id]?.token)
    if (!hasSession) {
      message.info('该服务端尚未登录，请先登录')
      navigate('/login')
      return
    }
    message.success(`已切换到：${useServerStore.getState().activeServer()?.name || id}`)
  }

  return (
    <div className="app-header ant-layout-header">
      <Space>
        <Button
          type="text"
          icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
          onClick={onToggle}
        />
        <Text strong style={{ color: 'var(--text)' }}>
          网络控制台
        </Text>
      </Space>

      <Space size="middle" wrap>
        <Select
          style={{ minWidth: 180 }}
          value={activeServerId ?? undefined}
          placeholder="选择服务端"
          options={servers.map((s) => ({
            value: s.id,
            label: s.name || s.id,
          }))}
          onChange={onSwitchServer}
          popupMatchSelectWidth={false}
        />

        <Space size={6}>
          <span className={`server-status-dot ${serverOnline ? 'online' : 'offline'}`} />
          <Text type="secondary">
            {activeServer()?.name || 'Server'}: {serverOnline ? 'Online' : 'Offline'}
          </Text>
        </Space>

        <Button
          type="text"
          icon={mode === 'dark' ? <SunOutlined /> : <MoonOutlined />}
          onClick={toggleTheme}
          title="切换主题"
        />

        <Button type="text" icon={<SettingOutlined />} onClick={() => navigate('/settings')} />

        <Dropdown
          menu={{
            items: [
              {
                key: 'user',
                label: user?.username || 'Admin',
                icon: <UserOutlined />,
                disabled: true,
              },
              { type: 'divider' },
              {
                key: 'logout',
                label: '退出当前服务端',
                icon: <LogoutOutlined />,
                onClick: () => {
                  logout()
                  navigate('/login')
                },
              },
            ],
          }}
        >
          <Button type="text">
            <Badge status="success" />
            <Space>
              <UserOutlined />
              {user?.username || 'Admin'}
            </Space>
          </Button>
        </Dropdown>
      </Space>
    </div>
  )
}
