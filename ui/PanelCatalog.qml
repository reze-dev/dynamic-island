pragma Singleton

import QtQuick

// The panel registry.
//
// Nine views, in the order the keybinds walk them. Each one carries its own
// icon, its own height and its own subtitle, because each panel is supposed
// to feel like its own thing rather than a page in a book.
QtObject {
    readonly property var names: [
        "context",
        "media",
        "notifications",
        "clipboard",
        "calendar",
        "todos",
        "notes",
        "network",
        "theme"
    ]

    readonly property var subtitles: [
        "what is happening on this machine",
        "now playing, and what else is queued",
        "history, and what is still unread",
        "recent clips, pinned and searchable",
        "the next few hours, hour by hour",
        "time-stamped tasks and a pomodoro",
        "a scratchpad that saves itself",
        "wifi, ethernet, bluetooth, battery",
        "wallpapers and the palette they bring"
    ]

    readonly property var glyphs: [
        "spark",
        "music",
        "bell",
        "clipboard",
        "calendar",
        "check-square",
        "note",
        "wifi",
        "image"
    ]

    /// How tall each panel wants to be. The island morphs to fit, so a
    /// calendar can be taller than a media player.
    readonly property var heights: [
        340,
        330,
        430,
        390,
        470,
        450,
        470,
        420,
        460
    ]

    /// The keybind label for each view.
    readonly property var shortcuts: [
        "Mod+Ctrl+1",
        "Mod+Ctrl+2",
        "Mod+Ctrl+3",
        "Mod+Ctrl+4",
        "Mod+Ctrl+5",
        "Mod+Ctrl+6",
        "Mod+Ctrl+7",
        "Mod+Ctrl+8",
        "Mod+Ctrl+9"
    ]

    function isValid(index) {
        return index >= 0 && index < names.length;
    }

    function clamp(index) {
        return isValid(index) ? index : 0;
    }

    function name(index) {
        return isValid(index) ? names[index] : names[0];
    }

    function glyph(index) {
        return isValid(index) ? glyphs[index] : glyphs[0];
    }

    function heightFor(index) {
        return isValid(index) ? heights[index] : heights[0];
    }

    function subtitle(index) {
        return isValid(index) ? subtitles[index] : subtitles[0];
    }

    function shortcut(index) {
        return isValid(index) ? shortcuts[index] : shortcuts[0];
    }
}
