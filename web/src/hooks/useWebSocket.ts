import { useCallback, useEffect, useRef, useState } from 'react'
import { useAuthStore } from '@/stores/authStore'

export type WsStatus = 'connecting' | 'open' | 'closed' | 'error'

interface UseWebSocketOptions<T> {
  /** e.g. /ws/dashboard */
  path: string
  enabled?: boolean
  onMessage?: (data: T) => void
  reconnectMs?: number
}

export function useWebSocket<T = unknown>({
  path,
  enabled = true,
  onMessage,
  reconnectMs = 3000,
}: UseWebSocketOptions<T>) {
  const [status, setStatus] = useState<WsStatus>('closed')
  const [lastMessage, setLastMessage] = useState<T | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const timerRef = useRef<number | null>(null)
  const onMessageRef = useRef(onMessage)
  onMessageRef.current = onMessage

  const clearTimer = () => {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current)
      timerRef.current = null
    }
  }

  const connect = useCallback(() => {
    if (!enabled) return
    clearTimer()
    try {
      wsRef.current?.close()
    } catch {
      /* ignore */
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const token = useAuthStore.getState().token || localStorage.getItem('token')
    const qs = token ? `?token=${encodeURIComponent(token)}` : ''
    const url = `${protocol}//${window.location.host}${path}${qs}`

    setStatus('connecting')
    const ws = new WebSocket(url)
    wsRef.current = ws

    ws.onopen = () => setStatus('open')
    ws.onerror = () => setStatus('error')
    ws.onclose = () => {
      setStatus('closed')
      if (enabled) {
        timerRef.current = window.setTimeout(() => connect(), reconnectMs)
      }
    }
    ws.onmessage = (ev) => {
      try {
        const data = JSON.parse(ev.data as string) as T
        setLastMessage(data)
        onMessageRef.current?.(data)
      } catch {
        /* ignore non-json */
      }
    }
  }, [enabled, path, reconnectMs])

  useEffect(() => {
    if (!enabled) {
      clearTimer()
      wsRef.current?.close()
      setStatus('closed')
      return
    }
    connect()
    return () => {
      clearTimer()
      wsRef.current?.close()
    }
  }, [connect, enabled])

  const send = useCallback((data: unknown) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(typeof data === 'string' ? data : JSON.stringify(data))
    }
  }, [])

  return { status, lastMessage, send, reconnect: connect }
}

export default useWebSocket
