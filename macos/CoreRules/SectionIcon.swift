// The icons of the library's sections in the sidebar, by the sections'
// ids in `cr_store::catalog`: SF Symbols, and the sword the GNOME and
// Windows apps draw for the weapons, which SF Symbols lacks.

import SwiftUI

struct SectionIconView: View {
    let section: String

    var body: some View {
        if section == "weapons" {
            Sword()
                .fill(.tint)
                .frame(width: 16, height: 16)
        } else {
            Image(systemName: SectionIcon.name(section))
        }
    }
}

/// Material Design Icons' sword (Pictogrammers, Apache License 2.0), on
/// a 16-point grid, as the Windows app draws it.
struct Sword: Shape {
    private static let data =
        "M4.61,3.33 L3.33,3.33 L9.33,9.33 L10,8.71 M13.31,12.75 L12.75,13.31 C12.49,13.57 12.07,13.57 11.81,13.31 L9.73,11.23 L7.94,13 L7,12.06 L7.95,11.11 L2,5.17 L2,2 L5.17,2 L11.11,7.95 L12.06,7 L13,7.94 L11.22,9.72 L13.3,11.8 C13.57,12.07 13.57,12.49 13.31,12.75 Z"

    func path(in rect: CGRect) -> Path {
        let scale = min(rect.width, rect.height) / 16
        var path = Path()
        var command = Character("M")
        var numbers: [CGFloat] = []
        func point(_ i: Int) -> CGPoint {
            CGPoint(x: rect.minX + numbers[i] * scale, y: rect.minY + numbers[i + 1] * scale)
        }
        func flush() {
            switch command {
            case "M" where numbers.count >= 2: path.move(to: point(0))
            case "L" where numbers.count >= 2: path.addLine(to: point(0))
            case "C" where numbers.count >= 6:
                path.addCurve(to: point(4), control1: point(0), control2: point(2))
            case "Z": path.closeSubpath()
            default: break
            }
            numbers = []
        }
        for token in Self.data.split(separator: " ") {
            var text = Substring(token)
            if let first = text.first, first.isLetter {
                flush()
                command = first
                text = text.dropFirst()
            }
            for n in text.split(separator: ",") {
                if let value = Double(n) { numbers.append(CGFloat(value)) }
            }
            if command == "C" && numbers.count == 6 || command != "C" && numbers.count == 2 {
                flush()
            }
        }
        flush()
        return path
    }
}

enum SectionIcon {
    static func name(_ section: String) -> String {
        switch section {
        case "characters": return "person.2"
        case "races": return "figure.stand"
        case "classes": return "shield.lefthalf.filled"
        case "kits": return "backpack"
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
