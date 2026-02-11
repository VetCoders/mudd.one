// mudd.one — Main Window Controller
// Created by M&K (c)2026 VetCoders

import AppKit

class MainWindowController: NSWindowController {
    private let mainViewController = MainSplitViewController()

    init() {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1200, height: 800),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "mudd.one"
        window.center()
        window.setFrameAutosaveName("MuddMainWindow")
        window.contentViewController = mainViewController
        window.minSize = NSSize(width: 800, height: 600)
        super.init(window: window)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError()
    }
}
