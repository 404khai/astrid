import tempfile
from pathlib import Path
import unittest
import compare_routing as comparison


class ComparisonContracts(unittest.TestCase):
    def test_identical_fixture_and_grade_rejects_test_tampering_or_failed_verification(self):
        with tempfile.TemporaryDirectory() as root:
            first, second = Path(root) / "a", Path(root) / "b"
            comparison.make_fixture(first, "coding")
            comparison.make_fixture(second, "coding")
            before = comparison.inventory(first)
            self.assertEqual(before, comparison.inventory(second))
            source = first / "src/lib.rs"
            tests = source.read_bytes().split(b"#[cfg(test)]", 1)[-1]
            success = {"exit_code": 0, "stdout": "2 passed"}
            self.assertFalse(comparison.coding_grade(first, before, tests, success))
            source.write_text(source.read_text().replace("let name = name;", "let name = name.trim();"))
            (first / "GREETING.txt").write_bytes(b"Hello, Astrid!\n")
            self.assertTrue(comparison.coding_grade(first, before, tests, success))
            self.assertFalse(comparison.coding_grade(first, before, tests, {"exit_code": 1, "stdout": "2 passed"}))
            source.write_text(source.read_text().replace('" Astrid "', '"Astrid"'))
            self.assertFalse(comparison.coding_grade(first, before, tests, success))
            source.unlink()
            self.assertFalse(comparison.coding_grade(first, before, tests, success))

    def test_notes_copies_preserve_exact_frozen_bytes(self):
        with tempfile.TemporaryDirectory() as root:
            folder = Path(root) / "notes"
            source = comparison.make_fixture(folder, "notes")
            self.assertEqual(comparison.inventory(folder), comparison.inventory(source))


if __name__ == "__main__":
    unittest.main()
