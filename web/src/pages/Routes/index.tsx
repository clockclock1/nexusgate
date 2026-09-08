import { useCallback, useEffect, useState } from 'react'
import {
  Button,
  Form,
  Input,
  InputNumber,
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
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons'
import { nodesApi, routesApi, servicesApi } from '@/api'
import type { NodeInfo, RouteInfo, ServiceInfo } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

export default function RoutesPage() {
  const [routes, setRoutes] = useState<RouteInfo[]>([])
  const [nodes, setNodes] = useState<NodeInfo[]>([])
  const [services, setServices] = useState<ServiceInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [modalOpen, setModalOpen] = useState(false)
  const [editing, setEditing] = useState<RouteInfo | null>(null)
  const [nodeFilter, setNodeFilter] = useState<string | undefined>()
  const [form] = Form.useForm<Partial<RouteInfo>>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [rts, nds, svcs] = await Promise.all([
        routesApi.list({ node_id: nodeFilter }),
        nodesApi.list(),
        servicesApi.list(),
      ])
      setRoutes(rts)
      setNodes(nds)
      setServices(svcs)
    } catch (err) {
      message.error(getErrorMessage(err, '加载路由列表失败'))
      setRoutes([])
    } finally {
      setLoading(false)
    }
  }, [nodeFilter])

  useEffect(() => {
    load()
  }, [load])

  const openCreate = () => {
    setEditing(null)
    form.resetFields()
    form.setFieldsValue({ enabled: true, protocol: 'tcp' })
    setModalOpen(true)
  }

  const openEdit = (record: RouteInfo) => {
    setEditing(record)
    form.setFieldsValue(record)
    setModalOpen(true)
  }

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields()
      if (editing) {
        await routesApi.update(editing.route_id, values)
        message.success('路由已更新')
      } else {
        await routesApi.create(values)
        message.success('路由已创建')
      }
      setModalOpen(false)
      load()
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '保存路由失败'))
    }
  }

  const handleRemove = async (id: string) => {
    try {
      await routesApi.remove(id)
      message.success('路由已删除')
      load()
    } catch (err) {
      message.error(getErrorMessage(err, '删除路由失败'))
    }
  }

  const columns: ColumnsType<RouteInfo> = [
    { title: '路由 ID', dataIndex: 'route_id', ellipsis: true, render: (v) => <code>{v}</code> },
    { title: '名称', dataIndex: 'name' },
    { title: '公网端口', dataIndex: 'public_port', width: 100 },
    { title: '协议', dataIndex: 'protocol', width: 80 },
    { title: '节点', dataIndex: 'node_name', render: (v, r) => v || r.node_id },
    { title: '服务', dataIndex: 'service_name', render: (v, r) => v || r.service_id },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 80,
      render: (v: boolean) => <Tag color={v ? 'success' : 'default'}>{v ? '启用' : '禁用'}</Tag>,
    },
    {
      title: '操作',
      key: 'actions',
      width: 140,
      render: (_, r) => (
        <Space>
          <Button type="link" size="small" onClick={() => openEdit(r)}>
            编辑
          </Button>
          <Popconfirm title="确定删除该路由？" onConfirm={() => handleRemove(r.route_id)}>
            <Button type="link" size="small" danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  const selectedNodeId = Form.useWatch('node_id', form)
  const filteredServices = services.filter((s) => !selectedNodeId || s.node_id === selectedNodeId)

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          路由管理
        </Title>
        <Space wrap>
          <Select
            placeholder="按节点筛选"
            allowClear
            style={{ width: 180 }}
            value={nodeFilter}
            onChange={setNodeFilter}
            options={nodes.map((n) => ({ value: n.node_id, label: n.name || n.node_id }))}
          />
          <Button icon={<ReloadOutlined />} onClick={load}>
            刷新
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
            新建路由
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Table
          rowKey="route_id"
          columns={columns}
          dataSource={routes}
          loading={loading}
          scroll={{ x: 1000 }}
          locale={{ emptyText: '暂无路由' }}
        />
      </div>

      <Modal
        title={editing ? '编辑路由' : '新建路由'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSubmit}
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="node_id" label="节点" rules={[{ required: true, message: '请选择节点' }]}>
            <Select
              options={nodes.map((n) => ({ value: n.node_id, label: n.name || n.node_id }))}
              placeholder="选择节点"
              onChange={() => form.setFieldValue('service_id', undefined)}
            />
          </Form.Item>
          <Form.Item name="service_id" label="服务" rules={[{ required: true, message: '请选择服务' }]}>
            <Select
              options={filteredServices.map((s) => ({
                value: s.service_id,
                label: `${s.name} (${s.local_addr}:${s.local_port})`,
              }))}
              placeholder="选择服务"
            />
          </Form.Item>
          <Form.Item name="public_port" label="公网端口" rules={[{ required: true }]}>
            <InputNumber min={1} max={65535} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="protocol" label="协议" rules={[{ required: true }]}>
            <Select
              options={[
                { value: 'tcp', label: 'TCP' },
                { value: 'udp', label: 'UDP' },
                { value: 'http', label: 'HTTP' },
                { value: 'https', label: 'HTTPS' },
              ]}
            />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  )
}
