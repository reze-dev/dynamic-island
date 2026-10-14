pragma Singleton

import QtQuick

QtObject {
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

    function uptime(secs) {
        if (!secs || secs <= 0)
            return "0m";
        var h = Math.floor(secs / 3600);
        var m = Math.floor((secs % 3600) / 60);
        if (h > 0)
            return h + "h " + m + "m";
        return m + "m";
    }

    function compactTime(date) {
        function pad(n) {
            return n < 10 ? "0" + n : "" + n;
        }
        return pad(date.getHours()) + ":" + pad(date.getMinutes());
    }
}