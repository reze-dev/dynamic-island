# The dev loop for the Dynamic Island Shell.
#
# Run every recipe inside the devshell: `nix develop`, then `just <recipe>`.
# `just` is in nativeBuildInputs, so it is on PATH without any prefix.
#
# The one rule that matters: never assume target/debug/isled matches the
# source. `cargo clippy` and `cargo test` both leave the binary at
# core/target/debug/isled untouched, so a check-then-deploy sequence serves
# whatever was last actually built. Every recipe that restarts the daemon
# therefore routes through `build` first.

repo := justfile_directory()
bin := repo / "core/target/debug/isled"
ui := repo / "ui"

# Service names, taken from the units in ./systemd so that this file and the
# eventual Home Manager module name them the same thing. The module defines
# systemd.user.services.isled and .quickshell; these unit files are the same
# names pointed at the dev binary. When the module is enabled it overwrites
# these files, so `just restart` keeps working across the switch.
daemon := "isled"
shell := "quickshell"

# Where the user units live.
units := env("XDG_CONFIG_HOME", env("HOME") + "/.config") / "systemd/user"

export ISLED_SOCKET := env_var_or_default("ISLED_SOCKET", env("XDG_RUNTIME_DIR") + "/isled/isled.sock")

default:
    @just --list --unsorted

# ---------------------------------------------------------------- rust check

# fmt + clippy + test, the way CI would. Does NOT touch the binary.
check:
    cd {{repo}}/core && cargo fmt --all --check
    cd {{repo}}/core && cargo clippy --workspace --all-targets -- -D warnings
    cd {{repo}}/core && cargo test --workspace

# Faster and louder than `check`: fixes formatting in place and prints failures.
fix:
    cd {{repo}}/core && cargo fmt --all
    cd {{repo}}/core && cargo clippy --workspace --all-targets -- -D warnings
    cd {{repo}}/core && cargo test --workspace

test:
    cd {{repo}}/core && cargo test --workspace

# The only recipe that produces the binary the daemon runs. Everything that
# restarts a service depends on this, which is what makes a stale daemon
# impossible rather than merely unlikely.
build:
    cd {{repo}}/core && cargo build --workspace

# Fails if the binary on disk predates the source tree. `just build` is the
# fix; not usually worth calling on its own.
binary-is-fresh:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -x "{{bin}}" ]; then
        echo "no binary at {{bin}} - run: just build" >&2
        exit 1
    fi
    newest=$(find "{{repo}}/core" -name '*.rs' -newer "{{bin}}" | head -1)
    if [ -n "$newest" ]; then
        echo "STALE: $newest is newer than the binary - run: just build" >&2
        exit 1
    fi

# Builds the package from the working tree. The `path:` prefix matters: a
# bare path resolves to the *git* tree, and the git tree does not have the
# files that have not been committed yet. `nix build .#isled` is the
# reproducible build, but it only sees committed sources.
release:
    nix build "path:{{repo}}#isled" --print-build-logs

# The reproducible build: whatever is committed, nothing else.
release-committed:
    nix build {{repo}}#isled --print-build-logs

# ------------------------------------------------------------- qml / nix fmt

fmt-nix:
    nix run nixpkgs#alejandra -- {{repo}}/flake.nix {{repo}}/nix/module.nix {{repo}}/nix/isled-package.nix

fmt-qml:
    #!/usr/bin/env bash
    set -euo pipefail
    for dir in {{ui}} {{ui}}/panels {{ui}}/components; do
        qmlformat -i "$dir" >/dev/null
    done

# Static analysis of the QML. The import paths matter: without them qmllint
# cannot see QtQuick or Quickshell and every type comes back as missing.
lint-qml:
    #!/usr/bin/env bash
    set -uo pipefail
    qml_root="$(dirname "$(dirname "$(readlink -f "$(command -v qmllint)")")")/lib/qt-6/qml"
    qs_root="$(dirname "$(dirname "$(readlink -f "$(command -v quickshell)")")")/lib/qt-6/qml"
    if [ ! -d "$qml_root" ] || [ ! -d "$qs_root" ]; then
        echo "could not find the QML import roots" >&2
        exit 1
    fi
    status=0
    for f in $(find {{ui}} -name '*.qml' | sort); do
        if ! out=$(qmllint -I "$qml_root" -I "$qs_root" "$f" 2>&1); then
            echo "$out"
            status=1
        fi
    done
    exit $status

# Evaluates the Home Manager module against the live home-manager master.
# Catches option drift without booting a machine.
eval-module: build
    nix-instantiate --eval --strict -A execstart /tmp/isle-eval2/eval-on.nix >/dev/null
    nix-instantiate --eval --strict -A execstart /tmp/isle-eval2/eval.nix >/dev/null

# Everything, in the order that catches problems earliest.
check-all: check lint-qml binary-is-fresh

# ---------------------------------------------------------------- services

# Copies the unit files into place and starts them. Real units rather than
# transient ones, so they survive this terminal closing and get restarted on
# crash; systemd also unlinks the socket reliably on SIGTERM, which is what
# keeps the next start from failing with "address already in use".
install-units:
    mkdir -p {{units}}
    cp {{repo}}/systemd/{{daemon}}.service {{units}}/
    cp {{repo}}/systemd/{{shell}}.service {{units}}/
    systemctl --user daemon-reload
    @echo "installed {{units}}/{{daemon}}.service"
    @echo "installed {{units}}/{{shell}}.service"

# `--now` also enables them at graphical-session.target, so they come back
# after a reboot. Without it they only run until next login.
# Retires the transient units from earlier experiments. A transient unit
# cannot be enabled and blocks the file-backed one of the same name, so this
# has to run once before the switchover sticks.
retire-transient:
    #!/usr/bin/env bash
    set -euo pipefail
    for unit in {{daemon}} quickshell-isle; do
        if systemctl --user show "$unit" -p FragmentPath --value 2>/dev/null \
            | grep -q transient; then
            systemctl --user stop "$unit" 2>/dev/null || true
            echo "retired the transient $unit.service"
        fi
    done
    systemctl --user reset-failed 2>/dev/null || true

start: install-units retire-transient
    systemctl --user enable --now {{daemon}} {{shell}}
    @echo "started {{daemon}} and {{shell}}"

# Stops and disables without removing the unit files.
down:
    systemctl --user disable --now {{daemon}} {{shell}} 2>/dev/null || true

stop:
    systemctl --user stop {{daemon}} {{shell}} 2>/dev/null || true
    systemctl --user reset-failed {{daemon}} {{shell}} 2>/dev/null || true

# Removes the unit files entirely, returning to no daemon at all.
uninstall-units:
    rm -f {{units}}/{{daemon}}.service {{units}}/{{shell}}.service
    systemctl --user daemon-reload
    @echo "removed the unit files"

# Runs isled against the live session bus long enough to prove it will not
# steal the name, then leaves. The dev daemon stays in its safe default; this
# is the explicit check that the claim path is still correct.
claim-check:
    #!/usr/bin/env bash
    set -uo pipefail
    before=$(gdbus call --session --dest org.freedesktop.DBus \
        --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.GetNameOwner \
        org.freedesktop.Notifications 2>&1)
    "{{bin}}" --claim-notifications >/tmp/isled-claim-check.log 2>&1 &
    pid=$!
    trap "kill $pid 2>/dev/null" EXIT
    sleep 2
    after=$(gdbus call --session --dest org.freedesktop.DBus \
        --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.GetNameOwner \
        org.freedesktop.Notifications 2>&1)
    if [ "$before" = "$after" ]; then
        echo "PASS  the notification bus name stayed with $after"
    else
        echo "FAIL  the name moved: $before -> $after"
        exit 1
    fi
    grep -o 'name already taken on the bus' /tmp/isled-claim-check.log | sed 's/^/      /' | head -1

# Build first, then restart, so the running daemon is always the built one.
deploy: build
    systemctl --user restart {{daemon}}
    @echo "restarted {{daemon}}"

# Build and bounce the UI, for QML-only changes.
deploy-ui: build
    systemctl --user restart {{shell}}
    @echo "restarted {{shell}}"

# Reload the unit files if they changed, without restarting.
reload:
    systemctl --user daemon-reload

status:
    @systemctl --user is-active {{daemon}} {{shell}} 2>/dev/null || true
    @echo ""
    @echo "socket: ${ISLED_SOCKET}"
    @[ -S "${ISLED_SOCKET}" ] && echo "socket: present" || echo "socket: absent"

logs *args:
    journalctl --user -u {{daemon}} -u {{shell}} -n 80 --no-pager {{args}}

# Errors only, for both units. Trailing "clean" means nothing is wrong.
errors:
    #!/usr/bin/env bash
    journalctl --user -u {{daemon}} -u {{shell}} --no-pager 2>/dev/null \
        | grep -aiE 'error|cannot|referenceerror|typeerror' \
        | grep -av 'socket error' \
        | grep -av 'QLocalSocket::ServerNotFound' \
        | tail -20 || true
    echo "--- (nothing above means clean) ---"

# ---------------------------------------------------------------- driving it

# `just call view.set '{"index":2}'` - the CLI is the keybind path too.
call cmd args="":
    @{{bin}} call {{cmd}} '{{args}}'

# Injects a notification without claiming the bus name, so the panel is
# testable while another notification daemon is still running. This is the
# escape hatch for "the panel is empty and I cannot tell why".
notify summary="isle" body="from just" urgency="2":
    @{{bin}} call notifications.send \
        '{"app_name":"just","summary":"{{summary}}","body":"{{body}}","hints":{"urgency":{"Byte":{{urgency}}}}}'

dismiss id:
    @{{bin}} call notifications.dismiss '{"id":{{id}}}'

clear-all:
    @{{bin}} call notifications.clear '{}'

view index:
    @{{bin}} call view.set '{"index":{{index}}}'

# Pushes an activity into the island's queue, which is how the triggered
# state is exercised without waiting for a real event. `--sticky` keeps it
# as the ambient condition, so the idle dot takes its colour.
activity kind="media" title="Glasshouse" subtitle="Aether" sticky="true":
    @{{bin}} call activity.trigger \
        '{"kind":"{{kind}}","title":"{{title}}","subtitle":"{{subtitle}}","sticky":{{sticky}},"glyph":"music"}'

# The todo list. `just todo "standup 15m"` makes a todo due in a quarter of
# an hour, which is the inline deadline syntax the panel also understands.
todo text:
    @{{bin}} call todos.add '{"text":"{{text}}"}'

todos:
    @{{bin}} call todos.list

timer minutes="25":
    @{{bin}} call timer.start '{"minutes":{{minutes}}}'

timer-stop:
    @{{bin}} call timer.stop '{}'

notes:
    @{{bin}} call notes.list

wallpapers:
    @{{bin}} call wallpaper.list

wallpaper path:
    @{{bin}} call wallpaper.set '{"path":"{{path}}"}'

# The whole context in one call: what the island knows, what is on the list,
# what the clock is doing, and what is on the screen.
context:
    @{{bin}} call context.summary

# Snapshot as pretty JSON: `just snapshot | jq '.data.notifications'`
snapshot:
    #!/usr/bin/env bash
    timeout 5 socat -u "UNIX-CONNECT:${ISLED_SOCKET}" - 2>/dev/null | head -1 | jq '.data'

# Watch unsolicited events, which is how you prove a feature works without
# the UI: one line per event, with the topic highlighted.
events:
    #!/usr/bin/env bash
    timeout 8 socat -u "UNIX-CONNECT:${ISLED_SOCKET}" - 2>/dev/null \
        | jq -r '.t + " " + (.topic // "-")' | uniq -c

# ------------------------------------------------------------- one-shot ui

# Loads the config and exits, so it is safe in scripts. Prints nothing on
# success, which is the signal.
ui-check:
    #!/usr/bin/env bash
    set -euo pipefail
    # The shell on its own, and then the preview rig, which puts the island
    # into every one of its appearances. A component that only exists in the
    # peek view is never created by the first of these, and a missing import
    # in it is a runtime error rather than a load error.
    out=$(timeout 20 quickshell -p {{ui}} 2>&1 || true)
    preview=$(cd {{ui}} && QT_QPA_PLATFORM=offscreen timeout 40 quickshell -p preview.qml 2>&1 || true)
    # The service warnings are environmental: a machine with no bluetooth
    # daemon, no UPower or no notification bus is not a broken shell.
    # The trailing `|| true` matters: with `set -e` and `pipefail`, a grep
    # that matches nothing would otherwise fail the whole recipe.
    bad=$(echo "$out$preview" \
        | grep -iE 'error|cannot assign|unavailable|binding loop|ReferenceError|TypeError' \
        | grep -avE 'bluez|UPower|portal|QLocalSocket|ServerNotFound|qt.qpa.services|No PanelWindow backend' \
        || true)
    if [ -n "$bad" ]; then
        echo "$bad" | head -8
        exit 1
    fi
    echo "qml loads clean"

# ------------------------------------------------------------------ the demo

# A scripted demo: the daemon on a scratch socket, the real shell, and a
# sequence of events the island reacts to. Watch it on the screen.
demo:
    #!/usr/bin/env bash
    set -euo pipefail
    if systemctl --user is-active --quiet quickshell 2>/dev/null; then
        echo "note: the installed shell is already running, so the demo's own" >&2
        echo "      island will sit next to it. Stop it with: just stop" >&2
        echo >&2
    fi
    {{repo}}/tools/demo.sh

# Triggers a single event, so you can poke at the island yourself. The
# daemon has to be running: `just start`, or `just demo` in another shell.
#
#   just trigger media | notification | todo | clipboard | timer | panel
trigger what="notification":
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -S "${ISLED_SOCKET}" ]; then
        echo "no daemon at ${ISLED_SOCKET}" >&2
        echo "start one with: just start   (or: just demo)" >&2
        exit 1
    fi
    case "{{what}}" in
    media)
        {{bin}} call activity.trigger \
            '{"kind":"media","title":"Glasshouse","subtitle":"Aether","glyph":"music","sticky":true,"source":"spotify","tone":"info"}'
        ;;
    notification)
        {{bin}} call notifications.send \
            '{"app_name":"northstar","summary":"Build completed","body":"dev shell evaluated successfully","hints":{"urgency":{"Byte":1}}}'
        ;;
    todo)
        {{bin}} call todos.add '{"text":"ship the island","due_in_minutes":0}'
        ;;
    clipboard)
        {{bin}} call clipboard.add '{"text":"nix develop .#devShells.x86_64-linux.default"}'
        ;;
    timer)
        {{bin}} call timer.start '{"minutes":25}'
        ;;
    panel)
        {{bin}} call view.set 0
        ;;
    *)
        echo "unknown trigger: {{what}}" >&2
        echo "one of: media notification todo clipboard timer panel" >&2
        exit 1
        ;;
    esac

# The preview with real data in it: a daemon on a scratch socket, seeded with
# wallpapers, a calendar, notes, todos and notifications, and then every
# panel photographed with something in it. An empty panel is a forgiving
# thing — it hides every layout fault there is.
preview-live:
    #!/usr/bin/env bash
    set -euo pipefail
    {{repo}}/tools/preview-live.sh

# The morph, as a GIF. Renders the island headless, photographs it while the
# animations run, and assembles the frames. The one thing a screenshot
# cannot show.
demo-anim dir="/tmp/isle-anim":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{dir}}/frames"
    rm -f "{{dir}}/frames"/*.png
    cd {{ui}}
    ISLED_DEMO_FRAMES="{{dir}}/frames" QT_QPA_PLATFORM=offscreen \
        timeout 90 quickshell -p demo.qml >/dev/null 2>&1 || true
    frames=$(ls -1 "{{dir}}/frames"/*.png 2>/dev/null | wc -l)
    echo "captured $frames frames"
    if [ "$frames" -lt 2 ]; then
        echo "not enough frames to make a gif" >&2
        exit 1
    fi
    magick -delay 4 -loop 0 "{{dir}}/frames"/*.png "{{dir}}/island.gif"
    echo "--- {{dir}}/island.gif"

# ------------------------------------------------------------- the preview

# Renders the island in each of its appearances and writes the pictures out.
# The design is the part of this project that no test can check, so being
# able to *look* at it without a compositor is worth the small rig.
preview dir="/tmp/isle-preview":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{dir}}"
    rm -f "{{dir}}"/*.png
    cd {{ui}}
    QT_QPA_PLATFORM=offscreen timeout 120 quickshell -p preview.qml >/dev/null 2>&1 || true
    ls -1 "{{dir}}"/*.png 2>/dev/null | sed 's|.*/|  |'
    echo "--- written to {{dir}}"

# -------------------------------------------------------------- the tray

# A fake tray item, so the tray can be exercised without an application
# that has one. Needs a daemon running with --claim-tray.
#
#   just tray
#   just tray "Steam" "steam"
tray title="Fake Item" icon="mpv":
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{repo}}/core
    cargo build --offline --example fake-tray 2>/dev/null
    ./target/debug/examples/fake-tray --title "{{title}}" --icon "{{icon}}"

# ------------------------------------------------------------ scratch dirs

clean-tmp:
    rm -rf /tmp/isled-*
    @echo "removed /tmp/isled-*"
