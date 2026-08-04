//! The Item Text parser: clipboard text in, a structured item out.
//!
//! Two properties carry the safety of everything downstream.
//!
//! **Only the explicit-mod section is ever consulted.** It is located structurally — the section
//! after the one holding `Item Level:` — and not by leaning on the `Prefix Modifier` annotation,
//! because that annotation is a client setting. Without section awareness a corrupted implicit or an
//! enchantment could read as a Hit ([ADR 0002](../../../docs/adr/0002-roll-cycle-and-hit-latch.md)).
//!
//! **The same Item Text parses identically with Advanced Mod Descriptions on and off.** Mods are
//! grouped by which Mod Group their lines belong to, never by the annotation, so removing every
//! annotation and every inline `(lo-hi)` bound changes nothing but the annotation field.
//!
//! Anything in the explicit-mod section that is neither an annotation, nor a parenthesised
//! description, nor a line the pool recognises is recorded as [`Item::unrecognised`]. It is not
//! silently dropped: a mod line we fail to recognise is exactly how a Hit would be misread as a
//! Miss, so it has to reach a Verdict as an anomaly.

use crate::pool::{Generation, ModPool};

/// Why an Item Text could not be read.
///
/// Every variant means the app does not know what the item is, which is an `Unknown` Verdict and a
/// Resync — never a Roll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreadable {
    /// The text does not look like a PoE item at all — no `Item Class:` line.
    NotItemText,
    /// No `Rarity:` line, or one naming a rarity this app does not know.
    Rarity(String),
    /// No `Item Level:` line, or one whose number will not parse.
    ItemLevel(String),
    /// The item has no name line.
    NoName,
    /// The Base declares implicit modifiers, which would sit between `Item Level:` and the explicit
    /// mods and shift the section the parser reads.
    ///
    /// Ghastly Eye Jewel has none. This exists so that the day a Base with implicits is added, the
    /// app refuses to read rather than quietly assessing the wrong section.
    ImplicitsUnsupported {
        /// How many the pool says the Base has.
        count: usize,
    },
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotItemText => write!(f, "not item text — no `Item Class:` line"),
            Self::Rarity(detail) => write!(f, "cannot read the rarity: {detail}"),
            Self::ItemLevel(detail) => write!(f, "cannot read the item level: {detail}"),
            Self::NoName => write!(f, "the item has no name line"),
            Self::ImplicitsUnsupported { count } => write!(
                f,
                "this base has {count} implicit modifier(s); the parser only handles bases with none"
            ),
        }
    }
}

impl std::error::Error for Unreadable {}

/// An item's rarity, which fixes how many mods it can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    /// No modifiers. An alteration cannot be applied.
    Normal,
    /// Up to one prefix and one suffix. The only rarity a Craft Session ever works on.
    Magic,
    /// Up to two of each. An alteration cannot be applied.
    Rare,
    /// Not craftable with alterations.
    Unique,
}

impl std::fmt::Display for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The game's own word, so a log line reads back as the text it came from.
        f.write_str(match self {
            Self::Normal => "Normal",
            Self::Magic => "Magic",
            Self::Rare => "Rare",
            Self::Unique => "Unique",
        })
    }
}

impl Rarity {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "Normal" => Some(Self::Normal),
            "Magic" => Some(Self::Magic),
            "Rare" => Some(Self::Rare),
            "Unique" => Some(Self::Unique),
            _ => None,
        }
    }
}

/// What a Craft Session pins about the item in front of it.
///
/// Compared read-to-read to catch the human hovering something else — the wrong-item `Halt`.
/// `Requirements: Level:` is deliberately absent: it moves with whichever mods rolled, so pinning it
/// would make every Roll look like a different item. Some captures omit that section entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemIdentity {
    rarity: Rarity,
    item_class: String,
    base_name: String,
    item_level: u32,
}

impl ItemIdentity {
    /// The rarity.
    pub fn rarity(&self) -> Rarity {
        self.rarity
    }

    /// The item class as the game prints it — `Abyss Jewels`.
    pub fn item_class(&self) -> &str {
        &self.item_class
    }

    /// The Base's name, with the affix decoration stripped — `Ghastly Eye Jewel`.
    ///
    /// When the name line does not contain the pool's Base at all, this is the whole name line
    /// instead, so that a different Base is loudly different rather than silently equal.
    pub fn base_name(&self) -> &str {
        &self.base_name
    }

    /// The item level, which gates which tiers can spawn at all.
    pub fn item_level(&self) -> u32 {
        self.item_level
    }
}

/// The game's `{ Prefix Modifier "Annealed" (Tier: 4) — Damage, Physical, Minion }` line.
///
/// Present only with **Advanced Mod Descriptions** on. Read so it can be logged and cross-checked
/// against the tier the numbers imply; it never reaches a Verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    generation: Generation,
    affix_name: String,
    tier: u8,
    tags: Vec<String>,
}

impl Annotation {
    /// Prefix or suffix, per the game.
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// The name the game gives *this tier* of the group — `Annealed`, `of Order`.
    pub fn affix_name(&self) -> &str {
        &self.affix_name
    }

    /// The tier the game claims. Logged, cross-checked, never trusted.
    pub fn tier(&self) -> u8 {
        self.tier
    }

    /// The tag list, empty when the game prints none.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Read one annotation line, or `None` if it is not one.
    ///
    /// Mirrors `matching.mod_info_line` in `data/ghastly-eye-jewel.json`:
    /// `{ <type> "<name>" (Tier: <n>) — <tags> }`, with the name, tier and tags each optional.
    fn parse(line: &str) -> Option<Self> {
        let inner = line.strip_prefix('{')?.strip_suffix('}')?.trim();

        let generation = if inner.starts_with("Prefix") {
            Generation::Prefix
        } else if inner.starts_with("Suffix") {
            Generation::Suffix
        } else {
            return None;
        };

        let affix_name = inner
            .split_once('"')
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(name, _)| name.to_string())
            .unwrap_or_default();

        let tier = inner
            .split_once("(Tier: ")
            .and_then(|(_, rest)| rest.split_once(')'))
            .and_then(|(n, _)| n.trim().parse().ok())?;

        // The em-dash and everything after it is the tag list. `of Instinct` has neither.
        let tags = inner
            .split_once('—')
            .map(|(_, tags)| {
                tags.split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            generation,
            affix_name,
            tier,
            tags,
        })
    }
}

/// One modifier as it appears on the item: the lines it rendered, the values rolled into them, and
/// the Mod Group they belong to.
///
/// A mod is usually one line, but not always — `of Training` renders both
/// `Minions have #% increased Attack Speed` and `Minions have #% increased Cast Speed`, and its two
/// values must be assessed together against the tier's two bands.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMod {
    group_id: String,
    generation: Generation,
    lines: Vec<String>,
    match_strings: Vec<String>,
    values: Vec<f64>,
    annotation: Option<Annotation>,
}

impl ParsedMod {
    /// The Mod Group this mod belongs to — `MinionAddedPhysicalDamage`.
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    /// Prefix or suffix, per the pool rather than per the annotation.
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// The rendered lines, verbatim.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The rendered lines with every value replaced by `#`.
    pub fn match_strings(&self) -> &[String] {
        &self.match_strings
    }

    /// The values rolled into this mod, in the order they were printed.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The game's annotation, if Advanced Mod Descriptions was on.
    pub fn annotation(&self) -> Option<&Annotation> {
        self.annotation.as_ref()
    }
}

/// A parsed item: what it is, and what is in its explicit-mod section.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    identity: ItemIdentity,
    base_matches_pool: bool,
    mods: Vec<ParsedMod>,
    unrecognised: Vec<String>,
    annotated: bool,
}

impl Item {
    /// What a Craft Session pins.
    pub fn identity(&self) -> &ItemIdentity {
        &self.identity
    }

    /// Is this the Base the pool describes? A `false` here cannot be assessed at all.
    pub fn base_matches_pool(&self) -> bool {
        self.base_matches_pool
    }

    /// The explicit mods, in the order they were printed. Nothing from any other section.
    pub fn mods(&self) -> &[ParsedMod] {
        &self.mods
    }

    /// Lines in the explicit-mod section the pool did not recognise.
    ///
    /// Always empty for every capture we have. Non-empty means either the game's wording changed or
    /// the pool is missing a mod, and either way the app is reading an item it does not understand.
    pub fn unrecognised(&self) -> &[String] {
        &self.unrecognised
    }

    /// Did this text carry annotations — i.e. was Advanced Mod Descriptions on?
    pub fn annotated(&self) -> bool {
        self.annotated
    }

    /// Find a mod by its Mod Group.
    pub fn mod_of_group(&self, group_id: &str) -> Option<&ParsedMod> {
        self.mods.iter().find(|m| m.group_id == group_id)
    }
}

/// Read one Item Text.
///
/// The pool is needed during parsing, not after it: recognising which lines are mods, and which
/// belong together, is a question about the Base.
pub fn parse_item_text(text: &str, pool: &ModPool) -> Result<Item, Unreadable> {
    if pool.implicit_count() != 0 {
        return Err(Unreadable::ImplicitsUnsupported {
            count: pool.implicit_count(),
        });
    }

    let sections = split_sections(text);
    let header = sections.first().ok_or(Unreadable::NotItemText)?;

    let item_class = header
        .iter()
        .find_map(|l| l.strip_prefix("Item Class:"))
        .ok_or(Unreadable::NotItemText)?
        .trim()
        .to_string();

    let rarity_word = header
        .iter()
        .find_map(|l| l.strip_prefix("Rarity:"))
        .ok_or_else(|| Unreadable::Rarity("no `Rarity:` line".into()))?
        .trim();
    let rarity = Rarity::parse(rarity_word)
        .ok_or_else(|| Unreadable::Rarity(format!("unknown rarity {rarity_word:?}")))?;

    // The name is whatever follows the `Rarity:` line in the header, joined — a Unique has two
    // name lines, everything else has one.
    let name_line = header
        .iter()
        .skip_while(|l| !l.starts_with("Rarity:"))
        .nth(1)
        .ok_or(Unreadable::NoName)?
        .trim()
        .to_string();

    let (ilvl_section, item_level) = sections
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            s.iter()
                .find_map(|l| l.strip_prefix("Item Level:"))
                .map(|v| (i, v.trim().to_string()))
        })
        .ok_or_else(|| Unreadable::ItemLevel("no `Item Level:` line".into()))?;
    let item_level: u32 = item_level
        .parse()
        .map_err(|_| Unreadable::ItemLevel(format!("{item_level:?} is not a number")))?;

    let base_matches_pool = name_line.contains(pool.base_name());
    let base_name = if base_matches_pool {
        pool.base_name().to_string()
    } else {
        name_line.clone()
    };

    let mut mods = Vec::new();
    let mut unrecognised = Vec::new();
    let mut annotated = false;

    if let Some(section) = sections.get(ilvl_section + 1).filter(|s| !is_trailer(s)) {
        let mut pending: Option<Annotation> = None;
        for line in section.iter() {
            if let Some(annotation) = Annotation::parse(line) {
                annotated = true;
                pending = Some(annotation);
                continue;
            }
            if is_description(line) {
                continue;
            }

            let (match_string, values) = normalise(line);
            let Some(group) = pool.group(&match_string) else {
                unrecognised.push((*line).to_string());
                continue;
            };

            // Consecutive lines of the same Mod Group are one mod. This is what makes the two
            // display forms agree: it never consults the annotation to find a mod's boundaries.
            match mods.last_mut() {
                Some(ParsedMod {
                    group_id,
                    lines,
                    match_strings,
                    values: acc,
                    ..
                }) if group_id == group.id() => {
                    lines.push((*line).to_string());
                    match_strings.push(match_string);
                    acc.extend(values);
                }
                _ => mods.push(ParsedMod {
                    group_id: group.id().to_string(),
                    generation: group.generation(),
                    lines: vec![(*line).to_string()],
                    match_strings: vec![match_string],
                    values,
                    annotation: pending.take(),
                }),
            }
        }
    }

    Ok(Item {
        identity: ItemIdentity {
            rarity,
            item_class,
            base_name,
            item_level,
        },
        base_matches_pool,
        mods,
        unrecognised,
        annotated,
    })
}

/// Split an Item Text on its `--------` separators.
fn split_sections(text: &str) -> Vec<Vec<&str>> {
    let mut sections = vec![Vec::new()];
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-') {
            sections.push(Vec::new());
        } else if !trimmed.is_empty() {
            sections
                .last_mut()
                .expect("there is always a current section")
                .push(trimmed);
        }
    }
    sections
}

/// Is this section the trailer rather than the explicit-mod section?
///
/// A deliberate whitelist: an unrecognised section is treated as the explicit-mod section, where its
/// lines become [`Item::unrecognised`] and the app fails closed. The other way round — guessing that
/// an unfamiliar section is a trailer — would silently report no mods, and no mods is a Miss.
fn is_trailer(section: &[&str]) -> bool {
    section.iter().all(|l| {
        l.starts_with("Place into")
            || l.starts_with("Note:")
            || matches!(*l, "Corrupted" | "Mirrored" | "Split" | "Unmodifiable")
    })
}

/// Is this a parenthesised explanation the game appends to a mod, rather than a mod?
///
/// `(Recently refers to the past 4 seconds)`. It contains numbers, so it has to be excluded
/// structurally rather than by looking for digits.
fn is_description(line: &str) -> bool {
    line.starts_with('(') && line.ends_with(')')
}

/// Replace every rolled value in a line with `#`, returning the normalised line and the values.
///
/// This is `matching.value_placeholder` from `data/ghastly-eye-jewel.json` by hand:
/// `[+-]?\d+(?:\.\d+)?(?:\((?<lo>[^)-]*)-(?<hi>[^)]+)\))?`
///
/// Hand-written so that `crates/core` needs no regex engine; `tests/parser.rs` holds it to the
/// documented pattern across every real capture.
fn normalise(line: &str) -> (String, Vec<f64>) {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut values = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if let Some((value, end)) = scan_value(line, i) {
            out.push('#');
            values.push(value);
            i = end;
        } else {
            let ch = line[i..].chars().next().expect("i is a char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }

    (out, values)
}

/// Try to read one value at `start`. Returns the rolled value and where it ends.
fn scan_value(line: &str, start: usize) -> Option<(f64, usize)> {
    let bytes = line.as_bytes();
    let mut i = start;

    let signed = matches!(bytes.get(i), Some(b'+' | b'-'));
    if signed {
        i += 1;
    }

    let digits_start = i;
    while matches!(bytes.get(i), Some(c) if c.is_ascii_digit()) {
        i += 1;
    }
    if i == digits_start {
        return None;
    }

    if bytes.get(i) == Some(&b'.') {
        let mut j = i + 1;
        while matches!(bytes.get(j), Some(c) if c.is_ascii_digit()) {
            j += 1;
        }
        if j > i + 1 {
            i = j;
        }
    }

    // A value is never part of a longer word: `T4` and `x2` are not rolled values.
    if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
        return None;
    }

    let value: f64 = line[start..i].parse().ok()?;

    // The optional inline bounds — `(9-12)`, `(3.3-4)`. `lo` may not contain `-`, per the pattern.
    if bytes.get(i) == Some(&b'(') {
        if let Some(close) = line[i..].find(')') {
            let inner = &line[i + 1..i + close];
            if let Some((lo, hi)) = inner.split_once('-') {
                if !lo.contains('-') && !hi.is_empty() && !hi.contains(')') {
                    i += close + 1;
                }
            }
        }
    }

    Some((value, i))
}
