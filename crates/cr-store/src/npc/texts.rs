//! Addresses of the program texts the NPC generator compares with or
//! copies: (Tsrp3.exe, Evermore.exe), 0 where Evermore has none. The
//! texts themselves are read from the installation's program at run time.

use super::program::Tx;

pub const AARAKOCRA: Tx = Tx(0x820924, 0x6c451c);
pub const ANCIENT_HISTORY: Tx = Tx(0x816edc, 0x6c2abc);
pub const ANIMAL: Tx = Tx(0x821b48, 0x6c43a8);
pub const ANIMAL_TRAINING: Tx = Tx(0x816d74, 0x6cd29c);
pub const BACKPACK: Tx = Tx(0x81fb3c, 0x6c5034);
pub const BARD: Tx = Tx(0x821368, 0x0);
pub const BELT_POUCH_LARGE: Tx = Tx(0x8208e0, 0x6d6a6c);
pub const BELT_POUCH_SMALL: Tx = Tx(0x820818, 0x6d6a10);
pub const BIT_AND_BRIDLE: Tx = Tx(0x8219f0, 0x0);
pub const BLENDING: Tx = Tx(0x820958, 0x0);
pub const BLIND_FIGHTING: Tx = Tx(0x811368, 0x6c2b40);
pub const BLOCK_AND_TACKLE: Tx = Tx(0x820a40, 0x6d6ae0);
pub const BOAT_PILOTING: Tx = Tx(0x820bb0, 0x6d6b88);
pub const BOLT_CASE: Tx = Tx(0x821934, 0x0);
pub const BOOTS_SOFT: Tx = Tx(0x820c00, 0x6d6b58);
pub const BRAIES: Tx = Tx(0x820860, 0x0);
pub const BREECHES: Tx = Tx(0x820724, 0x6d6a4c);
pub const BRIBE: Tx = Tx(0x8395b0, 0x6c3290);
pub const CANDLE: Tx = Tx(0x8206c4, 0x6d6920);
pub const CANNONS: Tx = Tx(0x820730, 0x0);
pub const CASSOCK: Tx = Tx(0x820a60, 0x6d6acc);
pub const CAVALIER: Tx = Tx(0x81089c, 0x6c2140);
pub const CENTAUR: Tx = Tx(0x82198c, 0x0);
pub const CHAINSE_CHEMISE: Tx = Tx(0x820908, 0x0);
pub const CHAOTIC_EVIL: Tx = Tx(0x80ea68, 0x6c1710);
pub const CHAOTIC_GOOD: Tx = Tx(0x80ea3c, 0x6c16f0);
pub const CHAOTIC_NEUTRAL: Tx = Tx(0x80eab0, 0x6c174c);
pub const CHARISMA: Tx = Tx(0x811160, 0x6c2768);
pub const CLAIRSENTIENT: Tx = Tx(0x81e274, 0x6c6034);
pub const CLIMB_WALLS: Tx = Tx(0x839574, 0x6c32c8);
pub const CLOAK_OF_PROTECTION_1: Tx = Tx(0x820e18, 0x0);
pub const CLOTHING: Tx = Tx(0x821b20, 0x6c43b8);
pub const COBBLING: Tx = Tx(0x816dbc, 0x6cd258);
pub const COMMON: Tx = Tx(0x8170d8, 0x6cd2f0);
pub const COMMON_GOWN: Tx = Tx(0x8207c8, 0x6d6a60);
pub const COMMON_ROBE: Tx = Tx(0x820868, 0x6d6a38);
pub const CONSTITUTION: Tx = Tx(0x811138, 0x6c278c);
pub const COPPER_PIECES: Tx = Tx(0x8177b8, 0x6c48b4);
pub const CRAMPONS: Tx = Tx(0x820c1c, 0x6d6b4c);
pub const DART: Tx = Tx(0x821448, 0x0);
pub const DART_SP: Tx = Tx(0x821440, 0x0);
pub const DAVID: Tx = Tx(0x81f288, 0x6d5ce4);
pub const DETECT_ILLUSION: Tx = Tx(0x8395a0, 0x6c3298);
pub const DETECT_MAGIC: Tx = Tx(0x839590, 0x6c32a8);
pub const DETECT_NOISE: Tx = Tx(0x839564, 0x6c32d4);
pub const DEXTERITY: Tx = Tx(0x81112c, 0x6c279c);
pub const DISGUISE: Tx = Tx(0x816b54, 0x6cbf28);
pub const DOG_GUARD: Tx = Tx(0x820c38, 0x0);
pub const DOG_HUNTING: Tx = Tx(0x820c50, 0x0);
pub const DOG_WAR: Tx = Tx(0x820c44, 0x0);
pub const DROW: Tx = Tx(0x820964, 0x0);
pub const DRY_RATIONS_1_WEEK: Tx = Tx(0x820660, 0x6c5010);
pub const DWARF: Tx = Tx(0x810904, 0x6c215c);
pub const DWARVEN: Tx = Tx(0x820974, 0x0);
pub const DWARVEN_EQUIPMENT: Tx = Tx(0x820594, 0x6d68ec);
pub const ELF: Tx = Tx(0x816d4c, 0x6ccfe0);
pub const ELVEN_EQUIPMENT: Tx = Tx(0x8205d0, 0x6d68c8);
pub const EMBROIDERED_ROBE: Tx = Tx(0x820748, 0x6d69ec);
pub const ENGINEERING: Tx = Tx(0x816f60, 0x6c2b0c);
pub const ESCAPING_BONDS: Tx = Tx(0x8395c4, 0x6c3274);
pub const EXPLORER: Tx = Tx(0x820cb0, 0x6d6b30);
pub const FAITHFUL_MOUNT: Tx = Tx(0x821370, 0x0);
pub const FALCONRY_CRE: Tx = Tx(0x820c70, 0x0);
pub const FALCON_TRAINED: Tx = Tx(0x820c80, 0x6d6b3c);
pub const FERRONNIERE: Tx = Tx(0x820778, 0x6d69c4);
pub const FIND_REMOVE_TRAPS: Tx = Tx(0x839530, 0x6c3304);
pub const FINE_FUR_CLOAK: Tx = Tx(0x820738, 0x0);
pub const FISHHOOK: Tx = Tx(0x820bd0, 0x6d6b7c);
pub const FISHING: Tx = Tx(0x816e04, 0x6cd214);
pub const FISHING_NET_10_FT_SQ: Tx = Tx(0x820bdc, 0x0);
pub const FLINT_AND_STEEL: Tx = Tx(0x820678, 0x6c4ff0);
pub const FOOD: Tx = Tx(0x821b74, 0x6c43b0);
pub const GARNACHE: Tx = Tx(0x820710, 0x6d6a90);
pub const GIPSER: Tx = Tx(0x8207b4, 0x6d6984);
pub const GLASS_BOTTLE: Tx = Tx(0x820d10, 0x6d6c4c);
pub const GNOME: Tx = Tx(0x816d38, 0x6ccff0);
pub const GNOMISH: Tx = Tx(0x820984, 0x0);
pub const GNOMISH_EQUIPMENT: Tx = Tx(0x8205e4, 0x6d68b4);
pub const GOLD_PIECES: Tx = Tx(0x8177d8, 0x6c4880);
pub const GOOD_CLOTH_CLOAK: Tx = Tx(0x820804, 0x6d6a24);
pub const GORGET: Tx = Tx(0x820764, 0x6d69dc);
pub const GOWN_COMMON: Tx = Tx(0x8208a8, 0x6d6a80);
pub const GRAPPLING_HOOK: Tx = Tx(0x820a68, 0x6d6abc);
pub const GRINKA: Tx = Tx(0x81f2a8, 0x6d62f4);
pub const HAIRFOOT_HALFLING: Tx = Tx(0x820118, 0x0);
pub const HALFLING: Tx = Tx(0x816d40, 0x6ccfe4);
pub const HALFLING_EQUIPMENT: Tx = Tx(0x8205b0, 0x6d68d8);
pub const HALF_ELF: Tx = Tx(0x82014c, 0x6c232c);
pub const HALF_OGRE: Tx = Tx(0x820140, 0x6c2338);
pub const HALF_ORC: Tx = Tx(0x820134, 0x6c2344);
pub const HEAL: Tx = Tx(0x815d74, 0x6cca40);
pub const HEALER_S_BAG: Tx = Tx(0x820afc, 0x6d6c3c);
pub const HEAVY: Tx = Tx(0x8213cc, 0x0);
pub const HERBALISM: Tx = Tx(0x816f90, 0x6c2ae8);
pub const HERBALIST: Tx = Tx(0x820b38, 0x6d6bf4);
pub const HERBS_PER_LB: Tx = Tx(0x820b44, 0x6d6be4);
pub const HIDE_IN_SHADOWS: Tx = Tx(0x839554, 0x6c32e4);
pub const HOLY_ITEM: Tx = Tx(0x820aa0, 0x6c4fe4);
pub const HORSEMAN: Tx = Tx(0x821420, 0x0);
pub const HORSE_HEAVY_WAR: Tx = Tx(0x821380, 0x0);
pub const HORSE_LIGHT_WAR: Tx = Tx(0x8213a8, 0x0);
pub const HORSE_MEDIUM_WAR: Tx = Tx(0x821394, 0x0);
pub const HORSE_RIDING: Tx = Tx(0x820ca0, 0x0);
pub const HOUSEHOLD: Tx = Tx(0x821b2c, 0x6c4384);
pub const HUMAN: Tx = Tx(0x811578, 0x6c2350);
pub const HUMAN_REGIONAL_DIALECT: Tx = Tx(0x82016c, 0x0);
pub const INTELLIGENCE: Tx = Tx(0x811150, 0x6c2774);
pub const JESTER: Tx = Tx(0x820cc8, 0x6d6b1c);
pub const KNIFE_SHEATH: Tx = Tx(0x821900, 0x0);
pub const KOBOLD: Tx = Tx(0x81711c, 0x6cd2cc);
pub const LANCE: Tx = Tx(0x8213c4, 0x6d5ba4);
pub const LANTERN_HOODED: Tx = Tx(0x820694, 0x6d6940);
pub const LAWFUL_EVIL: Tx = Tx(0x80ea4c, 0x6c1730);
pub const LAWFUL_GOOD: Tx = Tx(0x80e8fc, 0x6c15d8);
pub const LAWFUL_NEUTRAL: Tx = Tx(0x80ea84, 0x6c173c);
pub const LIGHT: Tx = Tx(0x821404, 0x0);
pub const LIRIPIPE: Tx = Tx(0x820cd0, 0x6d6b10);
pub const LOUISE: Tx = Tx(0x81f298, 0x6d62dc);
pub const LOWER_CLASS: Tx = Tx(0x8108a8, 0x6c2134);
pub const MEDIUM: Tx = Tx(0x8213e8, 0x0);
pub const MIDDLE_LOWER_CLASS: Tx = Tx(0x820084, 0x6d68a0);
pub const MISC: Tx = Tx(0x821b40, 0x6c4390);
pub const MISSILE_ATT: Tx = Tx(0x82094c, 0x6d6a9c);
pub const MISSILE_ATTRACTION: Tx = Tx(0x820938, 0x6d6aa8);
pub const MODERN_LANGUAGES: Tx = Tx(0x811a98, 0x6cd434);
pub const MOUNTAINEERING: Tx = Tx(0x816fdc, 0x6cd07c);
pub const MOVE_SILENTLY: Tx = Tx(0x839544, 0x6c32f4);
pub const MUGWORT: Tx = Tx(0x81f2a0, 0x6d62fc);
pub const MUSICAL_INSTRUMENT_SMALL: Tx = Tx(0x820aac, 0x6d6c20);
pub const NAVIGATION: Tx = Tx(0x817000, 0x6cd05c);
pub const NEUTRAL_EVIL: Tx = Tx(0x80ea58, 0x6c1720);
pub const NEUTRAL_GOOD: Tx = Tx(0x80ea2c, 0x6c1700);
pub const NORMA: Tx = Tx(0x81f290, 0x6d62e4);
pub const OF_BLENDING: Tx = Tx(0x820a34, 0x0);
pub const OIL_GREEK_FIRE_FLASK: Tx = Tx(0x8206ec, 0x0);
pub const OIL_LAMP_PER_FLASK: Tx = Tx(0x8206a4, 0x6d6928);
pub const ONLY_FROM_LIST_BELOW: Tx = Tx(0x84c814, 0x0);
/// The error when no class is given.
pub const NO_CLASS: Tx = Tx(0x81fa3c, 0x6d6808);
pub const OPEN_LOCKS: Tx = Tx(0x82134c, 0x0);
pub const OPEN_LOCKS_UC: Tx = Tx(0x839524, 0x6c3318);
pub const PAPER_PER_SHEET: Tx = Tx(0x820b9c, 0x6d6b98);
pub const PERFUME_PER_VIAL: Tx = Tx(0x8207a0, 0x0);
pub const PICK_POCKETS: Tx = Tx(0x839514, 0x6c3324);
pub const PIGEON_HOMING: Tx = Tx(0x820c60, 0x0);
pub const PIN: Tx = Tx(0x82076c, 0x6d69d8);
pub const PONY: Tx = Tx(0x8219a8, 0x0);
pub const PSYCHOKINETIC: Tx = Tx(0x81e284, 0x6c6024);
pub const PSYCHOMETABOLIC: Tx = Tx(0x81e294, 0x6c6014);
pub const PSYCHOPORTIVE: Tx = Tx(0x81e2a4, 0x6c6004);
pub const QUIVER: Tx = Tx(0x821910, 0x0);
pub const RATIONS_IRON_1_WEEK: Tx = Tx(0x820648, 0x6d6950);
pub const RATIONS_STANDARD_1_WEEK: Tx = Tx(0x820618, 0x6d6968);
pub const READING_WRITING: Tx = Tx(0x81700c, 0x6c2ad8);
pub const READ_LANGUAGES: Tx = Tx(0x839580, 0x6c32b8);
pub const READ_MAGIC: Tx = Tx(0x813a50, 0x6cbb0c);
pub const RELIGION: Tx = Tx(0x81702c, 0x6c2b00);
pub const RIDING_LAND: Tx = Tx(0x820c90, 0x0);
pub const ROBE_OF_THE_ARCHMAGI: Tx = Tx(0x820d20, 0x0);
pub const ROBE_OF_THE_ARCHMAGI_EVIL: Tx = Tx(0x820dfc, 0x0);
pub const ROBE_OF_THE_ARCHMAGI_GOOD: Tx = Tx(0x820d64, 0x0);
pub const ROBE_OF_THE_ARCHMAGI_NEUTRAL: Tx = Tx(0x820db0, 0x0);
pub const ROPE_SILK_50_FT: Tx = Tx(0x820b60, 0x6d6bc4);
pub const ROPE_USE: Tx = Tx(0x820b54, 0x6d6bd8);
pub const RUFF: Tx = Tx(0x820770, 0x6d69d0);
pub const RUNNING: Tx = Tx(0x817038, 0x6c2b38);
pub const SADDLE_BLANKET: Tx = Tx(0x821a00, 0x0);
pub const SADDLE_RIDING: Tx = Tx(0x8219e0, 0x0);
pub const SASH: Tx = Tx(0x82075c, 0x6d69e4);
pub const SCABBARD_HANGER_BALDRIC: Tx = Tx(0x821918, 0x0);
pub const SCHOLAR: Tx = Tx(0x820cdc, 0x6d6b08);
pub const SEA_ELF: Tx = Tx(0x82096c, 0x0);
pub const SERVICES: Tx = Tx(0x817510, 0x6c3be0);
pub const SEWING_NEEDLE: Tx = Tx(0x820b14, 0x6d6c08);
pub const SEXTANT: Tx = Tx(0x820b30, 0x6d6c00);
pub const SHIRT: Tx = Tx(0x82071c, 0x6d6a58);
pub const SIGNAL_WHISTLE: Tx = Tx(0x8206cc, 0x6d6910);
pub const SIGNET_RING_PERSONAL_SEAL: Tx = Tx(0x820784, 0x6d6994);
pub const SILVER_PIECES: Tx = Tx(0x8177c8, 0x6c48a4);
pub const SLOP: Tx = Tx(0x820900, 0x0);
pub const SPELLBOOK: Tx = Tx(0x822160, 0x6c7094);
pub const SPELLCRAFT: Tx = Tx(0x81704c, 0x6c2af4);
pub const SPYGLASS: Tx = Tx(0x820cbc, 0x6d6b24);
pub const STANDARD_HALF_ELF: Tx = Tx(0x820158, 0x0);
pub const STOUT_HALFLING: Tx = Tx(0x820108, 0x0);
pub const STRENGTH: Tx = Tx(0x811120, 0x6c27a8);
pub const SWANMAY: Tx = Tx(0x81f268, 0x6d61cc);
pub const TACK: Tx = Tx(0x821b38, 0x6c437c);
pub const TAILOR: Tx = Tx(0x820b0c, 0x6d6c18);
pub const TELEPATHIC: Tx = Tx(0x81e2b4, 0x6c5ff8);
pub const THE_SEP: Tx = Tx(0x81f8e8, 0x6d66a8);
pub const THIEVES_PICKS: Tx = Tx(0x820a78, 0x6c5000);
pub const TIGHTROPE_WALKING: Tx = Tx(0x817064, 0x6cd020);
pub const TORCH: Tx = Tx(0x8206bc, 0x6c4fdc);
pub const TRANSPORTATION: Tx = Tx(0x821b7c, 0x6c4398);
pub const TRANSPORTATION_ANIMALS: Tx = Tx(0x821b50, 0x6c4364);
pub const TRANSPORTATION_WATERCRAFT: Tx = Tx(0x821ba4, 0x6c4664);
pub const TRANSPORTATION_WHEELED: Tx = Tx(0x821b8c, 0x6c4680);
pub const TRUE_NEUTRAL: Tx = Tx(0x817388, 0x6c52fc);
pub const TUMBLING: Tx = Tx(0x80edb4, 0x6c2b2c);
pub const TUNNELING: Tx = Tx(0x8395b8, 0x6c3284);
pub const UPPER_CLASS: Tx = Tx(0x81fb24, 0x6c52b0);
pub const WEAPON: Tx = Tx(0x821b18, 0x6c36c8);
pub const WEAPONBLACK: Tx = Tx(0x820ad4, 0x6d6af4);
pub const WEMIC: Tx = Tx(0x821994, 0x0);
pub const WHETSTONE: Tx = Tx(0x820a54, 0x6d6ad4);
pub const WINESKIN: Tx = Tx(0x820600, 0x6c5028);
pub const WINTER_BLANKET: Tx = Tx(0x8206dc, 0x6d6900);
pub const WISDOM: Tx = Tx(0x811148, 0x6c2784);
pub const WRITING_INK_PER_VIAL: Tx = Tx(0x820b84, 0x6d6bac);

/// Every text above, read when the program is loaded.
pub const ALL: &[Tx] = &[
    AARAKOCRA,
    ANCIENT_HISTORY,
    ANIMAL,
    ANIMAL_TRAINING,
    BACKPACK,
    BARD,
    BELT_POUCH_LARGE,
    BELT_POUCH_SMALL,
    BIT_AND_BRIDLE,
    BLENDING,
    BLIND_FIGHTING,
    BLOCK_AND_TACKLE,
    BOAT_PILOTING,
    BOLT_CASE,
    BOOTS_SOFT,
    BRAIES,
    BREECHES,
    BRIBE,
    CANDLE,
    CANNONS,
    CASSOCK,
    CAVALIER,
    CENTAUR,
    CHAINSE_CHEMISE,
    CHAOTIC_EVIL,
    CHAOTIC_GOOD,
    CHAOTIC_NEUTRAL,
    CHARISMA,
    CLAIRSENTIENT,
    CLIMB_WALLS,
    CLOAK_OF_PROTECTION_1,
    CLOTHING,
    COBBLING,
    COMMON,
    COMMON_GOWN,
    COMMON_ROBE,
    CONSTITUTION,
    COPPER_PIECES,
    CRAMPONS,
    DART,
    DART_SP,
    DAVID,
    DETECT_ILLUSION,
    DETECT_MAGIC,
    DETECT_NOISE,
    DEXTERITY,
    DISGUISE,
    DOG_GUARD,
    DOG_HUNTING,
    DOG_WAR,
    DROW,
    DRY_RATIONS_1_WEEK,
    DWARF,
    DWARVEN,
    DWARVEN_EQUIPMENT,
    ELF,
    ELVEN_EQUIPMENT,
    EMBROIDERED_ROBE,
    ENGINEERING,
    ESCAPING_BONDS,
    EXPLORER,
    FAITHFUL_MOUNT,
    FALCONRY_CRE,
    FALCON_TRAINED,
    FERRONNIERE,
    FIND_REMOVE_TRAPS,
    FINE_FUR_CLOAK,
    FISHHOOK,
    FISHING,
    FISHING_NET_10_FT_SQ,
    FLINT_AND_STEEL,
    FOOD,
    GARNACHE,
    GIPSER,
    GLASS_BOTTLE,
    GNOME,
    GNOMISH,
    GNOMISH_EQUIPMENT,
    GOLD_PIECES,
    GOOD_CLOTH_CLOAK,
    GORGET,
    GOWN_COMMON,
    GRAPPLING_HOOK,
    GRINKA,
    HAIRFOOT_HALFLING,
    HALFLING,
    HALFLING_EQUIPMENT,
    HALF_ELF,
    HALF_OGRE,
    HALF_ORC,
    HEAL,
    HEALER_S_BAG,
    HEAVY,
    HERBALISM,
    HERBALIST,
    HERBS_PER_LB,
    HIDE_IN_SHADOWS,
    HOLY_ITEM,
    HORSEMAN,
    HORSE_HEAVY_WAR,
    HORSE_LIGHT_WAR,
    HORSE_MEDIUM_WAR,
    HORSE_RIDING,
    HOUSEHOLD,
    HUMAN,
    HUMAN_REGIONAL_DIALECT,
    INTELLIGENCE,
    JESTER,
    KNIFE_SHEATH,
    KOBOLD,
    LANCE,
    LANTERN_HOODED,
    LAWFUL_EVIL,
    LAWFUL_GOOD,
    LAWFUL_NEUTRAL,
    LIGHT,
    LIRIPIPE,
    LOUISE,
    LOWER_CLASS,
    MEDIUM,
    MIDDLE_LOWER_CLASS,
    MISC,
    MISSILE_ATT,
    MISSILE_ATTRACTION,
    MODERN_LANGUAGES,
    MOUNTAINEERING,
    MOVE_SILENTLY,
    MUGWORT,
    MUSICAL_INSTRUMENT_SMALL,
    NAVIGATION,
    NEUTRAL_EVIL,
    NEUTRAL_GOOD,
    NORMA,
    OF_BLENDING,
    OIL_GREEK_FIRE_FLASK,
    OIL_LAMP_PER_FLASK,
    ONLY_FROM_LIST_BELOW,
    NO_CLASS,
    OPEN_LOCKS,
    OPEN_LOCKS_UC,
    PAPER_PER_SHEET,
    PERFUME_PER_VIAL,
    PICK_POCKETS,
    PIGEON_HOMING,
    PIN,
    PONY,
    PSYCHOKINETIC,
    PSYCHOMETABOLIC,
    PSYCHOPORTIVE,
    QUIVER,
    RATIONS_IRON_1_WEEK,
    RATIONS_STANDARD_1_WEEK,
    READING_WRITING,
    READ_LANGUAGES,
    READ_MAGIC,
    RELIGION,
    RIDING_LAND,
    ROBE_OF_THE_ARCHMAGI,
    ROBE_OF_THE_ARCHMAGI_EVIL,
    ROBE_OF_THE_ARCHMAGI_GOOD,
    ROBE_OF_THE_ARCHMAGI_NEUTRAL,
    ROPE_SILK_50_FT,
    ROPE_USE,
    RUFF,
    RUNNING,
    SADDLE_BLANKET,
    SADDLE_RIDING,
    SASH,
    SCABBARD_HANGER_BALDRIC,
    SCHOLAR,
    SEA_ELF,
    SERVICES,
    SEWING_NEEDLE,
    SEXTANT,
    SHIRT,
    SIGNAL_WHISTLE,
    SIGNET_RING_PERSONAL_SEAL,
    SILVER_PIECES,
    SLOP,
    SPELLBOOK,
    SPELLCRAFT,
    SPYGLASS,
    STANDARD_HALF_ELF,
    STOUT_HALFLING,
    STRENGTH,
    SWANMAY,
    TACK,
    TAILOR,
    TELEPATHIC,
    THE_SEP,
    THIEVES_PICKS,
    TIGHTROPE_WALKING,
    TORCH,
    TRANSPORTATION,
    TRANSPORTATION_ANIMALS,
    TRANSPORTATION_WATERCRAFT,
    TRANSPORTATION_WHEELED,
    TRUE_NEUTRAL,
    TUMBLING,
    TUNNELING,
    UPPER_CLASS,
    WEAPON,
    WEAPONBLACK,
    WEMIC,
    WHETSTONE,
    WINESKIN,
    WINTER_BLANKET,
    WISDOM,
    WRITING_INK_PER_VIAL,
];
