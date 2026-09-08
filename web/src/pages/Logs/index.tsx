import { useCallback, useEffect, useState } from 'react'
import { Button, Select, Space, Typography, message } from 'antd'
import { ReloadOutlined } from '@ant-design/icons'
import dayjs from 'dayjs'
import { metricsApi } from '@/api'
import type { LogEntry, LogLevel } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

const levelOptions = [
  { value: 'trace', label: 'TRACE' },
  { value: 'debug', label: 'DEBUG' },
  { value: 'info', label: 'INFO' },
  { value: 'warn', label: 'WARN' },
  { value: 'error', label: 'ERROR' },
]

export default function LogsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [level, setLevel] = useState<LogLevel | undefined>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await metricsApi.logs({ level, limit: 500 })
      setLogs(data)
    } catch (err) {
      message.error(getErrorMessage(err, '加载日志失败'))
      setLogs([])
    } finally {
      setLoading(false)
    }
  }, [level])

  useEffect(() => {
    load()
  }, [load])

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          系统日志
        </Title>
        <Space wrap>
          <Select
            placeholder="日志级别"
            allowClear
            style={{ width: 120 }}
            value={level}
            onChange={setLevel}
            options={levelOptions}
          />
          <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
            刷新
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        {logs.length === 0 && !loading ? (
          <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)' }}>暂无日志</div>
        ) : (
          <div className="log-viewer">
            {logs.map((log, i) => (
              <div className="log-line" key={log.id || `${log.timestamp}-${i}`}>
                <span style={{ color: 'var(--text-muted)' }}>
                  {dayjs(log.timestamp).format('YYYY-MM-DD HH:mm:ss')}
                </span>
                <span className={`lvl-${log.level}`}>{log.level.toUpperCase()}</span>
                <span>
                  {log.target ? `[${log.target}] ` : ''}
                  {log.node_id ? `[${log.node_id}] ` : ''}
                  {log.message}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </Space>
  )
}
