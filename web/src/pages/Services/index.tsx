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
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons'
import { nodesApi, servicesApi } from '@/api'
import type { NodeInfo, ServiceInfo } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

export default function ServicesPage() {
  const [services, setServices] = useState<ServiceInfo[]>([])
  const [nodes, setNodes] = useState<NodeInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [modalOpen, setModalOpen] = useState(false)
  const [editing, setEditing] = useState<ServiceInfo | null>(null)
  const [nodeFilter, setNodeFilter] = useState<string | undefined>()
  const [form] = Form.useForm<Partial<ServiceInfo>>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [svcs, nds] = await Promise.all([
        servicesApi.list({ node_id: nodeFilter }),
        nodesApi.list(),
      ])
      setServices(svcs)
      setNodes(nds)
    } catch (err) {
      message.error(getErrorMessage(err, '加载服务列表失败'))
      setServices([])
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

  const openEdit = (record: ServiceInfo) => {
    setEditing(record)
    form.setFieldsValue(record)
    setModalOpen(true)
  }

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields()
      if (editing) {
        await servicesApi.update(editing.service_id, values)
        message.success('服务已更新')
      } else {
        await servicesApi.create(values)
        message.success('服务已创建')
      }
      setModalOpen(false)
      load()
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '保存服务失败'))
    }
  }

  const handleRemove = async (id: string) => {
    try {
      await servicesApi.remove(id)
      message.success('服务已删除')
      load()
    } catch (err) {
      message.error(getErrorMessage(err, '删除服务失败'))
    }
  }

  const columns: ColumnsType<ServiceInfo> = [
    { title: '服务 ID', dataIndex: 'service_id', ellipsis: true, render: (v) => <code>{v}</code> },
    { title: '名称', dataIndex: 'name' },
    { title: '节点', dataIndex: 'node_name', render: (v, r) => v || r.node_id },
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
    {
      title: '操作',
      key: 'actions',
      width: 140,
      render: (_, r) => (
        <Space>
          <Button type="link" size="small" onClick={() => openEdit(r)}>
            编辑
          </Button>
          <Popconfirm title="确定删除该服务？" onConfirm={() => handleRemove(r.service_id)}>
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
          服务管理
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
            新建服务
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Table
          rowKey="service_id"
          columns={columns}
          dataSource={services}
          loading={loading}
          scroll={{ x: 900 }}
          locale={{ emptyText: '暂无服务' }}
        />
      </div>

      <Modal
        title={editing ? '编辑服务' : '新建服务'}
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
            />
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
          <Form.Item name="local_addr" label="本地地址" rules={[{ required: true }]}>
            <Input placeholder="127.0.0.1" />
          </Form.Item>
          <Form.Item name="local_port" label="本地端口" rules={[{ required: true }]}>
            <Input type="number" />
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
