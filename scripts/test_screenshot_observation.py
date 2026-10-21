"""Deterministic CU-01 synthetic specification tests; no credentials or live I/O."""
from concurrent.futures import ThreadPoolExecutor
from dataclasses import replace
import json
import pickle
import struct
import unittest
import zlib

import screenshot_observation_fixture as spec


def chunk(kind, body):
    return (struct.pack(">I", len(body)) + kind + body
            + struct.pack(">I", zlib.crc32(kind + body)))


def png(width=2, height=2, raw=None, depth=8, color=2, packed=None):
    if raw is None:
        raw = (b"\0" + b"\x23\x45\x67" * width) * height
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, depth, color, 0, 0, 0))
            + chunk(b"IDAT", packed if packed is not None else zlib.compress(raw))
            + chunk(b"IEND", b""))


class ObservationContracts(unittest.TestCase):
    def setUp(self):
        self.now = 100.0
        self.cancelled = False
        self.binding = spec.Binding("fixture-principal", "run-1", "runtime-call-1",
                                    "provider-call-1", "exact-fixture-target", "screenshot")
        self.fixture = spec.ObservationFixture(
            self.binding.principal, self.binding.run, self.binding.target,
            lambda: self.now, lambda: self.cancelled)
        self.reads = 0

    def tearDown(self):
        self.fixture.close()

    def reset_fixture(self):
        self.fixture.close()
        self.setUp()

    def grant(self, binding=None, **kwargs):
        return spec.ObserveGrant(binding or self.binding, self.now, **kwargs)

    def source(self):
        self.reads += 1
        yield png()

    def observe(self, binding=None, source=None, grant=None):
        return self.fixture.observe(binding or self.binding, grant or self.grant(binding),
                                    source or self.source)

    def test_no_observe_authority_from_exec_presets_selection_or_resume(self):
        for unrelated in (None, "ask", "auto", "unbound", "exec-grant", {"resume": True},
                          {"machine": self.binding.target}):
            with self.subTest(unrelated=unrelated), self.assertRaisesRegex(spec.Rejected, "denied"):
                self.fixture.observe(self.binding, unrelated, self.source)
        self.assertEqual(self.reads, 0)

    def test_every_identity_operation_and_canonical_revision_bound(self):
        for field in ("principal", "run", "runtime_call", "provider_call", "target",
                      "operation", "revision"):
            value = "status" if field == "operation" else "different"
            changed = replace(self.binding, **{field: value})
            with self.subTest(field=field), self.assertRaises(spec.Rejected):
                self.fixture.observe(changed, self.grant(), self.source)
        self.assertEqual(self.reads, 0)
        for action in ("click", "type", "scroll", "key", "shell", "start", "open"):
            with self.assertRaises(spec.Rejected):
                self.grant(replace(self.binding, operation=action))
        with self.assertRaises(TypeError):
            spec.Binding(**{**self.binding.__dict__, "command": "anything"})

    def test_atomic_single_use_and_no_pickle_or_content_repr(self):
        grant = self.grant()
        def consume(_):
            try:
                grant.consume(self.binding, self.now, lambda: False)
                return True
            except spec.Rejected:
                return False
        with ThreadPoolExecutor(max_workers=8) as pool:
            self.assertEqual(sum(pool.map(consume, range(32))), 1)
        for value in (grant, self.fixture):
            with self.assertRaises(TypeError):
                pickle.dumps(value)
            self.assertNotIn(self.binding.target, repr(value))

    def test_expiry_and_pre_admission_cancellation_make_no_source_call(self):
        grant = self.grant(valid_for=1)
        self.now += 1
        with self.assertRaisesRegex(spec.Rejected, "expired"):
            self.observe(grant=grant)
        self.cancelled = True
        with self.assertRaisesRegex(spec.Rejected, "cancelled"):
            self.observe()
        self.assertEqual(self.reads, 0)

    def test_cancellation_racing_final_grant_check_prevents_admission(self):
        checks = iter((False, True))
        self.fixture._cancelled = lambda: next(checks)
        with self.assertRaisesRegex(spec.Rejected, "cancelled"):
            self.observe()
        self.fixture._cancelled = lambda: False
        self.assertEqual(self.reads, 0)
        self.assertEqual(self.fixture.accounting()["operations"], 0)

    def test_source_failure_consumes_grant_no_retry_no_arbitrary_error(self):
        grant = self.grant()
        def failed():
            self.reads += 1
            raise RuntimeError("private-screen-error-sentinel")
        with self.assertRaisesRegex(spec.Rejected, "^source_unknown$"):
            self.observe(source=failed, grant=grant)
        with self.assertRaisesRegex(spec.Rejected, "consumed"):
            self.observe(grant=grant)
        self.assertEqual(self.reads, 1)
        self.assertEqual(self.fixture.accounting()["operations"], 1)

    def test_valid_frame_accounting_and_zero_model_disclosure(self):
        self.assertEqual(self.observe(), {"validated": True, "width": 2, "height": 2})
        self.assertTrue(self.fixture.fresh(self.binding))
        counts = self.fixture.accounting()
        self.assertEqual(counts["received_png_bytes"], len(png()))
        self.assertEqual(counts["resident_pixels"], 4)
        self.assertEqual(counts["resident_rgba_bytes"], 16)
        self.assertEqual(counts["model_image_count"], 0)
        self.assertIsNone(counts["model_image_tokens"])
        self.assertIsNone(counts["model_image_cost"])
        self.assertNotIn(self.binding.target, json.dumps(counts))

    def test_replacement_failure_geometry_and_close_release_frames(self):
        self.observe()
        old = self.fixture._frame["rgba"]
        self.observe()
        self.assertEqual(len(old), 0)
        current = self.fixture._frame["rgba"]
        self.fixture.invalidate_geometry()
        self.assertEqual(len(current), 0)
        self.assertFalse(self.fixture.fresh(self.binding))
        self.fixture.close()
        with self.assertRaises(spec.Rejected):
            self.observe()

    def test_denial_and_malformed_replacement_never_leave_old_frame_fresh(self):
        self.observe()
        old = self.fixture._frame["rgba"]
        with self.assertRaises(spec.Rejected):
            self.observe(source=lambda: [b"not a PNG"])
        self.assertEqual(len(old), 0)
        self.assertFalse(self.fixture.fresh(self.binding))

    def test_retention_expiry_and_cancel_sweep_without_image_read(self):
        for cancel in (False, True):
            self.reset_fixture()
            self.observe()
            old = self.fixture._frame["rgba"]
            if cancel:
                self.cancelled = True
            else:
                self.now += spec.TTL
            self.fixture.sweep()
            self.assertEqual(len(old), 0)
            self.assertEqual(self.fixture.accounting()["resident_frames"], 0)
            self.fixture.close()

    def test_status_allowlist_no_logs_actions_target_or_geometry(self):
        binding = replace(self.binding, operation="status")
        payload = {"installed": True, "running": False, "log": "secret-sentinel",
                   "actions": [{"text": "secret-sentinel"}], "width": 999999}
        result = self.observe(binding, lambda: [json.dumps(payload).encode()])
        self.assertEqual(result, {"installed": True, "running": False})
        self.assertEqual(self.fixture.accounting()["resident_frames"], 0)
        for payload in (b'[]', b'{"installed":1,"running":true}', b'bad json',
                        b'{"installed":false,"running":true}',
                        b' ' * (spec.STATUS_BYTES + 1)):
            with self.assertRaises(spec.Rejected):
                self.observe(binding, lambda: [payload])

    def test_late_cancelled_or_deadline_source_releases_and_cannot_retry(self):
        for cancel in (False, True):
            self.reset_fixture()
            grant = self.grant()
            def late():
                self.reads += 1
                yield png()[:20]
                if cancel:
                    self.cancelled = True
                else:
                    self.now += 10
                yield png()[20:]
            with self.assertRaisesRegex(spec.Rejected, "unknown"):
                self.observe(source=late, grant=grant)
            self.assertEqual(self.fixture.accounting()["resident_frames"], 0)
            self.cancelled = False
            with self.assertRaises(spec.Rejected):
                self.observe(grant=grant)
            self.assertEqual(self.reads, 1)
            self.fixture.close()

    def test_per_frame_count_whole_window_and_status_count_limits(self):
        with self.assertRaisesRegex(spec.Rejected, "bytes_unknown"):
            self.observe(source=lambda: [b"x" * (spec.FRAME_BYTES + 1)])
        self.observe()
        with self.assertRaisesRegex(spec.Rejected, "count"):
            self.observe()
        self.assertEqual(self.reads, 1)
        self.reset_fixture()
        binding = replace(self.binding, operation="status")
        status = lambda: [b'{"installed":true,"running":false}']
        for _ in range(4):
            self.observe(binding, status)
        with self.assertRaisesRegex(spec.Rejected, "count"):
            self.observe(binding, status)
        self.reset_fixture()
        self.observe()
        self.now += 30
        with self.assertRaisesRegex(spec.Rejected, "window"):
            self.observe()

    def test_png_traffic_cap_includes_failed_frames_and_does_not_grow(self):
        # Malformed input still spends its received traffic and admitted count.
        for _ in range(2):
            with self.assertRaises(spec.Rejected):
                self.observe(source=lambda: [b"x" * spec.FRAME_BYTES])
        self.assertEqual(self.fixture.accounting()["received_png_bytes"], 2 * spec.FRAME_BYTES)
        with self.assertRaisesRegex(spec.Rejected, "count"):
            self.observe()
        self.assertEqual(self.reads, 0)
        self.assertEqual(self.fixture.accounting()["resident_frames"], 0)

    def test_call_change_is_stale_and_window_can_expire_during_receipt(self):
        self.observe()
        self.assertFalse(self.fixture.fresh(replace(self.binding, provider_call="next-call")))
        self.now += 25
        def late():
            yield png()[:20]
            self.now += 5
            yield png()[20:]
        with self.assertRaisesRegex(spec.Rejected, "deadline_unknown"):
            self.observe(source=late)
        self.assertFalse(self.fixture.fresh(self.binding))

    def test_png_rejects_crc_truncation_trailing_metadata_animation_formats_and_bombs(self):
        valid = png()
        bad_crc = bytearray(valid)
        bad_crc[30] ^= 1
        cases = [bytes(bad_crc), valid[:-1], valid + b"trailing", b"jpeg",
                 valid[:33] + chunk(b"tEXt", b"private-sentinel") + valid[33:],
                 valid[:33] + chunk(b"acTL", b"animation") + valid[33:],
                 png(raw=b"\1" + b"x" * 6 + b"\0" + b"x" * 6),
                 png(depth=16), png(color=6), png(width=0),
                 png(width=2561, height=1, raw=b""),
                 png(width=2560, height=2560, raw=b""),
                 png(raw=b"x" * 1_000_000),
                 png(packed=zlib.compress(b"x" * 14) + zlib.compress(b"trailing")),
                 png(packed=b"bad zlib")]
        for index, data in enumerate(cases):
            with self.subTest(index=index), self.assertRaises(spec.Rejected):
                spec.decode_fixture_png(data)


if __name__ == "__main__":
    unittest.main()
