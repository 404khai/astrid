"""Astrid's local voice (ADR 0035): Kokoro-82M on MLX behind JSON lines.

stdin:  {"type":"say","id":n,"text":..,"voice":"af_heart"} | {"type":"stop"}
stdout: {"type":"ready","sample_rate":24000}
        | {"type":"audio","id":n,"pcm":<base64 float32 mono, 24 kHz>}
        | {"type":"done","id":n} | {"type":"error","id":n,"text":..,"message":..}

Audio streams one sentence at a time so playback starts after the first
sentence. A "stop" (or a newer "say") abandons the current reply between
sentences. Nothing is written to disk; the model downloads once into the
Hugging Face cache.
"""

from __future__ import annotations

import base64
import json
import os
import queue
import re
import sys
import threading

MODEL = os.environ.get("ASTRID_VOICE_MODEL", "mlx-community/Kokoro-82M-bf16")
SAMPLE_RATE = 24_000
# Sentence ends, keeping the punctuation with the sentence.
SENTENCES = re.compile(r"(?<=[.!?…])\s+|\n+")
MAX_TEXT = 4_000

_out = threading.Lock()
# The protocol owns the real stdout; anything libraries print goes to stderr.
_protocol = os.fdopen(os.dup(1), "w", buffering=1)
os.dup2(2, 1)
sys.stdout = sys.stderr


def emit(message: dict) -> None:
    with _out:
        _protocol.write(json.dumps(message) + "\n")
        _protocol.flush()


def sentences(text: str) -> list[str]:
    return [s.strip() for s in SENTENCES.split(text[:MAX_TEXT]) if s.strip()]


def main() -> None:
    import numpy as np
    from mlx_audio.tts.utils import load_model

    model = load_model(MODEL)
    # Warm the pipeline so the first reply isn't paying for set-up.
    for _ in model.generate("Ready.", voice="af_heart", lang_code="a"):
        pass
    emit({"type": "ready", "sample_rate": SAMPLE_RATE})

    inbox: queue.Queue[dict | None] = queue.Queue()
    latest = {"id": None}

    def read() -> None:
        for line in sys.stdin:
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(message, dict):
                continue
            if message.get("type") == "stop":
                latest["id"] = None
            elif message.get("type") == "say":
                latest["id"] = message.get("id")
                inbox.put(message)
        inbox.put(None)

    threading.Thread(target=read, daemon=True).start()

    while (message := inbox.get()) is not None:
        rid, text = message.get("id"), str(message.get("text", ""))
        voice = str(message.get("voice") or "af_heart")
        try:
            for sentence in sentences(text):
                if latest["id"] != rid:
                    break
                for result in model.generate(sentence, voice=voice, lang_code="a", split_pattern=r"\n+"):
                    if latest["id"] != rid:
                        break
                    pcm = np.asarray(result.audio, dtype=np.float32)
                    emit({"type": "audio", "id": rid, "pcm": base64.b64encode(pcm.tobytes()).decode()})
            emit({"type": "done", "id": rid})
        except Exception as error:  # noqa: BLE001 - report and keep serving
            emit({"type": "error", "id": rid, "text": text, "message": str(error)[:300]})


if __name__ == "__main__":
    main()
