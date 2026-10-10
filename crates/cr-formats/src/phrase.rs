//! `Database/Phrase.DAT` (`CPhraseSpacingDoc`): a `CObList` of
//! `CPhraseSpacingOb` read with [`ObjectList`](crate::database::ObjectList).

use crate::record;

record! {
    /// Layout positions for one phrase (`CPhraseSpacingOb`, schema 88,
    /// Serialize 0x4ec3e5).
    pub struct PhraseSpacing {
        /// The phrase or layout key (`12`, `Roll Method ...`).
        name: String,
        /// Increasing positions (150, 169, 201, ...).
        spacing: [u32; 100],
        /// 0xFFFFFFFF, 0, or 1000/1010/1011.
        unknown_0x198: u32,
    }
}
