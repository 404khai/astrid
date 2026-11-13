# ADR 0035: Local spoken replies with Kokoro-82M

Status: Accepted (2026-11-13). The maintainer asked for Astrid to talk back and
answered "alright lets go for it" to option 1 below.

## Context

V2-03 needs Astrid to answer out loud. The maintainer suggested MisoTTS. The
development Mac is an Apple M3 with 16 GB of RAM.

- **MisoTTS 8B** (modified MIT): ~16 GB of bf16 weights. The README recommends
  ~24 GB of GPU memory, says CPU is slow, and doesn't mention MPS or MLX. It
  writes one WAV per request with no streaming API, and is English only. It
  won't run usefully on this Mac.
- **Kokoro-82M** (Apache-2.0): 82M parameters, runs on Apple Silicon through
  MLX (`mlx-audio`), 24 kHz output.

## Decision

1. `apps/astrid-voice`: a uv-managed Python process that runs
   `mlx-community/Kokoro-82M-bf16` through `mlx-audio`, with `misaki` for
   English text. JSON lines on stdin/stdout:
   - in: `say {id, text, voice}`, `stop`
   - out: `ready`, `audio {id, pcm: base64 float32 mono 24 kHz}`, `done`, `error`

   It streams one sentence at a time, keeps stdout for the protocol only, and
   writes nothing to disk. The model downloads once into the Hugging Face cache.
2. The presence host starts it lazily on the first reply and plays audio with
   `AVAudioPlayerNode`. Playback runs through the same analyser as the mic, so
   the voice ring moves with Astrid's voice in the `speaking` state. When the
   process is missing or fails, `AVSpeechSynthesizer` speaks the same reply
   through the same path.
3. What is spoken: the brief's title and summary, or up to ~600 characters of
   a plain answer. The full brief stays on screen.
4. When: answers to a dictated ask are read aloud. Typed asks stay silent
   unless the speaker button on the answer is pressed.
5. Barge-in: the mic, ⌥Space, a new ask or Esc stops speech immediately and
   drops any audio still arriving.
6. Timing: one metadata-only line per reply in
   `~/Library/Logs/Astrid/speech.jsonl`: engine, characters, first-audio,
   synthesis and total milliseconds, and whether it was interrupted. Never the
   text or audio. Speech happens after the runtime's run has ended, so it is a
   host log, not a run trace span.

## Alternatives

- macOS voice only: no dependencies, but noticeably robotic. It is kept as the
  fallback.
- MisoTTS on a remote GPU or Miso's API: best quality, but needs network and an
  account, and reply text would leave the Mac. Possible later behind the same
  JSON-lines contract.

## Consequences

Adds Python 3.12, MLX, spaCy and misaki (via `uv`) plus a ~330 MB model
download. English only. Voices are chosen with `--ask-voice` (default
`af_heart`).
