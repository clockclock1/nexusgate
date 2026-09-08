import { useCallback, useEffect, useState } from 'react'
import { Button, Segmented, Space, Typography, message } from 'antd'
import { ReloadOutlined } from '@ant-design/icons'
import { TrafficChart } from '@/components'
import { metricsApi } from '@/api'
import type { TrafficPoint } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

export default function TrafficPage() {
  const [interval, setInterval] = useState<'hourly' | 'daily'>('hourly')
  const [data, setData] = useState<TrafficPoint[]>([])
  const [loading, setLoading] = useState(true)
  const [showModeSplit, setShowModeSplit] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const res = await metricsApi.traffic({ interval })
      setData(res.points ?? [])
    } catch (err) {
      message.error(getErrorMessage(err, '加载流量数据失败'))
      setData([])
    } finally {
      setLoading(false)
    }
  }, [interval])

  useEffect(() => {
    load()
  }, [load])

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          流量统计
        </Title>
        <Space wrap>
          <Segmented
            value={interval}
            onChange={(v) => setInterval(v as 'hourly' | 'daily')}
            options={[
              { value: 'hourly', label: '按小时' },
              { value: 'daily', label: '按天' },
            ]}
          />
          <Segmented
            value={showModeSplit ? 'mode' : 'rxtx'}
            onChange={(v) => setShowModeSplit(v === 'mode')}
            options={[
              { value: 'rxtx', label: '上下行' },
              { value: 'mode', label: 'P2P/中继' },
            ]}
          />
          <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
            刷新
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <TrafficChart data={data} loading={loading} height={420} showModeSplit={showModeSplit} />
      </div>
    </Space>
  )
}
