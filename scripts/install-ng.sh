#!/usr/bin/env bash
# Bootstrap: install NexusGate manager as short command `ng`
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
# With mirror:
#   curl -fsSL https://ghproxy.net/https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
#   NG_MIRROR=https://ghproxy.net/ curl -fsSL ... | sudo bash
# Or from a local clone:
#   sudo bash scripts/install-ng.sh

set -euo pipefail

REPO="clockclock1/nexusgate"
ORIGIN_RAW="https://raw.githubusercontent.com/${REPO}/main/scripts/ng"
ORIGIN_INSTALL="https://raw.githubusercontent.com/${REPO}/main/scripts/install-ng.sh"
DEST="/usr/local/bin/ng"

DEFAULT_MIRRORS=(
  "https://ghproxy.net/"
  "https://mirror.ghproxy.com/"
  "https://ghfast.top/"
  "https://gh.ddlc.top/"
)

if [[ ${EUID} -ne 0 ]]; then
  echo "[install-ng] 请使用 root：sudo bash scripts/install-ng.sh" >&2
  exit 1
fi

normalize_mirror() {
  local m="${1:-}"
  [[ -z "$m" || "$m" == "official" || "$m" == "direct" ]] && { echo ""; return; }
  [[ "$m" != http://* && "$m" != https://* ]] && m="https://${m}"
  echo "${m%/}/"
}

build_candidates() {
  local origin="$1"
  local preferred
  preferred="$(normalize_mirror "${NG_MIRROR:-}")"
  if [[ -n "$preferred" ]]; then
    echo "${preferred}${origin}"
    echo "$origin"
  else
    echo "$origin"
  fi
  local m
  for m in "${DEFAULT_MIRRORS[@]}"; do
    m="$(normalize_mirror "$m")"
    [[ -n "$preferred" && "$m" == "$preferred" ]] && continue
    echo "${m}${origin}"
  done
}

download_failover() {
  local origin="$1"
  local dest="$2"
  local cand
  echo "[install-ng] 下载: ${origin}"
  while IFS= read -r cand; do
    [[ -z "$cand" ]] && continue
    if [[ "$cand" != "$origin" ]]; then
      echo "[install-ng] 切换镜像重试: ${cand}"
    fi
    if curl -fsSL --connect-timeout 15 --max-time 120 --retry 0 -o "$dest" "$cand"; then
      if [[ -s "$dest" ]]; then
        echo "[install-ng] 下载成功"
        return 0
      fi
    fi
    rm -f "$dest"
  done < <(build_candidates "$origin")
  echo "[install-ng] 所有源下载失败。可设置: NG_MIRROR=https://ghproxy.net/" >&2
  return 1
}

tmp="$(mktemp)"
cleanup() { rm -f "$tmp"; }
trap cleanup EXIT

if [[ -f "$(dirname "$0")/ng" ]]; then
  echo "[install-ng] 使用本地 scripts/ng"
  cp -f "$(dirname "$0")/ng" "$tmp"
else
  download_failover "$ORIGIN_RAW" "$tmp"
fi

grep -q 'NexusGate Linux manager' "$tmp" || {
  echo "[install-ng] 脚本内容校验失败" >&2
  exit 1
}

install -m 755 "$tmp" "$DEST"

# persist preferred mirror for later ng downloads
if [[ -n "${NG_MIRROR:-}" ]]; then
  mkdir -p /opt/nexusgate/.state
  normalize_mirror "$NG_MIRROR" >/opt/nexusgate/.state/mirror
  echo "[install-ng] 已写入默认镜像: $(cat /opt/nexusgate/.state/mirror)"
fi

echo "[install-ng] 已安装: ${DEST}"
echo
echo "[install-ng] 使用方式:"
echo "  ng                 # 数字菜单"
echo "  sudo ng mirror     # 设置 GitHub 镜像源"
echo "  sudo ng help"
echo
"$DEST" help | head -n 30 || true
