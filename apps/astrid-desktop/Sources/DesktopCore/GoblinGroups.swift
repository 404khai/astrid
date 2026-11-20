import Foundation

/// A mock group of goblins (cosmetic for now): a name and the bots in it, by
/// their file names in the Goblins folder. Kept in the app's preferences.
public struct GoblinGroup: Codable, Equatable, Identifiable, Sendable {
    public var id: UUID
    public var name: String
    public var members: [String]
    public var created: Date
    /// What the group is for, set in its side panel (absent in older groups).
    public var about: String?
    /// Each goblin's role in this group, by member id (absent in older groups).
    /// It is part of the goblin's prompt and kept in its memory.
    public var roles: [String: String]?

    public static let maxAbout = 280
    public static let maxRole = 120

    public func role(of member: String) -> String? {
        roles?[member].flatMap { $0.isEmpty ? nil : $0 }
    }

    public mutating func setRole(_ role: String, for member: String) {
        let role = String(role.trimmingCharacters(in: .whitespacesAndNewlines).prefix(Self.maxRole))
        var all = roles ?? [:]
        all[member] = role.isEmpty ? nil : role
        roles = all.isEmpty ? nil : all
    }

    public init(id: UUID = UUID(), name: String, members: [String], created: Date = Date(), about: String? = nil) {
        self.id = id
        self.name = String(name.prefix(60))
        self.about = about.map { String($0.prefix(Self.maxAbout)) }
        self.members = Array(members.prefix(GoblinGroups.maxMembers))
        self.created = created
    }
}

public enum GoblinGroups {
    public static let maxMembers = 10
    public static let maxGroups = 20

    /// A name from the members': "Sunee", "Sunee & Kirby", "Sunee, Kirby +3".
    public static func defaultName(_ names: [String]) -> String {
        switch names.count {
        case 0: return "New group"
        case 1: return names[0]
        case 2: return "\(names[0]) & \(names[1])"
        default: return "\(names[0]), \(names[1]) +\(names.count - 2)"
        }
    }

    public static func decode(_ data: Data?) -> [GoblinGroup] {
        guard let data, let groups = try? JSONDecoder().decode([GoblinGroup].self, from: data) else { return [] }
        return Array(groups.filter { !$0.members.isEmpty }.prefix(maxGroups))
    }

    public static func encode(_ groups: [GoblinGroup]) -> Data? {
        try? JSONEncoder().encode(Array(groups.prefix(maxGroups)))
    }
}
