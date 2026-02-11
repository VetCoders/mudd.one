// mudd.one — Canvas (image display + sequence navigator)
// Created by M&K (c)2026 VetCoders

import AppKit

class CanvasViewController: NSViewController {
    private let imageView = NSImageView()
    private let statusLabel = NSTextField(labelWithString: "No file loaded")
    private let sequenceSlider = NSSlider(value: 0, minValue: 0, maxValue: 0, target: nil, action: nil)
    private let frameLabel = NSTextField(labelWithString: "")

    private var frames: [FfiFrame] = []
    private var currentIndex: Int = 0

    override func loadView() {
        let container = NSView()
        container.wantsLayer = true
        view = container

        // Image view — fills center
        imageView.imageScaling = .scaleProportionallyUpOrDown
        imageView.imageAlignment = .alignCenter
        imageView.wantsLayer = true
        imageView.layer?.backgroundColor = NSColor.black.cgColor
        imageView.translatesAutoresizingMaskIntoConstraints = false

        // Status bar at top
        statusLabel.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        statusLabel.textColor = .secondaryLabelColor
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        // Sequence navigator at bottom
        sequenceSlider.target = self
        sequenceSlider.action = #selector(sliderChanged)
        sequenceSlider.isHidden = true
        sequenceSlider.translatesAutoresizingMaskIntoConstraints = false

        frameLabel.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        frameLabel.textColor = .secondaryLabelColor
        frameLabel.alignment = .center
        frameLabel.isHidden = true
        frameLabel.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(statusLabel)
        container.addSubview(imageView)
        container.addSubview(sequenceSlider)
        container.addSubview(frameLabel)

        NSLayoutConstraint.activate([
            statusLabel.topAnchor.constraint(equalTo: container.topAnchor, constant: 4),
            statusLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            statusLabel.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),

            imageView.topAnchor.constraint(equalTo: statusLabel.bottomAnchor, constant: 4),
            imageView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            imageView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            imageView.bottomAnchor.constraint(equalTo: sequenceSlider.topAnchor, constant: -4),

            sequenceSlider.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            sequenceSlider.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),
            sequenceSlider.bottomAnchor.constraint(equalTo: frameLabel.topAnchor, constant: -2),

            frameLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            frameLabel.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            frameLabel.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -4),
            frameLabel.heightAnchor.constraint(equalToConstant: 16),
        ])

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleFileSelected),
            name: .muddFileSelected,
            object: nil
        )
    }

    @objc private func handleFileSelected(_ notification: Notification) {
        guard let url = notification.userInfo?["url"] as? URL else { return }

        statusLabel.stringValue = "Loading \(url.lastPathComponent)..."

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            do {
                let loadedFrames = try loadFile(path: url.path)
                DispatchQueue.main.async {
                    self?.displayFrames(loadedFrames, filename: url.lastPathComponent)
                }
            } catch {
                DispatchQueue.main.async {
                    self?.statusLabel.stringValue = "Error: \(error.localizedDescription)"
                }
            }
        }
    }

    private func displayFrames(_ loadedFrames: [FfiFrame], filename: String) {
        frames = loadedFrames
        currentIndex = 0

        guard let first = frames.first else {
            statusLabel.stringValue = "No frames in \(filename)"
            return
        }

        let colorspace: String
        switch first.channels {
        case 1: colorspace = "Grayscale"
        case 3: colorspace = "RGB"
        case 4: colorspace = "RGBA"
        default: colorspace = "?\(first.channels)ch"
        }

        statusLabel.stringValue = "\(filename) | \(first.width)x\(first.height) | \(colorspace) | \(frames.count) frame(s)"

        // Sequence navigator
        if frames.count > 1 {
            sequenceSlider.isHidden = false
            frameLabel.isHidden = false
            sequenceSlider.maxValue = Double(frames.count - 1)
            sequenceSlider.integerValue = 0
            sequenceSlider.numberOfTickMarks = min(frames.count, 100)
            updateFrameLabel()
        } else {
            sequenceSlider.isHidden = true
            frameLabel.isHidden = true
        }

        showFrame(at: 0)

        NotificationCenter.default.post(
            name: .muddFramesLoaded,
            object: nil,
            userInfo: ["frames": frames]
        )
    }

    @objc private func sliderChanged() {
        let idx = sequenceSlider.integerValue
        guard idx != currentIndex, idx >= 0, idx < frames.count else { return }
        currentIndex = idx
        showFrame(at: idx)
        updateFrameLabel()
    }

    private func updateFrameLabel() {
        frameLabel.stringValue = "\(currentIndex + 1) / \(frames.count)"
    }

    private func showFrame(at index: Int) {
        guard index < frames.count else { return }
        let frame = frames[index]
        currentIndex = index

        guard let nsImage = makeNSImage(from: frame) else {
            statusLabel.stringValue = "Failed to create image from frame data"
            return
        }
        imageView.image = nsImage
    }

    private func makeNSImage(from frame: FfiFrame) -> NSImage? {
        let w = Int(frame.width)
        let h = Int(frame.height)
        let ch = Int(frame.channels)

        let bitsPerComponent = 8
        let bitsPerPixel = bitsPerComponent * ch
        let bytesPerRow = w * ch

        let colorSpace: CGColorSpace
        let bitmapInfo: CGBitmapInfo

        switch ch {
        case 1:
            colorSpace = CGColorSpaceCreateDeviceGray()
            bitmapInfo = CGBitmapInfo(rawValue: 0)
        case 3:
            colorSpace = CGColorSpaceCreateDeviceRGB()
            bitmapInfo = CGBitmapInfo(rawValue: 0)
        case 4:
            colorSpace = CGColorSpaceCreateDeviceRGB()
            bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue)
        default:
            return nil
        }

        guard frame.data.count >= w * h * ch else { return nil }

        guard let provider = CGDataProvider(data: frame.data as CFData) else {
            return nil
        }
        guard let cgImage = CGImage(
            width: w, height: h,
            bitsPerComponent: bitsPerComponent,
            bitsPerPixel: bitsPerPixel,
            bytesPerRow: bytesPerRow,
            space: colorSpace,
            bitmapInfo: bitmapInfo,
            provider: provider,
            decode: nil,
            shouldInterpolate: true,
            intent: .defaultIntent
        ) else { return nil }

        return NSImage(cgImage: cgImage, size: NSSize(width: w, height: h))
    }
}
