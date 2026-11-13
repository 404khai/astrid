import unittest

from astrid_voice import MAX_TEXT, sentences


class Sentences(unittest.TestCase):
    def test_splits_on_sentence_ends_and_lines(self):
        self.assertEqual(
            sentences("Two replies need you. Flight moved!\nReceipts… done?  Yes"),
            ["Two replies need you.", "Flight moved!", "Receipts…", "done?", "Yes"],
        )

    def test_keeps_decimals_and_drops_blanks(self):
        self.assertEqual(sentences("It costs $24.00 today.\n\n"), ["It costs $24.00 today."])

    def test_caps_text(self):
        self.assertLessEqual(sum(map(len, sentences("a. " * MAX_TEXT))), MAX_TEXT)


if __name__ == "__main__":
    unittest.main()
