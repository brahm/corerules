//! Members of the parts, kits and classes by their offset in the
//! originals' objects, the numbers the dialogs' accessors use (`+0x5d4`).
//! The record types name their members after these offsets; this maps
//! one back to the member, so an editor page can be described by the
//! offsets the original's dialog reads and writes. Written from the
//! `record!` definitions of `cr-formats`: one arm per member, arrays
//! giving one offset per element. Members the serializer stores apart
//! from their neighbours (`flags_0x486e`, `flags_0x27a0`,
//! `unknown_0x2fc8`) list each element's own offset.

#![allow(clippy::match_overlapping_arm)]

use cr_formats::Object;
use cr_formats::classes::ClassData;

use super::Value;
use cr_formats::parts::{ListOfListsOfOptions, PartData, PartHead, PartKitData};

/// One member, read only.
#[derive(Debug, Clone, Copy)]
pub enum Slot<'a> {
    U8(&'a u8),
    I32(&'a i32),
    Str(&'a String),
    List(&'a Vec<String>),
    Objs(&'a Vec<Object>),
    Choose(&'a ListOfListsOfOptions),
}

/// One member, to change.
#[derive(Debug)]
pub enum SlotMut<'a> {
    U8(&'a mut u8),
    I32(&'a mut i32),
    Str(&'a mut String),
    List(&'a mut Vec<String>),
    Objs(&'a mut Vec<Object>),
    Choose(&'a mut ListOfListsOfOptions),
}

impl Slot<'_> {
    /// The member as an editor shows it: a number, a text or a list.
    pub fn value(self) -> Value {
        match self {
            Self::U8(n) => Value::Int((*n).into()),
            Self::I32(n) => Value::Int((*n).into()),
            Self::Str(s) => Value::Text(s.clone()),
            Self::List(l) => Value::List(l.clone()),
            Self::Objs(_) | Self::Choose(_) => Value::List(Vec::new()),
        }
    }
}

impl SlotMut<'_> {
    /// Stores an editor's value; a byte keeps the low eight bits, as the
    /// originals' `(u8)` setters do.
    pub fn set(self, value: &Value) {
        match self {
            Self::U8(n) => *n = value.int() as u8,
            Self::I32(n) => *n = value.int() as i32,
            Self::Str(s) => *s = value.text(),
            Self::List(l) => *l = value.list(),
            Self::Objs(_) | Self::Choose(_) => {}
        }
    }
}

pub fn kit(k: &PartKitData, off: usize) -> Option<Slot<'_>> {
    match off {
        0x4..=0xe => Some(Slot::U8(&k.flags_0x4[off - 0x4])),
        0x10 => Some(Slot::List(&k.unknown_0x10)),
        0x24 => Some(Slot::List(&k.unknown_0x24)),
        0x38 => Some(Slot::List(&k.unknown_0x38)),
        0x4c..=0x70 => Some(Slot::U8(&k.unknown_0x4c[off - 0x4c])),
        0x71..=0x72 => Some(Slot::U8(&k.flags_0x71[off - 0x71])),
        0x73..=0x74 => k.flags_0x73.as_ref().map(|a| Slot::U8(&a[off - 0x73])),
        0x75..=0x7c => Some(Slot::U8(&k.flags_0x75[off - 0x75])),
        0x80 => Some(Slot::I32(&k.unknown_0x80)),
        0x84 => Some(Slot::U8(&k.flag_0x84)),
        0x88..=0x98 if (off - 0x88).is_multiple_of(4) => Some(Slot::I32(&k.unknown_0x88[(off - 0x88) / 4])),
        0x9c..=0xa4 if (off - 0x9c).is_multiple_of(4) => Some(Slot::Str(&k.unknown_0x9c[(off - 0x9c) / 4])),
        0xa8 => Some(Slot::List(&k.unknown_0xa8)),
        0xbc => Some(Slot::List(&k.unknown_0xbc)),
        0xd0 => Some(Slot::List(&k.unknown_0xd0)),
        0xe4 => Some(Slot::List(&k.unknown_0xe4)),
        0xf8 => Some(Slot::List(&k.unknown_0xf8)),
        0x10c => Some(Slot::List(&k.unknown_0x10c)),
        0x120..=0x124 if (off - 0x120).is_multiple_of(4) => k
            .unknown_0x120
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x120) / 4])),
        0x128 => k.unknown_0x128.as_ref().map(Slot::List),
        0x13c => k.unknown_0x13c.as_ref().map(Slot::I32),
        0x140 => k.unknown_0x140.as_ref().map(Slot::List),
        0x154 => k.unknown_0x154.as_ref().map(Slot::I32),
        0x158 => k.unknown_0x158.as_ref().map(Slot::List),
        0x16c..=0x170 if (off - 0x16c).is_multiple_of(4) => k
            .unknown_0x16c
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x16c) / 4])),
        0x174 => k.unknown_0x174.as_ref().map(Slot::Objs),
        0x188..=0x189 => k.flags_0x188.as_ref().map(|a| Slot::U8(&a[off - 0x188])),
        0x18c..=0x19c if (off - 0x18c).is_multiple_of(4) => k
            .unknown_0x18c
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x18c) / 4])),
        0x1a0 => k.unknown_0x1a0.as_ref().map(Slot::List),
        0x1b4 => k.unknown_0x1b4.as_ref().map(Slot::List),
        0x1c8 => k.unknown_0x1c8.as_ref().map(Slot::List),
        0x1dc => k.unknown_0x1dc.as_ref().map(Slot::List),
        0x1f0 => k.flag_0x1f0.as_ref().map(Slot::U8),
        0x1f4..=0x1f8 if (off - 0x1f4).is_multiple_of(4) => k
            .unknown_0x1f4
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x1f4) / 4])),
        0x1fc => k.flag_0x1fc.as_ref().map(Slot::U8),
        0x200..=0x204 if (off - 0x200).is_multiple_of(4) => k
            .unknown_0x200
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x200) / 4])),
        0x208 => k.unknown_0x208.as_ref().map(Slot::List),
        0x21c => k.unknown_0x21c.as_ref().map(Slot::List),
        0x230 => k.unknown_0x230.as_ref().map(Slot::List),
        0x244 => k.unknown_0x244.as_ref().map(Slot::Objs),
        0x258 => Some(Slot::List(&k.unknown_0x258)),
        0x26c => k.unknown_0x26c.as_ref().map(Slot::List),
        0x280 => k.unknown_0x280.as_ref().map(Slot::List),
        0x294 => k.unknown_0x294.as_ref().map(Slot::List),
        0x2a8 => k.unknown_0x2a8.as_ref().map(Slot::List),
        0x2bc => k.unknown_0x2bc.as_ref().map(Slot::List),
        0x2d0 => k.unknown_0x2d0.as_ref().map(Slot::List),
        0x2e4 => k.unknown_0x2e4.as_ref().map(Slot::List),
        0x2f8 => k.unknown_0x2f8.as_ref().map(Slot::Objs),
        0x30c..=0x314 => k.flags_0x30c.as_ref().map(|a| Slot::U8(&a[off - 0x30c])),
        0x318 => k.unknown_0x318.as_ref().map(Slot::I32),
        0x31c => k.unknown_0x31c.as_ref().map(Slot::Str),
        0x320 => k.unknown_0x320.as_ref().map(Slot::I32),
        0x324..=0x328 if (off - 0x324).is_multiple_of(4) => k
            .unknown_0x324
            .as_ref()
            .map(|a| Slot::Str(&a[(off - 0x324) / 4])),
        0x32c => k.unknown_0x32c.as_ref().map(Slot::I32),
        0x330 => k.unknown_0x330.as_ref().map(Slot::Choose),
        0x348 => k.flag_0x348.as_ref().map(Slot::U8),
        0x34c..=0x350 if (off - 0x34c).is_multiple_of(4) => k
            .unknown_0x34c
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x34c) / 4])),
        0x354 => k.unknown_0x354.as_ref().map(Slot::List),
        0x368 => k.unknown_0x368.as_ref().map(Slot::I32),
        0x36c => k.unknown_0x36c.as_ref().map(Slot::List),
        0x380 => k.flag_0x380.as_ref().map(Slot::U8),
        0x384 => Some(Slot::List(&k.unknown_0x384)),
        0x398 => k.unknown_0x398.as_ref().map(Slot::List),
        0x3ac => k.unknown_0x3ac.as_ref().map(Slot::List),
        0x3c0 => k.unknown_0x3c0.as_ref().map(Slot::List),
        0x3d4 => k.unknown_0x3d4.as_ref().map(Slot::List),
        0x3e8 => k.unknown_0x3e8.as_ref().map(Slot::List),
        0x3fc => k.unknown_0x3fc.as_ref().map(Slot::List),
        0x410 => k.unknown_0x410.as_ref().map(Slot::List),
        0x424 => k.unknown_0x424.as_ref().map(Slot::List),
        0x438..=0x439 => k.flags_0x438.as_ref().map(|a| Slot::U8(&a[off - 0x438])),
        0x43c..=0x440 if (off - 0x43c).is_multiple_of(4) => k
            .unknown_0x43c
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x43c) / 4])),
        0x444 => Some(Slot::U8(&k.flag_0x444)),
        0x448 => k.unknown_0x448.as_ref().map(Slot::I32),
        0x44c..=0x44e => k.flags_0x44c.as_ref().map(|a| Slot::U8(&a[off - 0x44c])),
        0x450..=0x460 if (off - 0x450).is_multiple_of(4) => k
            .unknown_0x450
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x450) / 4])),
        0x464..=0x465 => k.flags_0x464.as_ref().map(|a| Slot::U8(&a[off - 0x464])),
        0x468..=0x46c if (off - 0x468).is_multiple_of(4) => k
            .unknown_0x468
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x468) / 4])),
        0x470 => k.unknown_0x470.as_ref().map(Slot::Objs),
        0x484 => k.unknown_0x484.as_ref().map(Slot::List),
        0x498 => k.unknown_0x498.as_ref().map(Slot::List),
        0x4ac..=0x4b3 => k.flags_0x4ac.as_ref().map(|a| Slot::U8(&a[off - 0x4ac])),
        0x4b4 => k.flag_0x4b4.as_ref().map(Slot::U8),
        0x4b8 => k.unknown_0x4b8.as_ref().map(Slot::List),
        0x4cc => k.unknown_0x4cc.as_ref().map(Slot::List),
        0x4e0 => k.unknown_0x4e0.as_ref().map(Slot::List),
        0x4f4 => k.unknown_0x4f4.as_ref().map(Slot::List),
        0x508 => k.flag_0x508.as_ref().map(Slot::U8),
        0x50c..=0x514 if (off - 0x50c).is_multiple_of(4) => k
            .unknown_0x50c
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x50c) / 4])),
        0x518 => k.flag_0x518.as_ref().map(Slot::U8),
        0x51c => k.unknown_0x51c.as_ref().map(Slot::I32),
        0x520..=0x521 => k.flags_0x520.as_ref().map(|a| Slot::U8(&a[off - 0x520])),
        0x524 => k.unknown_0x524.as_ref().map(Slot::List),
        0x538 => k.flag_0x538.as_ref().map(Slot::U8),
        0x53c => k.unknown_0x53c.as_ref().map(Slot::List),
        0x550 => k.unknown_0x550.as_ref().map(Slot::List),
        0x564 => k.unknown_0x564.as_ref().map(Slot::I32),
        0x568 => k.unknown_0x568.as_ref().map(Slot::Choose),
        0x580 => k.flag_0x580.as_ref().map(Slot::U8),
        0x584 => k.starting_equipment.as_ref().map(Slot::List),
        0x598 => k.unknown_0x598.as_ref().map(Slot::List),
        0x5ac => k.unknown_0x5ac.as_ref().map(Slot::List),
        0x5c0 => k.unknown_0x5c0.as_ref().map(Slot::List),
        0x5d4..=0x5d9 => k.flags_0x5d4.as_ref().map(|a| Slot::U8(&a[off - 0x5d4])),
        0x5da => k.flag_0x5da.as_ref().map(Slot::U8),
        0x5dc => k.flag_0x5dc.as_ref().map(Slot::U8),
        0x5dd..=0x5de => k.flags_0x5dd.as_ref().map(|a| Slot::U8(&a[off - 0x5dd])),
        0x5df => k.flag_0x5df.as_ref().map(Slot::U8),
        0x5e0..=0x5f8 if (off - 0x5e0).is_multiple_of(4) => k
            .unknown_0x5e0
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x5e0) / 4])),
        0x5fc..=0x600 if (off - 0x5fc).is_multiple_of(4) => k
            .unknown_0x5fc
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x5fc) / 4])),
        0x604 => k.unknown_0x604.as_ref().map(Slot::I32),
        0x608..=0x614 if (off - 0x608).is_multiple_of(4) => k
            .unknown_0x608
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x608) / 4])),
        0x618 => k.flag_0x618.as_ref().map(Slot::U8),
        0x61c => k.unknown_0x61c.as_ref().map(Slot::Str),
        0x620 => k.unknown_0x620.as_ref().map(Slot::List),
        0x634 => k.bonus_spells.as_ref().map(Slot::Objs),
        0x648 => k.levels.as_ref().map(Slot::Objs),
        0x65c => k.allowed_classes.as_ref().map(Slot::Objs),
        0x670 => k.unknown_0x670.as_ref().map(Slot::I32),
        0x674 => k.unknown_0x674.as_ref().map(Slot::Str),
        _ => None,
    }
}

pub fn kit_mut(k: &mut PartKitData, off: usize) -> Option<SlotMut<'_>> {
    match off {
        0x4..=0xe => Some(SlotMut::U8(&mut k.flags_0x4[off - 0x4])),
        0x10 => Some(SlotMut::List(&mut k.unknown_0x10)),
        0x24 => Some(SlotMut::List(&mut k.unknown_0x24)),
        0x38 => Some(SlotMut::List(&mut k.unknown_0x38)),
        0x4c..=0x70 => Some(SlotMut::U8(&mut k.unknown_0x4c[off - 0x4c])),
        0x71..=0x72 => Some(SlotMut::U8(&mut k.flags_0x71[off - 0x71])),
        0x73..=0x74 => k
            .flags_0x73
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x73])),
        0x75..=0x7c => Some(SlotMut::U8(&mut k.flags_0x75[off - 0x75])),
        0x80 => Some(SlotMut::I32(&mut k.unknown_0x80)),
        0x84 => Some(SlotMut::U8(&mut k.flag_0x84)),
        0x88..=0x98 if (off - 0x88).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut k.unknown_0x88[(off - 0x88) / 4]))
        }
        0x9c..=0xa4 if (off - 0x9c).is_multiple_of(4) => {
            Some(SlotMut::Str(&mut k.unknown_0x9c[(off - 0x9c) / 4]))
        }
        0xa8 => Some(SlotMut::List(&mut k.unknown_0xa8)),
        0xbc => Some(SlotMut::List(&mut k.unknown_0xbc)),
        0xd0 => Some(SlotMut::List(&mut k.unknown_0xd0)),
        0xe4 => Some(SlotMut::List(&mut k.unknown_0xe4)),
        0xf8 => Some(SlotMut::List(&mut k.unknown_0xf8)),
        0x10c => Some(SlotMut::List(&mut k.unknown_0x10c)),
        0x120..=0x124 if (off - 0x120).is_multiple_of(4) => k
            .unknown_0x120
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x120) / 4])),
        0x128 => k.unknown_0x128.as_mut().map(SlotMut::List),
        0x13c => k.unknown_0x13c.as_mut().map(SlotMut::I32),
        0x140 => k.unknown_0x140.as_mut().map(SlotMut::List),
        0x154 => k.unknown_0x154.as_mut().map(SlotMut::I32),
        0x158 => k.unknown_0x158.as_mut().map(SlotMut::List),
        0x16c..=0x170 if (off - 0x16c).is_multiple_of(4) => k
            .unknown_0x16c
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x16c) / 4])),
        0x174 => k.unknown_0x174.as_mut().map(SlotMut::Objs),
        0x188..=0x189 => k
            .flags_0x188
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x188])),
        0x18c..=0x19c if (off - 0x18c).is_multiple_of(4) => k
            .unknown_0x18c
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x18c) / 4])),
        0x1a0 => k.unknown_0x1a0.as_mut().map(SlotMut::List),
        0x1b4 => k.unknown_0x1b4.as_mut().map(SlotMut::List),
        0x1c8 => k.unknown_0x1c8.as_mut().map(SlotMut::List),
        0x1dc => k.unknown_0x1dc.as_mut().map(SlotMut::List),
        0x1f0 => k.flag_0x1f0.as_mut().map(SlotMut::U8),
        0x1f4..=0x1f8 if (off - 0x1f4).is_multiple_of(4) => k
            .unknown_0x1f4
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x1f4) / 4])),
        0x1fc => k.flag_0x1fc.as_mut().map(SlotMut::U8),
        0x200..=0x204 if (off - 0x200).is_multiple_of(4) => k
            .unknown_0x200
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x200) / 4])),
        0x208 => k.unknown_0x208.as_mut().map(SlotMut::List),
        0x21c => k.unknown_0x21c.as_mut().map(SlotMut::List),
        0x230 => k.unknown_0x230.as_mut().map(SlotMut::List),
        0x244 => k.unknown_0x244.as_mut().map(SlotMut::Objs),
        0x258 => Some(SlotMut::List(&mut k.unknown_0x258)),
        0x26c => k.unknown_0x26c.as_mut().map(SlotMut::List),
        0x280 => k.unknown_0x280.as_mut().map(SlotMut::List),
        0x294 => k.unknown_0x294.as_mut().map(SlotMut::List),
        0x2a8 => k.unknown_0x2a8.as_mut().map(SlotMut::List),
        0x2bc => k.unknown_0x2bc.as_mut().map(SlotMut::List),
        0x2d0 => k.unknown_0x2d0.as_mut().map(SlotMut::List),
        0x2e4 => k.unknown_0x2e4.as_mut().map(SlotMut::List),
        0x2f8 => k.unknown_0x2f8.as_mut().map(SlotMut::Objs),
        0x30c..=0x314 => k
            .flags_0x30c
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x30c])),
        0x318 => k.unknown_0x318.as_mut().map(SlotMut::I32),
        0x31c => k.unknown_0x31c.as_mut().map(SlotMut::Str),
        0x320 => k.unknown_0x320.as_mut().map(SlotMut::I32),
        0x324..=0x328 if (off - 0x324).is_multiple_of(4) => k
            .unknown_0x324
            .as_mut()
            .map(|a| SlotMut::Str(&mut a[(off - 0x324) / 4])),
        0x32c => k.unknown_0x32c.as_mut().map(SlotMut::I32),
        0x330 => k.unknown_0x330.as_mut().map(SlotMut::Choose),
        0x348 => k.flag_0x348.as_mut().map(SlotMut::U8),
        0x34c..=0x350 if (off - 0x34c).is_multiple_of(4) => k
            .unknown_0x34c
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x34c) / 4])),
        0x354 => k.unknown_0x354.as_mut().map(SlotMut::List),
        0x368 => k.unknown_0x368.as_mut().map(SlotMut::I32),
        0x36c => k.unknown_0x36c.as_mut().map(SlotMut::List),
        0x380 => k.flag_0x380.as_mut().map(SlotMut::U8),
        0x384 => Some(SlotMut::List(&mut k.unknown_0x384)),
        0x398 => k.unknown_0x398.as_mut().map(SlotMut::List),
        0x3ac => k.unknown_0x3ac.as_mut().map(SlotMut::List),
        0x3c0 => k.unknown_0x3c0.as_mut().map(SlotMut::List),
        0x3d4 => k.unknown_0x3d4.as_mut().map(SlotMut::List),
        0x3e8 => k.unknown_0x3e8.as_mut().map(SlotMut::List),
        0x3fc => k.unknown_0x3fc.as_mut().map(SlotMut::List),
        0x410 => k.unknown_0x410.as_mut().map(SlotMut::List),
        0x424 => k.unknown_0x424.as_mut().map(SlotMut::List),
        0x438..=0x439 => k
            .flags_0x438
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x438])),
        0x43c..=0x440 if (off - 0x43c).is_multiple_of(4) => k
            .unknown_0x43c
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x43c) / 4])),
        0x444 => Some(SlotMut::U8(&mut k.flag_0x444)),
        0x448 => k.unknown_0x448.as_mut().map(SlotMut::I32),
        0x44c..=0x44e => k
            .flags_0x44c
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x44c])),
        0x450..=0x460 if (off - 0x450).is_multiple_of(4) => k
            .unknown_0x450
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x450) / 4])),
        0x464..=0x465 => k
            .flags_0x464
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x464])),
        0x468..=0x46c if (off - 0x468).is_multiple_of(4) => k
            .unknown_0x468
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x468) / 4])),
        0x470 => k.unknown_0x470.as_mut().map(SlotMut::Objs),
        0x484 => k.unknown_0x484.as_mut().map(SlotMut::List),
        0x498 => k.unknown_0x498.as_mut().map(SlotMut::List),
        0x4ac..=0x4b3 => k
            .flags_0x4ac
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x4ac])),
        0x4b4 => k.flag_0x4b4.as_mut().map(SlotMut::U8),
        0x4b8 => k.unknown_0x4b8.as_mut().map(SlotMut::List),
        0x4cc => k.unknown_0x4cc.as_mut().map(SlotMut::List),
        0x4e0 => k.unknown_0x4e0.as_mut().map(SlotMut::List),
        0x4f4 => k.unknown_0x4f4.as_mut().map(SlotMut::List),
        0x508 => k.flag_0x508.as_mut().map(SlotMut::U8),
        0x50c..=0x514 if (off - 0x50c).is_multiple_of(4) => k
            .unknown_0x50c
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x50c) / 4])),
        0x518 => k.flag_0x518.as_mut().map(SlotMut::U8),
        0x51c => k.unknown_0x51c.as_mut().map(SlotMut::I32),
        0x520..=0x521 => k
            .flags_0x520
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x520])),
        0x524 => k.unknown_0x524.as_mut().map(SlotMut::List),
        0x538 => k.flag_0x538.as_mut().map(SlotMut::U8),
        0x53c => k.unknown_0x53c.as_mut().map(SlotMut::List),
        0x550 => k.unknown_0x550.as_mut().map(SlotMut::List),
        0x564 => k.unknown_0x564.as_mut().map(SlotMut::I32),
        0x568 => k.unknown_0x568.as_mut().map(SlotMut::Choose),
        0x580 => k.flag_0x580.as_mut().map(SlotMut::U8),
        0x584 => k.starting_equipment.as_mut().map(SlotMut::List),
        0x598 => k.unknown_0x598.as_mut().map(SlotMut::List),
        0x5ac => k.unknown_0x5ac.as_mut().map(SlotMut::List),
        0x5c0 => k.unknown_0x5c0.as_mut().map(SlotMut::List),
        0x5d4..=0x5d9 => k
            .flags_0x5d4
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x5d4])),
        0x5da => k.flag_0x5da.as_mut().map(SlotMut::U8),
        0x5dc => k.flag_0x5dc.as_mut().map(SlotMut::U8),
        0x5dd..=0x5de => k
            .flags_0x5dd
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x5dd])),
        0x5df => k.flag_0x5df.as_mut().map(SlotMut::U8),
        0x5e0..=0x5f8 if (off - 0x5e0).is_multiple_of(4) => k
            .unknown_0x5e0
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x5e0) / 4])),
        0x5fc..=0x600 if (off - 0x5fc).is_multiple_of(4) => k
            .unknown_0x5fc
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x5fc) / 4])),
        0x604 => k.unknown_0x604.as_mut().map(SlotMut::I32),
        0x608..=0x614 if (off - 0x608).is_multiple_of(4) => k
            .unknown_0x608
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x608) / 4])),
        0x618 => k.flag_0x618.as_mut().map(SlotMut::U8),
        0x61c => k.unknown_0x61c.as_mut().map(SlotMut::Str),
        0x620 => k.unknown_0x620.as_mut().map(SlotMut::List),
        0x634 => k.bonus_spells.as_mut().map(SlotMut::Objs),
        0x648 => k.levels.as_mut().map(SlotMut::Objs),
        0x65c => k.allowed_classes.as_mut().map(SlotMut::Objs),
        0x670 => k.unknown_0x670.as_mut().map(SlotMut::I32),
        0x674 => k.unknown_0x674.as_mut().map(SlotMut::Str),
        _ => None,
    }
}
pub fn head(h: &PartHead, off: usize) -> Option<Slot<'_>> {
    match off {
        0x4 => Some(Slot::Str(&h.description)),
        0x198 => Some(Slot::Str(&h.name)),
        0x1b4 => Some(Slot::Str(&h.unknown_0x1b4)),
        0x1e4 => Some(Slot::I32(&h.unknown_0x1e4)),
        0x1e8 => Some(Slot::Str(&h.unknown_0x1e8)),
        0x1ec => Some(Slot::I32(&h.unknown_0x1ec)),
        0x1f0 => Some(Slot::I32(&h.unknown_0x1f0)),
        0x1f8..=0x214 if (off - 0x1f8).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x1f8[(off - 0x1f8) / 4]))
        }
        0x224 => Some(Slot::I32(&h.unknown_0x224)),
        0x230 => Some(Slot::I32(&h.unknown_0x230)),
        0x234 => Some(Slot::Str(&h.unknown_0x234)),
        0x238..=0x23c if (off - 0x238).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x238[(off - 0x238) / 4]))
        }
        0x240 => Some(Slot::Str(&h.unknown_0x240)),
        0x244 => Some(Slot::Str(&h.unknown_0x244)),
        0x248 => Some(Slot::I32(&h.unknown_0x248)),
        0x24c => Some(Slot::I32(&h.unknown_0x24c)),
        0x250..=0x274 if (off - 0x250).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x250[(off - 0x250) / 4]))
        }
        0x278 => Some(Slot::I32(&h.unknown_0x278)),
        0x27c..=0x2a4 if (off - 0x27c).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x27c[(off - 0x27c) / 4]))
        }
        0x2a8 => Some(Slot::I32(&h.unknown_0x2a8)),
        0x2ac => Some(Slot::I32(&h.unknown_0x2ac)),
        0x2b4..=0x2d4 if (off - 0x2b4).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x2b4[(off - 0x2b4) / 4]))
        }
        0x2d8..=0x2fc if (off - 0x2d8).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x2d8[(off - 0x2d8) / 4]))
        }
        0x300 => Some(Slot::Str(&h.unknown_0x300)),
        0x308 => Some(Slot::I32(&h.unknown_0x308)),
        0x30c..=0x354 if (off - 0x30c).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x30c[(off - 0x30c) / 4]))
        }
        0x394..=0x39c if (off - 0x394).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x394[(off - 0x394) / 4]))
        }
        0x3b4 => Some(Slot::Str(&h.unknown_0x3b4)),
        0x3b8..=0x3c4 if (off - 0x3b8).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x3b8[(off - 0x3b8) / 4]))
        }
        0x3c8 => Some(Slot::I32(&h.unknown_0x3c8)),
        0x3cc => Some(Slot::I32(&h.unknown_0x3cc)),
        0x3d0 => Some(Slot::Str(&h.unknown_0x3d0)),
        0x3d4 => Some(Slot::I32(&h.unknown_0x3d4)),
        0x3d8 => Some(Slot::I32(&h.unknown_0x3d8)),
        0x3dc => Some(Slot::I32(&h.unknown_0x3dc)),
        0x3e0 => Some(Slot::I32(&h.unknown_0x3e0)),
        0x3e4 => Some(Slot::Str(&h.unknown_0x3e4)),
        0x3e8 => Some(Slot::I32(&h.unknown_0x3e8)),
        0x3ec => Some(Slot::I32(&h.unknown_0x3ec)),
        0x3f0 => Some(Slot::I32(&h.unknown_0x3f0)),
        0x3f4 => Some(Slot::Str(&h.unknown_0x3f4)),
        0x3f8 => Some(Slot::I32(&h.unknown_0x3f8)),
        0x3fc => Some(Slot::I32(&h.unknown_0x3fc)),
        0x400 => Some(Slot::I32(&h.unknown_0x400)),
        0x404 => Some(Slot::I32(&h.unknown_0x404)),
        0x408 => Some(Slot::Str(&h.unknown_0x408)),
        0x40c..=0x418 if (off - 0x40c).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x40c[(off - 0x40c) / 4]))
        }
        0x41c => Some(Slot::Str(&h.unknown_0x41c)),
        0x44c..=0x454 if (off - 0x44c).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x44c[(off - 0x44c) / 4]))
        }
        0x458 => Some(Slot::Str(&h.unknown_0x458)),
        0x45c => Some(Slot::Str(&h.unknown_0x45c)),
        0x460 => Some(Slot::Str(&h.unknown_0x460)),
        0x464 => Some(Slot::Str(&h.unknown_0x464)),
        0x470..=0x47c if (off - 0x470).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x470[(off - 0x470) / 4]))
        }
        0x4cc => Some(Slot::I32(&h.unknown_0x4cc)),
        0x80c..=0x818 if (off - 0x80c).is_multiple_of(4) => {
            Some(Slot::I32(&h.unknown_0x80c[(off - 0x80c) / 4]))
        }
        _ => None,
    }
}

pub fn head_mut(h: &mut PartHead, off: usize) -> Option<SlotMut<'_>> {
    match off {
        0x4 => Some(SlotMut::Str(&mut h.description)),
        0x198 => Some(SlotMut::Str(&mut h.name)),
        0x1b4 => Some(SlotMut::Str(&mut h.unknown_0x1b4)),
        0x1e4 => Some(SlotMut::I32(&mut h.unknown_0x1e4)),
        0x1e8 => Some(SlotMut::Str(&mut h.unknown_0x1e8)),
        0x1ec => Some(SlotMut::I32(&mut h.unknown_0x1ec)),
        0x1f0 => Some(SlotMut::I32(&mut h.unknown_0x1f0)),
        0x1f8..=0x214 if (off - 0x1f8).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x1f8[(off - 0x1f8) / 4]))
        }
        0x224 => Some(SlotMut::I32(&mut h.unknown_0x224)),
        0x230 => Some(SlotMut::I32(&mut h.unknown_0x230)),
        0x234 => Some(SlotMut::Str(&mut h.unknown_0x234)),
        0x238..=0x23c if (off - 0x238).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x238[(off - 0x238) / 4]))
        }
        0x240 => Some(SlotMut::Str(&mut h.unknown_0x240)),
        0x244 => Some(SlotMut::Str(&mut h.unknown_0x244)),
        0x248 => Some(SlotMut::I32(&mut h.unknown_0x248)),
        0x24c => Some(SlotMut::I32(&mut h.unknown_0x24c)),
        0x250..=0x274 if (off - 0x250).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x250[(off - 0x250) / 4]))
        }
        0x278 => Some(SlotMut::I32(&mut h.unknown_0x278)),
        0x27c..=0x2a4 if (off - 0x27c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x27c[(off - 0x27c) / 4]))
        }
        0x2a8 => Some(SlotMut::I32(&mut h.unknown_0x2a8)),
        0x2ac => Some(SlotMut::I32(&mut h.unknown_0x2ac)),
        0x2b4..=0x2d4 if (off - 0x2b4).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x2b4[(off - 0x2b4) / 4]))
        }
        0x2d8..=0x2fc if (off - 0x2d8).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x2d8[(off - 0x2d8) / 4]))
        }
        0x300 => Some(SlotMut::Str(&mut h.unknown_0x300)),
        0x308 => Some(SlotMut::I32(&mut h.unknown_0x308)),
        0x30c..=0x354 if (off - 0x30c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x30c[(off - 0x30c) / 4]))
        }
        0x394..=0x39c if (off - 0x394).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x394[(off - 0x394) / 4]))
        }
        0x3b4 => Some(SlotMut::Str(&mut h.unknown_0x3b4)),
        0x3b8..=0x3c4 if (off - 0x3b8).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x3b8[(off - 0x3b8) / 4]))
        }
        0x3c8 => Some(SlotMut::I32(&mut h.unknown_0x3c8)),
        0x3cc => Some(SlotMut::I32(&mut h.unknown_0x3cc)),
        0x3d0 => Some(SlotMut::Str(&mut h.unknown_0x3d0)),
        0x3d4 => Some(SlotMut::I32(&mut h.unknown_0x3d4)),
        0x3d8 => Some(SlotMut::I32(&mut h.unknown_0x3d8)),
        0x3dc => Some(SlotMut::I32(&mut h.unknown_0x3dc)),
        0x3e0 => Some(SlotMut::I32(&mut h.unknown_0x3e0)),
        0x3e4 => Some(SlotMut::Str(&mut h.unknown_0x3e4)),
        0x3e8 => Some(SlotMut::I32(&mut h.unknown_0x3e8)),
        0x3ec => Some(SlotMut::I32(&mut h.unknown_0x3ec)),
        0x3f0 => Some(SlotMut::I32(&mut h.unknown_0x3f0)),
        0x3f4 => Some(SlotMut::Str(&mut h.unknown_0x3f4)),
        0x3f8 => Some(SlotMut::I32(&mut h.unknown_0x3f8)),
        0x3fc => Some(SlotMut::I32(&mut h.unknown_0x3fc)),
        0x400 => Some(SlotMut::I32(&mut h.unknown_0x400)),
        0x404 => Some(SlotMut::I32(&mut h.unknown_0x404)),
        0x408 => Some(SlotMut::Str(&mut h.unknown_0x408)),
        0x40c..=0x418 if (off - 0x40c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x40c[(off - 0x40c) / 4]))
        }
        0x41c => Some(SlotMut::Str(&mut h.unknown_0x41c)),
        0x44c..=0x454 if (off - 0x44c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x44c[(off - 0x44c) / 4]))
        }
        0x458 => Some(SlotMut::Str(&mut h.unknown_0x458)),
        0x45c => Some(SlotMut::Str(&mut h.unknown_0x45c)),
        0x460 => Some(SlotMut::Str(&mut h.unknown_0x460)),
        0x464 => Some(SlotMut::Str(&mut h.unknown_0x464)),
        0x470..=0x47c if (off - 0x470).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x470[(off - 0x470) / 4]))
        }
        0x4cc => Some(SlotMut::I32(&mut h.unknown_0x4cc)),
        0x80c..=0x818 if (off - 0x80c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut h.unknown_0x80c[(off - 0x80c) / 4]))
        }
        _ => None,
    }
}
pub fn data(d: &PartData, off: usize) -> Option<Slot<'_>> {
    match off {
        0x4..=0x194 if (off - 0x4).is_multiple_of(4) => d
            .unknown_0x4
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x4) / 4])),
        0x19c => Some(Slot::I32(&d.unknown_0x19c)),
        0x1a0 => Some(Slot::List(&d.unknown_0x1a0)),
        0x1b8 => Some(Slot::List(&d.unknown_0x1b8)),
        0x1cc => Some(Slot::List(&d.unknown_0x1cc)),
        0x1e0 => Some(Slot::Str(&d.unknown_0x1e0)),
        0x1f4 => Some(Slot::I32(&d.unknown_0x1f4)),
        0x218 => Some(Slot::I32(&d.unknown_0x218)),
        0x21c => Some(Slot::Str(&d.unknown_0x21c)),
        0x220 => Some(Slot::I32(&d.unknown_0x220)),
        0x228 => Some(Slot::I32(&d.unknown_0x228)),
        0x22c => Some(Slot::I32(&d.unknown_0x22c)),
        0x2b0 => Some(Slot::I32(&d.unknown_0x2b0)),
        0x358 => Some(Slot::List(&d.unknown_0x358)),
        0x36c => Some(Slot::List(&d.unknown_0x36c)),
        0x380 => Some(Slot::List(&d.unknown_0x380)),
        0x3a0 => Some(Slot::List(&d.unknown_0x3a0)),
        0x420..=0x424 if (off - 0x420).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x420[(off - 0x420) / 4]))
        }
        0x428 => Some(Slot::Str(&d.unknown_0x428)),
        0x42c => Some(Slot::I32(&d.unknown_0x42c)),
        0x430 => Some(Slot::Str(&d.unknown_0x430)),
        0x434 => Some(Slot::List(&d.unknown_0x434)),
        0x448 => d.flag_0x448.as_ref().map(Slot::U8),
        0x468 => Some(Slot::Str(&d.unknown_0x468)),
        0x46c => Some(Slot::Str(&d.unknown_0x46c)),
        0x480..=0x488 if (off - 0x480).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x480[(off - 0x480) / 4]))
        }
        0x48c..=0x4a0 if (off - 0x48c).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x48c[(off - 0x48c) / 4]))
        }
        0x4a4..=0x4c4 if (off - 0x4a4).is_multiple_of(4) => {
            Some(Slot::I32(&d.old_class_flags[(off - 0x4a4) / 4]))
        }
        0x4c8 => Some(Slot::I32(&d.unknown_0x4c8)),
        0x4d0 => d.flag_0x4d0.as_ref().map(Slot::U8),
        0x4e4 => Some(Slot::I32(&d.unknown_0x4e4)),
        0x4f0 => Some(Slot::I32(&d.unknown_0x4f0)),
        0x4fc => Some(Slot::List(&d.unknown_0x4fc)),
        0x510..=0x528 if (off - 0x510).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x510[(off - 0x510) / 4]))
        }
        0x52c => d.unknown_0x52c.as_ref().map(Slot::I32),
        0x530..=0x56c if (off - 0x530).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x530[(off - 0x530) / 4]))
        }
        0x570 => Some(Slot::I32(&d.unknown_0x570)),
        0x574..=0x57c if (off - 0x574).is_multiple_of(4) => {
            Some(Slot::Str(&d.unknown_0x574[(off - 0x574) / 4]))
        }
        0x580 => Some(Slot::Str(&d.unknown_0x580)),
        0x584..=0x594 if (off - 0x584).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x584[(off - 0x584) / 4]))
        }
        0x598 => Some(Slot::I32(&d.unknown_0x598)),
        0x59c..=0x5cc if (off - 0x59c).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x59c[(off - 0x59c) / 4]))
        }
        0x5d0 => Some(Slot::List(&d.unknown_0x5d0)),
        0x5e4..=0x640 if (off - 0x5e4).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x5e4[(off - 0x5e4) / 4]))
        }
        0x644 => Some(Slot::I32(&d.unknown_0x644)),
        0x648..=0x64c if (off - 0x648).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x648[(off - 0x648) / 4]))
        }
        0x650 => Some(Slot::I32(&d.unknown_0x650)),
        0x654 => Some(Slot::I32(&d.unknown_0x654)),
        0x658 => Some(Slot::I32(&d.unknown_0x658)),
        0x65c => Some(Slot::I32(&d.unknown_0x65c)),
        0x660 => Some(Slot::I32(&d.unknown_0x660)),
        0x664..=0x668 if (off - 0x664).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x664[(off - 0x664) / 4]))
        }
        0x66c..=0x674 if (off - 0x66c).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x66c[(off - 0x66c) / 4]))
        }
        0x678 => Some(Slot::I32(&d.unknown_0x678)),
        0x67c => Some(Slot::U8(&d.flag_0x67c)),
        0x67d => Some(Slot::U8(&d.flag_0x67d)),
        0x67e => Some(Slot::U8(&d.flag_0x67e)),
        0x67f => Some(Slot::U8(&d.flag_0x67f)),
        0x680..=0x681 => Some(Slot::U8(&d.flags_0x680[off - 0x680])),
        0x684 => Some(Slot::List(&d.unknown_0x684)),
        0x698 => Some(Slot::List(&d.unknown_0x698)),
        0x6ac => Some(Slot::List(&d.unknown_0x6ac)),
        0x6c0 => Some(Slot::List(&d.unknown_0x6c0)),
        0x6d4 => Some(Slot::List(&d.unknown_0x6d4)),
        0x6e8 => Some(Slot::List(&d.unknown_0x6e8)),
        0x6fc => Some(Slot::List(&d.unknown_0x6fc)),
        0x710..=0x714 if (off - 0x710).is_multiple_of(4) => {
            Some(Slot::Str(&d.unknown_0x710[(off - 0x710) / 4]))
        }
        0x718 => Some(Slot::Str(&d.unknown_0x718)),
        0x71c => Some(Slot::I32(&d.unknown_0x71c)),
        0x720 => Some(Slot::Str(&d.unknown_0x720)),
        0x724 => d.flag_0x724.as_ref().map(Slot::U8),
        0x72c..=0x740 if (off - 0x72c).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x72c[(off - 0x72c) / 4]))
        }
        0x744 => Some(Slot::I32(&d.unknown_0x744)),
        0x748 => Some(Slot::Objs(&d.unknown_0x748)),
        0x75c..=0x760 if (off - 0x75c).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x75c[(off - 0x75c) / 4]))
        }
        0x764 => Some(Slot::List(&d.unknown_0x764)),
        0x778..=0x77c if (off - 0x778).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x778[(off - 0x778) / 4]))
        }
        0x780..=0x785 => Some(Slot::U8(&d.flags_0x780[off - 0x780])),
        0x788 => Some(Slot::I32(&d.unknown_0x788)),
        0x78c => Some(Slot::I32(&d.unknown_0x78c)),
        0x790 => Some(Slot::I32(&d.unknown_0x790)),
        0x794 => Some(Slot::I32(&d.unknown_0x794)),
        0x798 => Some(Slot::I32(&d.unknown_0x798)),
        0x79c => Some(Slot::I32(&d.unknown_0x79c)),
        0x7a0 => Some(Slot::I32(&d.unknown_0x7a0)),
        0x7a4..=0x7b0 if (off - 0x7a4).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x7a4[(off - 0x7a4) / 4]))
        }
        0x7b4 => Some(Slot::I32(&d.unknown_0x7b4)),
        0x7b8 => Some(Slot::I32(&d.unknown_0x7b8)),
        0x7bc => Some(Slot::I32(&d.unknown_0x7bc)),
        0x7c0..=0x7c8 if (off - 0x7c0).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x7c0[(off - 0x7c0) / 4]))
        }
        0x7cc..=0x7e4 if (off - 0x7cc).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x7cc[(off - 0x7cc) / 4]))
        }
        0x7e8..=0x808 if (off - 0x7e8).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x7e8[(off - 0x7e8) / 4]))
        }
        0x81c..=0x828 if (off - 0x81c).is_multiple_of(4) => {
            Some(Slot::I32(&d.unknown_0x81c[(off - 0x81c) / 4]))
        }
        0x82c => d.allowed_classes.as_ref().map(Slot::List),
        0x840 => Some(Slot::List(&d.unknown_0x840)),
        0x854 => Some(Slot::List(&d.unknown_0x854)),
        0x868..=0x88c if (off - 0x868).is_multiple_of(4) => {
            Some(Slot::Str(&d.unknown_0x868[(off - 0x868) / 4]))
        }
        _ => None,
    }
}

pub fn data_mut(d: &mut PartData, off: usize) -> Option<SlotMut<'_>> {
    match off {
        0x4..=0x194 if (off - 0x4).is_multiple_of(4) => d
            .unknown_0x4
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x4) / 4])),
        0x19c => Some(SlotMut::I32(&mut d.unknown_0x19c)),
        0x1a0 => Some(SlotMut::List(&mut d.unknown_0x1a0)),
        0x1b8 => Some(SlotMut::List(&mut d.unknown_0x1b8)),
        0x1cc => Some(SlotMut::List(&mut d.unknown_0x1cc)),
        0x1e0 => Some(SlotMut::Str(&mut d.unknown_0x1e0)),
        0x1f4 => Some(SlotMut::I32(&mut d.unknown_0x1f4)),
        0x218 => Some(SlotMut::I32(&mut d.unknown_0x218)),
        0x21c => Some(SlotMut::Str(&mut d.unknown_0x21c)),
        0x220 => Some(SlotMut::I32(&mut d.unknown_0x220)),
        0x228 => Some(SlotMut::I32(&mut d.unknown_0x228)),
        0x22c => Some(SlotMut::I32(&mut d.unknown_0x22c)),
        0x2b0 => Some(SlotMut::I32(&mut d.unknown_0x2b0)),
        0x358 => Some(SlotMut::List(&mut d.unknown_0x358)),
        0x36c => Some(SlotMut::List(&mut d.unknown_0x36c)),
        0x380 => Some(SlotMut::List(&mut d.unknown_0x380)),
        0x3a0 => Some(SlotMut::List(&mut d.unknown_0x3a0)),
        0x420..=0x424 if (off - 0x420).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x420[(off - 0x420) / 4]))
        }
        0x428 => Some(SlotMut::Str(&mut d.unknown_0x428)),
        0x42c => Some(SlotMut::I32(&mut d.unknown_0x42c)),
        0x430 => Some(SlotMut::Str(&mut d.unknown_0x430)),
        0x434 => Some(SlotMut::List(&mut d.unknown_0x434)),
        0x448 => d.flag_0x448.as_mut().map(SlotMut::U8),
        0x468 => Some(SlotMut::Str(&mut d.unknown_0x468)),
        0x46c => Some(SlotMut::Str(&mut d.unknown_0x46c)),
        0x480..=0x488 if (off - 0x480).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x480[(off - 0x480) / 4]))
        }
        0x48c..=0x4a0 if (off - 0x48c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x48c[(off - 0x48c) / 4]))
        }
        0x4a4..=0x4c4 if (off - 0x4a4).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.old_class_flags[(off - 0x4a4) / 4]))
        }
        0x4c8 => Some(SlotMut::I32(&mut d.unknown_0x4c8)),
        0x4d0 => d.flag_0x4d0.as_mut().map(SlotMut::U8),
        0x4e4 => Some(SlotMut::I32(&mut d.unknown_0x4e4)),
        0x4f0 => Some(SlotMut::I32(&mut d.unknown_0x4f0)),
        0x4fc => Some(SlotMut::List(&mut d.unknown_0x4fc)),
        0x510..=0x528 if (off - 0x510).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x510[(off - 0x510) / 4]))
        }
        0x52c => d.unknown_0x52c.as_mut().map(SlotMut::I32),
        0x530..=0x56c if (off - 0x530).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x530[(off - 0x530) / 4]))
        }
        0x570 => Some(SlotMut::I32(&mut d.unknown_0x570)),
        0x574..=0x57c if (off - 0x574).is_multiple_of(4) => {
            Some(SlotMut::Str(&mut d.unknown_0x574[(off - 0x574) / 4]))
        }
        0x580 => Some(SlotMut::Str(&mut d.unknown_0x580)),
        0x584..=0x594 if (off - 0x584).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x584[(off - 0x584) / 4]))
        }
        0x598 => Some(SlotMut::I32(&mut d.unknown_0x598)),
        0x59c..=0x5cc if (off - 0x59c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x59c[(off - 0x59c) / 4]))
        }
        0x5d0 => Some(SlotMut::List(&mut d.unknown_0x5d0)),
        0x5e4..=0x640 if (off - 0x5e4).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x5e4[(off - 0x5e4) / 4]))
        }
        0x644 => Some(SlotMut::I32(&mut d.unknown_0x644)),
        0x648..=0x64c if (off - 0x648).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x648[(off - 0x648) / 4]))
        }
        0x650 => Some(SlotMut::I32(&mut d.unknown_0x650)),
        0x654 => Some(SlotMut::I32(&mut d.unknown_0x654)),
        0x658 => Some(SlotMut::I32(&mut d.unknown_0x658)),
        0x65c => Some(SlotMut::I32(&mut d.unknown_0x65c)),
        0x660 => Some(SlotMut::I32(&mut d.unknown_0x660)),
        0x664..=0x668 if (off - 0x664).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x664[(off - 0x664) / 4]))
        }
        0x66c..=0x674 if (off - 0x66c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x66c[(off - 0x66c) / 4]))
        }
        0x678 => Some(SlotMut::I32(&mut d.unknown_0x678)),
        0x67c => Some(SlotMut::U8(&mut d.flag_0x67c)),
        0x67d => Some(SlotMut::U8(&mut d.flag_0x67d)),
        0x67e => Some(SlotMut::U8(&mut d.flag_0x67e)),
        0x67f => Some(SlotMut::U8(&mut d.flag_0x67f)),
        0x680..=0x681 => Some(SlotMut::U8(&mut d.flags_0x680[off - 0x680])),
        0x684 => Some(SlotMut::List(&mut d.unknown_0x684)),
        0x698 => Some(SlotMut::List(&mut d.unknown_0x698)),
        0x6ac => Some(SlotMut::List(&mut d.unknown_0x6ac)),
        0x6c0 => Some(SlotMut::List(&mut d.unknown_0x6c0)),
        0x6d4 => Some(SlotMut::List(&mut d.unknown_0x6d4)),
        0x6e8 => Some(SlotMut::List(&mut d.unknown_0x6e8)),
        0x6fc => Some(SlotMut::List(&mut d.unknown_0x6fc)),
        0x710..=0x714 if (off - 0x710).is_multiple_of(4) => {
            Some(SlotMut::Str(&mut d.unknown_0x710[(off - 0x710) / 4]))
        }
        0x718 => Some(SlotMut::Str(&mut d.unknown_0x718)),
        0x71c => Some(SlotMut::I32(&mut d.unknown_0x71c)),
        0x720 => Some(SlotMut::Str(&mut d.unknown_0x720)),
        0x724 => d.flag_0x724.as_mut().map(SlotMut::U8),
        0x72c..=0x740 if (off - 0x72c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x72c[(off - 0x72c) / 4]))
        }
        0x744 => Some(SlotMut::I32(&mut d.unknown_0x744)),
        0x748 => Some(SlotMut::Objs(&mut d.unknown_0x748)),
        0x75c..=0x760 if (off - 0x75c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x75c[(off - 0x75c) / 4]))
        }
        0x764 => Some(SlotMut::List(&mut d.unknown_0x764)),
        0x778..=0x77c if (off - 0x778).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x778[(off - 0x778) / 4]))
        }
        0x780..=0x785 => Some(SlotMut::U8(&mut d.flags_0x780[off - 0x780])),
        0x788 => Some(SlotMut::I32(&mut d.unknown_0x788)),
        0x78c => Some(SlotMut::I32(&mut d.unknown_0x78c)),
        0x790 => Some(SlotMut::I32(&mut d.unknown_0x790)),
        0x794 => Some(SlotMut::I32(&mut d.unknown_0x794)),
        0x798 => Some(SlotMut::I32(&mut d.unknown_0x798)),
        0x79c => Some(SlotMut::I32(&mut d.unknown_0x79c)),
        0x7a0 => Some(SlotMut::I32(&mut d.unknown_0x7a0)),
        0x7a4..=0x7b0 if (off - 0x7a4).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x7a4[(off - 0x7a4) / 4]))
        }
        0x7b4 => Some(SlotMut::I32(&mut d.unknown_0x7b4)),
        0x7b8 => Some(SlotMut::I32(&mut d.unknown_0x7b8)),
        0x7bc => Some(SlotMut::I32(&mut d.unknown_0x7bc)),
        0x7c0..=0x7c8 if (off - 0x7c0).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x7c0[(off - 0x7c0) / 4]))
        }
        0x7cc..=0x7e4 if (off - 0x7cc).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x7cc[(off - 0x7cc) / 4]))
        }
        0x7e8..=0x808 if (off - 0x7e8).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x7e8[(off - 0x7e8) / 4]))
        }
        0x81c..=0x828 if (off - 0x81c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut d.unknown_0x81c[(off - 0x81c) / 4]))
        }
        0x82c => d.allowed_classes.as_mut().map(SlotMut::List),
        0x840 => Some(SlotMut::List(&mut d.unknown_0x840)),
        0x854 => Some(SlotMut::List(&mut d.unknown_0x854)),
        0x868..=0x88c if (off - 0x868).is_multiple_of(4) => {
            Some(SlotMut::Str(&mut d.unknown_0x868[(off - 0x868) / 4]))
        }
        _ => None,
    }
}
pub fn class(c: &ClassData, off: usize) -> Option<Slot<'_>> {
    match off {
        0x4 => Some(Slot::Str(&c.name)),
        0x8 => Some(Slot::Str(&c.unknown_0x8)),
        0xc => Some(Slot::I32(&c.unknown_0xc)),
        0x14..=0x18 if (off - 0x14).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0x14[(off - 0x14) / 4])),
        0x34..=0x48 if (off - 0x34).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0x34[(off - 0x34) / 4])),
        0x4c => Some(Slot::I32(&c.unknown_0x4c)),
        0x50 => Some(Slot::I32(&c.unknown_0x50)),
        0x54..=0x58 if (off - 0x54).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0x54[(off - 0x54) / 4])),
        0x5c => Some(Slot::List(&c.abilities)),
        0x70 => Some(Slot::List(&c.unknown_0x70)),
        0x84..=0x8c if (off - 0x84).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0x84[(off - 0x84) / 4])),
        0x90 => Some(Slot::I32(&c.unknown_0x90)),
        0x94..=0x9c if (off - 0x94).is_multiple_of(4) => c
            .unknown_0x94
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x94) / 4])),
        0xa0 => Some(Slot::List(&c.alignments)),
        0xb4..=0xb8 if (off - 0xb4).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0xb4[(off - 0xb4) / 4])),
        0xbc..=0xc4 if (off - 0xbc).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0xbc[(off - 0xbc) / 4])),
        0xc8 => Some(Slot::I32(&c.unknown_0xc8)),
        0xcc..=0xe0 if (off - 0xcc).is_multiple_of(4) => Some(Slot::I32(&c.unknown_0xcc[(off - 0xcc) / 4])),
        0x27c..=0x40c if (off - 0x27c).is_multiple_of(4) => c
            .unknown_0x27c
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x27c) / 4])),
        0x2724 => Some(Slot::List(&c.default_abilities)),
        0x2738 => Some(Slot::List(&c.unknown_0x2738)),
        0x274c => Some(Slot::List(&c.unknown_0x274c)),
        0x2760 => Some(Slot::List(&c.unknown_0x2760)),
        0x2774..=0x2784 if (off - 0x2774).is_multiple_of(4) => {
            Some(Slot::I32(&c.unknown_0x2774[(off - 0x2774) / 4]))
        }
        0x2788 => Some(Slot::Str(&c.unknown_0x2788)),
        0x278c => Some(Slot::U8(&c.flag_0x278c)),
        0x278d => c.flag_0x278d.as_ref().map(Slot::U8),
        0x278e..=0x278f => c.flags_0x278e.as_ref().map(|a| Slot::U8(&a[off - 0x278e])),
        0x2790 => c.flag_0x2790.as_ref().map(Slot::U8),
        0x2791 => c.flag_0x2791.as_ref().map(Slot::U8),
        0x2792 => c.flag_0x2792.as_ref().map(Slot::U8),
        0x2793..=0x2794 => c.flags_0x2793.as_ref().map(|a| Slot::U8(&a[off - 0x2793])),
        0x2795..=0x2796 => c.flags_0x2795.as_ref().map(|a| Slot::U8(&a[off - 0x2795])),
        0x2797 => c.flag_0x2797.as_ref().map(Slot::U8),
        0x2798 => Some(Slot::I32(&c.unknown_0x2798)),
        0x279c => c.flag_0x279c.as_ref().map(Slot::U8),
        0x279d => c.flag_0x279d.as_ref().map(Slot::U8),
        0x279e..=0x279f => c.flags_0x279e.as_ref().map(|a| Slot::U8(&a[off - 0x279e])),
        0x27a0 => c.flags_0x27a0.as_ref().map(|a| Slot::U8(&a[0])),
        0x27a1 => c.flag_0x27a1.as_ref().map(Slot::U8),
        0x27a2 => c.flags_0x27a0.as_ref().map(|a| Slot::U8(&a[1])),
        0x27a3 => c.flags_0x27a0.as_ref().map(|a| Slot::U8(&a[2])),
        0x27a4 => c.flag_0x27a4.as_ref().map(Slot::U8),
        0x27a5 => c.flags_0x486e.as_ref().map(|a| Slot::U8(&a[3])),
        0x2f8c => Some(Slot::Objs(&c.unknown_0x2f8c)),
        0x2fa0 => Some(Slot::List(&c.unknown_0x2fa0)),
        0x2fb4 => c.unknown_0x2fb4.as_ref().map(Slot::List),
        0x2fc8 => c.unknown_0x2fc8.as_ref().map(|a| Slot::I32(&a[0])),
        0x2fcc => Some(Slot::List(&c.unknown_0x2fcc)),
        0x2fe0 => c.unknown_0x2fe0.as_ref().map(Slot::List),
        0x2ff4 => c.unknown_0x2fc8.as_ref().map(|a| Slot::I32(&a[1])),
        0x2ff8 => Some(Slot::I32(&c.unknown_0x2ff8)),
        0x2ffc => c.flag_0x2ffc.as_ref().map(Slot::U8),
        0x3000 => c.unknown_0x3000.as_ref().map(Slot::Str),
        0x3004 => c.unknown_0x3004.as_ref().map(Slot::List),
        0x3018 => Some(Slot::List(&c.unknown_0x3018)),
        0x302c => Some(Slot::List(&c.unknown_0x302c)),
        0x3040 => Some(Slot::I32(&c.unknown_0x3040)),
        0x3044..=0x3048 => c.flags_0x3044.as_ref().map(|a| Slot::U8(&a[off - 0x3044])),
        0x3049..=0x304e => c.flags_0x3049.as_ref().map(|a| Slot::U8(&a[off - 0x3049])),
        0x3050..=0x31e4 if (off - 0x3050).is_multiple_of(4) => c
            .unknown_0x3050
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x3050) / 4])),
        0x31e8..=0x337c if (off - 0x31e8).is_multiple_of(4) => c
            .unknown_0x31e8
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x31e8) / 4])),
        0x3380..=0x3510 if (off - 0x3380).is_multiple_of(4) => c
            .unknown_0x3380
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x3380) / 4])),
        0x3514 => Some(Slot::Objs(&c.unknown_0x3514)),
        0x3528 => Some(Slot::Objs(&c.kit_bonuses)),
        0x353c..=0x3544 if (off - 0x353c).is_multiple_of(4) => {
            Some(Slot::Str(&c.unknown_0x353c[(off - 0x353c) / 4]))
        }
        0x3548 => Some(Slot::Str(&c.unknown_0x3548)),
        0x3550 => Some(Slot::I32(&c.unknown_0x3550)),
        0x3554 => c.flag_0x3554.as_ref().map(Slot::U8),
        0x3558 => c.unknown_0x3558.as_ref().map(Slot::List),
        0x485c..=0x4860 if (off - 0x485c).is_multiple_of(4) => {
            Some(Slot::I32(&c.unknown_0x485c[(off - 0x485c) / 4]))
        }
        0x4864..=0x4865 => c.flags_0x4864.as_ref().map(|a| Slot::U8(&a[off - 0x4864])),
        0x4868 => Some(Slot::Str(&c.unknown_0x4868)),
        0x486c => c.flag_0x486c.as_ref().map(Slot::U8),
        0x486d => c.flags_0x486e.as_ref().map(|a| Slot::U8(&a[4])),
        0x486e => c.flags_0x486e.as_ref().map(|a| Slot::U8(&a[0])),
        0x4870 => c.unknown_0x4870.as_ref().map(Slot::List),
        0x4884 => Some(Slot::Objs(&c.unknown_0x4884)),
        0x4898 => Some(Slot::List(&c.unknown_0x4898)),
        0x48ac => c.flags_0x486e.as_ref().map(|a| Slot::U8(&a[1])),
        0x48b0 => c.unknown_0x48b0.as_ref().map(Slot::List),
        0x48c4..=0x4a54 if (off - 0x48c4).is_multiple_of(4) => c
            .unknown_0x48c4
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x48c4) / 4])),
        0x4a58 => c.flags_0x486e.as_ref().map(|a| Slot::U8(&a[2])),
        0x4a5c..=0x4a6c if (off - 0x4a5c).is_multiple_of(4) => {
            Some(Slot::I32(&c.unknown_0x4a5c[(off - 0x4a5c) / 4]))
        }
        0x4a70..=0x4a9c if (off - 0x4a70).is_multiple_of(4) => c
            .unknown_0x4a70
            .as_ref()
            .map(|a| Slot::I32(&a[(off - 0x4a70) / 4])),
        0x4aa0..=0x4aa4 if (off - 0x4aa0).is_multiple_of(4) => {
            Some(Slot::Str(&c.unknown_0x4aa0[(off - 0x4aa0) / 4]))
        }
        0x4aa8..=0x4ac8 if (off - 0x4aa8).is_multiple_of(4) => {
            Some(Slot::I32(&c.unknown_0x4aa8[(off - 0x4aa8) / 4]))
        }
        _ => None,
    }
}

pub fn class_mut(c: &mut ClassData, off: usize) -> Option<SlotMut<'_>> {
    match off {
        0x4 => Some(SlotMut::Str(&mut c.name)),
        0x8 => Some(SlotMut::Str(&mut c.unknown_0x8)),
        0xc => Some(SlotMut::I32(&mut c.unknown_0xc)),
        0x14..=0x18 if (off - 0x14).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x14[(off - 0x14) / 4]))
        }
        0x34..=0x48 if (off - 0x34).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x34[(off - 0x34) / 4]))
        }
        0x4c => Some(SlotMut::I32(&mut c.unknown_0x4c)),
        0x50 => Some(SlotMut::I32(&mut c.unknown_0x50)),
        0x54..=0x58 if (off - 0x54).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x54[(off - 0x54) / 4]))
        }
        0x5c => Some(SlotMut::List(&mut c.abilities)),
        0x70 => Some(SlotMut::List(&mut c.unknown_0x70)),
        0x84..=0x8c if (off - 0x84).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x84[(off - 0x84) / 4]))
        }
        0x90 => Some(SlotMut::I32(&mut c.unknown_0x90)),
        0x94..=0x9c if (off - 0x94).is_multiple_of(4) => c
            .unknown_0x94
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x94) / 4])),
        0xa0 => Some(SlotMut::List(&mut c.alignments)),
        0xb4..=0xb8 if (off - 0xb4).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0xb4[(off - 0xb4) / 4]))
        }
        0xbc..=0xc4 if (off - 0xbc).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0xbc[(off - 0xbc) / 4]))
        }
        0xc8 => Some(SlotMut::I32(&mut c.unknown_0xc8)),
        0xcc..=0xe0 if (off - 0xcc).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0xcc[(off - 0xcc) / 4]))
        }
        0x27c..=0x40c if (off - 0x27c).is_multiple_of(4) => c
            .unknown_0x27c
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x27c) / 4])),
        0x2724 => Some(SlotMut::List(&mut c.default_abilities)),
        0x2738 => Some(SlotMut::List(&mut c.unknown_0x2738)),
        0x274c => Some(SlotMut::List(&mut c.unknown_0x274c)),
        0x2760 => Some(SlotMut::List(&mut c.unknown_0x2760)),
        0x2774..=0x2784 if (off - 0x2774).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x2774[(off - 0x2774) / 4]))
        }
        0x2788 => Some(SlotMut::Str(&mut c.unknown_0x2788)),
        0x278c => Some(SlotMut::U8(&mut c.flag_0x278c)),
        0x278d => c.flag_0x278d.as_mut().map(SlotMut::U8),
        0x278e..=0x278f => c
            .flags_0x278e
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x278e])),
        0x2790 => c.flag_0x2790.as_mut().map(SlotMut::U8),
        0x2791 => c.flag_0x2791.as_mut().map(SlotMut::U8),
        0x2792 => c.flag_0x2792.as_mut().map(SlotMut::U8),
        0x2793..=0x2794 => c
            .flags_0x2793
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x2793])),
        0x2795..=0x2796 => c
            .flags_0x2795
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x2795])),
        0x2797 => c.flag_0x2797.as_mut().map(SlotMut::U8),
        0x2798 => Some(SlotMut::I32(&mut c.unknown_0x2798)),
        0x279c => c.flag_0x279c.as_mut().map(SlotMut::U8),
        0x279d => c.flag_0x279d.as_mut().map(SlotMut::U8),
        0x279e..=0x279f => c
            .flags_0x279e
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x279e])),
        0x27a0 => c.flags_0x27a0.as_mut().map(|a| SlotMut::U8(&mut a[0])),
        0x27a1 => c.flag_0x27a1.as_mut().map(SlotMut::U8),
        0x27a2 => c.flags_0x27a0.as_mut().map(|a| SlotMut::U8(&mut a[1])),
        0x27a3 => c.flags_0x27a0.as_mut().map(|a| SlotMut::U8(&mut a[2])),
        0x27a4 => c.flag_0x27a4.as_mut().map(SlotMut::U8),
        0x27a5 => c.flags_0x486e.as_mut().map(|a| SlotMut::U8(&mut a[3])),
        0x2f8c => Some(SlotMut::Objs(&mut c.unknown_0x2f8c)),
        0x2fa0 => Some(SlotMut::List(&mut c.unknown_0x2fa0)),
        0x2fb4 => c.unknown_0x2fb4.as_mut().map(SlotMut::List),
        0x2fc8 => c.unknown_0x2fc8.as_mut().map(|a| SlotMut::I32(&mut a[0])),
        0x2fcc => Some(SlotMut::List(&mut c.unknown_0x2fcc)),
        0x2fe0 => c.unknown_0x2fe0.as_mut().map(SlotMut::List),
        0x2ff4 => c.unknown_0x2fc8.as_mut().map(|a| SlotMut::I32(&mut a[1])),
        0x2ff8 => Some(SlotMut::I32(&mut c.unknown_0x2ff8)),
        0x2ffc => c.flag_0x2ffc.as_mut().map(SlotMut::U8),
        0x3000 => c.unknown_0x3000.as_mut().map(SlotMut::Str),
        0x3004 => c.unknown_0x3004.as_mut().map(SlotMut::List),
        0x3018 => Some(SlotMut::List(&mut c.unknown_0x3018)),
        0x302c => Some(SlotMut::List(&mut c.unknown_0x302c)),
        0x3040 => Some(SlotMut::I32(&mut c.unknown_0x3040)),
        0x3044..=0x3048 => c
            .flags_0x3044
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x3044])),
        0x3049..=0x304e => c
            .flags_0x3049
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x3049])),
        0x3050..=0x31e4 if (off - 0x3050).is_multiple_of(4) => c
            .unknown_0x3050
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x3050) / 4])),
        0x31e8..=0x337c if (off - 0x31e8).is_multiple_of(4) => c
            .unknown_0x31e8
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x31e8) / 4])),
        0x3380..=0x3510 if (off - 0x3380).is_multiple_of(4) => c
            .unknown_0x3380
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x3380) / 4])),
        0x3514 => Some(SlotMut::Objs(&mut c.unknown_0x3514)),
        0x3528 => Some(SlotMut::Objs(&mut c.kit_bonuses)),
        0x353c..=0x3544 if (off - 0x353c).is_multiple_of(4) => {
            Some(SlotMut::Str(&mut c.unknown_0x353c[(off - 0x353c) / 4]))
        }
        0x3548 => Some(SlotMut::Str(&mut c.unknown_0x3548)),
        0x3550 => Some(SlotMut::I32(&mut c.unknown_0x3550)),
        0x3554 => c.flag_0x3554.as_mut().map(SlotMut::U8),
        0x3558 => c.unknown_0x3558.as_mut().map(SlotMut::List),
        0x485c..=0x4860 if (off - 0x485c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x485c[(off - 0x485c) / 4]))
        }
        0x4864..=0x4865 => c
            .flags_0x4864
            .as_mut()
            .map(|a| SlotMut::U8(&mut a[off - 0x4864])),
        0x4868 => Some(SlotMut::Str(&mut c.unknown_0x4868)),
        0x486c => c.flag_0x486c.as_mut().map(SlotMut::U8),
        0x486d => c.flags_0x486e.as_mut().map(|a| SlotMut::U8(&mut a[4])),
        0x486e => c.flags_0x486e.as_mut().map(|a| SlotMut::U8(&mut a[0])),
        0x4870 => c.unknown_0x4870.as_mut().map(SlotMut::List),
        0x4884 => Some(SlotMut::Objs(&mut c.unknown_0x4884)),
        0x4898 => Some(SlotMut::List(&mut c.unknown_0x4898)),
        0x48ac => c.flags_0x486e.as_mut().map(|a| SlotMut::U8(&mut a[1])),
        0x48b0 => c.unknown_0x48b0.as_mut().map(SlotMut::List),
        0x48c4..=0x4a54 if (off - 0x48c4).is_multiple_of(4) => c
            .unknown_0x48c4
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x48c4) / 4])),
        0x4a58 => c.flags_0x486e.as_mut().map(|a| SlotMut::U8(&mut a[2])),
        0x4a5c..=0x4a6c if (off - 0x4a5c).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x4a5c[(off - 0x4a5c) / 4]))
        }
        0x4a70..=0x4a9c if (off - 0x4a70).is_multiple_of(4) => c
            .unknown_0x4a70
            .as_mut()
            .map(|a| SlotMut::I32(&mut a[(off - 0x4a70) / 4])),
        0x4aa0..=0x4aa4 if (off - 0x4aa0).is_multiple_of(4) => {
            Some(SlotMut::Str(&mut c.unknown_0x4aa0[(off - 0x4aa0) / 4]))
        }
        0x4aa8..=0x4ac8 if (off - 0x4aa8).is_multiple_of(4) => {
            Some(SlotMut::I32(&mut c.unknown_0x4aa8[(off - 0x4aa8) / 4]))
        }
        _ => None,
    }
}
