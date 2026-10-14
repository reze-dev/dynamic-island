{
  description = "Dynamic Island Shell — development shell";

  inputs = {
    # Tracks unstable by design. This flake's nixpkgs only ever feeds the dev
    # shell: per PLAN §9.2 the repo is consumed by the target system flake as a
    # `path:` input, so the isled package that lands in the system closure is
    # built with the *target's* pkgs, not these. That is where the rule in
    # /home/reze/Michi/flake.nix applies — a second nixpkgs rev there means two
    # glibcs and freezes the compositor. Nothing here ends up in that closure.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    devShells = forEachSystem (
      pkgs: {
        default = pkgs.mkShell {
          name = "isle";

          nativeBuildInputs = with pkgs; [
            # Rust, from nixpkgs rather than rustup, so `cargo build` in here
            # sees the same compiler rustPlatform.buildRustPackage will use
            # for the isled package (PLAN §9.3).
            cargo
            clippy
            rust-analyzer
            rustc
            rustfmt

            # Native libraries isled links against (PLAN §3.4).
            bluez # bluer -> Bluetooth
            libxkbcommon
            openssl
            pipewire # pw-volume, screencast node presence
            pkg-config
            sqlite # rusqlite, plus the CLI for poking the catalog

            # QML side. NOTE: unstable carries quickshell 0.3.2 while the running session
            # is on 0.3.0, so `quickshell -c isle` here is a different build than
            # the one your compositor actually starts. That gap closes itself
            # when the system moves to unstable; until then it is the one place
            # where tracking unstable can cost an afternoon (PLAN §9.6).
            quickshell
            qt6.qtbase
            qt6.qtdeclarative # qmllint / qmlformat
            qt6.qtwayland

            # Wallpaper engine (PLAN §4.4).
            awww

            # Driving $XDG_RUNTIME_DIR/isled.sock by hand while developing
            # the IPC protocol (PLAN §3.2).
            socat
            jq
            curl

            # Rest of the loop.
            git
            just
            wl-clipboard # clipboard source (PLAN §3.4)
            wlr-randr # per-output geometry, to cross-check `awww query`
          ];

          shellHook = ''
            if [ -z "''${WAYLAND_DISPLAY:-}" ]; then
              echo "isle: WAYLAND_DISPLAY is unset - quickshell cannot open a layer-shell surface." >&2
            fi
          '';
        };
      }
    );

    packages = forEachSystem (
      pkgs: let
        cargoToml = pkgs.lib.importTOML ./core/Cargo.toml;
      in {
        isled = pkgs.rustPlatform.buildRustPackage {
          pname = "isled";
          version = cargoToml.workspace.package.version;

          src = pkgs.lib.fileset.toSource {
            root = ./core;
            fileset = pkgs.lib.fileset.unions [
              ./core/Cargo.toml
              ./core/Cargo.lock
              ./core/isled
              ./core/isle-core
            ];
          };

          cargoLock.lockFile = ./core/Cargo.lock;

          meta = {
            description = "Session daemon for the Dynamic Island Shell";
            license = pkgs.lib.licenses.gpl3Only;
            platforms = pkgs.lib.platforms.linux;
            mainProgram = "isled";
          };
        };

        default = self.packages.${pkgs.stdenv.hostPlatform.system}.isled;
      }
    );

    formatter = forEachSystem (pkgs: pkgs.alejandra);
  };
}
