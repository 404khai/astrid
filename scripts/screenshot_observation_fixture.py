"""CU-01 executable specification. Synthetic fixtures ONLY, never a runtime adapter.

No filesystem, network, model, session or trace integration. The narrow PNG decoder
deliberately accepts only RGB8, filter-0, single-IDAT synthetic PNGs. It is not a
production image decoder. Fake time and explicit sweep model owner cleanup.
"""

from dataclasses import dataclass
import struct
import threading
import zlib


REVISION = "cu01-ephemeral-v1"
FRAME_BYTES = 1024 * 1024
PIXELS = 4_096_000
DIMENSION = 2560
DECODED_BYTES = PIXELS * 4
STATUS_BYTES = 16 * 1024
TTL = 10
WINDOW = 30


class Rejected(Exception):
    """Fixed categories only: arbitrary source error/content is never retained."""


@dataclass(frozen=True)
class Binding:
    principal: str
    run: str
    runtime_call: str
    provider_call: str
    target: str
    operation: str
    revision: str = REVISION

    def validate(self):
        identities = (self.principal, self.run, self.runtime_call,
                      self.provider_call, self.target)
        if any(not isinstance(s, str) or not s or len(s) > 256
               or any(ord(c) < 32 or ord(c) == 127 for c in s) for s in identities):
            raise Rejected("binding")
        if self.operation not in ("status", "screenshot") or self.revision != REVISION:
            raise Rejected("arguments")


class ObserveGrant:
    """A distinct test grant; no exec grant or local preset can substitute."""

    def __init__(self, binding, now, valid_for=30):
        binding.validate()
        if not 0 < valid_for <= 30:
            raise Rejected("expiry")
        self._binding = binding
        self._expires = now + valid_for
        self._used = False
        self._lock = threading.Lock()

    def consume(self, binding, now, cancelled):
        with self._lock:
            if cancelled():
                raise Rejected("cancelled")
            if binding != self._binding:
                raise Rejected("binding")
            if now >= self._expires:
                raise Rejected("expired")
            if self._used:
                raise Rejected("consumed")
            self._used = True

    def __repr__(self):
        return "<ObserveGrant>"

    def __reduce__(self):
        raise TypeError("ephemeral grant")


def decode_fixture_png(data):
    """Return (width, height, RGBA). Reject everything outside fixture format.

    CRC/framing/allocation checks precede bounded zlib decompression. Ancillary
    chunks, animation, interlace, additional streams and trailing bytes fail closed.
    Python zlib is not a killable production decoder; input and output are finite.
    """
    if len(data) > FRAME_BYTES or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise Rejected("png")
    pos, chunks = 8, []
    for expected in (b"IHDR", b"IDAT", b"IEND"):
        if len(data) - pos < 12:
            raise Rejected("png")
        size = struct.unpack_from(">I", data, pos)[0]
        kind = data[pos + 4:pos + 8]
        end = pos + 12 + size
        if kind != expected or end > len(data):
            raise Rejected("png")
        body = data[pos + 8:pos + 8 + size]
        crc = struct.unpack_from(">I", data, pos + 8 + size)[0]
        if zlib.crc32(kind + body) != crc:
            raise Rejected("png")
        chunks.append(body)
        pos = end
    if pos != len(data) or chunks[2] or len(chunks[0]) != 13:
        raise Rejected("png")
    width, height, depth, color, compression, filtering, interlace = struct.unpack(
        ">IIBBBBB", chunks[0])
    if (not 0 < width <= DIMENSION or not 0 < height <= DIMENSION
            or width * height > PIXELS or width * height * 4 > DECODED_BYTES):
        raise Rejected("pixels")
    if (depth, color, compression, filtering, interlace) != (8, 2, 0, 0, 0):
        raise Rejected("format")
    expected_bytes = (width * 3 + 1) * height
    decoder = zlib.decompressobj()
    try:
        raw = decoder.decompress(chunks[1], expected_bytes + 1)
    except zlib.error:
        raise Rejected("png") from None
    if (len(raw) != expected_bytes or not decoder.eof
            or decoder.unconsumed_tail or decoder.unused_data):
        raise Rejected("png")
    rgba = bytearray(width * height * 4)
    for y in range(height):
        row = y * (width * 3 + 1)
        if raw[row] != 0:
            raise Rejected("format")
        for x in range(width):
            src, dst = row + 1 + x * 3, (y * width + x) * 4
            rgba[dst:dst + 4] = raw[src:src + 3] + b"\xff"
    return width, height, rgba


class ObservationFixture:
    """Run-owned specification with a fake source and clock, no remote transport.

    Source is a factory yielding bounded byte chunks, not actual I/O. No raw
    image accessor/serialization is offered. Tests inspect private state only.
    sweep() models a future owner timer; no idle timer exists here.
    """

    def __init__(self, principal, run, target, clock, cancelled=lambda: False):
        self._owner = principal, run, target
        self._clock, self._cancelled = clock, cancelled
        self._start = None
        self._operations = self._screenshots = self._traffic = 0
        self._frame = None
        self._closed = False

    def __repr__(self):
        return "<ObservationFixture>"

    def __reduce__(self):
        raise TypeError("ephemeral observation")

    def _release(self):
        if self._frame is not None:
            # Best-effort release, not a forensic erasure guarantee for Python.
            self._frame["rgba"].clear()
            self._frame = None

    def close(self):
        self._release()
        self._closed = True

    def sweep(self):
        if (self._closed or self._cancelled() or self._frame is not None
                and self._clock() >= self._frame["expires"]):
            self._release()

    def fresh(self, binding):
        self.sweep()
        return self._frame is not None and self._frame["binding"] == binding

    def invalidate_geometry(self):
        self._release()

    def accounting(self):
        self.sweep()
        return {"operations": self._operations, "screenshots": self._screenshots,
                "received_png_bytes": self._traffic,
                "resident_frames": int(self._frame is not None),
                "resident_rgba_bytes": len(self._frame["rgba"]) if self._frame else 0,
                "resident_pixels": self._frame["pixels"] if self._frame else 0,
                "model_image_count": 0, "model_image_tokens": None,
                "model_image_cost": None}

    def observe(self, binding, grant, source):
        self._release()  # failed/new observations never leave a previous frame fresh
        binding.validate()
        if self._closed or self._cancelled():
            raise Rejected("cancelled")
        if (binding.principal, binding.run, binding.target) != self._owner:
            raise Rejected("binding")
        if not isinstance(grant, ObserveGrant):
            raise Rejected("denied")
        now = self._clock()
        if self._start is not None and now >= self._start + WINDOW:
            raise Rejected("window")
        if self._operations >= 4 or binding.operation == "screenshot" and self._screenshots >= 2:
            raise Rejected("count")
        grant.consume(binding, now, self._cancelled)
        # This is the fixture admission point. No retries, even on source failure.
        self._operations += 1
        if self._start is None:
            self._start = now
        if binding.operation == "screenshot":
            self._screenshots += 1
        deadline = min(now + TTL, self._start + WINDOW)

        def check():
            if self._cancelled():
                raise Rejected("cancelled_unknown")
            if self._clock() >= deadline:
                raise Rejected("deadline_unknown")

        payload = bytearray()
        try:
            check()
            for chunk in source():
                check()
                if not isinstance(chunk, bytes):
                    raise Rejected("source_unknown")
                limit = FRAME_BYTES if binding.operation == "screenshot" else STATUS_BYTES
                # A real reader must bound each read by remaining allowance + probe.
                # Fixture chunks already exist; never copy an over-limit chunk.
                if len(payload) + len(chunk) > limit:
                    raise Rejected("bytes_unknown")
                if binding.operation == "screenshot":
                    if self._traffic + len(chunk) > 2 * FRAME_BYTES:
                        raise Rejected("traffic_unknown")
                    self._traffic += len(chunk)
                payload.extend(chunk)
            check()
            if binding.operation == "status":
                import json
                try:
                    status = json.loads(payload)
                except (ValueError, UnicodeError):
                    raise Rejected("status_unknown") from None
                if (not isinstance(status, dict) or type(status.get("installed")) is not bool
                        or type(status.get("running")) is not bool
                        or status["running"] and not status["installed"]):
                    raise Rejected("status_unknown")
                return {k: status[k] for k in ("installed", "running")}
            width, height, rgba = decode_fixture_png(payload)
            check()
            self._frame = {"binding": binding, "rgba": rgba, "pixels": width * height,
                           "width": width, "height": height, "expires": deadline}
            return {"validated": True, "width": width, "height": height}
        except Rejected:
            self._release()
            raise
        except Exception:
            self._release()
            raise Rejected("source_unknown") from None
        finally:
            payload.clear()
