# Client

内网 Edge 客户端：连接 Server、维持 Control Session、响应 CONNECT、建立 1:1 Data Connection。

```bash
# 在仓库根目录执行
cargo run -p p2p-edge -- --config client/config/edge.toml
```

配置文件：`client/config/edge.toml`（需填入管理面板生成的 `node_id` 与 `token`）
