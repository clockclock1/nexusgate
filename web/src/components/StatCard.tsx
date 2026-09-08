import type { ReactNode } from 'react'
import { Space } from 'antd'

interface StatCardProps {
  title: string
  value: ReactNode
  extra?: ReactNode
  icon?: ReactNode
  loading?: boolean
}

export default function StatCard({ title, value, extra, icon }: StatCardProps) {
  return (
    <div className="stat-card">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div>
          <div className="label">{title}</div>
          <div className="value">{value}</div>
          {extra ? <div className="extra">{extra}</div> : null}
        </div>
        {icon ? <div className="icon-wrap">{icon}</div> : null}
      </div>
    </div>
  )
}

export function StatGrid({ children }: { children: ReactNode }) {
  return <Space direction="vertical" style={{ width: '100%' }} size={16}>{children}</Space>
}
