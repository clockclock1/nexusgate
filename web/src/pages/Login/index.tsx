import { useEffect, useState } from 'react'
import { Button, Checkbox, Form, Input, Select, message } from 'antd'
import { LockOutlined, UserOutlined } from '@ant-design/icons'
import { useNavigate } from 'react-router-dom'
import { authApi } from '@/api'
import { useAuthStore } from '@/stores/authStore'
import { useServerStore } from '@/stores/serverStore'
import { getErrorMessage } from '@/utils/format'

const REMEMBER_KEY = 'p2p-remember-user'

interface LoginForm {
  username: string
  password: string
  remember: boolean
  server_id: string
}

export default function LoginPage() {
  const navigate = useNavigate()
  const setAuth = useAuthStore((s) => s.setAuth)
  const [loading, setLoading] = useState(false)
  const [form] = Form.useForm<LoginForm>()
  const servers = useServerStore((s) => s.servers)
  const activeServerId = useServerStore((s) => s.activeServerId)
  const loadServers = useServerStore((s) => s.loadServers)
  const setActiveServerId = useServerStore((s) => s.setActiveServerId)

  useEffect(() => {
    loadServers()
      .then(() => {
        const id = useServerStore.getState().activeServerId
        if (id) form.setFieldValue('server_id', id)
      })
      .catch(() => undefined)
  }, [loadServers, form])

  const onFinish = async (values: LoginForm) => {
    setLoading(true)
    try {
      setActiveServerId(values.server_id)
      const res = await authApi.login({
        username: values.username,
        password: values.password,
      })
      setAuth(res.token, res.user, values.server_id)
      if (values.remember) {
        localStorage.setItem(REMEMBER_KEY, values.username)
      } else {
        localStorage.removeItem(REMEMBER_KEY)
      }
      message.success('登录成功')
      navigate('/dashboard', { replace: true })
    } catch (err) {
      message.error(getErrorMessage(err, '登录失败，请检查用户名和密码'))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="login-page">
      <div className="login-panel">
        <h1>P2P Network</h1>
        <p className="subtitle">内网穿透 · 网络控制台</p>
        <Form
          form={form}
          layout="vertical"
          onFinish={onFinish}
          initialValues={{
            username: localStorage.getItem(REMEMBER_KEY) || '',
            remember: Boolean(localStorage.getItem(REMEMBER_KEY)),
            server_id: activeServerId || undefined,
          }}
          requiredMark={false}
        >
          <Form.Item
            name="server_id"
            label="服务端节点"
            rules={[{ required: true, message: '请选择服务端' }]}
          >
            <Select
              size="large"
              placeholder={servers.length ? '选择要登录的服务端' : '暂无服务端，请先在面板配置'}
              options={servers.map((s) => ({
                value: s.id,
                label: `${s.name} (${s.api_upstream})`,
              }))}
              onChange={(id) => setActiveServerId(id)}
            />
          </Form.Item>
          <Form.Item
            name="username"
            label="用户名"
            rules={[{ required: true, message: '请输入用户名' }]}
          >
            <Input prefix={<UserOutlined />} placeholder="admin" size="large" autoComplete="username" />
          </Form.Item>
          <Form.Item
            name="password"
            label="密码"
            rules={[{ required: true, message: '请输入密码' }]}
          >
            <Input.Password
              prefix={<LockOutlined />}
              placeholder="请输入密码"
              size="large"
              autoComplete="current-password"
            />
          </Form.Item>
          <Form.Item name="remember" valuePropName="checked">
            <Checkbox>记住我</Checkbox>
          </Form.Item>
          <Form.Item style={{ marginBottom: 0 }}>
            <Button type="primary" htmlType="submit" block size="large" loading={loading}>
              登录
            </Button>
          </Form.Item>
        </Form>
        <div className="login-hint">默认账号：admin / admin123（各服务端独立）</div>
      </div>
    </div>
  )
}
