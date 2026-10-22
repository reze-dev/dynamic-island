import QtQuick
import QtQuick.Shapes
import ".."

// An icon drawn from the path data in Icons.qml.
//
// Material Symbols are filled shapes rather than strokes, so the path is
// filled: the artwork already carries its own weight, and filling it keeps
// the outline and the filled variants interchangeable.
//
// The path is authored on a 24x24 grid and scaled to the item.
Item {
    id: root

    property string name: "spark"
    property color color: Theme.onSurface
    /// Whether to use the filled variant of the icon, when it has one.
    property bool filled: false

    implicitWidth: 24
    implicitHeight: 24

    readonly property real factor: width > 0 ? width / 24 : 1

    Shape {
        id: shape
        width: 24
        height: 24
        scale: root.factor
        transformOrigin: Item.TopLeft

        ShapePath {
            fillColor: root.color
            // The stroke is turned off rather than made thin: a zero width
            // still draws a hairline, and the icons are filled shapes.
            strokeColor: "transparent"
            strokeWidth: 0
            // SVG fills with the nonzero rule, and the Material artwork is
            // drawn for it. Qt defaults to the even-odd rule, which punches
            // holes in the icons that have a shape inside a shape.
            fillRule: ShapePath.WindingFill
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin

            PathSvg {
                path: root.filled ? Icons.fillPath(root.name) : Icons.path(root.name)
            }
        }
    }
}
