import QtQuick
import Quickshell.Networking
import Quickshell.Bluetooth
import Quickshell.Services.UPower
import ".."
import "../components"

// Network, bluetooth and battery.
//
// All three come from Quickshell's own services, which read
// NetworkManager, BlueZ and UPower directly. Nothing here goes through the
// daemon, because none of it is state the shell owns — it is the machine's
// state, and the machine already has good ways to ask.
//
// When a service is missing the panel says so (rather than rendering a
// blank page): "No wifi adapter" vs "No networks", "No bluetooth adapter"
// vs "No bluetooth devices", and the battery tile simply disappears when
// there is no battery to report on.
Item {
    id: panel

    property var bridge: null

    // ---- service availability ----

    // Networking.devices is empty when NetworkManager is unreachable (or
    // has no adapters). wifiEnabled stays false in that case.
    readonly property bool nmPresent: {
        var devs = Networking.devices;
        return devs && devs.values && devs.values.length > 0;
    }
    readonly property bool btPresent: {
        var ads = Bluetooth.adapters;
        return ads && ads.values && ads.values.length > 0;
    }

    // ---- network ----

    readonly property var devices: Networking.devices
    readonly property var wifiDevices: {
        var list = [];
        if (!devices)
            return list;
        for (var i = 0; i < devices.values.length; i++) {
            var device = devices.values[i];
            if (device.type === 1 /* wifi */)
                list.push(device);
        }
        return list;
    }
    readonly property var wiredDevices: {
        var list = [];
        if (!devices)
            return list;
        for (var i = 0; i < devices.values.length; i++) {
            var device = devices.values[i];
            if (device.type === 0 /* wired */)
                list.push(device);
        }
        return list;
    }
    readonly property var wiredConnected: {
        for (var i = 0; i < panel.wiredDevices.length; i++) {
            if (panel.wiredDevices[i].connected)
                return true;
        }
        return false;
    }

    // ---- bluetooth ----

    readonly property var adapters: Bluetooth.adapters
    readonly property var bluetoothDevices: Bluetooth.devices

    // ---- battery ----

    readonly property var battery: UPower.displayDevice
    readonly property bool hasBattery: battery && battery.isPresent === true

    // ---- brightness ----
    // From the daemon (`brightnessctl`); loaded lazily so a machine
    // without a backlight never blocks the rest of the panel.

    property int brightnessPercent: -1
    property bool brightnessBusy: false

    Component.onCompleted: {
        if (panel.bridge)
            panel.bridge.call("brightness.get", {}, function (ok, payload) {
                panel.brightnessPercent = ok && payload ? (payload.percent || 0) : -1;
            });
    }

    function cycleBrightness() {
        var steps = [100, 75, 50, 25];
        var current = panel.brightnessPercent;
        var next = 100;
        for (var i = 0; i < steps.length; i++) {
            if (steps[i] < current) {
                next = steps[i];
                break;
            }
        }
        panel.brightnessBusy = true;
        panel.bridge.call(
            "brightness.set",
            { "percent": next },
            function (ok, payload, error) {
                panel.brightnessBusy = false;
                if (ok && payload) {
                    panel.brightnessPercent = payload.percent;
                    if (panel.bridge)
                        panel.bridge.announce("Brightness at " + payload.percent + "%");
                } else {
                    var msg = error || "Brightness did not change.";
                    if (panel.bridge)
                        panel.bridge.announce(msg);
                }
            });
    }

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "Connections"
        note: Networking.wifiEnabled ? "WIFI ON" : "WIFI OFF"
    }

    // ---- the tiles ----

    Grid {
        id: tiles
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        columns: 2
        columnSpacing: Theme.spacingTight + 2
        rowSpacing: Theme.spacingTight + 2

        Tile {
            width: (tiles.width - tiles.columnSpacing) / 2
            icon: "wifi"
            label: "Wi-Fi"
            active: Networking.wifiEnabled
            detail: !panel.nmPresent
                ? "NetworkManager not running"
                : (panel.wifiDevices.length > 0
                    ? panel.wifiDevices.length + " adapter" + (panel.wifiDevices.length > 1 ? "s" : "")
                    : "no adapter")

            Toggle {
                id: wifiToggle
                anchors.right: parent.right
                anchors.rightMargin: Theme.spacingTight + 4
                anchors.verticalCenter: parent.verticalCenter
                checked: Networking.wifiEnabled
                onToggled: function (value) {
                    // NetworkManager exposes the radio through its own
                    // settings; Quickshell does not surface the write, so
                    // this is the one thing the panel cannot do on its own.
                    if (panel.bridge)
                        panel.bridge.callAndSay(
                            "network.wifi",
                            { "enabled": value },
                            value ? "Wi-Fi on" : "Wi-Fi off");
                }
            }

            trailing: wifiToggle
        }

        Tile {
            width: (tiles.width - tiles.columnSpacing) / 2
            icon: "ethernet"
            label: "Ethernet"
            active: panel.wiredConnected
            detail: panel.wiredDevices.length > 0
                ? panel.wiredDevices.length + " adapter"
                : "no adapter"
        }

        Tile {
            width: (tiles.width - tiles.columnSpacing) / 2
            icon: "bluetooth"
            label: "Bluetooth"
            active: panel.adapters && panel.adapters.values && panel.adapters.values.length > 0
                ? panel.adapters.values[0].enabled
                : false
            detail: !panel.btPresent
                ? "Bluetooth daemon not running"
                : (panel.adapters.values[0].enabled ? "on" : "off")
        }

        Tile {
            width: (tiles.width - tiles.columnSpacing) / 2
            icon: "battery"
            label: "Battery"
            visible: panel.hasBattery
            active: panel.hasBattery && panel.battery.percentage >= 20
            detail: panel.hasBattery
                ? (panel.battery.state === 1
                    ? "charging"
                    : (panel.battery.timeToEmpty > 0
                        ? Math.round(panel.battery.timeToEmpty / 60) + " min left"
                        : "on battery"))
                : ""
        }

        Tile {
            width: (tiles.width - tiles.columnSpacing) / 2
            icon: "sun"
            label: "Brightness"
            active: panel.brightnessPercent >= 20
            detail: panel.brightnessBusy
                ? "setting…"
                : panel.brightnessPercent >= 0
                    ? panel.brightnessPercent + "% · tap to step down"
                    : "no backlight reported"

            MouseArea {
                anchors.fill: parent
                enabled: panel.brightnessPercent >= 0 && !panel.brightnessBusy && panel.bridge
                cursorShape: Qt.PointingHandCursor
                onClicked: panel.cycleBrightness()
            }
        }
    }

    // ---- the networks ----

    SectionHeader {
        id: networksHeader
        anchors.top: tiles.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.left
        anchors.rightMargin: parent.width * 0.5
        anchors.leftMargin: Theme.spacing
        title: "Networks"
        note: panel.wifiDevices.length > 0 ? "SCAN" : ""
    }

    ListView {
        id: networks
        anchors.top: networksHeader.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.left
        anchors.rightMargin: parent.width * 0.5
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 4
        visible: panel.wifiDevices.length > 0

        model: {
            var list = [];
            if (panel.wifiDevices.length === 0)
                return list;
            var device = panel.wifiDevices[0];
            if (!device.networks || !device.networks.values)
                return list;
            for (var i = 0; i < device.networks.values.length; i++) {
                list.push(device.networks.values[i]);
            }
            return list;
        }

        delegate: Card {
            id: network
            required property var modelData

            width: networks.width
            height: 34
            radius: Theme.radiusMedium
            selected: network.modelData.connected === true

            Row {
                anchors.fill: parent
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.rightMargin: Theme.spacingTight
                spacing: Theme.spacingTight

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "wifi"
                    width: 14
                    height: 14
                    color: network.modelData.connected === true
                        ? Theme.primary
                        : Theme.onSurfaceVariant
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 90)
                    text: network.modelData.name
                    color: network.modelData.connected === true ? Theme.onSurface : Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.bodySmall.size
                    font.weight: Theme.type.bodySmall.weight
                    font.letterSpacing: Theme.type.bodySmall.tracking
                    elide: Text.ElideRight
                }

                // The signal strength, as three bars.
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2

                    Repeater {
                        model: 3

                        Rectangle {
                            required property int index
                            width: 3
                            height: 4 + index * 3
                            radius: 1
                            color: index < Math.round((network.modelData.signalStrength || 0) / 34)
                                ? Theme.primary
                                : Theme.surfaceFillHighest
                        }
                    }
                }

                Chip {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: network.modelData.connected === true
                    text: "ON"
                    tone: Theme.toneGood
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    if (network.modelData.connected === true)
                        network.modelData.disconnect();
                    else
                        network.modelData.requestConnect();
                }
            }
        }
    }

    Text {
        anchors.centerIn: networks
        visible: !panel.nmPresent
        text: "NetworkManager is not running."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    Text {
        anchors.centerIn: networks
        visible: panel.nmPresent && panel.wifiDevices.length === 0
        text: "No wifi networks."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    // ---- ethernet ----

    SectionHeader {
        id: wiredHeader
        anchors.top: tiles.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.52
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing
        visible: panel.wiredDevices.length > 0
        title: "Ethernet"
    }

    ListView {
        id: wired
        anchors.top: wiredHeader.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.52
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing
        height: Math.min(3, count) * 38
        visible: panel.wiredDevices.length > 0
        clip: true
        spacing: 4
        model: panel.wiredDevices

        delegate: Card {
            id: wire
            required property var modelData

            width: wired.width
            height: 34
            radius: Theme.radiusMedium

            Row {
                anchors.fill: parent
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.rightMargin: Theme.spacingTight
                spacing: Theme.spacingTight

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "ethernet"
                    width: 14
                    height: 14
                    color: wire.modelData.connected ? Theme.toneGood : Theme.onSurfaceVariant
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 80)
                    text: wire.modelData.name
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.bodySmall.size
                    font.weight: Theme.type.bodySmall.weight
                    font.letterSpacing: Theme.type.bodySmall.tracking
                    elide: Text.ElideRight
                }

                Chip {
                    anchors.verticalCenter: parent.verticalCenter
                    text: wire.modelData.connected ? "LINK" : "DOWN"
                    tone: wire.modelData.connected ? Theme.toneGood : Theme.onSurfaceVariant
                }
            }
        }
    }

    // ---- bluetooth devices ----

    SectionHeader {
        id: bluetoothHeader
        // With no wired devices the Ethernet header is invisible but still
        // takes up its height, which pushed this one down out of line with
        // the Networks header beside it.
        anchors.top: panel.wiredDevices.length > 0 ? wired.bottom : tiles.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.52
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing
        title: "Bluetooth"
        note: panel.adapters && panel.adapters.values && panel.adapters.values.length > 0
            ? (panel.adapters.values[0].enabled ? "ON" : "OFF")
            : "NO ADAPTER"
    }

    ListView {
        id: bluetoothList
        anchors.top: bluetoothHeader.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.52
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 4

        model: {
            var list = [];
            if (!panel.bluetoothDevices || !panel.bluetoothDevices.values)
                return list;
            for (var i = 0; i < panel.bluetoothDevices.values.length; i++) {
                list.push(panel.bluetoothDevices.values[i]);
            }
            return list;
        }

        delegate: Card {
            id: device
            required property var modelData

            width: bluetoothList.width
            height: 34
            radius: Theme.radiusMedium
            selected: device.modelData.connected === true

            Row {
                anchors.fill: parent
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.rightMargin: Theme.spacingTight
                spacing: Theme.spacingTight

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "bluetooth"
                    width: 14
                    height: 14
                    color: device.modelData.connected === true ? Theme.primary : Theme.onSurfaceVariant
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 80)
                    text: device.modelData.name ? device.modelData.name : device.modelData.address
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.bodySmall.size
                    font.weight: Theme.type.bodySmall.weight
                    font.letterSpacing: Theme.type.bodySmall.tracking
                    elide: Text.ElideRight
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: device.modelData.batteryAvailable === true
                    text: Math.round(device.modelData.battery) + "%"
                    color: Theme.onSurfaceVariant
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    if (device.modelData.connected === true)
                        device.modelData.disconnect();
                    else
                        device.modelData.connect();
                }
            }
        }
    }

    Text {
        anchors.centerIn: bluetoothList
        visible: !panel.btPresent
        text: "Bluetooth daemon is not running."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    Text {
        anchors.centerIn: bluetoothList
        visible: panel.btPresent && bluetoothList.count === 0
        text: "No bluetooth devices."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }
}
