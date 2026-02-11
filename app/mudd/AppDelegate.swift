// mudd.one — Veterinary Ultrasound Processing
// Created by M&K (c)2026 VetCoders

import AppKit

@main
class AppDelegate: NSObject, NSApplicationDelegate {
    var mainWindow: MainWindowController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        mainWindow = MainWindowController()
        mainWindow?.showWindow(nil)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
        true
    }
}
