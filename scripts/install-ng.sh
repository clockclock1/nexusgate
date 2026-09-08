#!/usr/bin/env bash
# Bootstrap: install NexusGate manager as short command `ng`
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
# Or from a local clone:
#   sudo bash scripts/install-ng.sh

set -euo pipefail

REPO_RAW="https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/ng"
DEST="/usr/local/bin/ng"

if [[ ${EUID} -ne 0 ]]; then
  echo "[install-ng] 请使用 root：sudo bash scripts/install-ng.sh" >&2
  exit 1
fi

tmp="$(mktemp)"
cleanup() { rm -f "$tmp"; }
trap cleanup EXIT

if [[ -f "$(dirname "$0")/ng" ]]; then
  echo "[install-ng] 使用本地 scripts/ng"
  cp -f "$(dirname "$0")/ng" "$tmp"
else
  echo "[install-ng] 从 GitHub 下载 scripts/ng ..."
  curl -fsSL "$REPO_RAW" -o "$tmp"
fi

grep -q 'NexusGate Linux manager' "$tmp" || {
  echo "[install-ng] 脚本内容校验失败" >&2
  exit 1
}

install -m 755 "$tmp" "$DEST"
echo "[install-ng] 已安装: ${DEST}"
echo "[install-ng] 现在可以随时执行: sudo ng"
echo "[install-ng] 或交互菜单: sudo ng menu"
"$DEST" help | head -n 20 || true
