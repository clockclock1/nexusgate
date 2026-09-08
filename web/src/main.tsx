import { StrictMode, useEffect, useMemo } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { App as AntApp, ConfigProvider, theme } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import dayjs from 'dayjs'
import 'dayjs/locale/zh-cn'
import App from './App'
import { useThemeStore } from '@/stores/themeStore'
import { useAuthStore } from '@/stores/authStore'
import '@/styles/global.css'

dayjs.locale('zh-cn')

function Root() {
  const mode = useThemeStore((s) => s.mode)
  const setMode = useThemeStore((s) => s.setMode)
  const hydrateUser = useAuthStore((s) => s.hydrateUser)

  useEffect(() => {
    const stored = useThemeStore.getState().mode
    document.documentElement.setAttribute('data-theme', stored)
    setMode(stored)
    hydrateUser()
  }, [setMode, hydrateUser])

  const antTheme = useMemo(
    () => ({
      algorithm: mode === 'dark' ? theme.darkAlgorithm : theme.defaultAlgorithm,
      token: {
        colorPrimary: '#14b8a6',
        colorInfo: '#22d3ee',
        borderRadius: 10,
        fontFamily: "'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif",
      },
      components: {
        Layout: {
          bodyBg: 'var(--bg-app)',
          siderBg: 'var(--bg-sider)',
          headerBg: 'var(--bg-header)',
        },
        Card: {
          colorBgContainer: 'var(--bg-card)',
        },
        Table: {
          colorBgContainer: 'var(--bg-card)',
        },
      },
    }),
    [mode],
  )

  return (
    <ConfigProvider locale={zhCN} theme={antTheme}>
      <AntApp>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </AntApp>
    </ConfigProvider>
  )
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
)
