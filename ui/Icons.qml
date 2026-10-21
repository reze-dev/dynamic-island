pragma Singleton

import QtQuick

// The icon set.
//
// Each icon is one SVG path string on a 24x24 grid, which is the same
// geometry the reference mock uses. Keeping them as path data rather than
// images means they stay sharp at any size and can be recoloured by the
// theme.
//
// Circles and rectangles from the originals have been rewritten as path
// segments so that every icon is a single drawable string.
QtObject {
    readonly property var paths: ({
        "activity": "M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2",
        "alert": "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z M12 9v4M12 17h.01",
        "arrow-up-right": "M7 17 17 7M7 7h10v10",
        "battery": "M2 7h16v10H2z M22 11v2",
        "bell": "M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9 M10 21h4",
        "bluetooth": "m7 7 10 10-5 5V2l5 5L7 17",
        "calendar": "M3 5h18v16H3z M16 3v4M8 3v4M3 11h18",
        "check": "m5 12 4 4L19 6",
        "check-square": "m9 11 3 3L22 4 M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11",
        "chevron-down": "m6 9 6 6 6-6",
        "chevron-right": "m9 18 6-6-6-6",
        "clipboard": "M8 4h12v16H8z M16 4V3a1 1 0 0 0-1-1H5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h3",
        "clock": "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0 M12 6v6l4 2",
        "close": "m18 6-12 12M6 6l12 12",
        "copy": "M8 8h12v12H8z M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3",
        "cpu": "M4 4h16v16H4z M9 9h6v6H9z M15 2v2M15 20v2M2 15h2M2 9h2M20 15h2M20 9h2M9 2v2M9 20v2",
        "disc": "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0 M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0 M5 5l2 2M17 17l2 2",
        "download": "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4 M7 10l5 5 5-5M12 15V3",
        "ethernet": "M9 2v6M15 2v6M6 8h12v4a6 6 0 0 1-12 0V8Z M12 18v4",
        "external": "M15 3h6v6 M10 14 21 3 M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6",
        "eye": "M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7z M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
        "folder": "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",
        "grid": "M3 3h7v7H3z M14 3h7v7h-7z M14 14h7v7h-7z M3 14h7v7H3z",
        "image": "M3 3h18v18H3z M7 9a2 2 0 1 0 4 0a2 2 0 1 0 -4 0 M21 15l-3.09-3.09a2 2 0 0 0-2.82 0L6 21",
        "info": "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0 M12 16v-4M12 8h.01",
        "list": "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01",
        "lock": "M3 11h18v11H3z M7 11V7a5 5 0 0 1 10 0v4",
        "moon": "M20.9 13A9 9 0 0 1 11 3.1 9 9 0 1 0 20.9 13Z",
        "music": "M9 18V5l12-2v13 M3 18a3 3 0 1 0 6 0a3 3 0 1 0 -6 0 M15 16a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
        "next": "m5 4 10 8-10 8z M19 5v14",
        "note": "M12 20h9 M16.5 3.5a2.12 2.12 0 0 1 3 3L8 18l-4 1 1-4Z",
        "pause": "M6 4h4v16H6z M14 4h4v16h-4z",
        "pin": "M12 17v5 M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z",
        "play": "m7 4 14 8-14 8z",
        "plus": "M12 5v14M5 12h14",
        "power": "M12 2v10 M18.4 6.6a9 9 0 1 1-12.77.04",
        "presentation": "M2 3h20M4 3v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V3M12 16v5M8 21h8",
        "prev": "M19 20 9 12l10-8z M5 19V5",
        "refresh": "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8 M21 3v5h-5 M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16 M8 16H3v5",
        "repeat": "m17 2 4 4-4 4 M3 11v-1a4 4 0 0 1 4-4h14 m-7 22-4-4 4-4 M21 13v1a4 4 0 0 1-4 4H3",
        "search": "M3 11a8 8 0 1 0 16 0a8 8 0 1 0 -16 0 M21 21l-4.3-4.3",
        "spark": "m12 3 1.9 5.8L20 11l-6.1 2.2L12 19l-1.9-5.8L4 11l6.1-2.2L12 3Z m19 14 1.2 2.8L23 18l-2.8 1.2L19 22l-1.2-2.8L15 18l2.8-1.2L19 14Z",
        "sun": "M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0 M12 2v2M12 20v2M4.93 4.93l1.42 1.42M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.42-1.41M17.66 6.34l1.41-1.41",
        "timer": "M4 13a8 8 0 1 0 16 0a8 8 0 1 0 -16 0 M12 9v4l2 1M9 2h6",
        "trash": "M3 6h18 M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6 M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
        "volume": "M11 5 6 9H2v6h4l5 4V5z M15.54 8.46a5 5 0 0 1 0 7.07",
        "wifi": "M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01",
        "zap": "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"
    })

    /// The path for an icon, or a sensible fallback.
    function path(name) {
        var value = paths[name];
        if (value === undefined)
            return paths["spark"];
        return value;
    }
}
