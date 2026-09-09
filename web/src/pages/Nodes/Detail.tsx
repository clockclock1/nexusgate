import { useCallback, useEffect, useState } from 'react'
import {
  Breadcrumb,
  Button,
  Descriptions,
  Modal,
  Popconfirm,
  Space,
  Spin,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd'
import type { ColumnsType } from 'antd/es/table'
import { ArrowLeftOutlined, ReloadOutlined } from '@ant-design/icons'
import { Link, useNavigate, useParams } from 'react-router-dom'
import dayjs from 'dayjs'
import { nodesApi } from '@/api'
import { ConnectionTable, TrafficChart } from '@/components'
import type { ConnectionInfo, NodeInfo, RouteInfo, ServiceInfo } from '@/types'
import { formatBytes, formatDuration, getErrorMessage } from '@/utils/format'

const { Title } = Typography

const statusMap: Record<string, { label: string; color: string }> = {
  online: { label: '在线', color: 'success' },
  offline: { label: '离线', color: 'default' },
  disabled: { label: '已禁用', color: 'warning' },
}

export default function NodeDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const [node, setNode] = useState<NodeInfo | null>(null)
  const [services, setServices] = useState<ServiceInfo[]>([])
  const [routes, setRoutes] = useState<RouteInfo[]>([])
  const [connections, setConnections] = useState<ConnectionInfo[]>([])
  const [traffic, setTraffic] = useState<{ timestamp: string; rx_bytes: number; tx_bytes: number }[]>([])
  const [loading, setLoading] = useState(true)
  const [tokenModal, setTokenModal] = useState<{ node_id: string; token: string } | null>(null)

  const load = useCallback(async () => {
    if (!id) return
    setLoading(true)
    try {
      const [n, svcs, rts, conns, tr] = await Promise.all([
        nodesApi.get(id),
        nodesApi.getServices(id),
        nodesApi.getRoutes(id),
        nodesApi.getConnections(id),
        nodesApi.getTraffic(id, { interval: 'hourly' }),
      ])
      setNode(n)
      setServices(svcs)
      setRoutes(rts)
      setConnections(conns)
      setTraffic(tr.points ?? [])
    } catch (err) {
      message.error(getErrorMessage(err, '加载节点详情失败'))
      setNode(null)
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => {
    load()
  }, [load])

  const handleRegen = async () => {
    if (!id) return
    try {
      const res = await nodesApi.regenToken(id)
      setTokenModal(res)
    } catch (err) {
      message.error(getErrorMessage(err, '重置 Token 失败'))
    }
  }

  if (loading && !node) {
    return (
      <div style={{ display: 'grid', placeItems: 'center', minHeight: 320 }}>
        <Spin size="large" tip="加载节点详情..." />
      </div>
    )
  }

  const st = node?.status ? statusMap[node.status] : null

  const serviceColumns: ColumnsType<ServiceInfo> = [
    { title: '服务 ID', dataIndex: 'service_id', ellipsis: true, render: (v) => <code>{v}</code> },
    { title: '名称', dataIndex: 'name' },
    { title: '协议', dataIndex: 'protocol', width: 80 },
    {
      title: '本地地址',
      key: 'addr',
      render: (_, r) => `${r.local_addr}:${r.local_port}`,
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 80,
      render: (v: boolean) => <Tag color={v ? 'success' : 'default'}>{v ? '启用' : '禁用'}</Tag>,
    },
  ]

  const routeColumns: ColumnsType<RouteInfo> = [
    { title: '路由 ID', dataIndex: 'route_id', ellipsis: true, render: (v) => <code>{v}</code> },
    { title: '名称', dataIndex: 'name' },
    { title: '公网端口', dataIndex: 'public_port', width: 100 },
    { title: '协议', dataIndex: 'protocol', width: 80 },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 80,
      render: (v: boolean) => <Tag color={v ? 'success' : 'default'}>{v ? '启用' : '禁用'}</Tag>,
    },
  ]

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Breadcrumb
        items={[
          { title: <Link to="/nodes">节点</Link> },
          { title: node?.name || id },
        ]}
      />

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Space>
          <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/nodes')}>
            返回
          </Button>
          <Title level={4} style={{ margin: 0 }}>
            {node?.name || id}
          </Title>
          {st ? <Tag color={st.color}>{st.label}</Tag> : null}
        </Space>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
            刷新
          </Button>
          <Popconfirm title="重置后旧 Token 立即失效，确定？" onConfirm={handleRegen}>
            <Button>换 Token</Button>
          </Popconfirm>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 20 }}>
        <Descriptions bordered size="small" column={{ xs: 1, sm: 2, md: 3 }}>
          <Descriptions.Item label="节点 ID">
            <code>{node?.node_id}</code>
          </Descriptions.Item>
          <Descriptions.Item label="公网 IP">{node?.public_ip || '-'}</Descriptions.Item>
          <Descriptions.Item label="内网 IP">{node?.private_ip || '-'}</Descriptions.Item>
          <Descriptions.Item label="NAT 类型">{node?.nat_type || '-'}</Descriptions.Item>
          <Descriptions.Item label="版本">{node?.version || '-'}</Descriptions.Item>
          <Descriptions.Item label="区域">{node?.region || '-'}</Descriptions.Item>
          <Descriptions.Item label="CPU">{node?.cpu != null ? `${node.cpu}%` : '-'}</Descriptions.Item>
          <Descriptions.Item label="内存">{node?.memory != null ? `${node.memory}%` : '-'}</Descriptions.Item>
          <Descriptions.Item label="运行时长">{formatDuration(node?.uptime_secs)}</Descriptions.Item>
          <Descriptions.Item label="连接数">{node?.connections ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="下行流量">{formatBytes(node?.rx_bytes)}</Descriptions.Item>
          <Descriptions.Item label="上行流量">{formatBytes(node?.tx_bytes)}</Descriptions.Item>
          <Descriptions.Item label="最后在线">
            {node?.last_seen ? dayjs(node.last_seen).format('YYYY-MM-DD HH:mm:ss') : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="标签">
            {node?.tags?.length ? node.tags.map((t) => <Tag key={t}>{t}</Tag>) : '-'}
          </Descriptions.Item>
        </Descriptions>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Tabs
          items={[
            {
              key: 'services',
              label: `服务 (${services.length})`,
              children: (
                <Table
                  rowKey="service_id"
                  columns={serviceColumns}
                  dataSource={services}
                  loading={loading}
                  pagination={false}
                  locale={{ emptyText: '暂无服务' }}
                  size="middle"
                />
              ),
            },
            {
              key: 'routes',
              label: `路由 (${routes.length})`,
              children: (
                <Table
                  rowKey="route_id"
                  columns={routeColumns}
                  dataSource={routes}
                  loading={loading}
                  pagination={false}
                  locale={{ emptyText: '暂无路由' }}
                  size="middle"
                />
              ),
            },
            {
              key: 'connections',
              label: `连接 (${connections.length})`,
              children: <ConnectionTable data={connections} loading={loading} />,
            },
            {
              key: 'traffic',
              label: '流量',
              children: <TrafficChart data={traffic} loading={loading} height={280} />,
            },
          ]}
        />
      </div>

      <Modal
        title="新 Token（旧 Token 立即失效）"
        open={!!tokenModal}
        onCancel={() => setTokenModal(null)}
        footer={[
          <Button key="close" type="primary" onClick={() => setTokenModal(null)}>
            关闭
          </Button>,
        ]}
      >
        {tokenModal ? (
          <Typography.Paragraph copyable={{ text: tokenModal.token }} style={{ wordBreak: 'break-all' }}>
            <Typography.Text code>{tokenModal.token}</Typography.Text>
          </Typography.Paragraph>
        ) : null}
      </Modal>
    </Space>
  )
}
