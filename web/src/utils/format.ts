export function formatBytes(bytes?: number | null): string {
  if (bytes == null || Number.isNaN(bytes)) return '-'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = Math.max(0, bytes)
  let i = 0
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024
    i += 1
  }
  const digits = i === 0 ? 0 : value < 10 ? 2 : 1
  return `${value.toFixed(digits)} ${units[i]}`
}

export function formatDuration(secs?: number | null): string {
  if (secs == null || secs < 0) return '-'
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = Math.floor(secs % 60)
  if (d > 0) return `${d}天 ${h}小时`
  if (h > 0) return `${h}小时 ${m}分`
  if (m > 0) return `${m}分 ${s}秒`
  return `${s}秒`
}

export function formatPercent(value?: number | null, digits = 1): string {
  if (value == null || Number.isNaN(value)) return '-'
  return `${(value * (value > 1 ? 1 : 100)).toFixed(digits)}%`.replace(/(\d+)\.0+%/, '$1%')
}

/** Accepts 0-1 or 0-100 rates and returns 0-100 for display */
export function toPercent(value?: number | null): number {
  if (value == null || Number.isNaN(value)) return 0
  return value <= 1 ? value * 100 : value
}

export function statusColor(status?: string): string {
  switch (status) {
    case 'online':
    case 'active':
    case 'listening':
      return '#14b8a6'
    case 'offline':
    case 'closed':
    case 'failed':
      return '#ef4444'
    case 'disabled':
    case 'degraded':
    case 'connecting':
      return '#f59e0b'
    default:
      return '#64748b'
  }
}

export function getErrorMessage(err: unknown, fallback = '请求失败'): string {
  if (!err) return fallback
  if (typeof err === 'string') return err
  const anyErr = err as {
    response?: { data?: { message?: string; error?: string } }
    message?: string
  }
  return (
    anyErr.response?.data?.message ||
    anyErr.response?.data?.error ||
    anyErr.message ||
    fallback
  )
}
