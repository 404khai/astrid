# astrid-voice

Astrid's local voice ([ADR 0035](../../docs/adr/0035-local-spoken-replies.md)):
Kokoro-82M on MLX behind JSON lines. The presence app starts it on demand.

```sh
cd apps/astrid-voice && uv sync     # Python 3.12, MLX, misaki; the model downloads on first run
echo '{"type":"say","id":1,"text":"Hello, I am Astrid."}' | .venv/bin/astrid-voice
```

stdin: `{"type":"say","id":n,"text":..,"voice":"af_heart"}` or `{"type":"stop"}`.
stdout: `ready`, then `audio` lines (`pcm`: base64 float32 mono 24 kHz, one per
sentence), then `done` (or `error`). Nothing is written to disk.
