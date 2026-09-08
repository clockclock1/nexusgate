import { useEffect, useMemo, useRef } from 'react'
import type { NetworkTopology } from '@/types'

interface NetworkGraphProps {
  data?: NetworkTopology | null
  height?: number
}

const STATUS_COLOR: Record<string, string> = {
  online: '#14b8a6',
  offline: '#64748b',
  disabled: '#f59e0b',
}

const EDGE_COLOR: Record<string, string> = {
  p2p: '#22d3ee',
  relay: '#f59e0b',
  unknown: '#64748b',
}

export default function NetworkGraph({ data, height = 480 }: NetworkGraphProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const nodes = data?.nodes ?? []
  const edges = data?.edges ?? []

  const layout = useMemo(() => {
    const cx = 480
    const cy = 240
    const r = Math.min(180, 40 + nodes.length * 18)
    return nodes.map((n, i) => {
      const angle = (Math.PI * 2 * i) / Math.max(nodes.length, 1) - Math.PI / 2
      return {
        ...n,
        x: cx + Math.cos(angle) * r,
        y: cy + Math.sin(angle) * r,
      }
    })
  }, [nodes])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const dpr = window.devicePixelRatio || 1
    const w = canvas.clientWidth
    const h = height
    canvas.width = w * dpr
    canvas.height = h * dpr
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

    ctx.clearRect(0, 0, w, h)
    ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--bg-elevated').trim() || '#152238'
    ctx.fillRect(0, 0, w, h)

    // subtle grid
    ctx.strokeStyle = 'rgba(100, 116, 139, 0.15)'
    ctx.lineWidth = 1
    for (let x = 0; x < w; x += 40) {
      ctx.beginPath()
      ctx.moveTo(x, 0)
      ctx.lineTo(x, h)
      ctx.stroke()
    }
    for (let y = 0; y < h; y += 40) {
      ctx.beginPath()
      ctx.moveTo(0, y)
      ctx.lineTo(w, y)
      ctx.stroke()
    }

    if (!layout.length) {
      ctx.fillStyle = '#8fa3bc'
      ctx.font = '14px sans-serif'
      ctx.textAlign = 'center'
      ctx.fillText('暂无拓扑数据', w / 2, h / 2)
      return
    }

    const scaleX = w / 960
    const scaleY = h / 480
    const pos = new Map(layout.map((n) => [n.id, { x: n.x * scaleX, y: n.y * scaleY }]))

    edges.forEach((e) => {
      const a = pos.get(e.source)
      const b = pos.get(e.target)
      if (!a || !b) return
      ctx.beginPath()
      ctx.strokeStyle = EDGE_COLOR[e.mode] || EDGE_COLOR.unknown
      ctx.lineWidth = e.mode === 'p2p' ? 2 : 1.5
      ctx.setLineDash(e.mode === 'relay' ? [6, 4] : [])
      ctx.moveTo(a.x, a.y)
      ctx.lineTo(b.x, b.y)
      ctx.stroke()
      ctx.setLineDash([])
    })

    layout.forEach((n) => {
      const p = pos.get(n.id)!
      ctx.beginPath()
      ctx.fillStyle = STATUS_COLOR[n.status] || STATUS_COLOR.offline
      ctx.arc(p.x, p.y, 14, 0, Math.PI * 2)
      ctx.fill()
      ctx.strokeStyle = 'rgba(255,255,255,0.35)'
      ctx.lineWidth = 2
      ctx.stroke()

      ctx.fillStyle = '#e8eef7'
      ctx.font = '12px sans-serif'
      ctx.textAlign = 'center'
      ctx.fillText(n.name || n.id, p.x, p.y + 28)
      if (n.nat_type) {
        ctx.fillStyle = '#8fa3bc'
        ctx.font = '10px sans-serif'
        ctx.fillText(n.nat_type, p.x, p.y + 42)
      }
    })
  }, [layout, edges, height])

  return (
    <div className="network-graph-wrap" style={{ height }}>
      <canvas ref={canvasRef} style={{ width: '100%', height }} />
      <div style={{ display: 'flex', gap: 16, padding: '8px 12px', fontSize: 12, color: 'var(--text-secondary)' }}>
        <span><i style={{ color: '#22d3ee' }}>━</i> P2P</span>
        <span><i style={{ color: '#f59e0b' }}>┅</i> 中继</span>
        <span><i style={{ color: '#14b8a6' }}>●</i> 在线</span>
        <span><i style={{ color: '#64748b' }}>●</i> 离线</span>
      </div>
    </div>
  )
}
