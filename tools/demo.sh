#!/usr/bin/env bash
#
# A scripted demo of the island.
#
# Starts the daemon on a scratch socket with a scratch state directory, starts
# the real UI, and then fires a sequence of events so the island can be seen
# reacting to them. Nothing here touches the real session: the daemon gets its
# own socket and its own state directory, and the notification bus name is
# left alone.
#
#   just demo
#
# Ctrl-C stops it and cleans up.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="$repo/core/target/debug/isled"
ui="$repo/ui"

# A different directory from the animation rig's, so the two do not wipe
# each other's output.
scratch="${ISLED_DEMO_DIR:-/tmp/isle-demo}"
socket="$scratch/isled.sock"

daemon_pid=""
ui_pid=""

cleanup() {
    [ -n "$ui_pid" ] && kill "$ui_pid" 2>/dev/null || true
    [ -n "$daemon_pid" ] && kill "$daemon_pid" 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

say() {
    printf '\n\033[1;35m▸ %s\033[0m\n' "$1"
}

beat() {
    printf '   \033[2m%s\033[0m\n' "$1"
    sleep "${2:-2}"
}

if [ ! -x "$bin" ]; then
    echo "isled is not built. Run: just build" >&2
    exit 1
fi

rm -rf "$scratch"
mkdir -p "$scratch/state"

export ISLED_SOCKET="$socket"
export XDG_STATE_HOME="$scratch/state"

say "starting the daemon on $socket"
"$bin" --no-claim-notifications >"$scratch/daemon.log" 2>&1 &
daemon_pid=$!
sleep 1

if [ ! -S "$socket" ]; then
    echo "the daemon did not come up. Log:" >&2
    cat "$scratch/daemon.log" >&2
    exit 1
fi

say "starting the shell"
(cd "$ui" && quickshell -p . >"$scratch/ui.log" 2>&1) &
ui_pid=$!
sleep 3

call() {
    "$bin" call "$1" "${2:-}" >/dev/null 2>&1 || true
}

# ---------------------------------------------------------------- the demo

say "the island at rest"
beat "a dot and a clock, and nothing else" 3

say "music starts playing"
call activity.trigger '{"kind":"media","title":"Glasshouse","subtitle":"Aether · Late Night Systems","glyph":"music","sticky":true,"source":"spotify","tone":"info"}'
beat "the island grows, and the dot takes the tone of the event" 3

say "a notification arrives"
call notifications.send '{"app_name":"northstar","summary":"Build completed","body":"dev shell evaluated successfully","hints":{"urgency":{"Byte":1}}}'
beat "the island grows again, this time in amber" 3

say "a todo comes due"
call todos.add '{"text":"ship the island","due_in_minutes":0}'
beat "the island grows once more, and the todo is now on the list" 3

say "the clipboard changes"
call clipboard.add '{"text":"nix develop .#devShells.x86_64-linux.default"}'
beat "a quiet event, but the island still reacts" 3

say "a pomodoro starts"
call timer.start '{"minutes":25}'
beat "the island shows the timer running" 3

say "opening the context panel"
call view.set 0
beat "the full panel: what is happening on the machine" 4

say "stepping through the panels"
for index in 1 2 3 4 5 6 7 8; do
    call view.set "$index"
    beat "$(printf '%s' "$index" | sed 's/^1$/media/;s/^2$/notifications/;s/^3$/clipboard/;s/^4$/calendar/;s/^5$/todos/;s/^6$/notes/;s/^7$/network/;s/^8$/theme/')" 1.4
done

say "back to the start"
call timer.stop '{}'
call view.set 0
beat "the panel folds away" 3

say "that is the island"
beat "Ctrl-C to stop the demo" 999
