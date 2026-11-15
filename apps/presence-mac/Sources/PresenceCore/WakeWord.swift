import Foundation

/// ADR 0034 amendment (wake word): finds "hey Astrid", "yo Astrid" or plain
/// "Astrid" in a running on-device transcript, and what was said after it.
public enum WakeWord {
    /// What on-device recognition often hears for the name.
    static let names = ["astrid", "astrud", "astrids", "astra'd", "a strid"]

    public struct Hit: Equatable {
        /// Words said after the wake phrase (empty: just the name).
        public var rest: String
        /// Where the wake phrase ended, in characters of the transcript, so a
        /// later partial result doesn't fire the same hit again.
        public var end: Int
    }

    /// The last wake phrase in `transcript` that starts at or after
    /// character `from`, on word boundaries ("Astridine" is not the name).
    public static func match(_ transcript: String, from: Int = 0) -> Hit? {
        let lower = normalize(transcript)
        let text = Array(lower)
        // The rest keeps the speaker's casing when lowercasing kept the length.
        let original = Array(transcript)
        let source = original.count == text.count ? original : text
        var best: Hit?
        for name in names {
            for prefix in ["hey ", "yo ", ""] {
                let phrase = Array(prefix + name)
                var i = 0
                while i + phrase.count <= text.count {
                    defer { i += 1 }
                    guard i >= from, Array(text[i..<i + phrase.count]) == phrase else { continue }
                    let before = i == 0 || !text[i - 1].isLetter
                    let afterIndex = i + phrase.count
                    let after = afterIndex == text.count || !text[afterIndex].isLetter
                    guard before, after else { continue }
                    let rest = String(source[afterIndex...]).trimmingCharacters(in: CharacterSet(charactersIn: " ,.!?:-"))
                    // Prefer the latest hit.
                    if best == nil || afterIndex > best!.end {
                        best = Hit(rest: rest, end: afterIndex)
                    }
                }
            }
        }
        return best
    }

    /// Lowercased, with curly apostrophes made straight. Same length as the
    /// input, so character offsets carry over.
    static func normalize(_ text: String) -> String {
        String(text.lowercased().map { $0 == "\u{2019}" ? "'" : $0 })
    }
}
