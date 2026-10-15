import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
    id: bridge

    readonly property string explicitSocket: Quickshell.env("ISLED_SOCKET")
    readonly property string runtimeDir: Quickshell.env("XDG_RUNTIME_DIR")

    readonly property string socketPath: (explicitSocket !== undefined && explicitSocket.length > 0)
        ? explicitSocket
        : runtimeDir + "/isled/isled.sock"

    property bool connected: socket.connected
    property var snapshot: ({})

    property var stats: ({})

    property var notifications: []
    property int unreadCount: 0
    property int tickCount: 0
    property string lastTopic: ""
    property string lastError: ""

    property string pending: ""
    property int consumed: 0

    property int nextId: 1

    signal viewRequested(int index)
    signal viewStepped(int delta)
    signal toggleRequested()
    signal focusRequested()

    function onNotificationAdded(notification) {
        if (notification === undefined || notification === null)
            return;
        let next = [];
        for (const existing of notifications) {
            if (existing.id !== notification.id)
                next.push(existing);
        }
        next.push(notification);
        notifications = next;
        unreadCount += 1;
    }

    function onNotificationRemoved(id) {
        if (id === undefined)
            return;
        notifications = notifications.filter((item) => item.id !== id);
    }

    function clearUnread() {
        unreadCount = 0;
    }

    function dismissNotification(id) {
        call("notifications.dismiss", {
            "id": id
        });
    }

    function clearNotifications() {
        call("notifications.clear", {});
    }

    function invokeAction(id, key) {
        call("notifications.action", {
            "id": id,
            "key": key
        });
    }

    function call(name, args) {
        var message = {
            "t": "cmd",
            "id": nextId,
            "name": name
        };
        if (args !== undefined)
            message.args = args;
        nextId++;
        socket.write(JSON.stringify(message) + "\n");
        socket.flush();
    }

    function feed(all) {
        if (all.length <= consumed)
            return;

        pending += all.slice(consumed);
        consumed = all.length;

        var lines = pending.split("\n");
        pending = lines.pop();
        for (var i = 0; i < lines.length; i++)
            consume(lines[i]);
    }

    function consume(line) {
        line = line.trim();
        if (line.length === 0)
            return;

        var message;
        try {
            message = JSON.parse(line);
        } catch (err) {
            lastError = "unparseable: " + line;
            return;
        }

        if (message.t === "state") {
            snapshot = message.data;
            if (message.data.notifications !== undefined)
                notifications = message.data.notifications;
        } else if (message.t === "event") {
            lastTopic = message.topic;
            if (message.topic === "system.stats") {
                stats = message.data;
                tickCount++;
            }
            else if (message.topic === "notifications.added")
                onNotificationAdded(message.data);
            else if (message.topic === "notifications.removed")
                onNotificationRemoved(message.data.id);
            else if (message.topic === "notifications.cleared")
                notifications = [];
            else if (message.topic === "view.set")
                viewRequested(message.data.index | 0);
            else if (message.topic === "view.step")
                viewStepped(message.data.delta | 0);
            else if (message.topic === "island.toggle")
                toggleRequested();
            else if (message.topic === "island.focus")
                focusRequested();
        }
    }

    property Socket socket: Socket {
        id: sock

        path: bridge.socketPath
        connected: false

        onConnectionStateChanged: {
            if (!sock.connected) {
                bridge.tickCount = 0;
                bridge.pending = "";
                bridge.consumed = 0;
            }
        }

        onError: (err) => {
            bridge.lastError = "socket error " + err;
        }

        parser: StdioCollector {
            id: collector

            waitForEnd: false
            onDataChanged: bridge.feed(collector.text)
        }
    }

    Component.onCompleted: {
        socket.connected = true
    }
}