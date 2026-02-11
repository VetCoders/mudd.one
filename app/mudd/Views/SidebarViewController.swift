// mudd.one — Sidebar (tools)
// Created by M&K (c)2026 VetCoders

import AppKit

class SidebarViewController: NSViewController {
    private let stackView = NSStackView()
    private let openButton = NSButton(title: "Open File...", target: nil, action: nil)
    private let autoRoiButton = NSButton(title: "Auto ROI", target: nil, action: nil)
    private let cropButton = NSButton(title: "Apply Crop", target: nil, action: nil)
    private let titleLabel = NSTextField(labelWithString: "mudd.one")

    override func loadView() {
        let container = NSView()
        container.wantsLayer = true
        view = container

        titleLabel.font = .boldSystemFont(ofSize: 16)
        titleLabel.alignment = .center

        openButton.bezelStyle = .rounded
        openButton.target = self
        openButton.action = #selector(openFile)

        autoRoiButton.bezelStyle = .rounded
        autoRoiButton.isEnabled = false

        cropButton.bezelStyle = .rounded
        cropButton.isEnabled = false

        let separator = NSBox()
        separator.boxType = .separator

        stackView.orientation = .vertical
        stackView.alignment = .centerX
        stackView.spacing = 8
        stackView.edgeInsets = NSEdgeInsets(top: 12, left: 12, bottom: 12, right: 12)
        stackView.translatesAutoresizingMaskIntoConstraints = false

        stackView.addArrangedSubview(titleLabel)
        stackView.addArrangedSubview(separator)
        stackView.addArrangedSubview(openButton)
        stackView.addArrangedSubview(autoRoiButton)
        stackView.addArrangedSubview(cropButton)

        // Spacer pushes buttons to top
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
    }

    @objc private func openFile() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [
            .init(filenameExtension: "dcm")!,
            .init(filenameExtension: "dicom")!,
            .png, .jpeg, .tiff, .bmp,
            .init(filenameExtension: "mp4")!,
            .init(filenameExtension: "avi")!,
            .init(filenameExtension: "mov")!,
        ]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.message = "Select a DICOM, image, or video file"

        panel.beginSheetModal(for: view.window!) { [weak self] response in
            guard response == .OK, let url = panel.url else { return }
            self?.loadFile(url: url)
        }
    }

    private func loadFile(url: URL) {
        NotificationCenter.default.post(
            name: .muddFileSelected,
            object: nil,
            userInfo: ["url": url]
        )
    }
}

extension Notification.Name {
    static let muddFileSelected = Notification.Name("muddFileSelected")
    static let muddFramesLoaded = Notification.Name("muddFramesLoaded")
}
