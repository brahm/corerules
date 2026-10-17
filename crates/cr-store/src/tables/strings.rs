//! Where the text the Table Rolls print from the originals' own code
//! sits in each program: (Tsrp3.exe, Evermore.exe) virtual addresses of
//! NUL-terminated strings, 0 when a program does not have one. The text
//! itself is read from the installation at run time ([`super::program`]).
//! Generated from the Expansion's code (the addresses its Table Rolls
//! code pushes) and the same strings found in Core Rules 2.0.

use super::program::Txt;

pub(super) const TURN_HEAD: &[Txt] = &[Txt(0x8277c0, 0x6dc82c), Txt(0x8277f0, 0x6dc7fc)];
pub(super) const UNDEAD: &[Txt] = &[
    Txt(0x82782c, 0x6dc7e4),
    Txt(0x82784c, 0x6dc7c4),
    Txt(0x82786c, 0x6dc7ac),
    Txt(0x82788c, 0x6dc794),
    Txt(0x8278ac, 0x6dc77c),
    Txt(0x8278cc, 0x6dc764),
    Txt(0x8278ec, 0x6dc74c),
    Txt(0x82790c, 0x6dc734),
    Txt(0x82792c, 0x6dc71c),
    Txt(0x82794c, 0x6dc704),
    Txt(0x82796c, 0x6dc6ec),
    Txt(0x82798c, 0x6dc6d4),
    Txt(0x8279ac, 0x6dc6bc),
];
pub(super) const TURN_NOTE: &[Txt] = &[
    Txt(0x8279c4, 0x6dc690),
    Txt(0x8279f0, 0x6dc65c),
    Txt(0x827a24, 0x6dc62c),
    Txt(0x827a54, 0x6dc604),
];
pub(super) const PRIEST_LEVELS: &[Txt] = &[
    Txt(0x828e28, 0x6ddb3c),
    Txt(0x828e3c, 0x6ddb28),
    Txt(0x828e50, 0x6ddb14),
    Txt(0x828e64, 0x6ddb00),
    Txt(0x828e78, 0x6ddaec),
    Txt(0x828e8c, 0x6ddad8),
    Txt(0x828ea0, 0x6ddac4),
    Txt(0x828eb4, 0x6ddab0),
    Txt(0x828ec8, 0x6dda9c),
    Txt(0x828edc, 0x6dda84),
    Txt(0x828ef4, 0x6dda6c),
    Txt(0x828f0c, 0x6dda58),
];
pub(super) const PALADIN_LEVELS: &[Txt] = &[
    Txt(0x828f20, 0x6dda44),
    Txt(0x828f34, 0x6dda30),
    Txt(0x828f48, 0x6dda1c),
    Txt(0x828f5c, 0x6dda08),
    Txt(0x828f70, 0x6dd9f4),
    Txt(0x828f84, 0x6dd9e0),
    Txt(0x828f98, 0x6dd9cc),
    Txt(0x828fac, 0x6dd9b8),
    Txt(0x828fc0, 0x6dd9a4),
    Txt(0x828fd4, 0x6dd98c),
    Txt(0x828fec, 0x6dd974),
    Txt(0x829004, 0x6dd960),
];
pub(super) const TURNING_CLASSES: &[Txt] = &[Txt(0x826e08, 0x6dc428), Txt(0x826e10, 0x6dc420)];
pub(super) const RANKS: &[Txt] = &[
    Txt(0x826ee0, 0x6dbe7c),
    Txt(0x826ef0, 0x6dbe74),
    Txt(0x826ef8, 0x6dbe6c),
    Txt(0x826f00, 0x6dbe60),
    Txt(0x826f0c, 0x6dbe58),
];
pub(super) const DISEASE_HEAD: &[Txt] = &[Txt(0x8263dc, 0x6db31c), Txt(0x826408, 0x6db2f0)];
pub(super) const DISEASE_ROWS: &[Txt] = &[
    Txt(0x826434, 0x6db2c4),
    Txt(0x826460, 0x6db298),
    Txt(0x82648c, 0x6db26c),
    Txt(0x8264b8, 0x6db240),
    Txt(0x8264e4, 0x6db214),
];
pub(super) const DISEASE_NOTE: &[Txt] = &[
    Txt(0x826514, 0x6db1e8),
    Txt(0x826540, 0x6db1b8),
    Txt(0x826570, 0x6db188),
    Txt(0x8265a0, 0x6db158),
    Txt(0x8265d0, 0x6db12c),
    Txt(0x8265fc, 0x6db0fc),
    Txt(0x82662c, 0x6db0d4),
    Txt(0x826658, 0x6db0a0),
    Txt(0x82668c, 0x6db06c),
    Txt(0x8266c0, 0x6db038),
    Txt(0x8266f4, 0x6db030),
];
pub(super) const CRUSH_HEAD: &[Txt] = &[Txt(0x826714, 0x6daff4), Txt(0x826738, 0x6dafd0)];
pub(super) const CRUSH_ROWS: &[Txt] = &[
    Txt(0x82675c, 0x6dafb0),
    Txt(0x82677c, 0x6daf90),
    Txt(0x82679c, 0x6daf70),
    Txt(0x8267bc, 0x6daf50),
    Txt(0x8267dc, 0x6daf30),
];
pub(super) const WOUNDING_HEAD: &[Txt] = &[Txt(0x826820, 0x6daefc), Txt(0x826838, 0x6daee4)];
pub(super) const WOUNDING_ROWS: &[Txt] = &[
    Txt(0x826850, 0x6daed0),
    Txt(0x826864, 0x6daeb8),
    Txt(0x82687c, 0x6dae9c),
    Txt(0x826898, 0x6dae80),
    Txt(0x8268b4, 0x6dae64),
];
pub(super) const WOUNDING_NOTE: &[Txt] = &[
    Txt(0x8268d8, 0x6dae34),
    Txt(0x826908, 0x6dae04),
    Txt(0x826938, 0x6dadd8),
    Txt(0x826964, 0x6dada8),
    Txt(0x826994, 0x6dada0),
];
pub(super) const FUNDS_FOR: Txt = Txt(0x827618, 0x6dc5c4);
pub(super) const CLASS_GROUPS: &[Txt] = &[
    Txt(0x826d1c, 0x6c55d0),
    Txt(0x826d24, 0x6c119c),
    Txt(0x826d2c, 0x6c55c8),
    Txt(0x826d34, 0x6c55d8),
];
pub(super) const FUNDS_NOTE: &[Txt] = &[
    Txt(0x827668, 0x6dc570),
    Txt(0x827698, 0x6dc53c),
    Txt(0x8276cc, 0x6dc508),
    Txt(0x827700, 0x6dc4d4),
    Txt(0x827734, 0x6dc4a0),
    Txt(0x827768, 0x6dc46c),
    Txt(0x82779c, 0x6dc448),
];
pub(super) const CASTER_LEVELS: &[Txt] = &[
    Txt(0x826e70, 0x6dc40c),
    Txt(0x826e84, 0x6dc3f8),
    Txt(0x826e98, 0x6dc3e4),
    Txt(0x826eac, 0x6dc3d0),
    Txt(0x826ec0, 0x6dc3bc),
];
pub(super) const SPELL_FAILURE: &[Txt] = &[Txt(0x82805c, 0x6dcdf4), Txt(0x82806c, 0x6dcdd8)];
pub(super) const RESULT_HEAD: &[Txt] = &[Txt(0x828088, 0x6dcdc8), Txt(0x828098, 0x6dcdb8)];
pub(super) const SCROLL_LEVELS: &[Txt] = &[
    Txt(0x826f20, 0x6dbed4),
    Txt(0x826f2c, 0x6dbec8),
    Txt(0x826f38, 0x6dbebc),
    Txt(0x826f44, 0x6dbeb4),
    Txt(0x826f4c, 0x6dbeac),
    Txt(0x826f54, 0x6dbea4),
    Txt(0x826f5c, 0x6dbe98),
    Txt(0x826f68, 0x6dbe8c),
];
pub(super) const SCROLL_USE: &[Txt] = &[Txt(0x8280b0, 0x6dce18), Txt(0x8280c0, 0x6dce04)];
pub(super) const LANGUAGES_HEAD: &[Txt] = &[Txt(0x82c594, 0x6def7c), Txt(0x82c5b0, 0x6dec28)];
pub(super) const LANGUAGES_NOTE: &[Txt] = &[
    Txt(0x82c5e8, 0x6dcd60),
    Txt(0x82c614, 0x6dcd30),
    Txt(0x82c644, 0x6dcd20),
];
pub(super) const RARITY: &[Txt] = &[
    Txt(0x826f80, 0x6cd2f0),
    Txt(0x826f88, 0x6cf810),
    Txt(0x826f94, 0x6cf808),
];
pub(super) const FOUND_IN: &[Txt] = &[
    Txt(0x8280f4, 0x6dce30),
    Txt(0x828110, 0x6dce28),
    Txt(0x828118, 0x6ae638),
    Txt(0x82811c, 0x44f605),
];
pub(super) const CANNOT: &[Txt] = &[
    Txt(0x828128, 0x6dcf74),
    Txt(0x828150, 0x6dcf50),
    Txt(0x828174, 0x6dcf2c),
    Txt(0x828198, 0x6dcf00),
];
pub(super) const CAN: &[Txt] = &[
    Txt(0x8281c4, 0x6dced8),
    Txt(0x8281ec, 0x6dceb0),
    Txt(0x828214, 0x6dce88),
    Txt(0x82823c, 0x6dce5c),
];
pub(super) const PURCHASE_HEAD: &[Txt] = &[Txt(0x8283b0, 0x6dce54), Txt(0x8283b8, 0x6dce4c)];
pub(super) const TARGETS_AFFECTED: &[Txt] = &[
    Txt(0x8270f8, 0x6dc3a0),
    Txt(0x827114, 0x6dc384),
    Txt(0x827130, 0x6dc368),
    Txt(0x82714c, 0x6dc34c),
];
pub(super) const WHOLE_HEAD: &[Txt] = &[
    Txt(0x8283e4, 0x6dd05c),
    Txt(0x82840c, 0x6dd034),
    Txt(0x828434, 0x6dd00c),
];
pub(super) const AREAS: &[Txt] = &[
    Txt(0x82845c, 0x6dcff0),
    Txt(0x828478, 0x6dcfd8),
    Txt(0x828494, 0x6dcfbc),
    Txt(0x8284b0, 0x6dcfa0),
];
pub(super) const AREA_SIZES: &[Txt] = &[
    Txt(0x828490, 0x6dcfd4),
    Txt(0x8284ac, 0x6dcfb8),
    Txt(0x8284c8, 0x6dcf9c),
];
pub(super) const MAX_DAMAGE: &[Txt] = &[
    Txt(0x827174, 0x6dc324),
    Txt(0x82719c, 0x6dc2fc),
    Txt(0x8271c4, 0x6dc2d4),
    Txt(0x8271ec, 0x6dc2ac),
];
pub(super) const SEVERITY_HEAD: &[Txt] = &[
    Txt(0x8284ec, 0x6dd108),
    Txt(0x828520, 0x6dd0d8),
    Txt(0x828550, 0x6dd0a4),
];
pub(super) const TRAIT_HEAD: &[Txt] = &[Txt(0x82c464, 0x6dc874), Txt(0x82c47c, 0x6dc85c)];
pub(super) const TRAITS: &[Txt] = &[
    Txt(0x82c01c, 0x48442e),
    Txt(0x82c020, 0x6dccb4),
    Txt(0x82c02c, 0x6dcca4),
    Txt(0x82c03c, 0x6dcc98),
    Txt(0x82c048, 0x6dcc8c),
    Txt(0x82c054, 0x6dcc7c),
    Txt(0x82c064, 0x6dcc74),
    Txt(0x82c06c, 0x6dcc6c),
    Txt(0x82c074, 0x6dcc64),
    Txt(0x82c07c, 0x6dcc5c),
    Txt(0x82c084, 0x6dcc54),
    Txt(0x82c08c, 0x6dcc48),
    Txt(0x82c098, 0x6dcc3c),
    Txt(0x82c0a4, 0x6dcc34),
    Txt(0x82c0ac, 0x6dcc28),
    Txt(0x82c0b8, 0x6dcc20),
    Txt(0x82c0c0, 0x6dcc14),
    Txt(0x82c0cc, 0x6dcc04),
    Txt(0x82c0dc, 0x6dcbfc),
    Txt(0x82c0e4, 0x6dcbec),
    Txt(0x82c0f4, 0x6dcbe0),
    Txt(0x82c100, 0x6dcbd8),
    Txt(0x82c108, 0x6dcbd0),
    Txt(0x82c110, 0x6dcbcc),
    Txt(0x82c114, 0x6d3d54),
    Txt(0x82c120, 0x6dcbc0),
    Txt(0x82c12c, 0x6dcbb4),
    Txt(0x82c138, 0x6dcbac),
    Txt(0x82c140, 0x6dcb9c),
    Txt(0x82c150, 0x6dcb90),
    Txt(0x82c15c, 0x6dcb88),
    Txt(0x82c164, 0x6dcb78),
    Txt(0x82c174, 0x6dcb70),
    Txt(0x82c17c, 0x6dcb68),
    Txt(0x82c184, 0x6dcb5c),
    Txt(0x82c190, 0x6dcb54),
    Txt(0x82c198, 0x6dcb48),
    Txt(0x82c1a4, 0x6dcb38),
    Txt(0x82c1b4, 0x6dcb2c),
    Txt(0x82c1c0, 0x6dcb20),
    Txt(0x82c1cc, 0x6dcb10),
    Txt(0x82c1dc, 0x6dcb08),
    Txt(0x82c1e4, 0x6dcaf8),
    Txt(0x82c1f4, 0x6dcaec),
    Txt(0x82c200, 0x6dcae0),
    Txt(0x82c20c, 0x6dcad8),
    Txt(0x82c214, 0x6dcad0),
    Txt(0x82c21c, 0x6dcac4),
    Txt(0x82c228, 0x6dcab8),
    Txt(0x82c234, 0x6dcab0),
    Txt(0x82c23c, 0x6dcaa4),
    Txt(0x82c248, 0x6dca9c),
    Txt(0x82c250, 0x6dca94),
    Txt(0x82c258, 0x6dca88),
    Txt(0x82c264, 0x6dca7c),
    Txt(0x82c270, 0x6dca70),
    Txt(0x82c27c, 0x6dca68),
    Txt(0x82c284, 0x6dca5c),
    Txt(0x82c290, 0x6dca50),
    Txt(0x82c29c, 0x6dca44),
    Txt(0x82c2a8, 0x6dca3c),
    Txt(0x82c2b0, 0x6dca34),
    Txt(0x82c2b8, 0x6dca2c),
    Txt(0x82c2c0, 0x6dca1c),
    Txt(0x82c2d0, 0x6dca10),
    Txt(0x82c2dc, 0x6dca04),
    Txt(0x82c2e8, 0x6dc9f8),
    Txt(0x82c2f4, 0x6dc9f0),
    Txt(0x82c2fc, 0x6dc9e4),
    Txt(0x82c308, 0x6dc9d8),
    Txt(0x82c314, 0x6dc9cc),
    Txt(0x82c320, 0x6dc9c0),
    Txt(0x82c32c, 0x6dc9b4),
    Txt(0x82c338, 0x6dc9ac),
    Txt(0x82c340, 0x6dc9a0),
    Txt(0x82c34c, 0x6dc994),
    Txt(0x82c358, 0x6dc98c),
    Txt(0x82c360, 0x6dc980),
    Txt(0x82c36c, 0x6dc974),
    Txt(0x82c378, 0x6dc968),
    Txt(0x82c384, 0x6dc960),
    Txt(0x82c38c, 0x6dc954),
    Txt(0x82c398, 0x6dc944),
    Txt(0x82c3a8, 0x6dc93c),
    Txt(0x82c3b0, 0x6dc930),
    Txt(0x82c3bc, 0x6dc924),
    Txt(0x82c3c8, 0x6dc918),
    Txt(0x82c3d4, 0x6dc90c),
    Txt(0x82c3e0, 0x6dc900),
    Txt(0x82c3ec, 0x6dc8f4),
    Txt(0x82c3f8, 0x6dc8ec),
    Txt(0x82c400, 0x6dc8e0),
    Txt(0x82c40c, 0x6dc8d8),
    Txt(0x82c414, 0x6dc8cc),
    Txt(0x82c420, 0x6dc8c0),
    Txt(0x82c42c, 0x6dc8b8),
    Txt(0x82c434, 0x6dc8b0),
    Txt(0x82c43c, 0x6dc8a4),
    Txt(0x82c448, 0x6dc89c),
    Txt(0x82c450, 0x6dc894),
    Txt(0x82c458, 0x6dc88c),
];
pub(super) const GEM_HEAD: &[Txt] = &[
    Txt(0x82cedc, 0x6dd224),
    Txt(0x82cee8, 0x6dd200),
    Txt(0x82cf0c, 0x6dd1dc),
];
pub(super) const GEMS: &[&[Txt]] = &[
    &[
        Txt(0x82cc44, 0x6dd49c),
        Txt(0x82cc4c, 0x6dd48c),
        Txt(0x82cc5c, 0x6dd480),
        Txt(0x82cc68, 0x6dd474),
        Txt(0x82cc74, 0x6dd468),
        Txt(0x82cc80, 0x6dd458),
        Txt(0x82cc90, 0x6dd44c),
        Txt(0x82cc9c, 0x6dd440),
        Txt(0x82cca8, 0x6dd434),
        Txt(0x82ccb4, 0x6dd424),
        Txt(0x82ccc4, 0x6dd414),
        Txt(0x82ccd4, 0x6dd408),
    ],
    &[
        Txt(0x82cce0, 0x6dd3fc),
        Txt(0x82ccec, 0x6dd3f0),
        Txt(0x82ccf8, 0x6dd3e4),
        Txt(0x82cd04, 0x6dd3d8),
        Txt(0x82cd10, 0x6dd3d0),
        Txt(0x82cd18, 0x6dd3c8),
        Txt(0x82cd20, 0x6dd3bc),
        Txt(0x82cd2c, 0x6dd3b4),
        Txt(0x82cd34, 0x6dd3a4),
        Txt(0x82cd44, 0x6dd398),
        Txt(0x82cd50, 0x6dd388),
        Txt(0x82cd60, 0x6dd374),
        Txt(0x82cd74, 0x6dd36c),
    ],
    &[
        Txt(0x82cd7c, 0x6dd364),
        Txt(0x82cd84, 0x6dd358),
        Txt(0x82cd90, 0x6dd34c),
        Txt(0x82cd9c, 0x6dd340),
        Txt(0x82cda8, 0x6dd338),
        Txt(0x82cdb0, 0x6dd330),
        Txt(0x82cdb8, 0x6d5f48),
        Txt(0x82cdc0, 0x6dd32c),
        Txt(0x82cdc4, 0x6dd324),
        Txt(0x82cdcc, 0x6dd31c),
        Txt(0x82cdd4, 0x6dd310),
    ],
    &[
        Txt(0x82cde0, 0x6dd304),
        Txt(0x82cdec, 0x6dd330),
        Txt(0x82cdf4, 0x6dd324),
        Txt(0x82cdfc, 0x6dd2fc),
        Txt(0x82ce04, 0x6dd31c),
        Txt(0x82ce0c, 0x6dd2f4),
    ],
    &[
        Txt(0x82ce14, 0x6dd2e8),
        Txt(0x82ce20, 0x6dd2dc),
        Txt(0x82ce2c, 0x6dd2d4),
        Txt(0x82ce34, 0x6dd2c0),
        Txt(0x82ce48, 0x6dd2b0),
        Txt(0x82ce58, 0x6dd2a4),
    ],
    &[
        Txt(0x82ce64, 0x6dd294),
        Txt(0x82ce74, 0x6dd28c),
        Txt(0x82ce7c, 0x6dd284),
        Txt(0x82ce84, 0x6dd27c),
        Txt(0x82ce8c, 0x6dd268),
        Txt(0x82cea0, 0x6dd260),
        Txt(0x82cea8, 0x6dd254),
        Txt(0x82ceb4, 0x6dd244),
    ],
];
pub(super) const IOUN_TITLE: Txt = Txt(0x82cf38, 0x6dd950);
pub(super) const IOUN_HEAD: &[Txt] = &[Txt(0x82cf4c, 0x6dd928), Txt(0x82cf74, 0x6dd900)];
pub(super) const IOUN: &[&[Txt]] = &[
    &[Txt(0x82cf9c, 0x6dd8cc)],
    &[Txt(0x82cfd0, 0x6dd898)],
    &[Txt(0x82d004, 0x6dd864)],
    &[Txt(0x82d038, 0x6dd830)],
    &[Txt(0x82d06c, 0x6dd7fc)],
    &[Txt(0x82d0a0, 0x6dd7c8)],
    &[Txt(0x82d0d4, 0x6dd78c)],
    &[Txt(0x82d110, 0x6dd754), Txt(0x82d148, 0x6dd728)],
    &[Txt(0x82d174, 0x6dd6ec)],
    &[Txt(0x82d1b0, 0x6dd6b8)],
    &[Txt(0x82d1e4, 0x6dd684), Txt(0x82d218, 0x6dd654)],
    &[Txt(0x82d248, 0x6dd620), Txt(0x82d27c, 0x6dd5f0)],
    &[Txt(0x82d2ac, 0x6dd5c8), Txt(0x82d2d4, 0x6dd5b4)],
    &[Txt(0x82d2e8, 0x6dd580)],
];
pub(super) const IOUN_DEAD: Txt = Txt(0x82d31c, 0x6dd54c);
pub(super) const IOUN_MAX: &[Txt] = &[Txt(0x82d354, 0x6dd53c)];
pub(super) const IOUN_ABSORB_4: &[Txt] = &[Txt(0x82d364, 0x6dd508), Txt(0x82d398, 0x6dd4d8)];
pub(super) const IOUN_ABSORB_8: &[Txt] = &[Txt(0x82d3c8, 0x6dd4a4), Txt(0x82d3fc, 0x6dd4d8)];
pub(super) const WILD_DEVOTION_HEAD: &[Txt] = &[Txt(0x82ee70, 0x6df9d8), Txt(0x82ee84, 0x6ddd94)];
pub(super) const WILD_SCIENCE_HEAD: &[Txt] = &[Txt(0x82ee98, 0x6df9c4), Txt(0x82eeac, 0x6ddd94)];
pub(super) const POWER_HEAD: &[Txt] = &[Txt(0x82a474, 0x6dbbf0), Txt(0x82a484, 0x6dbbe4)];
pub(super) const PURPOSE_HEAD: &[Txt] = &[Txt(0x82a4f0, 0x6db93c), Txt(0x82a500, 0x6dbc28)];
pub(super) const SPECIAL_POWER_HEAD: &[Txt] = &[Txt(0x82a510, 0x6dbbc8), Txt(0x82a52c, 0x6dbbac)];
pub(super) const PAGES: Txt = Txt(0x82b004, 0x6de628);
pub(super) const PAGES_HEAD: &[Txt] = &[Txt(0x82b00c, 0x6de600), Txt(0x82b034, 0x6de5d8)];
pub(super) const BEADS: &[Txt] = &[
    Txt(0x82b1a4, 0x6de478),
    Txt(0x82b1bc, 0x6de468),
    Txt(0x82b1cc, 0x6de44c),
];
pub(super) const BEAD_HEAD: &[Txt] = &[Txt(0x82b1f0, 0x6de430), Txt(0x82b20c, 0x6de414)];
pub(super) const PATCHES: Txt = Txt(0x82b380, 0x6de2a8);
pub(super) const PATCH_HEAD: &[Txt] = &[Txt(0x82b394, 0x6de290), Txt(0x82b3ac, 0x6de778)];
pub(super) const RESULT: &[Txt] = &[Txt(0x82e198, 0x6dcdc8), Txt(0x82e1a8, 0x6dcdb8)];
pub(super) const ITEM_HEAD: &[Txt] = &[Txt(0x829648, 0x6db794), Txt(0x829678, 0x6db764)];
pub(super) const JEWEL_HEAD: &[Txt] = &[Txt(0x8299a0, 0x6deb04), Txt(0x8299d4, 0x6dead0)];
pub(super) const HORSE_HEAD: &[Txt] = &[Txt(0x8258b4, 0x6dbb70), Txt(0x8258d8, 0x6dbb4c)];
pub(super) const HIGH_SPIRITED_HEAD: &[Txt] = &[Txt(0x829524, 0x6dbb28), Txt(0x829548, 0x6dbb04)];
pub(super) const PUNCH_HEAD: &[Txt] = &[Txt(0x825964, 0x6dbad4), Txt(0x825994, 0x6dbaac)];
pub(super) const WRESTLE_HEAD: &[Txt] = &[Txt(0x8259bc, 0x6dba7c), Txt(0x8259ec, 0x6dba54)];
pub(super) const SUBTABLE_89: &[Txt] = &[
    Txt(0x8296c8, 0x6de87c),
    Txt(0x8296d8, 0x6de86c),
    Txt(0x8296e8, 0x6de85c),
];
pub(super) const WIZARD_SCROLL_HEAD: &[Txt] = &[Txt(0x829720, 0x6de9a8), Txt(0x829788, 0x6de950)];
pub(super) const PRIEST_SCROLL_HEAD: &[Txt] = &[Txt(0x829754, 0x6de97c), Txt(0x829788, 0x6de950)];
pub(super) const SPELL_HEAD: &[Txt] = &[Txt(0x8297bc, 0x6de940), Txt(0x8297cc, 0x6de930)];
pub(super) const SUBTABLE_90: &[Txt] = &[Txt(0x82970e, 0x6de9e6), Txt(0x8297ee, 0x6de91e)];
pub(super) const SCROLL_ITEM_HEAD: &[Txt] = &[Txt(0x829800, 0x6de8dc), Txt(0x829830, 0x6de8ac)];
pub(super) const SUBTABLE_91: &[Txt] = &[Txt(0x829874, 0x6debe0), Txt(0x829894, 0x6debc0)];
pub(super) const SUBTABLE_96: &[Txt] = &[Txt(0x829964, 0x6deb80), Txt(0x829990, 0x6deb54)];
pub(super) const SUBTABLE_104: &[Txt] = &[Txt(0x829ea3, 0x6dea9f), Txt(0x829ec7, 0x6dea7b)];
pub(super) const ARMOR_HEAD: &[Txt] = &[Txt(0x829f58, 0x6ded60), Txt(0x829f64, 0x6dbbe4)];
pub(super) const AC_HEAD: &[Txt] = &[Txt(0x829f98, 0x6ded0c), Txt(0x829fc8, 0x6decdc)];
pub(super) const ARMOR_TYPE_HEAD: &[Txt] = &[Txt(0x82a018, 0x6dec90), Txt(0x82a048, 0x6dec60)];
pub(super) const WEAPON_HEAD: &[Txt] = &[Txt(0x82a078, 0x6dea58), Txt(0x82a088, 0x6dcdb8)];
pub(super) const SUBTABLE_108: &[Txt] = &[Txt(0x82a0b4, 0x6dea44), Txt(0x82a0e4, 0x6dea14)];
pub(super) const SWORD_HEAD: &[Txt] = &[Txt(0x82a0fc, 0x6dba24), Txt(0x82a12c, 0x6db9f4)];
pub(super) const OTHER_WEAPON_HEAD: &[Txt] = &[Txt(0x82a184, 0x6db9c4), Txt(0x82a1b4, 0x6db994)];
pub(super) const SUBTABLE_110: &[Txt] = &[
    Txt(0x82a28d, 0x6db731),
    Txt(0x82a2a1, 0x6db71d),
    Txt(0x82a2b5, 0x6db709),
    Txt(0x82a2c9, 0x6db6f5),
];
pub(super) const RESULT_88: &[Txt] = &[Txt(0x82e204, 0x6dcdc8), Txt(0x82e214, 0x6dbbe4)];
pub(super) const INTELLIGENCE_HEAD: &[Txt] = &[Txt(0x82a358, 0x6dbcb0), Txt(0x82a380, 0x6dbc88)];
pub(super) const ALIGNMENT_HEAD: &[Txt] = &[Txt(0x82eb98, 0x6dec44), Txt(0x82ebb4, 0x6dec28)];
pub(super) const ABILITY_HEAD: &[Txt] = &[Txt(0x82a428, 0x6dbc38), Txt(0x82a438, 0x6dbc28)];
pub(super) const SPECIAL_POWER_HEAD_B: &[Txt] = &[Txt(0x82a490, 0x6dbbc8), Txt(0x82a4ac, 0x6dbbac)];
pub(super) const HEAD_90001: &[Txt] = &[Txt(0x82ad2c, 0x6de848), Txt(0x82ad40, 0x6de820)];
pub(super) const HEAD_90002: &[Txt] = &[Txt(0x82ad54, 0x6de834), Txt(0x82ad68, 0x6de820)];
pub(super) const HEAD_90003: &[Txt] = &[Txt(0x82ad7c, 0x6de80c), Txt(0x82ad90, 0x6de820)];
pub(super) const HEAD_90004: &[Txt] = &[Txt(0x82ada4, 0x6de7ec), Txt(0x82adc4, 0x6de7cc)];
pub(super) const HEAD_90005: &[Txt] = &[Txt(0x82ade4, 0x6de7bc), Txt(0x82adf4, 0x6dbc28)];
pub(super) const HEAD_90006: &[Txt] = &[Txt(0x82ae04, 0x6de7a8), Txt(0x82ae18, 0x6de820)];
pub(super) const HEAD_90007: &[Txt] = &[Txt(0x82ae2c, 0x6de790), Txt(0x82ae44, 0x6de778)];
pub(super) const HEAD_90008: &[Txt] = &[Txt(0x82ae5c, 0x6de764), Txt(0x82ae70, 0x6de820)];
pub(super) const HEAD_90009: &[Txt] = &[Txt(0x82ae84, 0x6de74c), Txt(0x82ae9c, 0x6de778)];
pub(super) const HEAD_90010: &[Txt] = &[Txt(0x82aeb4, 0x6de728), Txt(0x82aed8, 0x6de704)];
pub(super) const HEAD_90011: &[Txt] = &[Txt(0x82aefc, 0x6de6e8), Txt(0x82af18, 0x6dec28)];
pub(super) const HEAD_90012: &[Txt] = &[Txt(0x82af34, 0x6de6d4), Txt(0x82af48, 0x6de6c0)];
pub(super) const HEAD_90013: &[Txt] = &[Txt(0x82af5c, 0x6de6a8), Txt(0x82af74, 0x6db35c)];
pub(super) const HEAD_90029: &[Txt] = &[Txt(0x82af8c, 0x6de694), Txt(0x82afa0, 0x6de680)];
pub(super) const HEAD_90015: &[Txt] = &[Txt(0x82afb4, 0x6de658), Txt(0x82afdc, 0x6de630)];
pub(super) const HEAD_90031: &[Txt] = &[
    Txt(0x82b05c, 0x6de5ac),
    Txt(0x82b088, 0x6de57c),
    Txt(0x82b0b8, 0x6de54c),
];
pub(super) const HEAD_90016: &[Txt] = &[
    Txt(0x82b0e8, 0x6de530),
    Txt(0x82b104, 0x6de4fc),
    Txt(0x82b138, 0x6de4c8),
];
pub(super) const HEAD_90032: &[Txt] = &[
    Txt(0x82b16c, 0x6de4b8),
    Txt(0x82b17c, 0x6de4a4),
    Txt(0x82b190, 0x6de490),
];
pub(super) const HEAD_90035: &[Txt] = &[
    Txt(0x82b228, 0x6de3fc),
    Txt(0x82b240, 0x6de3e0),
    Txt(0x82b25c, 0x6de3c4),
];
pub(super) const HEAD_90036: &[Txt] = &[
    Txt(0x82b278, 0x6de3a8),
    Txt(0x82b294, 0x6de388),
    Txt(0x82b2b4, 0x6de368),
];
pub(super) const HEAD_90037: &[Txt] = &[
    Txt(0x82b2d4, 0x6de344),
    Txt(0x82b2f8, 0x6de310),
    Txt(0x82b32c, 0x6de2dc),
];
pub(super) const HEAD_90038: &[Txt] = &[Txt(0x82b360, 0x6de2cc), Txt(0x82b370, 0x6de2bc)];
pub(super) const HEAD_90025: &[Txt] = &[
    Txt(0x82b414, 0x6de26c),
    Txt(0x82b438, 0x6de254),
    Txt(0x82b450, 0x6de230),
];
pub(super) const HEAD_90026: &[Txt] = &[Txt(0x82b474, 0x6de21c), Txt(0x82b488, 0x6de820)];
pub(super) const HEAD_90027: &[Txt] = &[
    Txt(0x82b49c, 0x6de1e4),
    Txt(0x82b4d4, 0x6de1ac),
    Txt(0x82b50c, 0x6de174),
];
pub(super) const HEAD_90017: &[Txt] = &[Txt(0x82b648, 0x6de048), Txt(0x82b674, 0x6de01c)];
pub(super) const HEAD_90028: &[Txt] = &[Txt(0x82b6a0, 0x6de00c), Txt(0x82b6b0, 0x6ddffc)];
pub(super) const HEAD_90030: &[Txt] = &[Txt(0x82b6d8, 0x6ddfbc), Txt(0x82b700, 0x6ddf98)];
pub(super) const HEAD_90048: &[Txt] = &[Txt(0x82b76c, 0x6ddf48), Txt(0x82b778, 0x6ddf3c)];
pub(super) const HEAD_90020: &[Txt] = &[Txt(0x82b784, 0x6ddf0c), Txt(0x82b7b4, 0x6ddedc)];
pub(super) const HEAD_90022: &[Txt] = &[Txt(0x82b844, 0x6ddeac), Txt(0x82b874, 0x6db764)];
pub(super) const HEAD_90024: &[Txt] = &[Txt(0x82b904, 0x6de7a8), Txt(0x82b918, 0x6de820)];
pub(super) const HEAD_90041: &[Txt] = &[Txt(0x82b98c, 0x6dde84), Txt(0x82b9b4, 0x6dde5c)];
pub(super) const HEAD_90042: &[Txt] = &[
    Txt(0x82b9dc, 0x6dde40),
    Txt(0x82b9f8, 0x6dde20),
    Txt(0x82ba18, 0x6dddf0),
    Txt(0x82ba48, 0x6dddc0),
];
pub(super) const HEAD_90043: &[Txt] = &[Txt(0x82ba78, 0x6ddda8), Txt(0x82ba90, 0x6ddd94)];
pub(super) const HEAD_90044: &[Txt] = &[Txt(0x82baa4, 0x6ddd84), Txt(0x82bab4, 0x6dcdb8)];
pub(super) const HEAD_90045: &[Txt] = &[Txt(0x82bac4, 0x6ddd64), Txt(0x82bae4, 0x6de7cc)];
pub(super) const GIANT_NOTE: &[Txt] = &[
    Txt(0x82b548, 0x6de144),
    Txt(0x82b578, 0x6de114),
    Txt(0x82b5a8, 0x6de0e4),
    Txt(0x82b5d8, 0x6de0b4),
    Txt(0x82b608, 0x6de084),
    Txt(0x82b638, 0x6de074),
];
pub(super) const PURSE: Txt = Txt(0x82b6c0, 0x6ddfe4);
pub(super) const PURSE_NOTE: &[Txt] = &[Txt(0x82b728, 0x6ddf70), Txt(0x82b750, 0x6ddf54)];
pub(super) const TRACK_SUBTABLES: &[Txt] = &[Txt(0x82b3c4, 0x6ddd54), Txt(0x82b3d4, 0x6ddd44)];
pub(super) const TRACK_HEAD: &[Txt] = &[Txt(0x82b3e4, 0x6ddd2c), Txt(0x82b3fc, 0x6ddd14)];
pub(super) const FREQUENCY_HEAD: &[Txt] = &[Txt(0x82e0f8, 0x6df760), Txt(0x82e108, 0x6df6c8)];
pub(super) const GENERAL_TRAIT_HEAD: &[Txt] = &[Txt(0x82e134, 0x6df770), Txt(0x82e148, 0x6ddd94)];
pub(super) const SEASONS: &[Txt] = &[
    Txt(0x826e24, 0x6dbe34),
    Txt(0x826e30, 0x6dbe2c),
    Txt(0x826e38, 0x6dbe24),
];
pub(super) const SEASON_HEADS: &[&[Txt]] = &[
    &[Txt(0x825a18, 0x6db91c), Txt(0x825a38, 0x6db8fc)],
    &[Txt(0x825a58, 0x6db8e4), Txt(0x825a70, 0x6db8cc)],
    &[Txt(0x825a88, 0x6db8b4), Txt(0x825aa0, 0x6db8cc)],
];
pub(super) const VALUE_HEAD: &[Txt] = &[Txt(0x82e1d4, 0x6df784), Txt(0x82e1e0, 0x6dbbe4)];
pub(super) const RESULT_111: &[Txt] = &[Txt(0x82eb10, 0x6dcdc8), Txt(0x82eb20, 0x6dcdb8)];
pub(super) const HEIGHT: &[Txt] = &[Txt(0x8257a4, 0x6dbd74), Txt(0x825794, 0x6dbd68)];
pub(super) const WEIGHT: &[Txt] = &[Txt(0x8257e8, 0x6dbd34), Txt(0x8257d8, 0x6dbd28)];
pub(super) const SEXES: &[Txt] = &[Txt(0x826da4, 0x6c2614), Txt(0x826dac, 0x6c260c)];
pub(super) const STARTING_AGE: Txt = Txt(0x825830, 0x6dbdd8);
pub(super) const MAXIMUM_AGE: Txt = Txt(0x825864, 0x6dbda4);
pub(super) const LEADER_HEAD: &[Txt] = &[Txt(0x82de90, 0x6df6b8), Txt(0x82dea0, 0x6dcdb8)];
pub(super) const TROOPS_HEAD: &[Txt] = &[Txt(0x82deb0, 0x6df690), Txt(0x82ded8, 0x6df668)];
pub(super) const ELITE_HEAD: &[Txt] = &[Txt(0x82df00, 0x6df654), Txt(0x82df14, 0x6de820)];
pub(super) const FOLLOWERS_HEAD: &[Txt] = &[Txt(0x82df48, 0x6df6d8), Txt(0x82df58, 0x6df6c8)];
pub(super) const FOLLOWER_HEAD: &[Txt] = &[Txt(0x82df84, 0x6df718), Txt(0x82dfb4, 0x6df6e8)];
pub(super) const SKILLS_HEAD: &[Txt] = &[Txt(0x82e000, 0x6df748), Txt(0x82e018, 0x6ddd14)];
pub(super) const EVENT_HEAD: &[Txt] = &[Txt(0x82ed4c, 0x6df95c), Txt(0x82ed58, 0x6dbbe4)];
pub(super) const SOCIAL_HEAD: &[Txt] = &[Txt(0x82edbc, 0x6df968), Txt(0x82edd0, 0x6de820)];
pub(super) const KIT_HEAD: &[Txt] = &[Txt(0x82ee04, 0x6df9a8), Txt(0x82ee10, 0x6df99c)];
pub(super) const COMPANION_HEAD: &[Txt] = &[Txt(0x82ee38, 0x6df9b4), Txt(0x82ee48, 0x6df6c8)];
pub(super) const PHOBIA_HEAD: &[Txt] = &[Txt(0x82ef5c, 0x6df9ec), Txt(0x82ef6c, 0x6dcdb8)];
pub(super) const LOCATION_HEAD: &[Txt] = &[Txt(0x8261e0, 0x6db374), Txt(0x8261f8, 0x6db35c)];
pub(super) const LOCATION_TARGETS: &[Txt] = &[
    Txt(0x827044, 0x6dc1b0),
    Txt(0x82705c, 0x6dc194),
    Txt(0x827078, 0x6dbf70),
];
pub(super) const WEAPON_TARGETS: &[Txt] = &[
    Txt(0x827488, 0x6dc1b0),
    Txt(0x8274a0, 0x6dc030),
    Txt(0x8274b4, 0x6dc018),
    Txt(0x8274cc, 0x6dbff0),
    Txt(0x8274f4, 0x6dbfd4),
];
pub(super) const WEAPON_TYPES: &[Txt] = &[
    Txt(0x82743c, 0x6dc074),
    Txt(0x827458, 0x6dc05c),
    Txt(0x827470, 0x6dc044),
];
pub(super) const WEAPON_SIZE_ADVANTAGE: &[Txt] = &[
    Txt(0x827510, 0x6dc15c),
    Txt(0x827528, 0x6dc140),
    Txt(0x827544, 0x6dc124),
];
pub(super) const WEAPON_SIZES: &[Txt] = &[
    Txt(0x827560, 0x6dbfb4),
    Txt(0x827580, 0x6dab34),
    Txt(0x8275a0, 0x6dbf94),
    Txt(0x8275c0, 0x6daaf4),
];
pub(super) const CRITICAL_TITLE: Txt = Txt(0x826c4c, 0x6daad0);
pub(super) const CRITICAL_LIST: Txt = Txt(0x82db3c, 0x6df180);
pub(super) const STRIKE_HEAD: &[Txt] = &[Txt(0x826c74, 0x6db438), Txt(0x826c90, 0x6db41c)];
pub(super) const CALLED_HEAD: &[Txt] = &[Txt(0x826cac, 0x6db40c), Txt(0x826cbc, 0x6db3fc)];
pub(super) const SEVERITY_EFFECT_HEAD: &[Txt] = &[Txt(0x826cd0, 0x6db3e8), Txt(0x826ce4, 0x6db3d4)];
pub(super) const SPELL_TARGETS: &[Txt] = &[
    Txt(0x827304, 0x6dc1b0),
    Txt(0x82731c, 0x6dc194),
    Txt(0x827338, 0x6dc174),
];
pub(super) const SPELL_SIZE_ADVANTAGE: &[Txt] = &[
    Txt(0x827358, 0x6dc15c),
    Txt(0x827370, 0x6dc140),
    Txt(0x82738c, 0x6dc124),
];
pub(super) const SPELL_MAX_DAMAGE: &[Txt] = &[
    Txt(0x8273a8, 0x6dc104),
    Txt(0x8273c8, 0x6dc0e8),
    Txt(0x8273e4, 0x6dc0c8),
    Txt(0x827404, 0x6dc0a4),
];
pub(super) const SPELL_CRITICAL_TITLE: Txt = Txt(0x8262b4, 0x6db454);
pub(super) const SPELL_CRITICAL_LIST: Txt = Txt(0x82dd30, 0x6db454);
pub(super) const SPELL_STRIKE_HEAD: &[Txt] = &[Txt(0x8262d8, 0x6db438), Txt(0x8262f4, 0x6db41c)];
pub(super) const SPELL_CALLED_HEAD: &[Txt] = &[Txt(0x826310, 0x6db40c), Txt(0x826320, 0x6db3fc)];
pub(super) const SPELL_SEVERITY_HEAD: &[Txt] = &[Txt(0x826334, 0x6db3e8), Txt(0x826348, 0x6db3d4)];
pub(super) const EXTRA_LOCATION_HEAD: &[Txt] = &[
    Txt(0x82635c, 0x6db3cc),
    Txt(0x826364, 0x6db438),
    Txt(0x826380, 0x6db3b0),
];
pub(super) const RANDOM_LOCATION: Txt = Txt(0x829018, 0x6ddcec);
pub(super) const INSANITY_HEAD: &[Txt] = &[Txt(0x825fd0, 0x6db6a0), Txt(0x825ff8, 0x6db678)];
pub(super) const INSANE: &[Txt] = &[Txt(0x82602c, 0x6db670), Txt(0x826034, 0x6db668)];
pub(super) const INSANITY_NOTE: &[Txt] = &[
    Txt(0x826040, 0x6db63c),
    Txt(0x82606c, 0x6db610),
    Txt(0x826098, 0x6db5e4),
    Txt(0x8260c4, 0x6db5b4),
    Txt(0x8260f4, 0x6db5a0),
];
pub(super) const CHART_HEAD: &[Txt] = &[
    Txt(0x826150, 0x6db574),
    Txt(0x82615c, 0x6db560),
    Txt(0x826170, 0x6db54c),
];
pub(super) const CHART_NOTE: &[Txt] = &[Txt(0x826184, 0x6db524), Txt(0x8261ac, 0x6db514)];

/// Every address above, for the checks against real programs.
#[cfg(test)]
pub(super) fn all() -> Vec<Txt> {
    let mut all = Vec::new();
    all.extend_from_slice(TURN_HEAD);
    all.extend_from_slice(UNDEAD);
    all.extend_from_slice(TURN_NOTE);
    all.extend_from_slice(PRIEST_LEVELS);
    all.extend_from_slice(PALADIN_LEVELS);
    all.extend_from_slice(TURNING_CLASSES);
    all.extend_from_slice(RANKS);
    all.extend_from_slice(DISEASE_HEAD);
    all.extend_from_slice(DISEASE_ROWS);
    all.extend_from_slice(DISEASE_NOTE);
    all.extend_from_slice(CRUSH_HEAD);
    all.extend_from_slice(CRUSH_ROWS);
    all.extend_from_slice(WOUNDING_HEAD);
    all.extend_from_slice(WOUNDING_ROWS);
    all.extend_from_slice(WOUNDING_NOTE);
    all.push(FUNDS_FOR);
    all.extend_from_slice(CLASS_GROUPS);
    all.extend_from_slice(FUNDS_NOTE);
    all.extend_from_slice(CASTER_LEVELS);
    all.extend_from_slice(SPELL_FAILURE);
    all.extend_from_slice(RESULT_HEAD);
    all.extend_from_slice(SCROLL_LEVELS);
    all.extend_from_slice(SCROLL_USE);
    all.extend_from_slice(LANGUAGES_HEAD);
    all.extend_from_slice(LANGUAGES_NOTE);
    all.extend_from_slice(RARITY);
    all.extend_from_slice(FOUND_IN);
    all.extend_from_slice(CANNOT);
    all.extend_from_slice(CAN);
    all.extend_from_slice(PURCHASE_HEAD);
    all.extend_from_slice(TARGETS_AFFECTED);
    all.extend_from_slice(WHOLE_HEAD);
    all.extend_from_slice(AREAS);
    all.extend_from_slice(AREA_SIZES);
    all.extend_from_slice(MAX_DAMAGE);
    all.extend_from_slice(SEVERITY_HEAD);
    all.extend_from_slice(TRAIT_HEAD);
    all.extend_from_slice(TRAITS);
    all.extend_from_slice(GEM_HEAD);
    all.extend(GEMS.iter().flat_map(|l| l.iter().copied()));
    all.push(IOUN_TITLE);
    all.extend_from_slice(IOUN_HEAD);
    all.extend(IOUN.iter().flat_map(|l| l.iter().copied()));
    all.push(IOUN_DEAD);
    all.extend_from_slice(IOUN_MAX);
    all.extend_from_slice(IOUN_ABSORB_4);
    all.extend_from_slice(IOUN_ABSORB_8);
    all.extend_from_slice(WILD_DEVOTION_HEAD);
    all.extend_from_slice(WILD_SCIENCE_HEAD);
    all.extend_from_slice(POWER_HEAD);
    all.extend_from_slice(PURPOSE_HEAD);
    all.extend_from_slice(SPECIAL_POWER_HEAD);
    all.push(PAGES);
    all.extend_from_slice(PAGES_HEAD);
    all.extend_from_slice(BEADS);
    all.extend_from_slice(BEAD_HEAD);
    all.push(PATCHES);
    all.extend_from_slice(PATCH_HEAD);
    all.extend_from_slice(RESULT);
    all.extend_from_slice(ITEM_HEAD);
    all.extend_from_slice(JEWEL_HEAD);
    all.extend_from_slice(HORSE_HEAD);
    all.extend_from_slice(HIGH_SPIRITED_HEAD);
    all.extend_from_slice(PUNCH_HEAD);
    all.extend_from_slice(WRESTLE_HEAD);
    all.extend_from_slice(SUBTABLE_89);
    all.extend_from_slice(WIZARD_SCROLL_HEAD);
    all.extend_from_slice(PRIEST_SCROLL_HEAD);
    all.extend_from_slice(SPELL_HEAD);
    all.extend_from_slice(SUBTABLE_90);
    all.extend_from_slice(SCROLL_ITEM_HEAD);
    all.extend_from_slice(SUBTABLE_91);
    all.extend_from_slice(SUBTABLE_96);
    all.extend_from_slice(SUBTABLE_104);
    all.extend_from_slice(ARMOR_HEAD);
    all.extend_from_slice(AC_HEAD);
    all.extend_from_slice(ARMOR_TYPE_HEAD);
    all.extend_from_slice(WEAPON_HEAD);
    all.extend_from_slice(SUBTABLE_108);
    all.extend_from_slice(SWORD_HEAD);
    all.extend_from_slice(OTHER_WEAPON_HEAD);
    all.extend_from_slice(SUBTABLE_110);
    all.extend_from_slice(RESULT_88);
    all.extend_from_slice(INTELLIGENCE_HEAD);
    all.extend_from_slice(ALIGNMENT_HEAD);
    all.extend_from_slice(ABILITY_HEAD);
    all.extend_from_slice(SPECIAL_POWER_HEAD_B);
    all.extend_from_slice(HEAD_90001);
    all.extend_from_slice(HEAD_90002);
    all.extend_from_slice(HEAD_90003);
    all.extend_from_slice(HEAD_90004);
    all.extend_from_slice(HEAD_90005);
    all.extend_from_slice(HEAD_90006);
    all.extend_from_slice(HEAD_90007);
    all.extend_from_slice(HEAD_90008);
    all.extend_from_slice(HEAD_90009);
    all.extend_from_slice(HEAD_90010);
    all.extend_from_slice(HEAD_90011);
    all.extend_from_slice(HEAD_90012);
    all.extend_from_slice(HEAD_90013);
    all.extend_from_slice(HEAD_90029);
    all.extend_from_slice(HEAD_90015);
    all.extend_from_slice(HEAD_90031);
    all.extend_from_slice(HEAD_90016);
    all.extend_from_slice(HEAD_90032);
    all.extend_from_slice(HEAD_90035);
    all.extend_from_slice(HEAD_90036);
    all.extend_from_slice(HEAD_90037);
    all.extend_from_slice(HEAD_90038);
    all.extend_from_slice(HEAD_90025);
    all.extend_from_slice(HEAD_90026);
    all.extend_from_slice(HEAD_90027);
    all.extend_from_slice(HEAD_90017);
    all.extend_from_slice(HEAD_90028);
    all.extend_from_slice(HEAD_90030);
    all.extend_from_slice(HEAD_90048);
    all.extend_from_slice(HEAD_90020);
    all.extend_from_slice(HEAD_90022);
    all.extend_from_slice(HEAD_90024);
    all.extend_from_slice(HEAD_90041);
    all.extend_from_slice(HEAD_90042);
    all.extend_from_slice(HEAD_90043);
    all.extend_from_slice(HEAD_90044);
    all.extend_from_slice(HEAD_90045);
    all.extend_from_slice(GIANT_NOTE);
    all.push(PURSE);
    all.extend_from_slice(PURSE_NOTE);
    all.extend_from_slice(TRACK_SUBTABLES);
    all.extend_from_slice(TRACK_HEAD);
    all.extend_from_slice(FREQUENCY_HEAD);
    all.extend_from_slice(GENERAL_TRAIT_HEAD);
    all.extend_from_slice(SEASONS);
    all.extend(SEASON_HEADS.iter().flat_map(|l| l.iter().copied()));
    all.extend_from_slice(VALUE_HEAD);
    all.extend_from_slice(RESULT_111);
    all.extend_from_slice(HEIGHT);
    all.extend_from_slice(WEIGHT);
    all.extend_from_slice(SEXES);
    all.push(STARTING_AGE);
    all.push(MAXIMUM_AGE);
    all.extend_from_slice(LEADER_HEAD);
    all.extend_from_slice(TROOPS_HEAD);
    all.extend_from_slice(ELITE_HEAD);
    all.extend_from_slice(FOLLOWERS_HEAD);
    all.extend_from_slice(FOLLOWER_HEAD);
    all.extend_from_slice(SKILLS_HEAD);
    all.extend_from_slice(EVENT_HEAD);
    all.extend_from_slice(SOCIAL_HEAD);
    all.extend_from_slice(KIT_HEAD);
    all.extend_from_slice(COMPANION_HEAD);
    all.extend_from_slice(PHOBIA_HEAD);
    all.extend_from_slice(LOCATION_HEAD);
    all.extend_from_slice(LOCATION_TARGETS);
    all.extend_from_slice(WEAPON_TARGETS);
    all.extend_from_slice(WEAPON_TYPES);
    all.extend_from_slice(WEAPON_SIZE_ADVANTAGE);
    all.extend_from_slice(WEAPON_SIZES);
    all.push(CRITICAL_TITLE);
    all.push(CRITICAL_LIST);
    all.extend_from_slice(STRIKE_HEAD);
    all.extend_from_slice(CALLED_HEAD);
    all.extend_from_slice(SEVERITY_EFFECT_HEAD);
    all.extend_from_slice(SPELL_TARGETS);
    all.extend_from_slice(SPELL_SIZE_ADVANTAGE);
    all.extend_from_slice(SPELL_MAX_DAMAGE);
    all.push(SPELL_CRITICAL_TITLE);
    all.push(SPELL_CRITICAL_LIST);
    all.extend_from_slice(SPELL_STRIKE_HEAD);
    all.extend_from_slice(SPELL_CALLED_HEAD);
    all.extend_from_slice(SPELL_SEVERITY_HEAD);
    all.extend_from_slice(EXTRA_LOCATION_HEAD);
    all.push(RANDOM_LOCATION);
    all.extend_from_slice(INSANITY_HEAD);
    all.extend_from_slice(INSANE);
    all.extend_from_slice(INSANITY_NOTE);
    all.extend_from_slice(CHART_HEAD);
    all.extend_from_slice(CHART_NOTE);
    all
}
