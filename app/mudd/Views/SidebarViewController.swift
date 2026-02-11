// mudd.one — Sidebar (tools)
// Created by M&K (c)2026 VetCoders

import AppKit

class SidebarViewController: NSViewController {
    private let stackView = NSStackView()
    private let titleLabel = NSTextField(labelWithString: "mudd.one")
    private let openButton = NSButton(title: "Open File...", target: nil, action: nil)

    // ROI
    private let roiSeparator = NSBox()
    private let roiLabel = NSTextField(labelWithString: "ROI")
    private let autoRoiButton = NSButton(title: "Auto ROI", target: nil, action: nil)
    private let cropButton = NSButton(title: "Apply Crop", target: nil, action: nil)
    private let clearRoiButton = NSButton(title: "Clear ROI", target: nil, action: nil)

    // Segmentation
    private let segSeparator = NSBox()
    private let segLabel = NSTextField(labelWithString: "Segmentation")
    private let initEngineButton = NSButton(title: "Init Engine...", target: nil, action: nil)
    private let engineStatusLabel = NSTextField(labelWithString: "Engine: not loaded")
    private let segModeButton = NSButton(title: "Prompt Mode", target: nil, action: nil)

    // Export
    private let exportSeparator = NSBox()
    private let exportLabel = NSTextField(labelWithString: "Export")
    private let exportButton = NSButton(title: "Export Dataset...", target: nil, action: nil)

    private var currentFrames: [FfiFrame] = []
    private var currentIndex: Int = 0
    private var currentRoi: FfiRoi?

    override func loadView() {
        let container = NSView()
        container.wantsLayer = true
        view = container

        titleLabel.font = .boldSystemFont(ofSize: 16)
        titleLabel.alignment = .center

        // Open
        openButton.bezelStyle = .rounded
        openButton.target = self
        openButton.action = #selector(openFile)

        // ROI section
        roiSeparator.boxType = .separator
        roiLabel.font = .boldSystemFont(ofSize: 11)
        roiLabel.textColor = .secondaryLabelColor

        autoRoiButton.bezelStyle = .rounded
        autoRoiButton.target = self
        autoRoiButton.action = #selector(detectAutoRoi)
        autoRoiButton.isEnabled = false

        cropButton.bezelStyle = .rounded
        cropButton.target = self
        cropButton.action = #selector(applyCrop)
        cropButton.isEnabled = false

        clearRoiButton.bezelStyle = .rounded
        clearRoiButton.target = self
        clearRoiButton.action = #selector(clearRoi)
        clearRoiButton.isEnabled = false

        // Segmentation section
        segSeparator.boxType = .separator
        segLabel.font = .boldSystemFont(ofSize: 11)
        segLabel.textColor = .secondaryLabelColor

        initEngineButton.bezelStyle = .rounded
        initEngineButton.target = self
        initEngineButton.action = #selector(openModelFile)

        engineStatusLabel.font = .systemFont(ofSize: 10)
        engineStatusLabel.textColor = .tertiaryLabelColor
        engineStatusLabel.alignment = .center

        segModeButton.bezelStyle = .rounded
        segModeButton.setButtonType(.toggle)
        segModeButton.target = self
        segModeButton.action = #selector(toggleSegMode)
        segModeButton.isEnabled = false

        // Export section
        exportSeparator.boxType = .separator
        exportLabel.font = .boldSystemFont(ofSize: 11)
        exportLabel.textColor = .secondaryLabelColor

        exportButton.bezelStyle = .rounded
        exportButton.target = self
        exportButton.action = #selector(exportDataset)
        exportButton.isEnabled = false

        // Layout
        stackView.orientation = .vertical
        stackView.alignment = .centerX
        stackView.spacing = 8
        stackView.edgeInsets = NSEdgeInsets(top: 12, left: 12, bottom: 12, right: 12)
        stackView.translatesAutoresizingMaskIntoConstraints = false

        stackView.addArrangedSubview(titleLabel)

        // File
        let fileSeparator = NSBox()
        fileSeparator.boxType = .separator
        stackView.addArrangedSubview(fileSeparator)
        stackView.addArrangedSubview(openButton)

        // ROI
        stackView.addArrangedSubview(roiSeparator)
        stackView.addArrangedSubview(roiLabel)
        stackView.addArrangedSubview(autoRoiButton)
        stackView.addArrangedSubview(cropButton)
        stackView.addArrangedSubview(clearRoiButton)

        // Segmentation
        stackView.addArrangedSubview(segSeparator)
        stackView.addArrangedSubview(segLabel)
        stackView.addArrangedSubview(initEngineButton)
        stackView.addArrangedSubview(engineStatusLabel)
        stackView.addArrangedSubview(segModeButton)

        // Export
        stackView.addArrangedSubview(exportSeparator)
        stackView.addArrangedSubview(exportLabel)
        stackView.addArrangedSubview(exportButton)

        // Spacer
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

        // Observe loaded frames
        NotificationCenter.default.addObserver(
            self, selector: #selector(handleFramesLoaded),
            name: .muddFramesLoaded, object: nil
        )
        NotificationCenter.default.addObserver(
            self, selector: #selector(handleRoiDetected),
            name: .muddRoiDetected, object: nil
        )
        NotificationCenter.default.addObserver(
            self, selector: #selector(handleManualRoi),
            name: .muddRoiManual, object: nil
        )
        NotificationCenter.default.addObserver(
            self, selector: #selector(handleIndexChanged),
            name: .muddCurrentIndexChanged, object: nil
        )
    }

    // MARK: - Notifications

    @objc private func handleFramesLoaded(_ notification: Notification) {
        guard let frames = notification.userInfo?["frames"] as? [FfiFrame] else { return }
        currentFrames = frames
        currentIndex = 0
        currentRoi = nil
        autoRoiButton.isEnabled = !frames.isEmpty
        cropButton.isEnabled = false
        clearRoiButton.isEnabled = false
        exportButton.isEnabled = !frames.isEmpty
    }

    @objc private func handleRoiDetected(_ notification: Notification) {
        guard let roi = notification.userInfo?["roi"] as? FfiRoi else { return }
        currentRoi = roi
        cropButton.isEnabled = true
        clearRoiButton.isEnabled = true
    }

    @objc private func handleManualRoi(_ notification: Notification) {
        guard let roi = notification.userInfo?["roi"] as? FfiRoi else { return }
        currentRoi = roi
        cropButton.isEnabled = true
        clearRoiButton.isEnabled = true
    }

    @objc private func handleIndexChanged(_ notification: Notification) {
        guard let index = notification.userInfo?["index"] as? Int else { return }
        currentIndex = index
        currentRoi = nil
        cropButton.isEnabled = false
        clearRoiButton.isEnabled = false
    }

    // MARK: - File

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
            name: .muddFileSelected, object: nil,
            userInfo: ["url": url]
        )
    }

    // MARK: - ROI

    @objc private func detectAutoRoi() {
        guard !currentFrames.isEmpty else { return }
        let frame = currentFrames[currentIndex]
        autoRoiButton.isEnabled = false
        autoRoiButton.title = "Detecting..."

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            do {
                let roi = try detectRoi(frame: frame)
                DispatchQueue.main.async {
                    self?.autoRoiButton.title = "Auto ROI"
                    self?.autoRoiButton.isEnabled = true
                    self?.currentRoi = roi
                    self?.cropButton.isEnabled = true
                    self?.clearRoiButton.isEnabled = true
                    NotificationCenter.default.post(
                        name: .muddRoiDetected, object: nil,
                        userInfo: ["roi": roi]
                    )
                }
            } catch {
                DispatchQueue.main.async {
                    self?.autoRoiButton.title = "Auto ROI"
                    self?.autoRoiButton.isEnabled = true
                    let alert = NSAlert()
                    alert.messageText = "ROI Detection Failed"
                    alert.informativeText = error.localizedDescription
                    alert.runModal()
                }
            }
        }
    }

    @objc private func applyCrop() {
        guard !currentFrames.isEmpty, let roi = currentRoi else { return }
        let idx = currentIndex
        let frame = currentFrames[idx]
        cropButton.isEnabled = false

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            do {
                let cropped = try cropFrame(frame: frame, roi: roi)
                DispatchQueue.main.async {
                    if idx < (self?.currentFrames.count ?? 0) {
                        self?.currentFrames[idx] = cropped
                    }
                    self?.currentRoi = nil
                    self?.cropButton.isEnabled = false
                    self?.clearRoiButton.isEnabled = false
                    NotificationCenter.default.post(
                        name: .muddFrameUpdated, object: nil,
                        userInfo: ["frame": cropped, "index": idx, "source": "crop"]
                    )
                }
            } catch {
                DispatchQueue.main.async {
                    self?.cropButton.isEnabled = true
                    let alert = NSAlert()
                    alert.messageText = "Crop Failed"
                    alert.informativeText = error.localizedDescription
                    alert.runModal()
                }
            }
        }
    }

    @objc private func clearRoi() {
        currentRoi = nil
        cropButton.isEnabled = false
        clearRoiButton.isEnabled = false
        NotificationCenter.default.post(
            name: .muddRoiDetected, object: nil,
            userInfo: ["roi": NSNull()]
        )
    }

    // MARK: - Segmentation

    @objc private func openModelFile() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.init(filenameExtension: "onnx")!]
        panel.message = "Select ONNX model file"
        panel.beginSheetModal(for: view.window!) { [weak self] response in
            guard response == .OK, let url = panel.url else { return }
            self?.loadEngineModel(path: url.path)
        }
    }

    private func loadEngineModel(path: String) {
        initEngineButton.isEnabled = false
        engineStatusLabel.stringValue = "Engine: loading..."

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            do {
                try initEngine(modelPath: path)
                let name = engineModelName() ?? "unknown"
                DispatchQueue.main.async {
                    self?.engineStatusLabel.stringValue = "Engine: \(name)"
                    self?.initEngineButton.isEnabled = true
                    self?.segModeButton.isEnabled = true
                }
            } catch {
                DispatchQueue.main.async {
                    self?.engineStatusLabel.stringValue = "Engine: failed"
                    self?.initEngineButton.isEnabled = true
                    let alert = NSAlert()
                    alert.messageText = "Engine Init Failed"
                    alert.informativeText = error.localizedDescription
                    alert.runModal()
                }
            }
        }
    }

    @objc private func toggleSegMode() {
        let active = segModeButton.state == .on
        segModeButton.title = active ? "Exit Prompt Mode" : "Prompt Mode"
        NotificationCenter.default.post(
            name: Notification.Name("muddSegModeChanged"), object: nil,
            userInfo: ["active": active]
        )
    }

    // MARK: - Export

    @objc private func exportDataset() {
        guard !currentFrames.isEmpty else { return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.folder]
        panel.nameFieldStringValue = "mudd_export"
        panel.message = "Select export directory"
        panel.canCreateDirectories = true

        // Use open panel for directory selection
        let openPanel = NSOpenPanel()
        openPanel.canChooseDirectories = true
        openPanel.canChooseFiles = false
        openPanel.canCreateDirectories = true
        openPanel.message = "Select export directory"

        openPanel.beginSheetModal(for: view.window!) { [weak self] response in
            guard response == .OK, let url = openPanel.url else { return }
            self?.showExportOptions(directory: url)
        }
    }

    private func showExportOptions(directory: URL) {
        let alert = NSAlert()
        alert.messageText = "Export Format"
        alert.informativeText = "Choose dataset export format"
        alert.addButton(withTitle: "YOLO")
        alert.addButton(withTitle: "COCO")
        alert.addButton(withTitle: "Cancel")

        let response = alert.runModal()
        guard response != .alertThirdButtonReturn else { return }

        let format = response == .alertFirstButtonReturn ? "yolo" : "coco"
        exportButton.isEnabled = false
        exportButton.title = "Exporting..."

        // Export runs on background, posts results
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            // TODO: Wire to Rust export functions when ready
            // For now just create the directory structure
            let formatDir = directory.appendingPathComponent(format)
            try? FileManager.default.createDirectory(at: formatDir, withIntermediateDirectories: true)

            DispatchQueue.main.async {
                self?.exportButton.title = "Export Dataset..."
                self?.exportButton.isEnabled = true
                let done = NSAlert()
                done.messageText = "Export"
                done.informativeText = "Export directory created: \(formatDir.path)"
                done.runModal()
            }
        }
    }
}

extension Notification.Name {
    static let muddFileSelected = Notification.Name("muddFileSelected")
    static let muddFramesLoaded = Notification.Name("muddFramesLoaded")
    static let muddRoiDetected = Notification.Name("muddRoiDetected")
    static let muddRoiManual = Notification.Name("muddRoiManual")
    static let muddFrameUpdated = Notification.Name("muddFrameUpdated")
    static let muddCurrentIndexChanged = Notification.Name("muddCurrentIndexChanged")
}
