# Development

## Layout（仓库根目录）

```text
server/     # p2p-server
client/     # p2p-edge
web/        # React admin
shared/     # common|protocol|control|transport|dataplane|p2p|router|security|metrics
```

## Commands

在仓库根目录执行：

```bash
cargo check
cargo test
cargo clippy --workspace --all-targets
cd web && npm run dev
cd web && npm run build
```

## Coding Rules

- No business data on control channel
- No multiplex tunnel for many streams in one connection
- Prefer traits for transport/dataplane extensions
- Never log secrets/tokens in plaintext
