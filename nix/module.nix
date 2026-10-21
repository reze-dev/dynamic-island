{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.isle;

  quickshellConfigName = "isle";

  awwwExec = lib.concatStringsSep " " (
    [
      (lib.getExe' cfg.awwwPackage "awww-daemon")
      "--namespace"
      "isle"
      "--layer"
      "background"
    ]
    ++ lib.optionals (cfg.awwwFormat != null) [
      "--format"
      cfg.awwwFormat
    ]
  );
in {
  options.programs.isle = {
    enable = lib.mkEnableOption "the Dynamic Island Shell";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./isled-package.nix {};
      defaultText = lib.literalExpression "pkgs.callPackage ./isled-package.nix { }";
      description = "The isled session daemon.";
    };

    quickshellPackage = lib.mkOption {
      type = lib.types.package;
      default = pkgs.quickshell;
      defaultText = lib.literalExpression "pkgs.quickshell";
      description = "Quickshell build used for the UI.";
    };

    uiPath = lib.mkOption {
      type = lib.types.path;
      default = ../ui;
      defaultText = lib.literalExpression "../ui";
      description = "Quickshell config directory.";
    };

    awwwPackage = lib.mkOption {
      type = lib.types.package;
      default = pkgs.awww;
      defaultText = lib.literalExpression "pkgs.awww";
      description = "awww, the wallpaper engine. Provides awww-daemon.";
    };

    enableAwww = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Run an awww daemon namespaced to isle.";
    };

    awwwFormat = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "bgr";
      description = ''
        Value for awww-daemon --format. Null omits the flag and lets awww
        negotiate, which is the safe default: the 3-channel formats (rgb, bgr)
        are not offered by every compositor, and forcing one makes the daemon
        exit at startup with a wl_shm protocol error. Only set this after
        confirming `awww query -n isle` succeeds with it.
      '';
    };

    claimNotifications = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Let isled own org.freedesktop.Notifications on the session bus.

        Off by default, and do not turn it on while another notification
        daemon is running. isled asks for the name with DoNotQueue only,
        deliberately omitting zbus's default ReplaceExisting, so it refuses to
        take the name rather than silently stealing it from whatever owns it.
        That is the transition safety property: with another daemon alive,
        isled logs "could not claim ... name already taken on the bus" and
        keeps running, and the existing daemon keeps every notification.

        Once this shell is replacing that daemon, the name is free and isled
        takes it. Check before flipping: if isled reports features.notifications
        = false, it did not get the name.
      '';
    };

    sessionTargets = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [
        "niri-session.target"
      ];
      defaultText = lib.literalExpression "[ \"niri-session.target\" ]";
      description = ''
        Session targets the island starts with.

        One target per compositor, and the list is not a fallback list: a
        unit whose WantedBy names a target that does not exist is simply
        never started. Listing a compositor that is not installed therefore
        stops the island from starting at all, rather than doing nothing
        useful. Add the ones that are actually installed.
      '';
    };

    forwardedEnvironment = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [
        "WAYLAND_DISPLAY"
        "XDG_RUNTIME_DIR"
        "XDG_SESSION_TYPE"
        "NIRI_SOCKET"
        "HYPRLAND_INSTANCE_SIGNATURE"
        "DBUS_SESSION_BUS_ADDRESS"
      ];
      description = ''
        Variables imported into isled and awww-daemon. They must already be in
        the systemd user manager for this to have any effect. Verify with
        `systemctl --user show isled -p Environment` after enabling: if the
        compositor adapter reports Unknown instead of niri/hyprland, this is
        the first thing to check.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."quickshell/${quickshellConfigName}".source = cfg.uiPath;

    systemd.user.targets.isle = {
      Unit = {
        Description = "Dynamic Island Shell session";
      };
      Install = {
        WantedBy = cfg.sessionTargets;
      };
    };

    systemd.user.services.isled = {
      Unit = {
        Description = "Dynamic Island Shell session daemon";
        After = ["graphical-session-pre.target"];
        Wants = ["graphical-session-pre.target"];
      };
      Service = {
        Type = "simple";
        ExecStart = lib.concatStringsSep " " (
          [(lib.getExe cfg.package)]
          ++ lib.optionals cfg.claimNotifications ["--claim-notifications"]
        );
        Restart = "on-failure";
        RestartSec = 2;
        RuntimeDirectory = "isled";
        Environment = "NIXOS_OZONE_WL=1";
        ImportEnvironment = cfg.forwardedEnvironment;
      };
      Install = {
        WantedBy = ["isle.target"];
      };
    };

    systemd.user.services.quickshell = {
      Unit = {
        Description = "Dynamic Island Shell UI";
        Requires = ["isled.service"];
        After = ["isled.service"];
      };
      Service = {
        Type = "simple";
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe' cfg.quickshellPackage "quickshell")
          "--config"
          quickshellConfigName
        ];
        Restart = "on-failure";
        RestartSec = 2;
        Environment = [
          "QT_QPA_PLATFORM=wayland"
          "NIXOS_OZONE_WL=1"
        ];
        ImportEnvironment = cfg.forwardedEnvironment;
      };
      Install = {
        WantedBy = ["isle.target"];
      };
    };

    systemd.user.services.awww-daemon = lib.mkIf cfg.enableAwww {
      Unit = {
        Description = "awww wallpaper daemon (isle namespace)";
        After = ["graphical-session-pre.target"];
      };
      Service = {
        Type = "simple";
        ExecStart = awwwExec;
        Restart = "on-failure";
        RestartSec = 2;
        ImportEnvironment = cfg.forwardedEnvironment;
      };
      Install = {
        WantedBy = ["isle.target"];
      };
    };
  };
}
