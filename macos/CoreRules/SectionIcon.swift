// The SF Symbols of the library's sections in the sidebar, by the
// sections' ids in `cr_store::catalog`, as the Windows app draws its own.

enum SectionIcon {
    static func name(_ section: String) -> String {
        switch section {
        case "characters": return "person.2"
        case "races": return "figure.stand"
        case "classes": return "shield.lefthalf.filled"
        case "kits": return "backpack"
        case "weapons": return "hammer"
        case "armor": return "shield"
        case "equipment": return "bag"
        case "magic-items": return "sparkles"
        case "nonweapon": return "wrench.and.screwdriver"
        case "parts": return "puzzlepiece"
        case "wizard-spells": return "wand.and.stars"
        case "priest-spells": return "sun.max"
        case "psionics": return "brain.head.profile"
        case "monsters": return "pawprint"
        case "monster-types": return "tortoise"
        case "traps": return "exclamationmark.triangle"
        case "encounters": return "figure.fencing"
        case "templates": return "doc.on.doc"
        case "tables": return "tablecells"
        case "treasure": return "dollarsign.circle"
        case "treasure-tables": return "list.bullet.rectangle"
        case "books": return "book"
        default: return "folder"
        }
    }
}
