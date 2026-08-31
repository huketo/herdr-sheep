//! Colors. Everything is a 256-color index so the pasture looks the same on
//! terminals without truecolor.

use crossterm::style::Color;

use crate::herdr::Status;

const fn c(index: u8) -> Color {
    Color::AnsiValue(index)
}

/// Hand-picked wool colors for breeds people actually run. The key is the
/// model provider the agent reports, falling back to the agent kind Herdr
/// detected.
const BREEDS: &[(&str, u8)] = &[
    ("claude", 215),
    ("anthropic", 215),
    ("codex", 252),
    ("openai", 252),
    ("gpt", 252),
    ("gemini", 111),
    ("google", 111),
    ("omp", 176),
    ("pi", 87),
    ("copilot", 117),
    ("cursor", 146),
    ("grok", 210),
    ("qwen", 141),
    ("kimi", 156),
    ("droid", 173),
    ("amp", 209),
    ("opencode", 79),
    ("cline", 108),
    ("kilo", 183),
    ("devin", 68),
];

/// Colors handed out to breeds not in `BREEDS`, chosen to stay distinguishable
/// from each other and from the hand-picked set.
const STRAY_WOOL: &[u8] = &[180, 151, 110, 175, 222, 144, 116, 182];

/// Wool color for a breed key.
pub fn wool(breed: &str) -> Color {
    if let Some((_, index)) = BREEDS.iter().find(|(name, _)| *name == breed) {
        return c(*index);
    }
    // Stable per breed so a sheep keeps its color across restarts.
    let hash = breed.bytes().fold(2166136261u32, |acc, byte| {
        (acc ^ byte as u32).wrapping_mul(16777619)
    });
    c(STRAY_WOOL[hash as usize % STRAY_WOOL.len()])
}

pub const FACE: Color = c(231);
pub const HORN: Color = c(179);
pub const LEG: Color = c(244);
pub const GROUND: Color = c(65);
pub const HEADER: Color = c(250);
pub const MUTED: Color = c(244);
pub const ACCENT: Color = c(45);
pub const ALERT: Color = c(220);
pub const CHEER: Color = c(84);
pub const ERROR: Color = c(203);

/// Accent color for a zone label and its decorations.
pub fn zone(status: Status) -> Color {
    match status {
        Status::Working => c(114),
        Status::Idle => c(108),
        Status::Blocked => ALERT,
        Status::Done => CHEER,
        Status::Unknown => c(103),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_breeds_get_their_own_color() {
        assert_eq!(wool("claude"), Color::AnsiValue(215));
        assert_ne!(wool("claude"), wool("codex"));
    }

    #[test]
    fn stray_breeds_are_stable_and_in_palette() {
        let first = wool("some-new-agent");
        assert_eq!(first, wool("some-new-agent"));
        let Color::AnsiValue(index) = first else {
            panic!("expected a 256-color index");
        };
        assert!(STRAY_WOOL.contains(&index));
    }

    #[test]
    fn every_status_has_a_distinct_zone_color() {
        let colors = [
            zone(Status::Working),
            zone(Status::Idle),
            zone(Status::Blocked),
            zone(Status::Done),
            zone(Status::Unknown),
        ];
        for (i, left) in colors.iter().enumerate() {
            for right in &colors[i + 1..] {
                assert_ne!(left, right);
            }
        }
    }
}
