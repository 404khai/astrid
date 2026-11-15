import Foundation

/// What Astrid says out loud, with written shorthand turned into words: "Nov
/// 25" is "November 25", "Mon, Nov 16" is "Monday, November 16", "2pm" is "2
/// PM". Only abbreviations next to a date are expanded, so "May", "Sun" or
/// "Wed" as ordinary words stay as they are. Display text is never changed.
public enum SpokenText {
    static let months = ["Jan": "January", "Feb": "February", "Mar": "March", "Apr": "April", "Jun": "June",
                         "Jul": "July", "Aug": "August", "Sep": "September", "Sept": "September",
                         "Oct": "October", "Nov": "November", "Dec": "December"]
    static let days = ["Mon": "Monday", "Tue": "Tuesday", "Tues": "Tuesday", "Wed": "Wednesday",
                       "Thu": "Thursday", "Thur": "Thursday", "Thurs": "Thursday", "Fri": "Friday",
                       "Sat": "Saturday", "Sun": "Sunday"]

    private static func replace(_ text: String, _ pattern: String, _ map: [String: String]) -> String {
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return text }
        var out = text
        for match in regex.matches(in: text, range: NSRange(text.startIndex..., in: text)).reversed() {
            guard let whole = Range(match.range, in: text), let word = Range(match.range(at: 1), in: text),
                  let full = map[String(text[word])] else { continue }
            let tail = text[word.upperBound..<whole.upperBound].replacingOccurrences(of: ".", with: "")
            out.replaceSubrange(Range(match.range, in: out)!, with: full + tail)
        }
        return out
    }

    public static func expand(_ text: String) -> String {
        let monthNames = months.keys.joined(separator: "|"), dayNames = days.keys.joined(separator: "|")
        // A month before a day number: "Nov 25", "Nov. 25", "Sept 3rd".
        var out = replace(text, "\\b(\(monthNames))\\.?(?= \\d{1,2}\\b)", months)
        // A month after one: "25 Nov".
        out = replace(out, "(?<=\\b\\d{1,2} )(\(monthNames))\\b\\.?", months)
        // A weekday before a date or a comma: "Mon, Nov 16", "Fri 20", "Tue Nov 17".
        let next = "(?=,| \\d{1,2}\\b| (?:\(months.values.joined(separator: "|"))\\b))"
        out = replace(out, "\\b(\(dayNames))\\.?\(next)", days)
        // "2pm", "4:30pm" read as letters, not "pim".
        out = out.replacingOccurrences(of: "(\\d)\\s?([ap])\\.?m\\.?\\b", with: "$1 $2M", options: [.regularExpression, .caseInsensitive])
        return out.replacingOccurrences(of: " aM", with: " AM").replacingOccurrences(of: " pM", with: " PM")
    }
}
