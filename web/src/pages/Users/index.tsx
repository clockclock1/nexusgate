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
import dayjs from 'dayjs'
import { usersApi } from '@/api'
import type { UserInfo, UserRole } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

const roleMap: Record<UserRole, { label: string; color: string }> = {
  admin: { label: '管理员', color: 'red' },
  operator: { label: '操作员', color: 'blue' },
  viewer: { label: '只读', color: 'default' },
}

interface UserForm extends Partial<UserInfo> {
  password?: string
}

export default function UsersPage() {
  const [users, setUsers] = useState<UserInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [modalOpen, setModalOpen] = useState(false)
  const [editing, setEditing] = useState<UserInfo | null>(null)
  const [form] = Form.useForm<UserForm>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await usersApi.list()
      setUsers(data)
    } catch (err) {
      message.error(getErrorMessage(err, '加载用户列表失败'))
      setUsers([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const openCreate = () => {
    setEditing(null)
    form.resetFields()
    form.setFieldsValue({ role: 'viewer', enabled: true })
    setModalOpen(true)
  }

  const openEdit = (record: UserInfo) => {
    setEditing(record)
    form.setFieldsValue(record)
    setModalOpen(true)
  }

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields()
      if (editing) {
        await usersApi.update(editing.user_id, values)
        message.success('用户已更新')
      } else {
        await usersApi.create(values)
        message.success('用户已创建')
      }
      setModalOpen(false)
      load()
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '保存用户失败'))
    }
  }

  const handleRemove = async (id: string) => {
    try {
      await usersApi.remove(id)
      message.success('用户已删除')
      load()
    } catch (err) {
      message.error(getErrorMessage(err, '删除用户失败'))
    }
  }

  const columns: ColumnsType<UserInfo> = [
    { title: '用户名', dataIndex: 'username' },
    {
      title: '角色',
      dataIndex: 'role',
      width: 100,
      render: (r: UserRole) => {
        const m = roleMap[r] || { label: r, color: 'default' }
        return <Tag color={m.color}>{m.label}</Tag>
      },
    },
    { title: '邮箱', dataIndex: 'email', render: (v) => v || '-' },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 80,
      render: (v: boolean) => <Tag color={v ? 'success' : 'default'}>{v ? '启用' : '禁用'}</Tag>,
    },
    {
      title: '最后登录',
      dataIndex: 'last_login',
      width: 170,
      render: (v) => (v ? dayjs(v).format('YYYY-MM-DD HH:mm:ss') : '-'),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      width: 170,
      render: (v) => (v ? dayjs(v).format('YYYY-MM-DD HH:mm:ss') : '-'),
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
          <Popconfirm title="确定删除该用户？" onConfirm={() => handleRemove(r.user_id)}>
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
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={4} style={{ margin: 0 }}>
          用户管理
        </Title>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={load}>
            刷新
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
            新建用户
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Table
          rowKey="user_id"
          columns={columns}
          dataSource={users}
          loading={loading}
          scroll={{ x: 900 }}
          locale={{ emptyText: '暂无用户' }}
        />
      </div>

      <Modal
        title={editing ? '编辑用户' : '新建用户'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSubmit}
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="username"
            label="用户名"
            rules={[{ required: true, message: '请输入用户名' }]}
          >
            <Input disabled={Boolean(editing)} />
          </Form.Item>
          <Form.Item
            name="password"
            label={editing ? '新密码（留空不修改）' : '密码'}
            rules={editing ? [] : [{ required: true, message: '请输入密码' }]}
          >
            <Input.Password />
          </Form.Item>
          <Form.Item name="role" label="角色" rules={[{ required: true }]}>
            <Select
              options={[
                { value: 'admin', label: '管理员' },
                { value: 'operator', label: '操作员' },
                { value: 'viewer', label: '只读' },
              ]}
            />
          </Form.Item>
          <Form.Item name="email" label="邮箱">
            <Input type="email" />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  )
}
