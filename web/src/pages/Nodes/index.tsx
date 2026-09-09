import { useCallback, useEffect, useState } from 'react'
import {
  Button,
  Form,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Typography,
  message,
} from 'antd'
import type { ColumnsType } from 'antd/es/table'
import { CopyOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons'
import { useNavigate } from 'react-router-dom'
import dayjs from 'dayjs'
import { nodesApi } from '@/api'
import type { NodeInfo } from '@/types'
import { formatBytes, getErrorMessage } from '@/utils/format'

const { Title, Paragraph, Text } = Typography

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
  const [createOpen, setCreateOpen] = useState(false)
  const [editOpen, setEditOpen] = useState(false)
  const [editing, setEditing] = useState<NodeInfo | null>(null)
  const [tokenModal, setTokenModal] = useState<{ node_id: string; token: string; title: string } | null>(
    null,
  )
  const [createForm] = Form.useForm<{ name: string; node_id?: string }>()
  const [editForm] = Form.useForm<{ name: string; enabled: boolean }>()

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

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      message.success('已复制到剪贴板')
    } catch {
      message.warning('复制失败，请手动选择文本')
    }
  }

  const handleCreate = async () => {
    try {
      const values = await createForm.validateFields()
      const created = await nodesApi.create({
        name: values.name,
        node_id: values.node_id || undefined,
      })
      setCreateOpen(false)
      createForm.resetFields()
      message.success('节点已创建')
      if (created.token) {
        setTokenModal({
          node_id: created.node_id,
          token: created.token,
          title: '节点凭证（仅显示一次）',
        })
      }
      load()
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '创建节点失败'))
    }
  }

  const openEdit = (record: NodeInfo) => {
    setEditing(record)
    editForm.setFieldsValue({
      name: record.name,
      enabled: record.enabled !== false && record.status !== 'disabled',
    })
    setEditOpen(true)
  }

  const handleEdit = async () => {
    if (!editing) return
    try {
      const values = await editForm.validateFields()
      await nodesApi.update(editing.node_id, values)
      message.success('节点已更新')
      setEditOpen(false)
      setEditing(null)
      load()
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '更新节点失败'))
    }
  }

  const handleRegen = async (id: string) => {
    try {
      const res = await nodesApi.regenToken(id)
      setTokenModal({
        node_id: res.node_id,
        token: res.token,
        title: '新 Token（旧 Token 立即失效）',
      })
    } catch (err) {
      message.error(getErrorMessage(err, '重置 Token 失败'))
    }
  }

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
      title: '创建时间',
      dataIndex: 'created_at',
      width: 160,
      render: (v) => (v ? dayjs(v).format('YYYY-MM-DD HH:mm:ss') : '-'),
    },
    {
      title: '操作',
      key: 'actions',
      width: 260,
      fixed: 'right',
      render: (_, r) => (
        <Space wrap size={0}>
          <Button type="link" size="small" onClick={() => navigate(`/nodes/${r.node_id}`)}>
            详情
          </Button>
          <Button type="link" size="small" onClick={() => openEdit(r)}>
            编辑
          </Button>
          <Popconfirm title="重置后旧 Token 立即失效，确定？" onConfirm={() => handleRegen(r.node_id)}>
            <Button type="link" size="small">
              换 Token
            </Button>
          </Popconfirm>
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
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: 12,
        }}
      >
        <Title level={4} style={{ margin: 0 }}>
          客户端节点
        </Title>
        <Space wrap>
          <Input.Search placeholder="搜索节点" allowClear style={{ width: 200 }} onSearch={setKeyword} />
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
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => {
              createForm.resetFields()
              setCreateOpen(true)
            }}
          >
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
          locale={{ emptyText: '暂无节点，请先添加' }}
        />
      </div>

      <Modal
        title="添加节点"
        open={createOpen}
        onOk={handleCreate}
        onCancel={() => setCreateOpen(false)}
        okText="创建"
        destroyOnClose
      >
        <Paragraph type="secondary" style={{ marginBottom: 16 }}>
          创建后会生成一次性 Token，写入 Edge 的 <Text code>edge.toml</Text> 即可上线。
        </Paragraph>
        <Form form={createForm} layout="vertical">
          <Form.Item name="name" label="节点名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input placeholder="例如 office-edge-1" />
          </Form.Item>
          <Form.Item name="node_id" label="节点 ID（可选）" extra="留空则自动生成 UUID">
            <Input placeholder="自定义 node_id" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="编辑节点"
        open={editOpen}
        onOk={handleEdit}
        onCancel={() => {
          setEditOpen(false)
          setEditing(null)
        }}
        okText="保存"
        destroyOnClose
      >
        <Form form={editForm} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch checkedChildren="启用" unCheckedChildren="禁用" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={tokenModal?.title || '节点 Token'}
        open={!!tokenModal}
        onCancel={() => setTokenModal(null)}
        footer={[
          <Button
            key="copy"
            type="primary"
            icon={<CopyOutlined />}
            onClick={() => tokenModal && copyText(tokenModal.token)}
          >
            复制 Token
          </Button>,
          <Button key="close" onClick={() => setTokenModal(null)}>
            关闭
          </Button>,
        ]}
      >
        {tokenModal ? (
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <div>
              <Text type="secondary">node_id</Text>
              <Paragraph copyable style={{ marginBottom: 0 }}>
                <Text code>{tokenModal.node_id}</Text>
              </Paragraph>
            </div>
            <div>
              <Text type="secondary">token（请立即保存，之后无法再次查看明文）</Text>
              <Paragraph copyable={{ text: tokenModal.token }} style={{ marginBottom: 0, wordBreak: 'break-all' }}>
                <Text code>{tokenModal.token}</Text>
              </Paragraph>
            </div>
          </Space>
        ) : null}
      </Modal>
    </Space>
  )
}
