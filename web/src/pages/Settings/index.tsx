import { useCallback, useEffect, useState } from 'react'
import {
  Button,
  Col,
  Form,
  Input,
  InputNumber,
  Row,
  Select,
  Space,
  Switch,
  Tabs,
  Typography,
  message,
} from 'antd'
import { ReloadOutlined, SaveOutlined } from '@ant-design/icons'
import { settingsApi } from '@/api'
import type { AppSettings } from '@/types'
import { getErrorMessage } from '@/utils/format'

const { Title } = Typography

export default function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [form] = Form.useForm<AppSettings>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await settingsApi.get()
      setSettings(data)
      form.setFieldsValue(data)
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
      const updated = await settingsApi.update(values)
      setSettings(updated)
      message.success('设置已保存')
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
            保存
          </Button>
        </Space>
      </div>

      <div className="page-card" style={{ padding: 20 }}>
        {!settings && !loading ? (
          <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)' }}>
            无法加载设置，请检查服务端连接
          </div>
        ) : (
          <Form form={form} layout="vertical" disabled={loading}>
            <Tabs
              items={[
                {
                  key: 'server',
                  label: '服务端',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={12}>
                        <Form.Item name={['server', 'listen_addr']} label="监听地址">
                          <Input />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12}>
                        <Form.Item name={['server', 'hostname']} label="主机名">
                          <Input />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['server', 'api_port']} label="API 端口">
                          <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['server', 'control_port']} label="控制端口">
                          <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['server', 'data_port']} label="数据端口">
                          <InputNumber min={1} max={65535} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
                {
                  key: 'security',
                  label: '安全',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={12}>
                        <Form.Item name={['security', 'enable_tls']} label="启用 TLS" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12}>
                        <Form.Item name={['security', 'require_auth']} label="要求认证" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12}>
                        <Form.Item name={['security', 'token_ttl_secs']} label="Token 有效期 (秒)">
                          <InputNumber min={60} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12}>
                        <Form.Item name={['security', 'allow_register']} label="允许注册" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
                {
                  key: 'network',
                  label: '网络',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={8}>
                        <Form.Item name={['network', 'mtu']} label="MTU">
                          <InputNumber min={576} max={9000} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['network', 'keepalive_secs']} label="Keepalive (秒)">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['network', 'dial_timeout_secs']} label="拨号超时 (秒)">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
                {
                  key: 'p2p',
                  label: 'P2P',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={12}>
                        <Form.Item name={['p2p', 'enable_hole_punch']} label="启用打洞" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12}>
                        <Form.Item name={['p2p', 'prefer_p2p']} label="优先 P2P" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12}>
                        <Form.Item name={['p2p', 'fallback_relay']} label="回退中继" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24}>
                        <Form.Item name={['p2p', 'stun_servers']} label="STUN 服务器">
                          <Select mode="tags" placeholder="输入 STUN 地址后回车" />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
                {
                  key: 'relay',
                  label: '中继',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={8}>
                        <Form.Item name={['relay', 'enable']} label="启用中继" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['relay', 'max_bandwidth_mbps']} label="最大带宽 (Mbps)">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['relay', 'max_connections']} label="最大连接数">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
                {
                  key: 'limits',
                  label: '限制',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={6}>
                        <Form.Item name={['limits', 'max_nodes']} label="最大节点数">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={6}>
                        <Form.Item name={['limits', 'max_services_per_node']} label="每节点最大服务">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={6}>
                        <Form.Item name={['limits', 'max_routes']} label="最大路由数">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={6}>
                        <Form.Item name={['limits', 'rate_limit_rps']} label="速率限制 (RPS)">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
                {
                  key: 'logging',
                  label: '日志',
                  children: (
                    <Row gutter={16}>
                      <Col xs={24} md={8}>
                        <Form.Item name={['logging', 'level']} label="日志级别">
                          <Select
                            options={[
                              { value: 'trace', label: 'TRACE' },
                              { value: 'debug', label: 'DEBUG' },
                              { value: 'info', label: 'INFO' },
                              { value: 'warn', label: 'WARN' },
                              { value: 'error', label: 'ERROR' },
                            ]}
                          />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['logging', 'retention_days']} label="保留天数">
                          <InputNumber min={1} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={8}>
                        <Form.Item name={['logging', 'enable_audit']} label="审计日志" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                    </Row>
                  ),
                },
              ]}
            />
          </Form>
        )}
      </div>
    </Space>
  )
}
