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
Item {
    id: panel

    property var bridge: null

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

    // ---- bluetooth ----

    readonly property var adapters: Bluetooth.adapters
    readonly property var bluetoothDevices: Bluetooth.devices

    // ---- battery ----

    readonly property var battery: UPower.displayDevice
    readonly property bool hasBattery: battery && battery.isPresent === true

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "Connections"
        note: Networking.wifiEnabled ? "WIFI ON" : "WIFI OFF"
    }

    // ---- the wifi toggle ----

    Card {
        id: wifiCard
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 44

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight + 2

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "wifi"
                width: 16
                height: 16
                color: Networking.wifiEnabled ? Theme.toneInfo : Theme.textFaint
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 80
                spacing: 1

                Text {
                    width: parent.width
                    text: "Wi-Fi"
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                }

                Text {
                    width: parent.width
                    text: panel.wifiDevices.length > 0
                        ? panel.wifiDevices.length + " adapter" + (panel.wifiDevices.length > 1 ? "s" : "")
                        : "no adapter"
                    color: Theme.textDim
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                }
            }

            Toggle {
                anchors.verticalCenter: parent.verticalCenter
                checked: Networking.wifiEnabled
                onToggled: function (value) {
                    // NetworkManager exposes the radio through its own
                    // settings; Quickshell does not surface the write, so
                    // this is the one thing the panel cannot do on its own.
                    if (panel.bridge)
                        panel.bridge.call("network.wifi", { "enabled": value });
                }
            }
        }
    }

    // ---- the networks ----

    SectionHeader {
        id: networksHeader
        anchors.top: wifiCard.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
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
            radius: Theme.radiusSmall
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
                        ? Theme.toneInfo
                        : Theme.textFaint
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 90)
                    text: network.modelData.name
                    color: network.modelData.connected === true ? Theme.text : Theme.textDim
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontSmall
                    font.weight: network.modelData.connected === true ? Font.DemiBold : Font.Normal
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
                                ? Theme.toneInfo
                                : Qt.rgba(1, 1, 1, 0.10)
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
        visible: networks.count === 0
        text: "No wifi networks."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontSmall
    }

    // ---- ethernet ----

    SectionHeader {
        id: wiredHeader
        anchors.top: networksHeader.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.left
        anchors.rightMargin: parent.width * 0.5
        anchors.leftMargin: Theme.spacing
        title: "Ethernet"
    }

    ListView {
        id: wired
        anchors.top: wiredHeader.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.left
        anchors.rightMargin: parent.width * 0.5
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 4
        model: panel.wiredDevices

        delegate: Card {
            id: wire
            required property var modelData

            width: wired.width
            height: 34
            radius: Theme.radiusSmall

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
                    color: wire.modelData.connected ? Theme.toneGood : Theme.textFaint
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 80)
                    text: wire.modelData.name
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontSmall
                    elide: Text.ElideRight
                }

                Chip {
                    anchors.verticalCenter: parent.verticalCenter
                    text: wire.modelData.connected ? "LINK" : "DOWN"
                    tone: wire.modelData.connected ? Theme.toneGood : Theme.textDim
                }
            }
        }
    }

    // ---- bluetooth ----

    SectionHeader {
        id: bluetoothHeader
        anchors.top: networksHeader.bottom
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
            radius: Theme.radiusSmall
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
                    color: device.modelData.connected === true ? Theme.toneInfo : Theme.textFaint
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 80)
                    text: device.modelData.name ? device.modelData.name : device.modelData.address
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontSmall
                    elide: Text.ElideRight
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: device.modelData.batteryAvailable === true
                    text: Math.round(device.modelData.battery) + "%"
                    color: Theme.textFaint
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontTiny
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
        visible: bluetoothList.count === 0
        text: "No bluetooth devices."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontSmall
    }

    // ---- battery ----

    Card {
        id: batteryCard
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.left
        anchors.rightMargin: parent.width * 0.5
        anchors.leftMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        height: 44
        visible: panel.hasBattery

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight + 2

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "battery"
                width: 16
                height: 16
                color: panel.battery.percentage < 20 ? Theme.toneCritical : Theme.toneGood
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 80
                spacing: 1

                Text {
                    width: parent.width
                    text: "Battery"
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                }

                Text {
                    width: parent.width
                    text: panel.battery.state === 1
                        ? "charging"
                        : (panel.battery.timeToEmpty > 0
                            ? Math.round(panel.battery.timeToEmpty / 60) + " min left"
                            : "on battery")
                    color: Theme.textDim
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                }
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: Math.round(panel.battery.percentage) + "%"
                color: Theme.text
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
        }
    }
}
