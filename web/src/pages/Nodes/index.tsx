import { useCallback, useEffect, useState } from 'react'
import {
  Button,
  Input,
  Popconfirm,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
} from 'antd'
import type { ColumnsType } from 'antd/es/table'
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons'
import { useNavigate } from 'react-router-dom'
import dayjs from 'dayjs'
import { nodesApi } from '@/api'
import type { NodeInfo } from '@/types'
import { formatBytes, getErrorMessage } from '@/utils/format'

const { Title } = Typography

const statusMap: Record<string, { label: string; color: string }> = {
  online: { label: '在线', color: 'success' },
  offline: { label: '离线', color: 'default' },
  disabled: { label: '已禁用', color: 'warning' },
}

export default function NodesPage() {
  const navigate = useNavigate()
  const [nodes, setNodes] = useState<NodeInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [statusFilter, setStatusFilter] = useState<string | undefined>()
  const [keyword, setKeyword] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await nodesApi.list({
        status: statusFilter,
        q: keyword || undefined,
      })
      setNodes(data)
    } catch (err) {
      message.error(getErrorMessage(err, '加载节点列表失败'))
      setNodes([])
    } finally {
      setLoading(false)
    }
  }, [statusFilter, keyword])

  useEffect(() => {
    load()
  }, [load])

  const handleRemove = async (id: string) => {
    try {
      await nodesApi.remove(id)
      message.success('节点已删除')
      load()
    } catch (err) {
      message.error(getErrorMessage(err, '删除节点失败'))
    }
  }

  const columns: ColumnsType<NodeInfo> = [
    {
      title: '节点 ID',
      dataIndex: 'node_id',
      ellipsis: true,
      render: (v, r) => (
        <Button type="link" size="small" onClick={() => navigate(`/nodes/${r.node_id}`)}>
          <code>{v}</code>
        </Button>
      ),
    },
    { title: '名称', dataIndex: 'name' },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (s: string) => {
        const m = statusMap[s] || { label: s, color: 'default' }
        return <Tag color={m.color}>{m.label}</Tag>
      },
    },
    { title: '公网 IP', dataIndex: 'public_ip', render: (v) => v || '-' },
    { title: '内网 IP', dataIndex: 'private_ip', render: (v) => v || '-' },
    { title: 'NAT', dataIndex: 'nat_type', width: 100, render: (v) => v || '-' },
    { title: '版本', dataIndex: 'version', width: 100 },
    {
      title: '连接数',
      dataIndex: 'connections',
      width: 80,
      render: (v) => v ?? '-',
    },
    {
      title: '流量',
      key: 'traffic',
      render: (_, r) => (
        <Space direction="vertical" size={0} style={{ fontSize: 12 }}>
          <span>↓ {formatBytes(r.rx_bytes)}</span>
          <span>↑ {formatBytes(r.tx_bytes)}</span>
        </Space>
      ),
    },
    {
      title: '最后在线',
      dataIndex: 'last_seen',
      width: 160,
      render: (v) => (v ? dayjs(v).format('YYYY-MM-DD HH:mm:ss') : '-'),
    },
    {
      title: '操作',
      key: 'actions',
      width: 120,
      render: (_, r) => (
        <Space>
          <Button type="link" size="small" onClick={() => navigate(`/nodes/${r.node_id}`)}>
            详情
          </Button>
          <Popconfirm title="确定删除该节点？" onConfirm={() => handleRemove(r.node_id)}>
            <Button type="link" size="small" danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          节点管理
        </Title>
        <Space wrap>
          <Input.Search
            placeholder="搜索节点"
            allowClear
            style={{ width: 200 }}
            onSearch={setKeyword}
          />
          <Select
            placeholder="状态筛选"
            allowClear
            style={{ width: 120 }}
            value={statusFilter}
            onChange={setStatusFilter}
            options={[
              { value: 'online', label: '在线' },
              { value: 'offline', label: '离线' },
              { value: 'disabled', label: '已禁用' },
            ]}
          />
          <Button icon={<ReloadOutlined />} onClick={load}>
            刷新
          </Button>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            添加节点
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Table
          rowKey="node_id"
          columns={columns}
          dataSource={nodes}
          loading={loading}
          scroll={{ x: 1200 }}
          locale={{ emptyText: '暂无节点' }}
        />
      </div>
    </Space>
  )
}
