import QtQuick
import QtQuick.Shapes
import ".."

// An icon drawn from the path data in Icons.qml.
//
// The path is authored on a 24x24 grid and scaled to the item, with the
// stroke width corrected so it stays visually constant rather than growing
// with the icon.
Item {
    id: root

    property string name: "spark"
    property color color: Theme.text
    property real strokeWidth: 1.7
    property bool filled: false

    implicitWidth: 16
    implicitHeight: 16

    readonly property real factor: width > 0 ? width / 24 : 1

    Shape {
        id: shape
        width: 24
        height: 24
        scale: root.factor
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.SoftwareRenderer

        ShapePath {
            strokeColor: root.filled ? "transparent" : root.color
            fillColor: root.filled ? root.color : "transparent"
            strokeWidth: root.strokeWidth / Math.max(root.factor, 0.01)
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin

            PathSvg {
                path: Icons.path(root.name)
            }
        }
    }
}
