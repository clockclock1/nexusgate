import { useCallback, useEffect, useState } from 'react'
import { Button, Select, Space, Typography, message } from 'antd'
import { ReloadOutlined } from '@ant-design/icons'
import { ConnectionTable } from '@/components'
import { metricsApi } from '@/api'
import type { ConnectionInfo } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

export default function ConnectionsPage() {
  const [connections, setConnections] = useState<ConnectionInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [mode, setMode] = useState<string | undefined>()
  const [status, setStatus] = useState<string | undefined>('active')

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await metricsApi.connections({ mode, status })
      setConnections(data)
    } catch (err) {
      message.error(getErrorMessage(err, '加载连接列表失败'))
      setConnections([])
    } finally {
      setLoading(false)
    }
  }, [mode, status])

  useEffect(() => {
    load()
    const t = window.setInterval(load, 10000)
    return () => window.clearInterval(t)
  }, [load])

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          连接监控
        </Title>
        <Space wrap>
          <Select
            placeholder="连接模式"
            allowClear
            style={{ width: 120 }}
            value={mode}
            onChange={setMode}
            options={[
              { value: 'p2p', label: 'P2P' },
              { value: 'relay', label: '中继' },
            ]}
          />
          <Select
            placeholder="状态"
            allowClear
            style={{ width: 120 }}
            value={status}
            onChange={setStatus}
            options={[
              { value: 'active', label: '活跃' },
              { value: 'connecting', label: '连接中' },
              { value: 'closed', label: '已关闭' },
              { value: 'failed', label: '失败' },
            ]}
          />
          <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
            刷新
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <ConnectionTable data={connections} loading={loading} />
      </div>
    </Space>
  )
}
