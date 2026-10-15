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

# Service names. These are transient units started by `just start`; the Home
# Manager module uses `isled` and `quickshell` instead.
daemon := "isled"
shell := "quickshell-isle"

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

# Starts both units as transient systemd user services. They outlive this
# recipe and survive a reboot only if you re-run `just start`.
start:
    #!/usr/bin/env bash
    set -euo pipefail
    systemctl --user stop {{daemon}} {{shell}} 2>/dev/null || true
    systemctl --user reset-failed {{daemon}} {{shell}} 2>/dev/null || true
    systemd-run --user --unit={{daemon}} --collect \
        --property=Restart=on-failure "{{bin}}" >/dev/null
    systemd-run --user --unit={{shell}} --collect \
        --property=Restart=on-failure \
        --setenv=WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}" \
        --setenv=XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" \
        "${QUICKSHELL:-/run/current-system/sw/bin/quickshell}" -p {{ui}} >/dev/null
    echo "started {{daemon}} and {{shell}}"

stop:
    systemctl --user stop {{daemon}} {{shell}} 2>/dev/null || true
    systemctl --user reset-failed {{daemon}} {{shell}} 2>/dev/null || true

# Build first, then restart, so the running daemon is always the built one.
deploy: build
    systemctl --user restart {{daemon}}
    @echo "restarted {{daemon}}"

# Build and bounce the UI, for QML-only changes.
deploy-ui: build
    systemctl --user restart {{shell}}
    @echo "restarted {{shell}}"

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
