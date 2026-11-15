import AppKit
import CoreAudio
import CoreMediaIO
import Intents
import PresenceCore

/// ADR 0042: what the host can tell about the user right now, for the
/// interruption rule. Every signal is read locally, on demand, and never
/// stored or sent anywhere. Each one is best effort; see the ADR.
enum SignalReader {
    /// Asks once for Focus status (macOS "Share Focus status" prompt).
    static func requestFocusAccess() {
        if INFocusStatusCenter.default.authorizationStatus == .notDetermined {
            INFocusStatusCenter.default.requestAuthorization { _ in }
        }
    }

    /// nil when Astrid isn't allowed to know.
    static func focus() -> Bool? {
        let center = INFocusStatusCenter.default
        guard center.authorizationStatus == .authorized else { return nil }
        return center.focusStatus.isFocused
    }

    static func typing() -> Bool {
        CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType: .keyDown) < 5
    }

    static func locked() -> Bool {
        guard let session = CGSessionCopyCurrentDictionary() as? [String: Any] else { return false }
        return session["CGSSessionScreenIsLocked"] as? Bool == true
    }

    static func microphoneInUse() -> Bool {
        var device = AudioObjectID(0)
        var size = UInt32(MemoryLayout<AudioObjectID>.size)
        var address = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyDefaultInputDevice,
                                                 mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        guard AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &address, 0, nil, &size, &device) == noErr, device != 0 else { return false }
        var running: UInt32 = 0
        size = UInt32(MemoryLayout<UInt32>.size)
        address.mSelector = kAudioDevicePropertyDeviceIsRunningSomewhere
        guard AudioObjectGetPropertyData(device, &address, 0, nil, &size, &running) == noErr else { return false }
        return running != 0
    }

    static func cameraInUse() -> Bool {
        var address = CMIOObjectPropertyAddress(mSelector: CMIOObjectPropertySelector(kCMIOHardwarePropertyDevices),
                                                mScope: CMIOObjectPropertyScope(kCMIOObjectPropertyScopeGlobal),
                                                mElement: CMIOObjectPropertyElement(kCMIOObjectPropertyElementMain))
        var size: UInt32 = 0
        guard CMIOObjectGetPropertyDataSize(CMIOObjectID(kCMIOObjectSystemObject), &address, 0, nil, &size) == 0, size > 0 else { return false }
        var devices = [CMIOObjectID](repeating: 0, count: Int(size) / MemoryLayout<CMIOObjectID>.size)
        var used: UInt32 = 0
        guard CMIOObjectGetPropertyData(CMIOObjectID(kCMIOObjectSystemObject), &address, 0, nil, size, &used, &devices) == 0 else { return false }
        address.mSelector = CMIOObjectPropertySelector(kCMIODevicePropertyDeviceIsRunningSomewhere)
        return devices.contains { device in
            var running: UInt32 = 0
            var got: UInt32 = 0
            return CMIOObjectGetPropertyData(device, &address, 0, nil, UInt32(MemoryLayout<UInt32>.size), &got, &running) == 0 && running != 0
        }
    }

    /// Apps that run only while the screen is being shared (Zoom's share host).
    static let sharingApps = ["us.zoom.CptHost", "com.apple.ScreenContinuity"]

    static func screenShared() -> Bool {
        NSWorkspace.shared.runningApplications.contains { app in
            sharingApps.contains(app.bundleIdentifier ?? "") || app.localizedName == "CptHost"
        }
    }

    /// The frontmost app owns a normal window as large as a whole display.
    static func fullScreen() -> Bool {
        guard let pid = NSWorkspace.shared.frontmostApplication?.processIdentifier,
              let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else { return false }
        let sizes = NSScreen.screens.map(\.frame.size)
        return windows.contains { window in
            guard window[kCGWindowOwnerPID as String] as? pid_t == pid, window[kCGWindowLayer as String] as? Int == 0,
                  let bounds = window[kCGWindowBounds as String] as? [String: CGFloat] else { return false }
            let size = CGSize(width: bounds["Width"] ?? 0, height: bounds["Height"] ?? 0)
            return sizes.contains { abs($0.width - size.width) < 1 && abs($0.height - size.height) < 1 }
        }
    }

    /// `ownMic`: Astrid itself holds the microphone (wake word or push-to-talk),
    /// so "mic in use" can't tell a call; the camera still can.
    static func current(busy: Bool, ownMic: Bool) -> InterruptionSignals {
        InterruptionSignals(focus: focus(), screenShared: screenShared(),
                            inCall: cameraInUse() || (!ownMic && microphoneInUse()),
                            typing: typing(), fullScreen: fullScreen(), busy: busy, locked: locked())
    }
}
