import { Badge, Button, Dropdown, Space, Typography } from 'antd'
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
import { useAuthStore } from '@/stores/authStore'
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

      <Space size="middle">
        <Space size={6}>
          <span className={`server-status-dot ${serverOnline ? 'online' : 'offline'}`} />
          <Text type="secondary">Server: {serverOnline ? 'Online' : 'Offline'}</Text>
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
                label: '退出登录',
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
