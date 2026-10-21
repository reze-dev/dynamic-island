pragma Singleton

import QtQuick

// Settings for the shell itself.
//
// These are read at startup and are not saved anywhere: they exist so the
// behaviour that is still being judged by eye can be flipped without a
// rebuild.
QtObject {
    /// 0 — the surface is full width and only the island inside it changes
    ///     shape. This is what Quickshell makes easy, and it never
    ///     reconfigures the layer surface.
    /// 1 — the surface itself is the size of the island, and it resizes as
    ///     the island grows.
    property int morphApproach: 0

    readonly property bool keyboardFocus: true

    /// Whether an expanding island tries to take the keyboard. Niri only
    /// grants it on a click, so this is a request rather than a guarantee.
    property bool focusOnExpand: false

    /// Whether the island reacts to events by opening itself.
    property bool autoTrigger: true

    /// How long a triggered island stays open before it folds away, in
    /// milliseconds. The daemon can override this per activity.
    property int defaultDwell: 5000

    readonly property string approachName: morphApproach === 0
        ? "B: fixed surface, animated island"
        : "A: the surface resizes with the island"
}
