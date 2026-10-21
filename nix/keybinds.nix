# Keybinds for the Dynamic Island Shell.
#
# The rule from the plan: the compositor never learns the UI's protocol. It
# only ever runs `isled call`, and the daemon turns that into an event the
# island reacts to. One definition serves both compositors, and the same
# commands work from a terminal.
#
# `Mod+1..9` is workspace switching in both niri and hyprland, so the scheme
# uses `Mod+Ctrl+1..9`, which is unbound by default.
#
# Usage from the system flake:
#
#   inputs.isle.url = "path:/home/reze/Projects/dynamic-island";
#   # ...
#   # in the niri module:
#   wayland.windowManager.niri.settings.binds =
#     lib.mkMerge [ inputs.isle.lib.niriBinds ( ...your own binds... ) ];
#
#   # in the hyprland module:
#   wayland.windowManager.hyprland.settings.bind =
#     lib.mkMerge [ inputs.isle.lib.hyprlandBinds ( ...your own binds... ) ];

{ lib, isled ? "isled" }:

let
  # The panels, in the order the island walks them.
  panels = [
    "context"
    "media"
    "notifications"
    "clipboard"
    "calendar"
    "todos"
    "notes"
    "network"
    "theme"
  ];

  # The command that opens a panel. The index is a bare number rather than
  # JSON, so a compositor config never has to quote anything.
  viewCommand = index: "${isled} call view.set ${toString index}";

  # One entry per panel, with its index.
  entries = lib.genList (i: {
    index = i;
    name = lib.elemAt panels i;
  }) (lib.length panels);
in
rec {
  # The commands themselves, so a config can be written by hand from the same
  # source of truth.
  viewCommands = lib.listToAttrs (
    lib.map (entry: {
      name = entry.name;
      value = viewCommand entry.index;
    }) entries
  );

  # ---- niri ----

  niriBinds = lib.listToAttrs (
    lib.concatLists [
      (lib.map (entry: {
        name = "Mod+Ctrl+${toString (entry.index + 1)}";
        value = {
          spawn = [
            isled
            "call"
            "view.set"
            (toString entry.index)
          ];
        };
      }) entries)
      [
        {
          name = "Mod+Ctrl+J";
          value = {
            spawn = [
              isled
              "call"
              "island.toggle"
            ];
          };
        }
        {
          name = "Mod+Ctrl+K";
          value = {
            spawn = [
              isled
              "call"
              "island.focus"
            ];
          };
        }
      ]
    ]
  );

  # ---- hyprland ----

  hyprlandBinds = lib.listToAttrs (
    lib.concatLists [
      (lib.map (entry: {
        name = "SUPER CTRL, ${toString (entry.index + 1)}";
        value = viewCommand entry.index;
      }) entries)
      [
        {
          name = "SUPER CTRL, J";
          value = "${isled} call island.toggle";
        }
        {
          name = "SUPER CTRL, K";
          value = "${isled} call island.focus";
        }
      ]
    ]
  );
}
