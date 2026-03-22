//! Text stylizing.
//!
//! The [`stylize`] function allows converting regular A-z letters and 0–9 numbers into stylized
//! variants. The text is converted according to the [`FontStyle`] enum.
//!
//! # Examples
//!
//! ```
//! use basalt_tui::stylized_text::{FontStyle, stylize};
//!
//! assert_eq!(stylize("My Heading", FontStyle::FrakturBold), "𝕸𝖞 𝕳𝖊𝖆𝖉𝖎𝖓𝖌");
//! ```

/// Enum representing different font styles.
///
/// - BlackBoardBold (𝔹𝕝𝕒𝕔𝕜𝔹𝕠𝕒𝕣𝕕𝔹𝕠𝕝𝕕)
/// - FrakturBold (𝕱𝖗𝖆𝖐𝖙𝖚𝖗𝕭𝖔𝖑𝖉)
/// - Script (𝓢𝓬𝓻𝓲𝓹𝓽)
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    /// Blackboard Bold (Double-struck) style (e.g., 𝕋𝕚𝕥𝕝𝕖).
    BlackBoardBold,
    /// Bold Fraktur style. (e.g., 𝕿𝖎𝖙𝖑𝖊)
    FrakturBold,
    /// Script style. (e.g., 𝓣𝓲𝓽𝓵𝓮)
    Script,
}

/// Stylizes the given input string using the specified [`FontStyle`].
///
/// Each character in the input is mapped to its corresponding stylized Unicode character based on
/// the provided style. Characters that do not have a stylized equivalent are returned unchanged.
///
/// # Examples
///
/// ```
/// use basalt_tui::stylized_text::{FontStyle, stylize};
///
/// assert_eq!(stylize("Black Board Bold", FontStyle::BlackBoardBold), "𝔹𝕝𝕒𝕔𝕜 𝔹𝕠𝕒𝕣𝕕 𝔹𝕠𝕝𝕕");
/// assert_eq!(stylize("Fraktur Bold", FontStyle::FrakturBold), "𝕱𝖗𝖆𝖐𝖙𝖚𝖗 𝕭𝖔𝖑𝖉");
/// assert_eq!(stylize("Script", FontStyle::Script), "𝓢𝓬𝓻𝓲𝓹𝓽");
/// ```
pub fn stylize(input: &str, style: FontStyle) -> String {
    input.chars().map(|c| stylize_char(c, style)).collect()
}

/// Returns the stylized Unicode character for a given `char` and [`FontStyle`].
///
/// Letters between A-z and number 0-9 are stylized. Characters outside the stylized range (e.g.,
/// punctuation) are returned as-is.
///
/// To find the corresponding stylized character, we add the remainder to the unicode character
/// range, which is achieved by subtracting the start position from the input `char`.
fn stylize_char(c: char, style: FontStyle) -> char {
    match style {
        FontStyle::BlackBoardBold => match c {
            'C' => char::from_u32(0x2102),
            'H' => char::from_u32(0x210D),
            'N' => char::from_u32(0x2115),
            'P' => char::from_u32(0x2119),
            'Q' => char::from_u32(0x211A),
            'R' => char::from_u32(0x211D),
            'Z' => char::from_u32(0x2124),
            'A'..='Z' => char::from_u32(0x1D538 + (c as u32 - 'A' as u32)),
            'a'..='z' => char::from_u32(0x1D552 + (c as u32 - 'a' as u32)),
            '0'..='9' => char::from_u32(0x1D7D8 + (c as u32 - '0' as u32)),
            _ => None,
        },
        FontStyle::FrakturBold => match c {
            'A'..='Z' => char::from_u32(0x1D56C + (c as u32 - 'A' as u32)),
            'a'..='z' => char::from_u32(0x1D586 + (c as u32 - 'a' as u32)),
            '0'..='9' => char::from_u32(0x1D7CE + (c as u32 - '0' as u32)),
            _ => None,
        },
        FontStyle::Script => match c {
            'A'..='Z' => char::from_u32(0x1D4D0 + (c as u32 - 'A' as u32)),
            'a'..='z' => char::from_u32(0x1D4EA + (c as u32 - 'a' as u32)),
            '0'..='9' => char::from_u32(0x1D7CE + (c as u32 - '0' as u32)),
            _ => None,
        },
    }
    .unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use similar_asserts::assert_eq;

    use super::*;

    #[test]
    fn test_stylize() {
        let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let tests = [
            (
                FontStyle::Script,
                "𝓐𝓑𝓒𝓓𝓔𝓕𝓖𝓗𝓘𝓙𝓚𝓛𝓜𝓝𝓞𝓟𝓠𝓡𝓢𝓣𝓤𝓥𝓦𝓧𝓨𝓩𝓪𝓫𝓬𝓭𝓮𝓯𝓰𝓱𝓲𝓳𝓴𝓵𝓶𝓷𝓸𝓹𝓺𝓻𝓼𝓽𝓾𝓿𝔀𝔁𝔂𝔃𝟎𝟏𝟐𝟑𝟒𝟓𝟔𝟕𝟖𝟗",
            ),
            (
                FontStyle::FrakturBold,
                "𝕬𝕭𝕮𝕯𝕰𝕱𝕲𝕳𝕴𝕵𝕶𝕷𝕸𝕹𝕺𝕻𝕼𝕽𝕾𝕿𝖀𝖁𝖂𝖃𝖄𝖅𝖆𝖇𝖈𝖉𝖊𝖋𝖌𝖍𝖎𝖏𝖐𝖑𝖒𝖓𝖔𝖕𝖖𝖗𝖘𝖙𝖚𝖛𝖜𝖝𝖞𝖟𝟎𝟏𝟐𝟑𝟒𝟓𝟔𝟕𝟖𝟗",
            ),
            (
                FontStyle::BlackBoardBold,
                "𝔸𝔹ℂ𝔻𝔼𝔽𝔾ℍ𝕀𝕁𝕂𝕃𝕄ℕ𝕆ℙℚℝ𝕊𝕋𝕌𝕍𝕎𝕏𝕐ℤ𝕒𝕓𝕔𝕕𝕖𝕗𝕘𝕙𝕚𝕛𝕜𝕝𝕞𝕟𝕠𝕡𝕢𝕣𝕤𝕥𝕦𝕧𝕨𝕩𝕪𝕫𝟘𝟙𝟚𝟛𝟜𝟝𝟞𝟟𝟠𝟡",
            ),
        ];

        tests
            .iter()
            .for_each(|test| assert_eq!(stylize(text, test.0), test.1));
    }
}
