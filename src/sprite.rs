//! The sheep, and the pasture it stands in.
//!
//! The Herdr mascot is a side-profile ram with a curled horn and a shell
//! prompt for a face, so every frame keeps `@` for the horn and a `>_`-shaped
//! muzzle. Art is authored facing left only; facing right is the mirror.
//!
//! Scenery art lives here too, so the modules that reserve space for it can
//! read its dimensions from the same place as the sheep's.

use crossterm::style::Color;

use crate::herdr::Status;
use crate::theme;

/// Sprite box width in terminal columns.
pub const SPRITE_W: i32 = 8;
/// Sprite box height in rows.
pub const SPRITE_H: i32 = 3;

/// Barn width in terminal columns.
pub const BARN_W: i32 = 11;
/// Barn height in rows, the last of which stands on the fence.
pub const BARN_H: i32 = 4;

/// The barn on the far side of the pasture: a gambrel roof over a hayloft
/// vent, and a shut door on the ground. Padded to its box like the sheep, so
/// it can be blitted without measuring rows.
pub const BARN: [&str; BARN_H as usize] = [
    "   _____   ", //
    "  /_____\\  ",
    " /|  ^  |\\ ",
    "  |_|X|_|  ",
];

/// Which part of the sheep a glyph belongs to. Drives its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Wool,
    Face,
    Horn,
    Leg,
}

impl Role {
    pub fn color(self, wool: Color) -> Color {
        match self {
            Role::Wool => wool,
            Role::Face => theme::FACE,
            Role::Horn => theme::HORN,
            Role::Leg => theme::LEG,
        }
    }
}

/// Glyph alphabets are disjoint per role, so a frame needs no separate mask.
pub fn role_of(ch: char) -> Role {
    match ch {
        '@' => Role::Horn,
        '>' | '<' | 'o' | '^' | '-' | '_' | 'x' => Role::Face,
        '\'' | '"' | '/' | '\\' | '|' => Role::Leg,
        _ => Role::Wool,
    }
}

fn mirror_char(ch: char) -> char {
    match ch {
        '(' => ')',
        ')' => '(',
        '<' => '>',
        '>' => '<',
        '/' => '\\',
        '\\' => '/',
        ',' => '.',
        '.' => ',',
        other => other,
    }
}

/// One drawable pose. Art rows are ASCII, so byte indexing is column indexing.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    rows: [&'static str; SPRITE_H as usize],
    flip: bool,
}

impl Frame {
    /// Glyph at sprite-space `(col, row)`, already mirrored when facing right.
    /// Returns `None` for transparent cells.
    pub fn glyph(&self, col: i32, row: i32) -> Option<char> {
        if col < 0 || row < 0 || col >= SPRITE_W || row >= SPRITE_H {
            return None;
        }
        let source_col = if self.flip { SPRITE_W - 1 - col } else { col };
        let byte = self.rows[row as usize]
            .as_bytes()
            .get(source_col as usize)
            .copied()
            .unwrap_or(b' ');
        let ch = byte as char;
        if ch == ' ' {
            return None;
        }
        Some(if self.flip { mirror_char(ch) } else { ch })
    }
}

// Every row is padded to exactly SPRITE_W columns so mirroring is symmetric
// and a right-facing sheep does not drift sideways.

/// Head up, all four legs planted.
const STAND: [&str; 3] = [
    " ,@~~~. ", //
    "(>_ ~~ )", " ''  '' ",
];

/// Gallop cycle: the body art is shared, only the legs move.
const RUN_LEGS: [&str; 4] = [
    " /'  '\\ ", //
    " ''  '' ",
    " \\'  '/ ",
    " ''  '' ",
];

/// Head down in the grass.
const GRAZE: [&str; 3] = [
    " ,@~~~. ", //
    " ( ~~ ) ", " >_  '' ",
];

/// Standing to attention, eye wide open.
const ALERT: [&str; 3] = [
    " ,@~~~. ", //
    "(o_ ~~ )", " ''  '' ",
];

/// Mid-jump, eye squeezed shut with joy.
const CHEER: [&str; 3] = [
    " ,@~~~. ", //
    "(^_ ~~ )",
    " \\'  '/ ",
];

/// Lying down, legs tucked under the wool.
const SLEEP: [&str; 3] = [
    " ,@~~~. ", //
    "(-_ ~~ )", " ~~~~~~ ",
];

/// Pose for a status at animation time `t` seconds, facing `left`.
pub fn frame(status: Status, t: f32, facing_left: bool) -> Frame {
    let rows = match status {
        Status::Working => {
            let step = (t * 9.0) as usize % RUN_LEGS.len();
            [STAND[0], STAND[1], RUN_LEGS[step]]
        }
        // Two seconds of head-up watching, three of head-down munching.
        Status::Idle => {
            if (t % 5.0) < 2.0 {
                STAND
            } else {
                GRAZE
            }
        }
        Status::Blocked => ALERT,
        Status::Done => CHEER,
        Status::Unknown => SLEEP,
    };
    Frame {
        rows,
        flip: !facing_left,
    }
}

/// Four-column glyph for the compact list, used when the pasture cannot fit.
pub const MINI: &str = "(>_@";

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite_art() -> Vec<&'static str> {
        let mut rows = Vec::new();
        rows.extend_from_slice(&STAND);
        rows.extend_from_slice(&RUN_LEGS);
        rows.extend_from_slice(&GRAZE);
        rows.extend_from_slice(&ALERT);
        rows.extend_from_slice(&CHEER);
        rows.extend_from_slice(&SLEEP);
        rows
    }

    fn all_art() -> Vec<&'static str> {
        let mut rows = sprite_art();
        rows.push(MINI);
        rows
    }

    #[test]
    fn art_is_ascii_and_exactly_fills_the_sprite_box() {
        for row in sprite_art() {
            assert!(row.is_ascii(), "non-ascii art row {row:?}");
            assert_eq!(
                row.len(),
                SPRITE_W as usize,
                "art row must be padded to the sprite box: {row:?}"
            );
        }
        assert!(MINI.is_ascii());
    }

    #[test]
    fn the_barn_fills_its_own_box() {
        // The layout reserves BARN_W by BARN_H for it, so a row that disagrees
        // would paint outside the space it was given.
        for row in BARN {
            assert!(row.is_ascii(), "non-ascii barn row {row:?}");
            assert_eq!(
                row.len(),
                BARN_W as usize,
                "barn row must be padded to its box: {row:?}"
            );
        }
        assert_eq!(BARN.len(), BARN_H as usize);
    }

    #[test]
    fn every_glyph_has_exactly_one_role() {
        // The role alphabets must stay disjoint, otherwise a fluff glyph would
        // suddenly be painted as a face.
        for row in all_art() {
            for ch in row.chars().filter(|ch| *ch != ' ') {
                let role = role_of(ch);
                match ch {
                    '@' => assert_eq!(role, Role::Horn),
                    '>' | '<' | 'o' | '^' | '-' | '_' | 'x' => assert_eq!(role, Role::Face),
                    '\'' | '"' | '/' | '\\' | '|' => assert_eq!(role, Role::Leg),
                    _ => assert_eq!(role, Role::Wool, "glyph {ch:?} landed in the wrong role"),
                }
            }
        }
    }

    fn render(frame: &Frame) -> Vec<String> {
        (0..SPRITE_H)
            .map(|row| {
                (0..SPRITE_W)
                    .map(|col| frame.glyph(col, row).unwrap_or(' '))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn facing_left_matches_the_authored_art() {
        let left = frame(Status::Blocked, 0.0, true);
        assert_eq!(render(&left), vec![" ,@~~~. ", "(o_ ~~ )", " ''  '' "]);
    }

    #[test]
    fn facing_right_mirrors_the_face_and_body() {
        let right = frame(Status::Blocked, 0.0, false);
        assert_eq!(render(&right), vec![" ,~~~@. ", "( ~~ _o)", " ''  '' "]);
    }

    #[test]
    fn mirroring_keeps_the_sheep_inside_the_same_columns() {
        // Padding is symmetric, so a mirrored pose must occupy the same
        // leading and trailing blank columns.
        for status in [
            Status::Working,
            Status::Idle,
            Status::Blocked,
            Status::Done,
            Status::Unknown,
        ] {
            let left = render(&frame(status, 1.7, true));
            let right = render(&frame(status, 1.7, false));
            for (l, r) in left.iter().zip(&right) {
                let l_ink: Vec<usize> = l
                    .char_indices()
                    .filter(|(_, ch)| *ch != ' ')
                    .map(|(i, _)| SPRITE_W as usize - 1 - i)
                    .collect();
                let r_ink: Vec<usize> = r
                    .char_indices()
                    .filter(|(_, ch)| *ch != ' ')
                    .map(|(i, _)| i)
                    .collect();
                let mut expected = l_ink;
                expected.sort_unstable();
                assert_eq!(expected, r_ink, "{status:?} drifted when mirrored");
            }
        }
    }

    #[test]
    fn running_cycles_legs_but_keeps_the_body() {
        let poses: Vec<Vec<String>> = (0..4)
            .map(|step| render(&frame(Status::Working, step as f32 / 9.0 + 0.001, true)))
            .collect();
        // Legs differ across the cycle.
        assert!(poses.iter().any(|pose| pose[2] != poses[0][2]));
        // Head and body do not.
        for pose in &poses {
            assert_eq!(pose[1], poses[0][1]);
        }
    }

    #[test]
    fn idle_alternates_between_watching_and_grazing() {
        let watching = render(&frame(Status::Idle, 0.5, true));
        let grazing = render(&frame(Status::Idle, 3.5, true));
        assert_ne!(watching, grazing);
        // The grazing pose puts the muzzle down on the grass row.
        assert!(grazing[2].contains(">_"));
        assert!(watching[1].contains(">_"));
    }

    #[test]
    fn glyph_outside_the_box_is_transparent() {
        let pose = frame(Status::Idle, 0.0, true);
        assert_eq!(pose.glyph(-1, 0), None);
        assert_eq!(pose.glyph(SPRITE_W, 0), None);
        assert_eq!(pose.glyph(0, SPRITE_H), None);
    }
}
