import QtQuick
import QtQuick.Shapes

Item {
    id: root

    property color fillColor: "transparent"
    property color strokeColor: "transparent"
    property real strokeWidth: 0
    property real cornerRadius: 24
    property real cornerPower: 4.2
    readonly property string pathData: buildPath(
        width, height, cornerRadius, cornerPower
    )

    function point(value) {
        return Number(value.toFixed(3))
    }

    function buildPath(shapeWidth, shapeHeight, requestedRadius, power) {
        const inset = Math.max(0, root.strokeWidth / 2)
        const left = inset
        const top = inset
        const right = Math.max(left, shapeWidth - inset)
        const bottom = Math.max(top, shapeHeight - inset)
        const radius = Math.max(0, Math.min(
            requestedRadius,
            (right - left) / 2,
            (bottom - top) / 2
        ))
        if (radius === 0)
            return "M " + left + " " + top + " H " + right
                + " V " + bottom + " H " + left + " Z"

        const exponent = 2 / Math.max(2, power)
        const segments = 12
        let path = "M " + root.point(left + radius) + " " + root.point(top)
            + " L " + root.point(right - radius) + " " + root.point(top)

        function curvePoint(corner, step) {
            const angle = Math.PI * step / (2 * segments)
            const sine = Math.pow(Math.sin(angle), exponent)
            const cosine = Math.pow(Math.cos(angle), exponent)
            if (corner === 0)
                return [right - radius + radius * sine, top + radius - radius * cosine]
            if (corner === 1)
                return [right - radius + radius * cosine, bottom - radius + radius * sine]
            if (corner === 2)
                return [left + radius - radius * sine, bottom - radius + radius * cosine]
            return [left + radius - radius * cosine, top + radius - radius * sine]
        }

        for (let corner = 0; corner < 4; ++corner) {
            if (corner === 1)
                path += " L " + root.point(right) + " " + root.point(bottom - radius)
            else if (corner === 2)
                path += " L " + root.point(left + radius) + " " + root.point(bottom)
            else if (corner === 3)
                path += " L " + root.point(left) + " " + root.point(top + radius)
            for (let step = 1; step <= segments; ++step) {
                const coordinates = curvePoint(corner, step)
                path += " L " + root.point(coordinates[0])
                    + " " + root.point(coordinates[1])
            }
        }
        return path + " Z"
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            fillColor: root.fillColor
            strokeColor: root.strokeColor
            strokeWidth: root.strokeWidth
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.pathData }
        }
    }
}
