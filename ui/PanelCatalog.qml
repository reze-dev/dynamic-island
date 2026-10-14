pragma Singleton

import QtQuick

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
        "workspaces, weather, cpu, memory",
        "mpris players and now playing",
        "history and per-app settings",
        "pinned entries and search",
        "agenda and upcoming events",
        "add, complete, reorder",
        "autosaving scratchpad",
        "wifi, bluetooth, volume, brightness",
        "wallpapers and palette"
    ]

    function isValid(index) {
        return index >= 0 && index < names.length;
    }

    function clamp(index) {
        if (isValid(index))
            return index;
        return 0;
    }
}