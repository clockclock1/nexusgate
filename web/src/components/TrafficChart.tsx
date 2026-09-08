import ReactECharts from 'echarts-for-react'
import dayjs from 'dayjs'
import type { TrafficPoint } from '@/types'
import { useThemeStore } from '@/stores/themeStore'
import { formatBytes } from '@/utils/format'

interface TrafficChartProps {
  data?: TrafficPoint[]
  height?: number
  loading?: boolean
  showModeSplit?: boolean
}

export default function TrafficChart({
  data = [],
  height = 320,
  loading,
  showModeSplit = false,
}: TrafficChartProps) {
  const mode = useThemeStore((s) => s.mode)
  const isDark = mode === 'dark'
  const textColor = isDark ? '#8fa3bc' : '#64748b'
  const splitLine = isDark ? '#1e334d' : '#e2e8f0'

  const times = data.map((d) => dayjs(d.timestamp).format(data.length > 24 ? 'MM-DD' : 'HH:mm'))

  const option = {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      valueFormatter: (v: number) => formatBytes(v),
    },
    legend: {
      textStyle: { color: textColor },
      top: 0,
    },
    grid: { left: 48, right: 20, top: 40, bottom: 28 },
    xAxis: {
      type: 'category',
      data: times,
      axisLabel: { color: textColor },
      axisLine: { lineStyle: { color: splitLine } },
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        color: textColor,
        formatter: (v: number) => formatBytes(v),
      },
      splitLine: { lineStyle: { color: splitLine, type: 'dashed' } },
    },
    series: showModeSplit
      ? [
          {
            name: 'P2P',
            type: 'line',
            smooth: true,
            showSymbol: false,
            areaStyle: { opacity: 0.12 },
            itemStyle: { color: '#14b8a6' },
            data: data.map((d) => d.p2p_bytes ?? 0),
          },
          {
            name: '中继',
            type: 'line',
            smooth: true,
            showSymbol: false,
            areaStyle: { opacity: 0.12 },
            itemStyle: { color: '#f59e0b' },
            data: data.map((d) => d.relay_bytes ?? 0),
          },
        ]
      : [
          {
            name: '下行 RX',
            type: 'line',
            smooth: true,
            showSymbol: false,
            areaStyle: { opacity: 0.15 },
            itemStyle: { color: '#22d3ee' },
            data: data.map((d) => d.rx_bytes),
          },
          {
            name: '上行 TX',
            type: 'line',
            smooth: true,
            showSymbol: false,
            areaStyle: { opacity: 0.12 },
            itemStyle: { color: '#14b8a6' },
            data: data.map((d) => d.tx_bytes),
          },
        ],
  }

  if (!data.length && !loading) {
    return (
      <div
        style={{
          height,
          display: 'grid',
          placeItems: 'center',
          color: textColor,
          border: `1px dashed ${splitLine}`,
          borderRadius: 8,
        }}
      >
        暂无流量数据
      </div>
    )
  }

  return (
    <ReactECharts
      option={option}
      style={{ height, width: '100%' }}
      showLoading={loading}
      notMerge
    />
  )
}
