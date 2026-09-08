import { useCallback, useEffect, useState } from 'react'
import { Button, Col, Row, Space, Typography, message } from 'antd'
import { ReloadOutlined } from '@ant-design/icons'
import { NetworkGraph, StatCard } from '@/components'
import { metricsApi } from '@/api'
import type { NetworkTopology, P2pStats } from '@/types'
import { formatPercent, getErrorMessage } from '@/utils/format'

const { Title } = Typography

export default function P2PPage() {
  const [topology, setTopology] = useState<NetworkTopology | null>(null)
  const [stats, setStats] = useState<P2pStats | null>(null)
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [topo, p2p] = await Promise.all([metricsApi.topology(), metricsApi.p2pStats()])
      setTopology(topo)
      setStats(p2p)
    } catch (err) {
      message.error(getErrorMessage(err, '加载 P2P 网络数据失败'))
      setTopology(null)
      setStats(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
    const t = window.setInterval(load, 20000)
    return () => window.clearInterval(t)
  }, [load])

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={4} style={{ margin: 0 }}>
          P2P 网络
        </Title>
        <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
          刷新
        </Button>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <StatCard title="P2P 连接" value={stats?.p2p_connections ?? '-'} extra="直连连接数" />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard title="中继连接" value={stats?.relay_connections ?? '-'} extra="回退中继连接" />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="P2P 成功率"
            value={stats ? formatPercent(stats.p2p_success_rate) : '-'}
            extra="打洞/直连成功率"
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="平均延迟"
            value={stats?.avg_latency_ms != null ? `${stats.avg_latency_ms.toFixed(1)} ms` : '-'}
            extra={
              stats
                ? `打洞成功 ${stats.hole_punch_success ?? 0} / 失败 ${stats.hole_punch_fail ?? 0}`
                : undefined
            }
          />
        </Col>
      </Row>

      <div className="page-card" style={{ padding: 16 }}>
        <Title level={5} style={{ marginTop: 0 }}>
          网络拓扑
        </Title>
        <NetworkGraph data={topology} height={520} />
      </div>
    </Space>
  )
}
