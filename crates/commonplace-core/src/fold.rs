//! Case and diacritic folding for matching and sorting: the recorded
//! [ULTRA] divergence of spec §5 row 58. nvUltra "strips diacritics
//! (for example, ü or é) in addition to ignoring case", while NV's
//! match is byte-wise strstr over lowercased UTF-8 with diacritics
//! significant. Commonplace folds both queries and caches with this
//! one function, so matching and the alpha sorts share a single
//! canonical key.
//!
//! Scope, pinned by row 58: a character folds when its Unicode
//! canonical decomposition (NFD) is exactly one ASCII base letter
//! plus combining marks. That is every Latin diacritic-bearing letter
//! (Latin-1 Supplement, Extended-A/B, Extended Additional: French,
//! German umlauts, Spanish, Vietnamese, and kin). Letters without
//! such decompositions (æ, ø, đ, ł, þ, ß) are base letters of their
//! languages, not accented letters; they stay themselves. The table
//! below is generated from the Unicode canonical decompositions and
//! sorted by codepoint for binary search; regenerate with the same
//! NFD rule if the engine ever widens its scope.

/// Folding entries `(precomposed char, ASCII base letter)`, sorted by
/// the char for binary search.
const FOLD_TABLE: &[(char, u8)] = &[
    ('\u{00c0}', b'A'),
    ('\u{00c1}', b'A'),
    ('\u{00c2}', b'A'),
    ('\u{00c3}', b'A'),
    ('\u{00c4}', b'A'),
    ('\u{00c5}', b'A'),
    ('\u{00c7}', b'C'),
    ('\u{00c8}', b'E'),
    ('\u{00c9}', b'E'),
    ('\u{00ca}', b'E'),
    ('\u{00cb}', b'E'),
    ('\u{00cc}', b'I'),
    ('\u{00cd}', b'I'),
    ('\u{00ce}', b'I'),
    ('\u{00cf}', b'I'),
    ('\u{00d1}', b'N'),
    ('\u{00d2}', b'O'),
    ('\u{00d3}', b'O'),
    ('\u{00d4}', b'O'),
    ('\u{00d5}', b'O'),
    ('\u{00d6}', b'O'),
    ('\u{00d9}', b'U'),
    ('\u{00da}', b'U'),
    ('\u{00db}', b'U'),
    ('\u{00dc}', b'U'),
    ('\u{00dd}', b'Y'),
    ('\u{00e0}', b'a'),
    ('\u{00e1}', b'a'),
    ('\u{00e2}', b'a'),
    ('\u{00e3}', b'a'),
    ('\u{00e4}', b'a'),
    ('\u{00e5}', b'a'),
    ('\u{00e7}', b'c'),
    ('\u{00e8}', b'e'),
    ('\u{00e9}', b'e'),
    ('\u{00ea}', b'e'),
    ('\u{00eb}', b'e'),
    ('\u{00ec}', b'i'),
    ('\u{00ed}', b'i'),
    ('\u{00ee}', b'i'),
    ('\u{00ef}', b'i'),
    ('\u{00f1}', b'n'),
    ('\u{00f2}', b'o'),
    ('\u{00f3}', b'o'),
    ('\u{00f4}', b'o'),
    ('\u{00f5}', b'o'),
    ('\u{00f6}', b'o'),
    ('\u{00f9}', b'u'),
    ('\u{00fa}', b'u'),
    ('\u{00fb}', b'u'),
    ('\u{00fc}', b'u'),
    ('\u{00fd}', b'y'),
    ('\u{00ff}', b'y'),
    ('\u{0100}', b'A'),
    ('\u{0101}', b'a'),
    ('\u{0102}', b'A'),
    ('\u{0103}', b'a'),
    ('\u{0104}', b'A'),
    ('\u{0105}', b'a'),
    ('\u{0106}', b'C'),
    ('\u{0107}', b'c'),
    ('\u{0108}', b'C'),
    ('\u{0109}', b'c'),
    ('\u{010a}', b'C'),
    ('\u{010b}', b'c'),
    ('\u{010c}', b'C'),
    ('\u{010d}', b'c'),
    ('\u{010e}', b'D'),
    ('\u{010f}', b'd'),
    ('\u{0112}', b'E'),
    ('\u{0113}', b'e'),
    ('\u{0114}', b'E'),
    ('\u{0115}', b'e'),
    ('\u{0116}', b'E'),
    ('\u{0117}', b'e'),
    ('\u{0118}', b'E'),
    ('\u{0119}', b'e'),
    ('\u{011a}', b'E'),
    ('\u{011b}', b'e'),
    ('\u{011c}', b'G'),
    ('\u{011d}', b'g'),
    ('\u{011e}', b'G'),
    ('\u{011f}', b'g'),
    ('\u{0120}', b'G'),
    ('\u{0121}', b'g'),
    ('\u{0122}', b'G'),
    ('\u{0123}', b'g'),
    ('\u{0124}', b'H'),
    ('\u{0125}', b'h'),
    ('\u{0128}', b'I'),
    ('\u{0129}', b'i'),
    ('\u{012a}', b'I'),
    ('\u{012b}', b'i'),
    ('\u{012c}', b'I'),
    ('\u{012d}', b'i'),
    ('\u{012e}', b'I'),
    ('\u{012f}', b'i'),
    ('\u{0130}', b'I'),
    ('\u{0134}', b'J'),
    ('\u{0135}', b'j'),
    ('\u{0136}', b'K'),
    ('\u{0137}', b'k'),
    ('\u{0139}', b'L'),
    ('\u{013a}', b'l'),
    ('\u{013b}', b'L'),
    ('\u{013c}', b'l'),
    ('\u{013d}', b'L'),
    ('\u{013e}', b'l'),
    ('\u{0143}', b'N'),
    ('\u{0144}', b'n'),
    ('\u{0145}', b'N'),
    ('\u{0146}', b'n'),
    ('\u{0147}', b'N'),
    ('\u{0148}', b'n'),
    ('\u{014c}', b'O'),
    ('\u{014d}', b'o'),
    ('\u{014e}', b'O'),
    ('\u{014f}', b'o'),
    ('\u{0150}', b'O'),
    ('\u{0151}', b'o'),
    ('\u{0154}', b'R'),
    ('\u{0155}', b'r'),
    ('\u{0156}', b'R'),
    ('\u{0157}', b'r'),
    ('\u{0158}', b'R'),
    ('\u{0159}', b'r'),
    ('\u{015a}', b'S'),
    ('\u{015b}', b's'),
    ('\u{015c}', b'S'),
    ('\u{015d}', b's'),
    ('\u{015e}', b'S'),
    ('\u{015f}', b's'),
    ('\u{0160}', b'S'),
    ('\u{0161}', b's'),
    ('\u{0162}', b'T'),
    ('\u{0163}', b't'),
    ('\u{0164}', b'T'),
    ('\u{0165}', b't'),
    ('\u{0168}', b'U'),
    ('\u{0169}', b'u'),
    ('\u{016a}', b'U'),
    ('\u{016b}', b'u'),
    ('\u{016c}', b'U'),
    ('\u{016d}', b'u'),
    ('\u{016e}', b'U'),
    ('\u{016f}', b'u'),
    ('\u{0170}', b'U'),
    ('\u{0171}', b'u'),
    ('\u{0172}', b'U'),
    ('\u{0173}', b'u'),
    ('\u{0174}', b'W'),
    ('\u{0175}', b'w'),
    ('\u{0176}', b'Y'),
    ('\u{0177}', b'y'),
    ('\u{0178}', b'Y'),
    ('\u{0179}', b'Z'),
    ('\u{017a}', b'z'),
    ('\u{017b}', b'Z'),
    ('\u{017c}', b'z'),
    ('\u{017d}', b'Z'),
    ('\u{017e}', b'z'),
    ('\u{01a0}', b'O'),
    ('\u{01a1}', b'o'),
    ('\u{01af}', b'U'),
    ('\u{01b0}', b'u'),
    ('\u{01cd}', b'A'),
    ('\u{01ce}', b'a'),
    ('\u{01cf}', b'I'),
    ('\u{01d0}', b'i'),
    ('\u{01d1}', b'O'),
    ('\u{01d2}', b'o'),
    ('\u{01d3}', b'U'),
    ('\u{01d4}', b'u'),
    ('\u{01d5}', b'U'),
    ('\u{01d6}', b'u'),
    ('\u{01d7}', b'U'),
    ('\u{01d8}', b'u'),
    ('\u{01d9}', b'U'),
    ('\u{01da}', b'u'),
    ('\u{01db}', b'U'),
    ('\u{01dc}', b'u'),
    ('\u{01de}', b'A'),
    ('\u{01df}', b'a'),
    ('\u{01e0}', b'A'),
    ('\u{01e1}', b'a'),
    ('\u{01e6}', b'G'),
    ('\u{01e7}', b'g'),
    ('\u{01e8}', b'K'),
    ('\u{01e9}', b'k'),
    ('\u{01ea}', b'O'),
    ('\u{01eb}', b'o'),
    ('\u{01ec}', b'O'),
    ('\u{01ed}', b'o'),
    ('\u{01f0}', b'j'),
    ('\u{01f4}', b'G'),
    ('\u{01f5}', b'g'),
    ('\u{01f8}', b'N'),
    ('\u{01f9}', b'n'),
    ('\u{01fa}', b'A'),
    ('\u{01fb}', b'a'),
    ('\u{0200}', b'A'),
    ('\u{0201}', b'a'),
    ('\u{0202}', b'A'),
    ('\u{0203}', b'a'),
    ('\u{0204}', b'E'),
    ('\u{0205}', b'e'),
    ('\u{0206}', b'E'),
    ('\u{0207}', b'e'),
    ('\u{0208}', b'I'),
    ('\u{0209}', b'i'),
    ('\u{020a}', b'I'),
    ('\u{020b}', b'i'),
    ('\u{020c}', b'O'),
    ('\u{020d}', b'o'),
    ('\u{020e}', b'O'),
    ('\u{020f}', b'o'),
    ('\u{0210}', b'R'),
    ('\u{0211}', b'r'),
    ('\u{0212}', b'R'),
    ('\u{0213}', b'r'),
    ('\u{0214}', b'U'),
    ('\u{0215}', b'u'),
    ('\u{0216}', b'U'),
    ('\u{0217}', b'u'),
    ('\u{0218}', b'S'),
    ('\u{0219}', b's'),
    ('\u{021a}', b'T'),
    ('\u{021b}', b't'),
    ('\u{021e}', b'H'),
    ('\u{021f}', b'h'),
    ('\u{0226}', b'A'),
    ('\u{0227}', b'a'),
    ('\u{0228}', b'E'),
    ('\u{0229}', b'e'),
    ('\u{022a}', b'O'),
    ('\u{022b}', b'o'),
    ('\u{022c}', b'O'),
    ('\u{022d}', b'o'),
    ('\u{022e}', b'O'),
    ('\u{022f}', b'o'),
    ('\u{0230}', b'O'),
    ('\u{0231}', b'o'),
    ('\u{0232}', b'Y'),
    ('\u{0233}', b'y'),
    ('\u{1e00}', b'A'),
    ('\u{1e01}', b'a'),
    ('\u{1e02}', b'B'),
    ('\u{1e03}', b'b'),
    ('\u{1e04}', b'B'),
    ('\u{1e05}', b'b'),
    ('\u{1e06}', b'B'),
    ('\u{1e07}', b'b'),
    ('\u{1e08}', b'C'),
    ('\u{1e09}', b'c'),
    ('\u{1e0a}', b'D'),
    ('\u{1e0b}', b'd'),
    ('\u{1e0c}', b'D'),
    ('\u{1e0d}', b'd'),
    ('\u{1e0e}', b'D'),
    ('\u{1e0f}', b'd'),
    ('\u{1e10}', b'D'),
    ('\u{1e11}', b'd'),
    ('\u{1e12}', b'D'),
    ('\u{1e13}', b'd'),
    ('\u{1e14}', b'E'),
    ('\u{1e15}', b'e'),
    ('\u{1e16}', b'E'),
    ('\u{1e17}', b'e'),
    ('\u{1e18}', b'E'),
    ('\u{1e19}', b'e'),
    ('\u{1e1a}', b'E'),
    ('\u{1e1b}', b'e'),
    ('\u{1e1c}', b'E'),
    ('\u{1e1d}', b'e'),
    ('\u{1e1e}', b'F'),
    ('\u{1e1f}', b'f'),
    ('\u{1e20}', b'G'),
    ('\u{1e21}', b'g'),
    ('\u{1e22}', b'H'),
    ('\u{1e23}', b'h'),
    ('\u{1e24}', b'H'),
    ('\u{1e25}', b'h'),
    ('\u{1e26}', b'H'),
    ('\u{1e27}', b'h'),
    ('\u{1e28}', b'H'),
    ('\u{1e29}', b'h'),
    ('\u{1e2a}', b'H'),
    ('\u{1e2b}', b'h'),
    ('\u{1e2c}', b'I'),
    ('\u{1e2d}', b'i'),
    ('\u{1e2e}', b'I'),
    ('\u{1e2f}', b'i'),
    ('\u{1e30}', b'K'),
    ('\u{1e31}', b'k'),
    ('\u{1e32}', b'K'),
    ('\u{1e33}', b'k'),
    ('\u{1e34}', b'K'),
    ('\u{1e35}', b'k'),
    ('\u{1e36}', b'L'),
    ('\u{1e37}', b'l'),
    ('\u{1e38}', b'L'),
    ('\u{1e39}', b'l'),
    ('\u{1e3a}', b'L'),
    ('\u{1e3b}', b'l'),
    ('\u{1e3c}', b'L'),
    ('\u{1e3d}', b'l'),
    ('\u{1e3e}', b'M'),
    ('\u{1e3f}', b'm'),
    ('\u{1e40}', b'M'),
    ('\u{1e41}', b'm'),
    ('\u{1e42}', b'M'),
    ('\u{1e43}', b'm'),
    ('\u{1e44}', b'N'),
    ('\u{1e45}', b'n'),
    ('\u{1e46}', b'N'),
    ('\u{1e47}', b'n'),
    ('\u{1e48}', b'N'),
    ('\u{1e49}', b'n'),
    ('\u{1e4a}', b'N'),
    ('\u{1e4b}', b'n'),
    ('\u{1e4c}', b'O'),
    ('\u{1e4d}', b'o'),
    ('\u{1e4e}', b'O'),
    ('\u{1e4f}', b'o'),
    ('\u{1e50}', b'O'),
    ('\u{1e51}', b'o'),
    ('\u{1e52}', b'O'),
    ('\u{1e53}', b'o'),
    ('\u{1e54}', b'P'),
    ('\u{1e55}', b'p'),
    ('\u{1e56}', b'P'),
    ('\u{1e57}', b'p'),
    ('\u{1e58}', b'R'),
    ('\u{1e59}', b'r'),
    ('\u{1e5a}', b'R'),
    ('\u{1e5b}', b'r'),
    ('\u{1e5c}', b'R'),
    ('\u{1e5d}', b'r'),
    ('\u{1e5e}', b'R'),
    ('\u{1e5f}', b'r'),
    ('\u{1e60}', b'S'),
    ('\u{1e61}', b's'),
    ('\u{1e62}', b'S'),
    ('\u{1e63}', b's'),
    ('\u{1e64}', b'S'),
    ('\u{1e65}', b's'),
    ('\u{1e66}', b'S'),
    ('\u{1e67}', b's'),
    ('\u{1e68}', b'S'),
    ('\u{1e69}', b's'),
    ('\u{1e6a}', b'T'),
    ('\u{1e6b}', b't'),
    ('\u{1e6c}', b'T'),
    ('\u{1e6d}', b't'),
    ('\u{1e6e}', b'T'),
    ('\u{1e6f}', b't'),
    ('\u{1e70}', b'T'),
    ('\u{1e71}', b't'),
    ('\u{1e72}', b'U'),
    ('\u{1e73}', b'u'),
    ('\u{1e74}', b'U'),
    ('\u{1e75}', b'u'),
    ('\u{1e76}', b'U'),
    ('\u{1e77}', b'u'),
    ('\u{1e78}', b'U'),
    ('\u{1e79}', b'u'),
    ('\u{1e7a}', b'U'),
    ('\u{1e7b}', b'u'),
    ('\u{1e7c}', b'V'),
    ('\u{1e7d}', b'v'),
    ('\u{1e7e}', b'V'),
    ('\u{1e7f}', b'v'),
    ('\u{1e80}', b'W'),
    ('\u{1e81}', b'w'),
    ('\u{1e82}', b'W'),
    ('\u{1e83}', b'w'),
    ('\u{1e84}', b'W'),
    ('\u{1e85}', b'w'),
    ('\u{1e86}', b'W'),
    ('\u{1e87}', b'w'),
    ('\u{1e88}', b'W'),
    ('\u{1e89}', b'w'),
    ('\u{1e8a}', b'X'),
    ('\u{1e8b}', b'x'),
    ('\u{1e8c}', b'X'),
    ('\u{1e8d}', b'x'),
    ('\u{1e8e}', b'Y'),
    ('\u{1e8f}', b'y'),
    ('\u{1e90}', b'Z'),
    ('\u{1e91}', b'z'),
    ('\u{1e92}', b'Z'),
    ('\u{1e93}', b'z'),
    ('\u{1e94}', b'Z'),
    ('\u{1e95}', b'z'),
    ('\u{1e96}', b'h'),
    ('\u{1e97}', b't'),
    ('\u{1e98}', b'w'),
    ('\u{1e99}', b'y'),
    ('\u{1ea0}', b'A'),
    ('\u{1ea1}', b'a'),
    ('\u{1ea2}', b'A'),
    ('\u{1ea3}', b'a'),
    ('\u{1ea4}', b'A'),
    ('\u{1ea5}', b'a'),
    ('\u{1ea6}', b'A'),
    ('\u{1ea7}', b'a'),
    ('\u{1ea8}', b'A'),
    ('\u{1ea9}', b'a'),
    ('\u{1eaa}', b'A'),
    ('\u{1eab}', b'a'),
    ('\u{1eac}', b'A'),
    ('\u{1ead}', b'a'),
    ('\u{1eae}', b'A'),
    ('\u{1eaf}', b'a'),
    ('\u{1eb0}', b'A'),
    ('\u{1eb1}', b'a'),
    ('\u{1eb2}', b'A'),
    ('\u{1eb3}', b'a'),
    ('\u{1eb4}', b'A'),
    ('\u{1eb5}', b'a'),
    ('\u{1eb6}', b'A'),
    ('\u{1eb7}', b'a'),
    ('\u{1eb8}', b'E'),
    ('\u{1eb9}', b'e'),
    ('\u{1eba}', b'E'),
    ('\u{1ebb}', b'e'),
    ('\u{1ebc}', b'E'),
    ('\u{1ebd}', b'e'),
    ('\u{1ebe}', b'E'),
    ('\u{1ebf}', b'e'),
    ('\u{1ec0}', b'E'),
    ('\u{1ec1}', b'e'),
    ('\u{1ec2}', b'E'),
    ('\u{1ec3}', b'e'),
    ('\u{1ec4}', b'E'),
    ('\u{1ec5}', b'e'),
    ('\u{1ec6}', b'E'),
    ('\u{1ec7}', b'e'),
    ('\u{1ec8}', b'I'),
    ('\u{1ec9}', b'i'),
    ('\u{1eca}', b'I'),
    ('\u{1ecb}', b'i'),
    ('\u{1ecc}', b'O'),
    ('\u{1ecd}', b'o'),
    ('\u{1ece}', b'O'),
    ('\u{1ecf}', b'o'),
    ('\u{1ed0}', b'O'),
    ('\u{1ed1}', b'o'),
    ('\u{1ed2}', b'O'),
    ('\u{1ed3}', b'o'),
    ('\u{1ed4}', b'O'),
    ('\u{1ed5}', b'o'),
    ('\u{1ed6}', b'O'),
    ('\u{1ed7}', b'o'),
    ('\u{1ed8}', b'O'),
    ('\u{1ed9}', b'o'),
    ('\u{1eda}', b'O'),
    ('\u{1edb}', b'o'),
    ('\u{1edc}', b'O'),
    ('\u{1edd}', b'o'),
    ('\u{1ede}', b'O'),
    ('\u{1edf}', b'o'),
    ('\u{1ee0}', b'O'),
    ('\u{1ee1}', b'o'),
    ('\u{1ee2}', b'O'),
    ('\u{1ee3}', b'o'),
    ('\u{1ee4}', b'U'),
    ('\u{1ee5}', b'u'),
    ('\u{1ee6}', b'U'),
    ('\u{1ee7}', b'u'),
    ('\u{1ee8}', b'U'),
    ('\u{1ee9}', b'u'),
    ('\u{1eea}', b'U'),
    ('\u{1eeb}', b'u'),
    ('\u{1eec}', b'U'),
    ('\u{1eed}', b'u'),
    ('\u{1eee}', b'U'),
    ('\u{1eef}', b'u'),
    ('\u{1ef0}', b'U'),
    ('\u{1ef1}', b'u'),
    ('\u{1ef2}', b'Y'),
    ('\u{1ef3}', b'y'),
    ('\u{1ef4}', b'Y'),
    ('\u{1ef5}', b'y'),
    ('\u{1ef6}', b'Y'),
    ('\u{1ef7}', b'y'),
    ('\u{1ef8}', b'Y'),
    ('\u{1ef9}', b'y'),
];

/// Fold one char: the table's ASCII base letter (lowercased) when the
/// char is a diacritic-bearing precomposed letter, else `None`.
fn fold_char(c: char) -> Option<char> {
    FOLD_TABLE
        .binary_search_by_key(&c, |&(probe, _)| probe)
        .ok()
        .map(|i| (FOLD_TABLE[i].1 as char).to_ascii_lowercase())
}

/// Fold a string: per character, the diacritic table first (its base
/// letter, lowercased), otherwise `char::to_lowercase` with any
/// combining-mark leftovers dropped (the simple lowercase of U+0130 İ
/// is `i` + U+0307; the table catches İ itself, so the guard only
/// backs up the exotic case).
///
/// The function is deliberately context-free (per `char`, never
/// `str::to_lowercase`, whose final-sigma rule makes the result
/// position-dependent) and one-to-one on characters: folding a token
/// equals folding the whole string and slicing, which the incremental
/// filter's resume offsets rely on. Idempotent; the result matches
/// ASCII needles against accented text and vice versa (`bucher` finds
/// `Bücher`).
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if let Some(base) = fold_char(c) {
            out.push(base);
            continue;
        }
        for lc in c.to_lowercase() {
            if !is_combining_mark(lc) {
                out.push(lc);
            }
        }
    }
    out
}

/// Combining diacritical marks (the leftovers a lowercase expansion
/// can leave behind). Only U+0300..U+036F is reachable through
/// `to_lowercase` here; broader mark families never appear in a
/// lowercase mapping of a letter this engine folds.
fn is_combining_mark(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_passes_through() {
        assert_eq!(fold("Hello, world! 123"), "hello, world! 123");
    }

    #[test]
    fn case_folds() {
        assert_eq!(fold("MixedCASE"), "mixedcase");
        assert_eq!(fold("STRASSE"), "strasse");
    }

    #[test]
    fn the_manual_example_bucher_matches_buecher() {
        // The nvUltra manual's own example (row 58)
        assert_eq!(fold("Bücher"), "bucher");
        assert_eq!(fold("bucher"), fold("Bücher"));
    }

    #[test]
    fn diacritics_strip_across_the_latin_sets() {
        assert_eq!(fold("Ångström"), "angstrom");
        assert_eq!(fold("café"), "cafe");
        assert_eq!(fold("naïve façade"), "naive facade");
        // Vietnamese (Extended Additional)
        assert_eq!(fold("Việt Nam"), "viet nam");
        // Czech (Extended-A)
        assert_eq!(fold("Dvořák"), "dvorak");
    }

    #[test]
    fn two_step_decompositions_fully_strip() {
        // ǟ = ä + macron = a + diaeresis + macron: one pass must reach 'a'
        assert_eq!(fold("ǟ colder Ǭ"), "a colder o");
    }

    #[test]
    fn base_letters_without_decompositions_stay() {
        // Row 58: these are letters, not diacritics
        assert_eq!(fold("Ærø"), "ærø");
        assert_eq!(fold("straße"), "straße");
        assert_eq!(fold("ĐđŁłÞþ"), "đđłłþþ");
    }

    #[test]
    fn fold_is_idempotent() {
        let once = fold("Übers Äxl Ǟhã");
        assert_eq!(fold(&once), once);
    }

    #[test]
    fn dotted_capital_i_folds_dotless() {
        // U+0130's simple lowercase is `i` + U+0307; the table entry
        // must win, leaving no combining dot (the review's row-58
        // counterexample)
        assert_eq!(fold("İstanbul"), "istanbul");
        assert!(!fold("İstanbul").contains('\u{307}'));
        assert_eq!(fold("İ"), "i");
    }

    #[test]
    fn fold_is_context_free_and_char_preserving() {
        // str::to_lowercase moves Greek final sigma by position;
        // per-char folding must not
        assert_eq!(fold("Σ"), fold("ΑΣ").strip_prefix('α').unwrap());
        // Every table entry folds to exactly one char, and so does
        // every char the table does not catch (the only Unicode
        // growing lowercase, İ, is in the table): folding a token
        // equals slicing the folded whole, which the resume offsets
        // rely on.
        for c in ['İ', 'ß', 'ẞ', 'Σ', 'Æ', 'Ǆ', 'ǵ', '\u{a0}'] {
            assert_eq!(fold(&c.to_string()).chars().count(), 1, "char {c:?}");
        }
        for &(c, _) in FOLD_TABLE {
            assert_eq!(fold(&c.to_string()).chars().count(), 1, "table char {c:?}");
        }
    }

    #[test]
    fn greek_and_cyrillic_are_out_of_scope() {
        // No ASCII base letter exists for these; folding must not mangle them
        assert_eq!(fold("Привет"), "привет");
        assert_eq!(fold("Αθήνα"), "αθήνα");
    }

    #[test]
    fn table_is_sorted_for_binary_search() {
        for pair in FOLD_TABLE.windows(2) {
            assert!(pair[0].0 < pair[1].0, "table unsorted at {:?}", pair);
        }
    }

    #[test]
    fn every_entry_folds_to_itself_after_one_pass() {
        // fold_char returns the base letter lowercased (fold output is
        // all-lowercase), whatever the entry's recorded case.
        for &(c, base) in FOLD_TABLE {
            assert_eq!(fold_char(c), Some((base as char).to_ascii_lowercase()));
        }
    }
}
