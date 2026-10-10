//! Psionic powers: `Database/Psionic.dat` and `UserDbas/PsionicU.dat`
//! (`CPsionicPowerDoc`), a `CObList` of `CPsionicPowerOb` read with
//! [`ObjectList`](crate::database::ObjectList).

use crate::record;

record! {
    /// One psionic power (`CPsionicPowerOb`, schema 96, Serialize 0x4ed36c).
    pub struct PsionicPower {
        /// `Aura Sight`.
        name: String,
        /// Unique per power (2, 3, 4, ...); probably an id or help id.
        unknown_0x24: u32,
        /// 0..4; probably the discipline.
        unknown_0x28: u32,
        /// 0 or 1 (173 ones, 58 zeros); probably devotion vs science.
        unknown_0x2c: u32,
        /// 3..10.
        unknown_0x30: u32,
        /// 0 or 1.
        unknown_0x34: u32,
        /// 0 or 1.
        unknown_0x38: u32,
        /// PSP cost as text (`9/3`, `varies`).
        cost: String,
        /// Often the initial cost (`9/3` gives 9).
        unknown_0x48: u32,
        unknown_0x44: u32,
        /// `50 yards`, `touch`, `0`.
        range: String,
        /// `Personal`, `individual`.
        area: String,
        description: String,
        unknown_0x40: u32,
        unknown_0x1c: u32,
        /// Names of the powers required first (`Mindlink`; up to 3).
        prerequisites: Vec<String>,
    }
}
