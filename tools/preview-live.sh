#!/usr/bin/env bash
#
# A seeded preview.
#
# Starts the daemon on a scratch socket with a directory of wallpapers, a
# calendar and some notes in it, fills the todo list and the notification
# history, and then photographs every panel with real data in it.
#
# An empty panel is a forgiving thing. Every layout fault there is hides
# behind a panel with nothing in it.
#
#   just preview-live

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="$repo/core/target/debug/isled"
ui="$repo/ui"

seed="${ISLED_SEED_DIR:-/tmp/isle-seed}"
out="${ISLED_PREVIEW_DIR:-/tmp/isle-live}"
socket="$seed/isled.sock"

daemon_pid=""
cleanup() {
    [ -n "$daemon_pid" ] && kill "$daemon_pid" 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

if [ ! -x "$bin" ]; then
    echo "isled is not built. Run: just build" >&2
    exit 1
fi

# ---- the seed data ----

rm -rf "$seed"
mkdir -p "$seed/walls" "$seed/cal" "$seed/notes" "$seed/state"

for index in 1 2 3 4 5 6 7 8; do
    if [ $((index % 2)) -eq 0 ]; then a="#2b1a3f"; b="#6b5b95"; else a="#1f3a5f"; b="#8fd3f4"; fi
    magick -size 400x250 "gradient:${a}-${b}" -pointsize 60 -fill white \
        -gravity center -annotate 0 "$index" "$seed/walls/wall-$index.png"
done

cat >"$seed/cal/work.ics" <<'ICS'
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//isle//test//EN
BEGIN:VEVENT
UID:e1
DTSTART:20261021T090000Z
DTEND:20261021T093000Z
SUMMARY:Platform sync
LOCATION:Room 3
END:VEVENT
BEGIN:VEVENT
UID:e2
DTSTART:20261021T113000Z
DTEND:20261021T121500Z
SUMMARY:Review the island geometry
LOCATION:Infra team
END:VEVENT
BEGIN:VEVENT
UID:e3
DTSTART:20261021T140000Z
DTEND:20261021T150000Z
SUMMARY:Read Rust ownership and borrowing
DESCRIPTION:Chapter 4
END:VEVENT
END:VCALENDAR
ICS

echo "# Scratch notes" >"$seed/notes/ideas.md"
echo "# Meeting prep" >"$seed/notes/prep.md"

export ISLED_SOCKET="$socket"
export XDG_STATE_HOME="$seed/state"

"$bin" --no-claim-notifications >"$seed/daemon.log" 2>&1 &
daemon_pid=$!
sleep 1

# ---- the data ----

call() { "$bin" call "$1" "${2:-}" >/dev/null 2>&1 || true; }

# The dedicated commands, so the live stores are re-pointed rather than
# just the settings being remembered.
call wallpaper.setdir '{"dir":"'"$seed"'/walls"}'
call notes.dir '{"dir":"'"$seed"'/notes"}'
call calendar.dir '{"dir":"'"$seed"'/cal"}'
call wallpaper.rescan

call todos.add '{"text":"fix the island geometry","due_in_minutes":30}'
call todos.add '{"text":"write the keybind module","due_in_minutes":120,"priority":2}'
call todos.add '{"text":"read the niri IPC docs"}'
call todos.add '{"text":"ship the preview rig","due_in_minutes":-60}'

call notifications.send '{"app_name":"northstar","summary":"Build completed","body":"dev shell evaluated successfully","hints":{"urgency":{"Byte":1}}}'
call notifications.send '{"app_name":"solar","summary":"12 packages are ready to update","body":"when convenient","hints":{"urgency":{"Byte":0}}}'
call notifications.send '{"app_name":"systemd","summary":"Timed out waiting for the device","body":"the unit entered the failed state","hints":{"urgency":{"Byte":2}}}'

call clipboard.add '{"text":"nix develop .#devShells.x86_64-linux.default"}'
call clipboard.add '{"text":"https://quickshell.org/"}'
call clipboard.add '{"text":"/home/reze/Projects/dynamic-island/core"}'
call clipboard.add '{"text":"git log --oneline -10"}'

call notes.new '{"title":"Ideas for the island"}'
call activity.trigger '{"kind":"media","title":"Glasshouse","subtitle":"Aether · Late Night Systems","glyph":"music","sticky":true,"source":"spotify","tone":"info"}'

sleep 1

# ---- the pictures ----

rm -rf "$out"
mkdir -p "$out"
cd "$ui"
ISLED_PREVIEW_DIR="$out" ISLED_SOCKET="$socket" QT_QPA_PLATFORM=offscreen \
    timeout 90 quickshell -p preview.qml >"$seed/preview.log" 2>&1 || true

ls -1 "$out"/*.png 2>/dev/null | sed 's|.*/|  |'
echo "--- written to $out"
