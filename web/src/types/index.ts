export interface ManagedServer {
  id: string
  name: string
  api_upstream: string
}

export type NodeStatus = 'online' | 'offline' | 'disabled'

export interface NodeInfo {
  node_id: string
  name: string
  status: NodeStatus
  enabled?: boolean
  token?: string
  public_ip?: string
  private_ip?: string
  nat_type?: string
  version: string
  cpu?: number
  memory?: number
  uptime_secs?: number
  last_seen?: string
  connections?: number
  rx_bytes?: number
  tx_bytes?: number
  region?: string
  tags?: string[]
  created_at?: string
}

export type ServiceProtocol = 'tcp' | 'udp' | 'http' | 'https'

export interface ServiceInfo {
  service_id: string
  name: string
  node_id: string
  node_name?: string
  protocol: ServiceProtocol
  local_addr: string
  local_port: number
  enabled: boolean
  description?: string
  created_at?: string
  updated_at?: string
}

export interface RouteInfo {
  route_id: string
  name: string
  public_port: number
  protocol: ServiceProtocol
  node_id: string
  node_name?: string
  service_id: string
  service_name?: string
  enabled: boolean
  description?: string
  created_at?: string
}

export type ConnMode = 'p2p' | 'relay' | 'unknown'
export type ConnStatus = 'active' | 'closed' | 'connecting' | 'failed'

export interface ConnectionInfo {
  conn_id: string
  node_id: string
  node_name?: string
  peer_node_id?: string
  peer_name?: string
  service_id?: string
  service_name?: string
  mode: ConnMode
  status: ConnStatus
  protocol?: string
  local_addr?: string
  remote_addr?: string
  rx_bytes?: number
  tx_bytes?: number
  duration_secs?: number
  started_at?: string
}

export interface DashboardStats {
  nodes_total: number
  nodes_online: number
  connections_active: number
  traffic_rx_bytes: number
  traffic_tx_bytes: number
  p2p_rate: number
  relay_rate: number
  services_total?: number
  routes_total?: number
}

export interface TrafficPoint {
  timestamp: string
  rx_bytes: number
  tx_bytes: number
  p2p_bytes?: number
  relay_bytes?: number
}

export interface TrafficSeries {
  interval: 'hourly' | 'daily'
  points: TrafficPoint[]
}

export type UserRole = 'admin' | 'operator' | 'viewer'

export interface UserInfo {
  user_id: string
  username: string
  role: UserRole
  enabled: boolean
  email?: string
  last_login?: string
  created_at?: string
}

export interface ServerInfo {
  status: 'online' | 'offline' | 'degraded'
  version: string
  uptime_secs: number
  cpu: number
  memory: number
  memory_total?: number
  listen_addr?: string
  ports?: ServerPort[]
  node_count?: number
  connection_count?: number
  started_at?: string
  hostname?: string
}

export interface ServerPort {
  name: string
  port: number
  protocol: string
  status: 'listening' | 'closed' | 'error'
}

export interface ServerConfig {
  listen_addr: string
  api_port: number
  control_port: number
  data_port: number
  gateway_port: number
  max_connections: number
  jwt_ttl_secs: number
  enable_relay: boolean
  enable_p2p: boolean
  config_path?: string
  restart_required_for_ports?: boolean
}

export interface AppSettings {
  server: {
    listen_addr: string
    api_port: number
    control_port: number
    data_port: number
    gateway_port: number
    config_path?: string
  }
  security: {
    jwt_ttl_secs: number
    jwt_secret_set?: boolean
    /** write-only */
    jwt_secret?: string
  }
  p2p: {
    enable_p2p: boolean
  }
  relay: {
    enable_relay: boolean
    max_connections: number
  }
  notes?: {
    ports_need_restart?: boolean
    admin_seed?: string
  }
}

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'

export interface LogEntry {
  id?: string
  timestamp: string
  level: LogLevel
  target?: string
  node_id?: string
  message: string
}

export interface TopologyNode {
  id: string
  name: string
  status: NodeStatus
  nat_type?: string
}

export interface TopologyEdge {
  source: string
  target: string
  mode: ConnMode
  weight?: number
}

export interface NetworkTopology {
  nodes: TopologyNode[]
  edges: TopologyEdge[]
}

export interface P2pStats {
  p2p_connections: number
  relay_connections: number
  p2p_success_rate: number
  avg_latency_ms?: number
  hole_punch_success?: number
  hole_punch_fail?: number
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  user: UserInfo
  expires_at?: string
}

export interface ApiError {
  message: string
  code?: string
}

export interface PageResult<T> {
  items: T[]
  total: number
}
