import { useCallback, useEffect, useState } from 'react'
import { Badge, Col, Row, Space, Typography } from 'antd'
import {
  ApiOutlined,
  ClusterOutlined,
  NodeIndexOutlined,
  SwapOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons'
import { StatCard, TrafficChart, ConnectionTable } from '@/components'
import { metricsApi } from '@/api'
import useWebSocket from '@/hooks/useWebSocket'
import type { ConnectionInfo, DashboardStats, TrafficPoint } from '@/types'
import { formatBytes, formatPercent, getErrorMessage, toPercent } from '@/utils/format'
import { message } from 'antd'

const { Title, Text } = Typography

interface DashboardWsPayload {
  stats?: DashboardStats
  connections?: ConnectionInfo[]
  traffic_point?: TrafficPoint
}

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null)
  const [traffic, setTraffic] = useState<TrafficPoint[]>([])
  const [connections, setConnections] = useState<ConnectionInfo[]>([])
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [dash, trafficRes, connRes] = await Promise.all([
        metricsApi.dashboard(),
        metricsApi.traffic({ interval: 'hourly' }),
        metricsApi.connections({ status: 'active' }),
      ])
      setStats(dash)
      setTraffic(trafficRes.points ?? [])
      setConnections(connRes.slice(0, 20))
    } catch (err) {
      message.error(getErrorMessage(err, '加载仪表盘数据失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const onWsMessage = useCallback((data: DashboardWsPayload) => {
    if (data.stats) setStats(data.stats)
    if (data.connections) setConnections(data.connections.slice(0, 20))
    if (data.traffic_point) {
      setTraffic((prev) => {
        const next = [...prev, data.traffic_point!]
        return next.length > 48 ? next.slice(-48) : next
      })
    }
  }, [])

  const { status: wsStatus } = useWebSocket<DashboardWsPayload>({
    path: '/ws/dashboard',
    onMessage: onWsMessage,
  })

  const wsLabel =
    wsStatus === 'open' ? '实时' : wsStatus === 'connecting' ? '连接中' : '离线'

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={4} style={{ margin: 0 }}>
          仪表盘
        </Title>
        <Badge
          status={wsStatus === 'open' ? 'success' : wsStatus === 'connecting' ? 'processing' : 'default'}
          text={<Text type="secondary">WebSocket {wsLabel}</Text>}
        />
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="在线节点"
            value={stats ? `${stats.nodes_online} / ${stats.nodes_total}` : '-'}
            extra="当前在线 / 总节点"
            icon={<ClusterOutlined />}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="活跃连接"
            value={stats?.connections_active ?? '-'}
            extra="实时连接数"
            icon={<SwapOutlined />}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="流量 (RX / TX)"
            value={
              stats
                ? `${formatBytes(stats.traffic_rx_bytes)} / ${formatBytes(stats.traffic_tx_bytes)}`
                : '-'
            }
            extra="累计流量"
            icon={<ThunderboltOutlined />}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="P2P 成功率"
            value={stats ? formatPercent(stats.p2p_rate) : '-'}
            extra={`中继占比 ${stats ? formatPercent(stats.relay_rate) : '-'}`}
            icon={<NodeIndexOutlined />}
          />
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <StatCard title="服务总数" value={stats?.services_total ?? '-'} icon={<ApiOutlined />} />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard title="路由总数" value={stats?.routes_total ?? '-'} icon={<NodeIndexOutlined />} />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="P2P 占比"
            value={stats ? `${toPercent(stats.p2p_rate).toFixed(1)}%` : '-'}
            extra="直连优先"
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="中继占比"
            value={stats ? `${toPercent(stats.relay_rate).toFixed(1)}%` : '-'}
            extra="回退中继"
          />
        </Col>
      </Row>

      <div className="page-card" style={{ padding: 16 }}>
        <Title level={5} style={{ marginTop: 0 }}>
          流量趋势
        </Title>
        <TrafficChart data={traffic} loading={loading} height={300} />
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Title level={5} style={{ marginTop: 0 }}>
          活跃连接
        </Title>
        <ConnectionTable data={connections} loading={loading} pagination={{ pageSize: 8 }} />
      </div>
    </Space>
  )
}
