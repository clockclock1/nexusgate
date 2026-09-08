import { Table, Tag, Space } from 'antd'
import type { ColumnsType } from 'antd/es/table'
import dayjs from 'dayjs'
import type { ConnectionInfo } from '@/types'
import { formatBytes, formatDuration } from '@/utils/format'

interface ConnectionTableProps {
  data: ConnectionInfo[]
  loading?: boolean
  pagination?: boolean | object
}

const modeColor: Record<string, string> = {
  p2p: 'cyan',
  relay: 'orange',
  unknown: 'default',
}

const statusColor: Record<string, string> = {
  active: 'success',
  connecting: 'processing',
  closed: 'default',
  failed: 'error',
}

export default function ConnectionTable({ data, loading, pagination = true }: ConnectionTableProps) {
  const columns: ColumnsType<ConnectionInfo> = [
    {
      title: '连接 ID',
      dataIndex: 'conn_id',
      ellipsis: true,
      width: 140,
      render: (v) => <code>{v}</code>,
    },
    {
      title: '节点',
      dataIndex: 'node_name',
      render: (v, r) => v || r.node_id,
    },
    {
      title: '对端',
      dataIndex: 'peer_name',
      render: (v, r) => v || r.peer_node_id || '-',
    },
    {
      title: '服务',
      dataIndex: 'service_name',
      render: (v, r) => v || r.service_id || '-',
    },
    {
      title: '模式',
      dataIndex: 'mode',
      width: 90,
      render: (mode: string) => (
        <Tag color={modeColor[mode] || 'default'}>{mode === 'p2p' ? 'P2P' : mode === 'relay' ? '中继' : mode}</Tag>
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (s: string) => (
        <Tag color={statusColor[s] || 'default'}>
          {{ active: '活跃', connecting: '连接中', closed: '已关闭', failed: '失败' }[s] || s}
        </Tag>
      ),
    },
    {
      title: '流量',
      key: 'traffic',
      render: (_, r) => (
        <Space size={4} direction="vertical" style={{ fontSize: 12 }}>
          <span>↓ {formatBytes(r.rx_bytes)}</span>
          <span>↑ {formatBytes(r.tx_bytes)}</span>
        </Space>
      ),
    },
    {
      title: '时长',
      dataIndex: 'duration_secs',
      width: 110,
      render: (v) => formatDuration(v),
    },
    {
      title: '开始时间',
      dataIndex: 'started_at',
      width: 160,
      render: (v) => (v ? dayjs(v).format('YYYY-MM-DD HH:mm:ss') : '-'),
    },
  ]

  return (
    <Table
      rowKey={(r) => r.conn_id}
      columns={columns}
      dataSource={data}
      loading={loading}
      pagination={pagination === false ? false : { pageSize: 10, showSizeChanger: true, ...(typeof pagination === 'object' ? pagination : {}) }}
      size="middle"
      scroll={{ x: 1000 }}
      locale={{ emptyText: '暂无连接' }}
    />
  )
}
