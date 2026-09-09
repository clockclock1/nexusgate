import { useCallback, useEffect, useState } from 'react'
import {
  Alert,
  Button,
  Col,
  Descriptions,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Progress,
  Row,
  Space,
  Spin,
  Switch,
  Table,
  Tag,
  Typography,
  message,
} from 'antd'
import type { ColumnsType } from 'antd/es/table'
import { ReloadOutlined, SaveOutlined } from '@ant-design/icons'
import dayjs from 'dayjs'
import { serverApi } from '@/api'
import type { ServerConfig, ServerInfo, ServerPort } from '@/types'
import { formatBytes, formatDuration, getErrorMessage } from '@/utils/format'

const { Title } = Typography

const statusMap: Record<string, { label: string; color: string }> = {
  online: { label: '在线', color: 'success' },
  offline: { label: '离线', color: 'error' },
  degraded: { label: '降级', color: 'warning' },
}

export default function ServerPage() {
  const [info, setInfo] = useState<ServerInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [restarting, setRestarting] = useState(false)
  const [saving, setSaving] = useState(false)
  const [form] = Form.useForm<Partial<ServerConfig>>()

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [data, cfg] = await Promise.all([serverApi.getInfo(), serverApi.getConfig()])
      setInfo(data)
      form.setFieldsValue(cfg)
    } catch (err) {
      message.error(getErrorMessage(err, '获取服务端信息失败'))
      setInfo(null)
    } finally {
      setLoading(false)
    }
  }, [form])

  useEffect(() => {
    load()
    const t = window.setInterval(() => {
      serverApi
        .getInfo()
        .then(setInfo)
        .catch(() => undefined)
    }, 15000)
    return () => window.clearInterval(t)
  }, [load])

  const handleSave = async () => {
    try {
      const values = await form.validateFields()
      setSaving(true)
      const updated = await serverApi.updateConfig(values)
      form.setFieldsValue(updated)
      message.success('配置已写入 server.toml（端口变更需重启生效）')
    } catch (err) {
      if ((err as { errorFields?: unknown }).errorFields) return
      message.error(getErrorMessage(err, '保存配置失败'))
    } finally {
      setSaving(false)
    }
  }

  const handleRestart = async () => {
    setRestarting(true)
    try {
      await serverApi.restart()
      message.success('已发送重启指令，请稍候刷新')
      setTimeout(load, 3000)
    } catch (err) {
      message.error(getErrorMessage(err, '重启失败'))
    } finally {
      setRestarting(false)
    }
  }

  const portColumns: ColumnsType<ServerPort> = [
    { title: '名称', dataIndex: 'name' },
    { title: '端口', dataIndex: 'port', width: 100 },
    { title: '协议', dataIndex: 'protocol', width: 90 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (s: string) => {
        const map: Record<string, string> = {
          listening: '监听中',
          closed: '已关闭',
          error: '错误',
        }
        const color = s === 'listening' ? 'success' : s === 'error' ? 'error' : 'default'
        return <Tag color={color}>{map[s] || s}</Tag>
      },
    },
  ]

  const st = info?.status ? statusMap[info.status] : null

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={4} style={{ margin: 0 }}>
          服务端
        </Title>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={load} loading={loading}>
            刷新
          </Button>
          <Button type="primary" icon={<SaveOutlined />} onClick={handleSave} loading={saving}>
            保存配置
          </Button>
          <Popconfirm
            title="确定重启服务端？"
            description="进程将退出并由 systemd 拉起（需 Restart=always）"
            onConfirm={handleRestart}
          >
            <Button danger loading={restarting}>
              重启服务
            </Button>
          </Popconfirm>
        </Space>
      </div>

      <Alert
        type="warning"
        showIcon
        message="监听端口 / 地址修改保存后需重启才会真正换绑"
      />

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={14}>
          <div className="page-card" style={{ padding: 20 }}>
            <Spin spinning={loading}>
              <Descriptions title="运行状态" bordered size="small" column={{ xs: 1, sm: 2 }}>
                <Descriptions.Item label="状态">
                  {st ? (
                    <Tag color={st.color}>
                      <span className={`server-status-dot ${info?.status}`} />
                      {st.label}
                    </Tag>
                  ) : (
                    '-'
                  )}
                </Descriptions.Item>
                <Descriptions.Item label="版本">{info?.version ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="主机名">{info?.hostname ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="监听地址">{info?.listen_addr ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="运行时长">
                  {formatDuration(info?.uptime_secs)}
                </Descriptions.Item>
                <Descriptions.Item label="启动时间">
                  {info?.started_at ? dayjs(info.started_at).format('YYYY-MM-DD HH:mm:ss') : '-'}
                </Descriptions.Item>
                <Descriptions.Item label="节点数">{info?.node_count ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="连接数">{info?.connection_count ?? '-'}</Descriptions.Item>
              </Descriptions>
            </Spin>
          </div>
        </Col>
        <Col xs={24} lg={10}>
          <div className="page-card" style={{ padding: 20 }}>
            <Title level={5} style={{ marginTop: 0 }}>
              资源使用
            </Title>
            <Space direction="vertical" style={{ width: '100%' }} size={20}>
              <div>
                <div style={{ marginBottom: 8, color: 'var(--text-secondary)' }}>CPU</div>
                <Progress percent={Math.round(info?.cpu ?? 0)} status="active" strokeColor="#14b8a6" />
              </div>
              <div>
                <div style={{ marginBottom: 8, color: 'var(--text-secondary)' }}>
                  内存
                  {info?.memory_total
                    ? ` (${formatBytes(info.memory)} / ${formatBytes(info.memory_total)})`
                    : ''}
                </div>
                <Progress
                  percent={Math.round(info?.memory ?? 0)}
                  status="active"
                  strokeColor="#22d3ee"
                />
              </div>
            </Space>
          </div>
        </Col>
      </Row>

      <div className="page-card" style={{ padding: 20 }}>
        <Title level={5} style={{ marginTop: 0 }}>
          运行配置
        </Title>
        <Form form={form} layout="vertical" disabled={loading}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="listen_addr" label="监听地址" rules={[{ required: true }]}>
                <Input />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="api_port" label="API" rules={[{ required: true }]}>
                <InputNumber min={1} max={65535} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="control_port" label="Control" rules={[{ required: true }]}>
                <InputNumber min={1} max={65535} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="data_port" label="Data" rules={[{ required: true }]}>
                <InputNumber min={1} max={65535} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="gateway_port" label="Gateway" rules={[{ required: true }]}>
                <InputNumber min={1} max={65535} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="max_connections" label="最大连接数">
                <InputNumber min={1} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="jwt_ttl_secs" label="JWT TTL (秒)">
                <InputNumber min={60} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="enable_p2p" label="P2P" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="enable_relay" label="中继" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
          </Row>
        </Form>
      </div>

      <div className="page-card" style={{ padding: 16 }}>
        <Title level={5} style={{ marginTop: 0 }}>
          端口监听
        </Title>
        <Table
          rowKey={(r) => `${r.name}-${r.port}`}
          columns={portColumns}
          dataSource={info?.ports ?? []}
          loading={loading}
          pagination={false}
          locale={{ emptyText: '暂无端口信息' }}
          size="middle"
        />
      </div>
    </Space>
  )
}
