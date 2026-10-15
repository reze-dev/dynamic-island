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

release:
    nix build {{repo}}#isled --print-build-logs

# ------------------------------------------------------------- qml / nix fmt

fmt-nix:
    nix run nixpkgs#alejandra -- {{repo}}/flake.nix {{repo}}/nix/module.nix {{repo}}/nix/isled-package.nix

fmt-qml:
    #!/usr/bin/env bash
    set -euo pipefail
    for dir in {{ui}} {{ui}}/panels; do
        qmlformat -i "$dir" >/dev/null
    done

# Evaluates the Home Manager module against the live home-manager master.
# Catches option drift without booting a machine.
eval-module: build
    nix-instantiate --eval --strict -A execstart /tmp/isle-eval2/eval-on.nix >/dev/null
    nix-instantiate --eval --strict -A execstart /tmp/isle-eval2/eval.nix >/dev/null

# Everything, in the order that catches problems earliest.
check-all: check fmt-nix binary-is-fresh

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
    out=$(timeout 20 quickshell -p {{ui}} 2>&1 || true)
    if echo "$out" | grep -qiE 'error|cannot assign|unavailable'; then
        echo "$out" | grep -iE 'error|cannot assign|unavailable' | head -5
        exit 1
    fi
    echo "qml loads clean"

# ------------------------------------------------------------ scratch dirs

clean-tmp:
    rm -rf /tmp/isled-*
    @echo "removed /tmp/isled-*"
