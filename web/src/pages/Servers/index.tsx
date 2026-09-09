import { useCallback, useEffect, useState } from 'react'
import {
  Button,
  Form,
  Input,
  Modal,
  Popconfirm,
  Space,
  Switch,
  Table,
  Tag,
  Typography,
  message,
} from 'antd'
import type { ColumnsType } from 'antd/es/table'
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons'
import { managedServersApi } from '@/api/managedServers'
import { applyServerSession } from '@/stores/authStore'
import { useServerStore } from '@/stores/serverStore'
import type { ManagedServer } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title, Paragraph, Text } = Typography

export default function ServersPage() {
  const servers = useServerStore((s) => s.servers)
  const defaultServerId = useServerStore((s) => s.defaultServerId)
  const activeServerId = useServerStore((s) => s.activeServerId)
  const loadServers = useServerStore((s) => s.loadServers)
  const upsertServer = useServerStore((s) => s.upsertServer)
  const removeServer = useServerStore((s) => s.removeServer)
  const setDefaultServer = useServerStore((s) => s.setDefaultServer)
  const setActiveServerId = useServerStore((s) => s.setActiveServerId)

  const [loading, setLoading] = useState(true)
  const [modalOpen, setModalOpen] = useState(false)
  const [editing, setEditing] = useState<ManagedServer | null>(null)
  const [probeMap, setProbeMap] = useState<Record<string, boolean | null>>({})
  const [hubStatus, setHubStatus] = useState<{
    enabled: boolean
    control_port?: number
    data_port?: number
    servers_online?: number
    edges_online?: number
  } | null>(null)
  const [hubPeers, setHubPeers] = useState<
    Array<{ node_id: string; role: string; name?: string; version?: string; online: boolean }>
  >([])
  const [form] = Form.useForm<{ id: string; name: string; api_upstream: string; make_default: boolean }>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      await loadServers()
      const [st, peers] = await Promise.all([
        managedServersApi.hubStatus(),
        managedServersApi.hubPeers(),
      ])
      setHubStatus(st)
      setHubPeers(peers.peers || [])
    } catch (err) {
      message.error(getErrorMessage(err, '加载服务端列表失败'))
    } finally {
      setLoading(false)
    }
  }, [loadServers])

  useEffect(() => {
    load()
  }, [load])

  const probeAll = async () => {
    const list = useServerStore.getState().servers
    const next: Record<string, boolean | null> = {}
    await Promise.all(
      list.map(async (s) => {
        try {
          const r = await managedServersApi.probe(s.id)
          next[s.id] = r.ok
        } catch {
          next[s.id] = false
        }
      }),
    )
    setProbeMap(next)
  }

  useEffect(() => {
    if (servers.length) probeAll()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [servers.map((s) => s.id).join(',')])

  const openCreate = () => {
    setEditing(null)
    form.resetFields()
    form.setFieldsValue({ make_default: false })
    setModalOpen(true)
  }

  const openEdit = (record: ManagedServer) => {
    setEditing(record)
    form.setFieldsValue({
      id: record.id,
      name: record.name,
      api_upstream: record.api_upstream,
      make_default: record.id === defaultServerId,
    })
    setModalOpen(true)
  }

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields()
      await upsertServer({
        id: values.id.trim(),
        name: values.name.trim(),
        api_upstream: values.api_upstream.trim(),
        make_default: values.make_default,
      })
      message.success(editing ? '服务端已更新' : '服务端已添加')
      setModalOpen(false)
      probeAll()
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '保存失败'))
    }
  }

  const handleSelect = (id: string) => {
    setActiveServerId(id)
    applyServerSession()
    message.success('已切换当前服务端')
    if (!useServerStore.getState().activeServerId) return
    // auth sync; layout will re-check
    window.dispatchEvent(new CustomEvent('nexus-server-changed'))
  }

  const columns: ColumnsType<ManagedServer> = [
    {
      title: 'ID',
      dataIndex: 'id',
      render: (v) => <code>{v}</code>,
    },
    { title: '名称', dataIndex: 'name' },
    {
      title: 'API 地址',
      dataIndex: 'api_upstream',
      render: (v) => <Text code>{v}</Text>,
    },
    {
      title: '探测',
      key: 'probe',
      width: 100,
      render: (_, r) => {
        const ok = probeMap[r.id]
        if (ok == null) return <Tag>检测中</Tag>
        return ok ? <Tag color="success">可达</Tag> : <Tag color="error">不可达</Tag>
      },
    },
    {
      title: '标记',
      key: 'flags',
      width: 160,
      render: (_, r) => (
        <Space>
          {r.id === activeServerId ? <Tag color="blue">当前</Tag> : null}
          {r.id === defaultServerId ? <Tag>默认</Tag> : null}
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 280,
      render: (_, r) => (
        <Space wrap size={0}>
          <Button type="link" size="small" disabled={r.id === activeServerId} onClick={() => handleSelect(r.id)}>
            切换
          </Button>
          <Button type="link" size="small" onClick={() => openEdit(r)}>
            编辑
          </Button>
          <Button
            type="link"
            size="small"
            disabled={r.id === defaultServerId}
            onClick={async () => {
              try {
                await setDefaultServer(r.id)
                message.success('已设为默认')
              } catch (err) {
                message.error(getErrorMessage(err, '设置默认失败'))
              }
            }}
          >
            设默认
          </Button>
          <Popconfirm
            title="确定删除该服务端节点？"
            disabled={servers.length <= 1}
            onConfirm={async () => {
              try {
                await removeServer(r.id)
                applyServerSession()
                message.success('已删除')
                window.dispatchEvent(new CustomEvent('nexus-server-changed'))
              } catch (err) {
                message.error(getErrorMessage(err, '删除失败'))
              }
            }}
          >
            <Button type="link" size="small" danger disabled={servers.length <= 1}>
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
        <div>
          <Title level={4} style={{ margin: 0 }}>
            服务端节点
          </Title>
          <Paragraph type="secondary" style={{ marginBottom: 0, marginTop: 4 }}>
            面板内嵌 Admin Hub：服务端/客户端可 P2P 接入（失败则中转）。HTTP 反代作为回退。
          </Paragraph>
        </div>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={() => { load(); probeAll() }}>
            刷新
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
            添加服务端
          </Button>
        </Space>
      </div>

      {hubStatus?.enabled ? (
        <div className="page-card" style={{ padding: 16 }}>
          <Title level={5} style={{ marginTop: 0 }}>
            Admin Hub 在线拓扑
          </Title>
          <Paragraph type="secondary">
            Control {hubStatus.control_port ?? 7100} / Data {hubStatus.data_port ?? 7101} · 服务端在线{' '}
            {hubStatus.servers_online ?? 0} · 客户端在线 {hubStatus.edges_online ?? 0}
          </Paragraph>
          <Table
            rowKey="node_id"
            size="small"
            pagination={false}
            dataSource={hubPeers}
            locale={{ emptyText: '暂无 Hub 对端（请配置 server/edge 的 hub_host + hub_token）' }}
            columns={[
              { title: '节点 ID', dataIndex: 'node_id', render: (v) => <code>{v}</code> },
              {
                title: '角色',
                dataIndex: 'role',
                width: 100,
                render: (r: string) => (
                  <Tag color={r === 'server' ? 'blue' : 'green'}>{r === 'server' ? '服务端' : '客户端'}</Tag>
                ),
              },
              { title: '名称', dataIndex: 'name', render: (v) => v || '-' },
              { title: '版本', dataIndex: 'version', width: 100, render: (v) => v || '-' },
              {
                title: '状态',
                dataIndex: 'online',
                width: 80,
                render: (v: boolean) => <Tag color={v ? 'success' : 'default'}>{v ? '在线' : '离线'}</Tag>,
              },
            ]}
          />
        </div>
      ) : null}

      <div className="page-card" style={{ padding: 16 }}>
        <Title level={5} style={{ marginTop: 0 }}>
          注册的服务端（HTTP 回退）
        </Title>
        <Table
          rowKey="id"
          columns={columns}
          dataSource={servers}
          loading={loading}
          pagination={false}
          locale={{ emptyText: '暂无服务端，请先添加' }}
        />
      </div>

      <Modal
        title={editing ? '编辑服务端' : '添加服务端'}
        open={modalOpen}
        onOk={handleSubmit}
        onCancel={() => setModalOpen(false)}
        okText="保存"
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="id"
            label="ID"
            rules={[{ required: true, message: '请输入 ID' }]}
            extra="唯一标识，例如 bj-1 / local"
          >
            <Input disabled={!!editing} placeholder="local" />
          </Form.Item>
          <Form.Item name="name" label="显示名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input placeholder="北京节点" />
          </Form.Item>
          <Form.Item
            name="api_upstream"
            label="API 地址 (host:port)"
            rules={[{ required: true, message: '请输入上游地址' }]}
            extra="管理面板将反代到该地址，例如 127.0.0.1:3000 或 10.0.0.2:3000"
          >
            <Input placeholder="127.0.0.1:3000" />
          </Form.Item>
          <Form.Item name="make_default" label="设为默认" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  )
}
