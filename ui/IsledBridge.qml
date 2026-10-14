import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
    id: bridge

    readonly property string socketPath: Quickshell.env("ISLED_SOCKET") !== ""
        ? Quickshell.env("ISLED_SOCKET")
        : Quickshell.env("XDG_RUNTIME_DIR") + "/isled/isled.sock"

    property bool connected: socket.connected
    property var snapshot: ({})
    property int tickCount: 0
    property string lastTopic: ""
    property string lastError: ""

    property string pending: ""
    property int consumed: 0

    property int nextId: 1

    signal viewRequested(int index)
    signal toggleRequested()

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
            console.log("[isle] snapshot: compositor=" + (message.data?.session?.compositor ?? "?")
                + " wayland=" + (message.data?.session?.wayland_display ?? "?")
                + " version=" + message.data?.version);
        } else if (message.t === "event") {
            lastTopic = message.topic;
            if (message.topic === "session.tick")
                tickCount++;
            else if (message.topic === "view.set")
                viewRequested(message.data.index | 0);
            else if (message.topic === "island.toggle")
                toggleRequested();
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