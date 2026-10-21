pragma Singleton

import QtQuick

// Formatting helpers. Everything the shell renders as text goes through here
// so the units and the rounding stay consistent between panels.
QtObject {
    function pad2(n) {
        var value = Math.floor(n);
        return value < 10 ? "0" + value : "" + value;
    }

    /// The wall clock, in the machine's own time zone.
    function clock(date) {
        var d = date || new Date();
        return pad2(d.getHours()) + ":" + pad2(d.getMinutes());
    }

    /// The clock with seconds, for the panels that want them.
    function clockSeconds(date) {
        var d = date || new Date();
        return clock(d) + ":" + pad2(d.getSeconds());
    }

    function bytesToGiB(bytes) {
        if (!bytes || bytes <= 0)
            return "0.0";
        return (bytes / (1024 * 1024 * 1024)).toFixed(1);
    }

    function percent(fraction) {
        if (fraction === undefined || fraction === null || fraction <= 0)
            return "0%";
        return Math.round(fraction * 100) + "%";
    }

    /// A fraction that may be zero, as a percentage.
    function fractionPercent(fraction) {
        if (fraction === undefined || fraction === null)
            return "0%";
        return Math.round(fraction * 100) + "%";
    }

    function uptime(secs) {
        if (!secs || secs <= 0)
            return "0m";
        var h = Math.floor(secs / 3600);
        var m = Math.floor((secs % 3600) / 60);
        if (h > 0)
            return h + "h " + m + "m";
        return m + "m";
    }

    /// "just now", "5m", "3h", "2d" — how long ago something happened.
    function ago(timestamp, now) {
        if (!timestamp)
            return "";
        var reference = now || Math.floor(Date.now() / 1000);
        var delta = Math.max(0, reference - timestamp);
        if (delta < 60)
            return "now";
        if (delta < 3600)
            return Math.floor(delta / 60) + "m";
        if (delta < 86400)
            return Math.floor(delta / 3600) + "h";
        return Math.floor(delta / 86400) + "d";
    }

    /// A duration in milliseconds, as m:ss or h:mm:ss.
    function duration(ms) {
        if (ms === undefined || ms === null)
            return "--:--";
        var total = Math.max(0, Math.floor(ms / 1000));
        var h = Math.floor(total / 3600);
        var m = Math.floor((total % 3600) / 60);
        var s = total % 60;
        if (h > 0)
            return h + ":" + pad2(m) + ":" + pad2(s);
        return m + ":" + pad2(s);
    }

    /// A time of day, from a Unix timestamp.
    function timeOfDay(timestamp) {
        if (!timestamp)
            return "";
        var d = new Date(timestamp * 1000);
        return pad2(d.getHours()) + ":" + pad2(d.getMinutes());
    }

    /// A date, as "Tue 8 Oct".
    function shortDate(timestamp) {
        if (!timestamp)
            return "";
        var d = new Date(timestamp * 1000);
        var days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        var months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        return days[d.getDay()] + " " + d.getDate() + " " + months[d.getMonth()];
    }

    /// How far away a moment is, in words.
    function until(timestamp, now) {
        if (!timestamp)
            return "";
        var reference = now || Math.floor(Date.now() / 1000);
        var delta = timestamp - reference;
        if (delta < 0)
            return ago(timestamp, reference) + " ago";
        if (delta < 60)
            return "in " + delta + "s";
        if (delta < 3600)
            return "in " + Math.floor(delta / 60) + "m";
        if (delta < 86400)
            return "in " + Math.floor(delta / 3600) + "h";
        return "in " + Math.floor(delta / 86400) + "d";
    }

    /// The hour bucket an event falls into, as a label.
    function hourLabel(timestamp) {
        if (!timestamp)
            return "";
        var d = new Date(timestamp * 1000);
        return pad2(d.getHours()) + ":00";
    }

    /// A one-line summary of a path: the file name.
    function fileName(path) {
        if (!path)
            return "";
        var parts = String(path).split("/");
        return parts[parts.length - 1];
    }
}
