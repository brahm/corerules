//! Traps: `Database/Traps.dat` and `UserDbas/TrapsU.dat` (`CTrapDoc`), a
//! `CObList` of `CTrapOb` read with
//! [`ObjectList`](crate::database::ObjectList).

use crate::record;

record! {
    /// One trap (`CTrapOb`, schema 95, Serialize 0x53cc18): seven strings,
    /// read from member +0x1c down to +0x04.
    pub struct Trap {
        /// `Deadfall`.
        name: String,
        /// `5-30 (5d6)`.
        damage: String,
        /// `none` for Deadfall.
        unknown_0x14: String,
        unknown_0x10: String,
        unknown_0xc: String,
        unknown_0x8: String,
        /// `various` for Deadfall.
        unknown_0x4: String,
    }
}
