// mudd.one — Inspector panel (right sidebar)
// Created by M&K (c)2026 VetCoders

import AppKit

class InspectorViewController: NSViewController {
    private let stackView = NSStackView()
    private let titleLabel = NSTextField(labelWithString: "Inspector")
    private let infoLabel = NSTextField(wrappingLabelWithString: "Open a file to see details")

    override func loadView() {
        let container = NSView()
        container.wantsLayer = true
        view = container

        titleLabel.font = .boldSystemFont(ofSize: 14)
        titleLabel.alignment = .center

        infoLabel.font = .systemFont(ofSize: 11)
        infoLabel.textColor = .secondaryLabelColor

        stackView.orientation = .vertical
        stackView.alignment = .leading
        stackView.spacing = 8
        stackView.edgeInsets = NSEdgeInsets(top: 12, left: 12, bottom: 12, right: 12)
        stackView.translatesAutoresizingMaskIntoConstraints = false

        stackView.addArrangedSubview(titleLabel)
        stackView.addArrangedSubview(infoLabel)

        let spacer = NSView()
        spacer.setContentHuggingPriority(.defaultLow, for: .vertical)
        stackView.addArrangedSubview(spacer)

        container.addSubview(stackView)
        NSLayoutConstraint.activate([
            stackView.topAnchor.constraint(equalTo: container.topAnchor),
            stackView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            stackView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            stackView.bottomAnchor.constraint(equalTo: container.bottomAnchor),
        ])

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleFramesLoaded),
            name: .muddFramesLoaded,
            object: nil
        )
    }

    @objc private func handleFramesLoaded(_ notification: Notification) {
        guard let frames = notification.userInfo?["frames"] as? [FfiFrame],
              let first = frames.first else { return }

        let colorspace: String
        switch first.channels {
        case 1: colorspace = "Grayscale"
        case 3: colorspace = "RGB"
        case 4: colorspace = "RGBA"
        default: colorspace = "\(first.channels) channels"
        }

        let dataSize = frames.reduce(0) { $0 + $1.data.count }
        let dataMB = String(format: "%.1f", Double(dataSize) / 1_048_576.0)

        infoLabel.stringValue = """
        Dimensions: \(first.width) x \(first.height)
        Color space: \(colorspace)
        Frames: \(frames.count)
        Data size: \(dataMB) MB
        Stride: \(first.width * UInt32(first.channels)) bytes/row
        """
    }
}
