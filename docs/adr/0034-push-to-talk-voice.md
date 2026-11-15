# ADR 0034: Push-to-talk voice on the presence ask surface

Status: Accepted (2026-11-13, maintainer: "let's work on V2-03 ... we use the voice ring listening component, it should react to sound")

## Context

V2-03 adds voice to Astrid's presence. The ask surface (ADR 0033) is a web UI
inside a native `WKWebView` panel. Microphone capture and speech recognition
can run in the page (`getUserMedia`, `webkitSpeechRecognition`) or in the host.

## Decision

- The native host owns the microphone (`AVAudioEngine`) and speech-to-text
  (`SFSpeechRecognizer` with `requiresOnDeviceRecognition = true`). If on-device
  recognition isn't available, listening fails with a visible error. It never
  falls back to server recognition.
- The mic is open only while the user holds the mic button (release sends), or
  between two taps. Esc cancels without sending. Audio and transcripts are not
  written anywhere. Holding ⌥Space anywhere (a Carbon hot key, so no
  Accessibility permission) opens the panel and listens until release. The final transcript is submitted exactly like a typed ask,
  so runtime permissions and approvals are unchanged.
- Bridge messages: page → host `{type:"listen", on}`; host → page
  `{type:"voice", level}` (0–1, from RMS dBFS), `{type:"transcript", text, final}`
  and `{type:"voice-error", message}`.
- The page draws the `voice-ring` `VoiceRing` around the listening mark, driven
  by the host's level. In a plain browser (dev), the ring follows the page mic
  and a scripted transcript stands in for speech.

## Alternatives

- Web Speech API in the page: in WebKit it may use Apple's servers, and its
  availability in `WKWebView` isn't documented. Rejected.
- Cloud streaming STT: a new provider, and audio would leave the Mac. Deferred.

## Not in this slice

TTS/speaking state, barge-in, a configurable hotkey, and trace latency
spans. Voice direction ([VoicePositionKit](https://libraries.dev/voice-position))
is iOS-only and needs an iPhone's stereo mics. It feeds `VoiceRing`'s `direction`
prop and belongs with the phone companion (V2-06).

## Amendment 2026-11-15: wake word (opt-in)

The maintainer asked for a wake word, "either hey astrid, Astrid, Yo astrid", to
make Astrid more "jarvis, baymax". This relaxes push-to-talk **only when the user
starts presence with `--wake`**. It is the agent's call, pending review.

- **Opt-in, visible.** It is off by default. With `--wake` the microphone stays
  open (macOS shows its microphone indicator) and `WakeListener` runs on-device
  recognition only (`requiresOnDeviceRecognition`, no fallback to Apple's
  servers). Recognition tasks renew every 50 s.
- **What leaves the listener.** Only the words after "hey Astrid", "yo Astrid" or
  "Astrid" (`WakeWord.match`, on word boundaries, with a few common
  mis-hearings). After a 1.2 s pause they go to the page as a voice ask. The
  name alone opens voice mode and listens. Audio and other transcripts are
  never stored, logged or sent.
- **One microphone user.** The listener pauses while push-to-talk holds the mic
  and while Astrid speaks, so its own voice never wakes it. A waiting operator
  card is left to its own voice answers.
