import QtQuick

Item {
    id: root
    property bool quiet: false

    implicitWidth: 180
    implicitHeight: 138

    Canvas {
        id: drawing
        anchors.fill: parent
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const context = getContext("2d")
            context.clearRect(0, 0, width, height)

            context.fillStyle = root.quiet ? "#F0EEE6" : "#FAF9F5"
            context.beginPath()
            context.moveTo(width * 0.16, height * 0.18)
            context.bezierCurveTo(
                width * 0.34, height * 0.02,
                width * 0.78, height * 0.08,
                width * 0.87, height * 0.29
            )
            context.bezierCurveTo(
                width * 0.98, height * 0.56,
                width * 0.78, height * 0.91,
                width * 0.49, height * 0.94
            )
            context.bezierCurveTo(
                width * 0.22, height * 0.98,
                width * 0.04, height * 0.72,
                width * 0.10, height * 0.47
            )
            context.closePath()
            context.fill()

            context.strokeStyle = "#141413"
            context.fillStyle = "#141413"
            context.lineWidth = Math.max(6, width * 0.045)
            context.lineCap = "round"
            context.lineJoin = "round"

            context.beginPath()
            context.moveTo(width * 0.25, height * 0.60)
            context.bezierCurveTo(
                width * 0.38, height * 0.23,
                width * 0.55, height * 0.80,
                width * 0.73, height * 0.38
            )
            context.stroke()

            context.beginPath()
            context.moveTo(width * 0.34, height * 0.72)
            context.bezierCurveTo(
                width * 0.48, height * 0.48,
                width * 0.60, height * 0.72,
                width * 0.78, height * 0.63
            )
            context.stroke()

            const dot = Math.max(5, width * 0.035)
            for (const point of [[0.24, 0.35], [0.79, 0.30], [0.76, 0.76]]) {
                context.beginPath()
                context.arc(width * point[0], height * point[1], dot, 0, Math.PI * 2)
                context.fill()
            }
        }
    }
}
