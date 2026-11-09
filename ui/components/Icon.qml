import QtQuick
import QtQuick.Shapes
import ".."

// An icon.
//
// Material Symbols is a font, and a font is the best renderer there is
// for a glyph: it is hinted for the size it is drawn at, it takes its
// colour from the text, and it comes with the variable axes the icons
// were designed around — FILL for the filled variants, wght for the
// weight.
//
// The text is drawn with the platform's own font engine rather than
// Qt's, because the other path turns the glyph into a path and
// rasterises that, which flattens the curves at the wrong resolution and
// gives the icons the ragged edges the font was meant to avoid.
//
// The baked path data is the fallback, for a machine without the font
// installed. It is the same artwork, taken from the font's own outlines.
Item {
    id: root

    property string name: "spark"
    property color color: Theme.onSurface
    /// Whether to use the filled variant of the icon, when it has one.
    property bool filled: false

    implicitWidth: 24
    implicitHeight: 24

    Text {
        id: glyph
        anchors.fill: parent
        // A name the font does not know is drawn as the name itself, which
        // is a long smear rather than an icon. The width gives it away.
        visible: Icons.available && implicitWidth <= width * 1.6
        text: Icons.ligature(root.name)
        color: root.color
        font.family: Icons.family
        font.pixelSize: Math.round(root.height)
        // The FILL axis is how Material draws the filled variant of an
        // icon. It is a map of axis to value, not a list.
        font.variableAxes: { "FILL": root.filled ? 1 : 0 }
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        renderType: Text.NativeRendering
        clip: true
    }

    // The outline path is drawn in the 24-unit Material grid, y-flipped
    // from the font's frame. The Shape sits centred, and the scale grows
    // about the centre, so the glyph lands in the middle of its plate
    // instead of the top-left corner.
    Shape {
        anchors.centerIn: parent
        width: 24
        height: 24
        scale: Math.max(0.01, Math.min(root.width, root.height) / 24)
        transformOrigin: Item.Center
        visible: !Icons.available

        ShapePath {
            fillColor: root.color
            strokeColor: "transparent"
            // SVG fills with the nonzero rule, and the Material artwork is
            // drawn for it. Qt defaults to the even-odd rule, which punches
            // holes in the icons that have a shape inside a shape.
            fillRule: ShapePath.WindingFill

            PathSvg {
                path: root.filled ? Icons.fillPath(root.name) : Icons.path(root.name)
            }
        }
    }
}
