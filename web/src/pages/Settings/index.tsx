import { useCallback, useEffect, useState } from 'react'
import {
  Alert,
  Button,
  Col,
  Form,
  Input,
  InputNumber,
  Row,
  Space,
  Switch,
  Typography,
  message,
} from 'antd'
import { ReloadOutlined, SaveOutlined } from '@ant-design/icons'
import { settingsApi } from '@/api'
import type { AppSettings } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title, Paragraph } = Typography

type SettingsForm = AppSettings & {
  security: AppSettings['security'] & { jwt_secret?: string }
}

export default function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [form] = Form.useForm<SettingsForm>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await settingsApi.get()
      setSettings(data)
      form.setFieldsValue({
        ...data,
        security: { ...data.security, jwt_secret: '' },
      })
    } catch (err) {
      message.error(getErrorMessage(err, '加载设置失败'))
      setSettings(null)
    } finally {
      setLoading(false)
    }
  }, [form])

  useEffect(() => {
    load()
  }, [load])

  const handleSave = async () => {
    try {
      const values = await form.validateFields()
      setSaving(true)
      const payload: AppSettings = {
        server: values.server,
        security: {
          jwt_ttl_secs: values.security.jwt_ttl_secs,
          ...(values.security.jwt_secret ? { jwt_secret: values.security.jwt_secret } : {}),
        },
        p2p: values.p2p,
        relay: values.relay,
      }
      const updated = await settingsApi.update(payload)
      setSettings(updated)
      form.setFieldsValue({
        ...updated,
        security: { ...updated.security, jwt_secret: '' },
      })
      message.success('设置已写入 server.toml')
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '保存设置失败'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={4} style={{ margin: 0 }}>
          系统设置
        </Title>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
            重新加载
          </Button>
          <Button type="primary" icon={<SaveOutlined />} onClick={handleSave} loading={saving}>
            保存到 server.toml
          </Button>
        </Space>
      </div>

      <Alert
        type="info"
        showIcon
        message="服务端运行参数由管理面板维护"
        description={
          <Paragraph style={{ marginBottom: 0 }}>
            保存会持久化到 <code>{settings?.server?.config_path || 'server.toml'}</code>
            。监听地址与端口变更需在「服务端」页面重启后生效；管理员密码请在「用户管理」修改。
          </Paragraph>
        }
      />

      <div className="page-card" style={{ padding: 20 }}>
        {!settings && !loading ? (
          <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)' }}>
            无法加载设置，请检查服务端连接
          </div>
        ) : (
          <Form form={form} layout="vertical" disabled={loading}>
            <Title level={5}>监听与端口</Title>
            <Row gutter={16}>
              <Col xs={24} md={12}>
                <Form.Item name={['server', 'listen_addr']} label="监听地址" rules={[{ required: true }]}>
                  <Input placeholder="0.0.0.0" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item name={['server', 'api_port']} label="API 端口" rules={[{ required: true }]}>
                  <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item name={['server', 'gateway_port']} label="Gateway 端口" rules={[{ required: true }]}>
                  <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item name={['server', 'control_port']} label="Control 端口" rules={[{ required: true }]}>
                  <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item name={['server', 'data_port']} label="Data 端口" rules={[{ required: true }]}>
                  <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
            </Row>

            <Title level={5}>安全</Title>
            <Row gutter={16}>
              <Col xs={24} md={8}>
                <Form.Item name={['security', 'jwt_ttl_secs']} label="JWT 有效期 (秒)">
                  <InputNumber min={60} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col xs={24} md={16}>
                <Form.Item
                  name={['security', 'jwt_secret']}
                  label="JWT Secret"
                  extra={
                    settings?.security?.jwt_secret_set
                      ? '已配置。留空则保持不变；填写新值将覆盖并建议立即重启。'
                      : '建议设置为足够长的随机串'
                  }
                >
                  <Input.Password placeholder="留空表示不修改" autoComplete="new-password" />
                </Form.Item>
              </Col>
            </Row>

            <Title level={5}>P2P / 中继</Title>
            <Row gutter={16}>
              <Col xs={24} md={8}>
                <Form.Item name={['p2p', 'enable_p2p']} label="启用 P2P" valuePropName="checked">
                  <Switch />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item name={['relay', 'enable_relay']} label="启用中继兜底" valuePropName="checked">
                  <Switch />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item name={['relay', 'max_connections']} label="最大连接数">
                  <InputNumber min={1} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
            </Row>
          </Form>
        )}
      </div>
    </Space>
  )
}
