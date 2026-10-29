// Prints a character's sheet: the PDF cr-app draws, which Save as PDF…
// writes too, through the system's print panel.

import AppKit
import PDFKit

struct PDFDocumentPrinter {
    private let document: PDFDocument

    init?(data: Data) {
        guard let document = PDFDocument(data: data) else { return nil }
        self.document = document
    }

    /// Opens the print panel on `window`, on the paper the PDF was drawn
    /// for.
    func print(in window: NSWindow?, letter: Bool) {
        let info = NSPrintInfo.shared.copy() as! NSPrintInfo
        // Points: 8.5 × 11 in, or 210 × 297 mm.
        info.paperSize = letter ? NSSize(width: 612, height: 792) : NSSize(width: 595, height: 842)
        info.topMargin = 0
        info.bottomMargin = 0
        info.leftMargin = 0
        info.rightMargin = 0
        guard
            let operation = document.printOperation(
                for: info, scalingMode: .pageScaleDownToFit, autoRotate: true)
        else { return }
        if let window {
            operation.runModal(for: window, delegate: nil, didRun: nil, contextInfo: nil)
        } else {
            operation.run()
        }
    }
}
