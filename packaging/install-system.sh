#!/usr/bin/env bash
# Dynamic Island — system install (sudo required).
#
# Installs like the user-local tarball but to system paths:
#   /usr/local/bin/isled
#   /usr/local/share/quickshell/isle
#   /usr/lib/systemd/user/isled.service + quickshell.service
#   /etc/isle/ (settings seed, currently FEDORA.md + sway binds)
#
# Usage:
#   sudo ./packaging/install-system.sh [--destdir DIR] [--dry-run]
#
# --destdir prefixes every path (for packaging/testing). --dry-run prints
# what would be installed without writing.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
DEST="${DEST:-}"
DRY=0

if [[ "${1:-}" == "--destdir" ]]; then
  DEST="$2"; shift 2
fi
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY=1; shift
fi

# The tarball layout is also accepted as source: ./isle-fedora/{bin,ui,systemd}
SRC_BIN=""
SRC_UI=""
SRC_SYSTEMD=""
for cand in "$REPO" "$HERE" "$(pwd)/isle-fedora"; do
  if [[ -x "$cand/bin/isled" ]]; then SRC_BIN="$cand/bin/isled"; SRC_UI="$cand/ui"; SRC_SYSTEMD="$cand/systemd"; break; fi
  if [[ -x "$cand/isle-fedora/bin/isled" ]]; then SRC_BIN="$cand/isle-fedora/bin/isled"; SRC_UI="$cand/isle-fedora/ui"; SRC_SYSTEMD="$cand/isle-fedora/systemd"; break; fi
done
if [[ -z "$SRC_BIN" ]]; then
  # Fall back to the dev build so the script is testable from the repo.
  SRC_BIN="$REPO/core/target/debug/isled"
  SRC_UI="$REPO/ui"
  SRC_SYSTEMD="$REPO/systemd"
fi
[[ -x "$SRC_BIN" ]] || { echo "isled binary not found (build with: nix develop && just build)" >&2; exit 1; }

do_install() {
  local src="$1" dst="$2" mode="$3"
  if [[ "$DRY" == "1" ]]; then
    echo "would install -m $mode $src -> $DEST$dst"
  else
    install -d "$(dirname "$DEST$dst")"
    if [[ "$mode" == "dir" ]]; then
      cp -R "$src/." "$DEST$dst/"
    else
      install -m "$mode" "$src" "$DEST$dst"
    fi
  fi
}

echo "Dynamic Island — system install"
echo "  source: $SRC_BIN"
echo "  destdir: ${DEST:-/}  dry-run: $DRY"
do_install "$SRC_BIN" "/usr/local/bin/isled" "0755"
do_install "$SRC_UI" "/usr/local/share/quickshell/isle" "dir"
for unit in isled quickshell; do
  tpl="$SRC_SYSTEMD/${unit}.service.template"
  if [[ -f "$tpl" ]]; then
    do_install "$tpl" "/usr/lib/systemd/user/${unit}.service.template" "0644"
  else
    echo "note: $tpl missing (unit will be rendered by install.sh / install-units)" >&2
  fi
done
do_install "$REPO/FEDORA.md" "/usr/local/share/doc/dynamic-island/FEDORA.md" "0644"
do_install "$REPO/tools/sway-binds.conf" "/etc/isle/sway-binds.conf" "0644"

echo "done. Enable with: systemctl --user enable --now isled quickshell"
echo "Verify overlay: swaymsg -t get_tree | grep -A2 quickshell"
