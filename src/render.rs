//! Double-buffered cell grid with a diffing flush.
//!
//! The pasture animates every frame, but only a few hundred cells change, so
//! the renderer paints into a grid and emits just the runs that differ from the
//! previous frame.

use std::io::Write;

use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};
use crossterm::{cursor, queue, terminal};
use unicode_width::UnicodeWidthChar;

/// Foreground styling for one cell. Background is always the terminal default,
/// so the pasture inherits the user's theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bold: bool,
    pub dim: bool,
    pub reverse: bool,
}

impl Style {
    pub const fn fg(fg: Color) -> Self {
        Self {
            fg,
            bold: false,
            dim: false,
            reverse: false,
        }
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    pub const fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::fg(Color::Reset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cell {
    ch: char,
    style: Style,
    /// Right half of a double-width glyph: never emitted, only compared.
    continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
            continuation: false,
        }
    }
}

pub struct Screen {
    width: u16,
    height: u16,
    front: Vec<Cell>,
    back: Vec<Cell>,
    /// Set after a resize: the terminal was cleared, so nothing may be skipped.
    full_repaint: bool,
}

impl Screen {
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            front: vec![Cell::default(); len],
            back: vec![Cell::default(); len],
            full_repaint: true,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if width == self.width && height == self.height {
            return;
        }
        *self = Self::new(width, height);
    }

    /// Reset the frame being composed. The previous frame stays available for
    /// diffing.
    pub fn begin_frame(&mut self) {
        self.back.fill(Cell::default());
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Paint one glyph. Double-width glyphs claim the following cell too.
    pub fn put(&mut self, x: i32, y: i32, ch: char, style: Style) {
        if ch == '\0' {
            return;
        }
        let Some(idx) = self.index(x, y) else { return };
        let width = ch.width().unwrap_or(0);
        if width == 0 {
            return;
        }
        self.back[idx] = Cell {
            ch,
            style,
            continuation: false,
        };
        if width > 1 {
            if let Some(next) = self.index(x + 1, y) {
                self.back[next] = Cell {
                    ch: '\0',
                    style,
                    continuation: true,
                };
            }
        }
    }

    /// Paint `text` from `x`, stopping at `max_width` terminal columns.
    /// Returns the columns actually consumed.
    pub fn text(&mut self, x: i32, y: i32, text: &str, style: Style, max_width: i32) -> i32 {
        let mut col = 0;
        for ch in text.chars() {
            if ch == '\n' || ch == '\r' {
                continue;
            }
            let w = ch.width().unwrap_or(0) as i32;
            if w == 0 {
                continue;
            }
            if col + w > max_width {
                break;
            }
            self.put(x + col, y, ch, style);
            col += w;
        }
        col
    }

    /// Paint `text` truncated to `max_width` columns with a trailing ellipsis
    /// when it does not fit.
    pub fn text_clipped(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        style: Style,
        max_width: i32,
    ) -> i32 {
        if max_width <= 0 {
            return 0;
        }
        if display_width(text) <= max_width {
            return self.text(x, y, text, style, max_width);
        }
        let used = self.text(x, y, text, style, max_width - 1);
        self.put(x + used, y, '~', style.dim());
        used + 1
    }

    /// Fill a horizontal run with one glyph.
    pub fn hfill(&mut self, x: i32, y: i32, len: i32, ch: char, style: Style) {
        for offset in 0..len {
            self.put(x + offset, y, ch, style);
        }
    }

    /// Emit the cells that changed since the last flush.
    pub fn flush(&mut self, out: &mut impl Write) -> std::io::Result<()> {
        if self.full_repaint {
            queue!(out, terminal::Clear(terminal::ClearType::All))?;
            self.front.fill(Cell::default());
            // A cleared terminal shows blanks; make blank cells match so the
            // diff below still skips them.
        }

        let width = self.width as usize;
        let mut pending_style: Option<Style> = None;

        for y in 0..self.height as usize {
            let row = y * width;
            let mut x = 0usize;
            while x < width {
                let idx = row + x;
                if self.back[idx] == self.front[idx] {
                    x += 1;
                    continue;
                }
                if self.back[idx].continuation {
                    // Handled when its lead glyph was emitted; if the lead did
                    // not change, repaint from the lead instead.
                    x += 1;
                    continue;
                }

                queue!(out, cursor::MoveTo(x as u16, y as u16))?;
                // Emit a run of consecutive differing cells sharing one style.
                while x < width {
                    let idx = row + x;
                    if self.back[idx].continuation {
                        // A lead glyph always advances past its own
                        // continuation, so reaching one means the run is no
                        // longer aligned. End it and let the outer loop
                        // re-anchor the cursor.
                        break;
                    }
                    if self.back[idx] == self.front[idx] {
                        break;
                    }
                    let cell = self.back[idx];
                    if pending_style != Some(cell.style) {
                        apply_style(out, cell.style)?;
                        pending_style = Some(cell.style);
                    }
                    write!(out, "{}", cell.ch)?;
                    let advance = cell.ch.width().unwrap_or(1).max(1);
                    x += advance;
                }
            }
        }

        if pending_style.is_some() {
            queue!(out, SetAttribute(Attribute::Reset))?;
        }
        out.flush()?;

        std::mem::swap(&mut self.front, &mut self.back);
        self.full_repaint = false;
        Ok(())
    }

    /// The frame being composed, as plain text rows without styling.
    ///
    /// Used by the `--snapshot` mode and by tests that assert on layout.
    pub fn snapshot_text(&self) -> Vec<String> {
        (0..self.height as usize)
            .map(|y| {
                let row = y * self.width as usize;
                (0..self.width as usize)
                    .map(|x| self.back[row + x])
                    .filter(|cell| !cell.continuation)
                    .map(|cell| cell.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }
}

fn apply_style(out: &mut impl Write, style: Style) -> std::io::Result<()> {
    queue!(out, SetAttribute(Attribute::Reset))?;
    if style.bold {
        queue!(out, SetAttribute(Attribute::Bold))?;
    }
    if style.dim {
        queue!(out, SetAttribute(Attribute::Dim))?;
    }
    if style.reverse {
        queue!(out, SetAttribute(Attribute::Reverse))?;
    }
    queue!(out, SetForegroundColor(style.fg))?;
    Ok(())
}

/// Terminal columns `text` occupies.
pub fn display_width(text: &str) -> i32 {
    text.chars()
        .filter_map(|ch| ch.width())
        .map(|w| w as i32)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump(screen: &Screen) -> String {
        let mut out = String::new();
        for y in 0..screen.height as usize {
            for x in 0..screen.width as usize {
                let cell = screen.back[y * screen.width as usize + x];
                if !cell.continuation {
                    out.push(cell.ch);
                }
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn text_stops_at_max_width() {
        let mut screen = Screen::new(10, 1);
        screen.begin_frame();
        let used = screen.text(0, 0, "abcdefghij", Style::default(), 4);
        assert_eq!(used, 4);
        assert_eq!(dump(&screen), "abcd      \n");
    }

    #[test]
    fn wide_glyphs_claim_two_columns() {
        let mut screen = Screen::new(8, 1);
        screen.begin_frame();
        // Each hangul syllable is two columns wide, so three fit in six.
        let used = screen.text(0, 0, "한글판", Style::default(), 8);
        assert_eq!(used, 6);
        assert_eq!(display_width("한글판"), 6);
        // The dump skips continuation cells, so the row is 3 glyphs + 2 blanks.
        assert_eq!(dump(&screen), "한글판  \n");
    }

    #[test]
    fn wide_glyph_is_dropped_rather_than_split() {
        let mut screen = Screen::new(8, 1);
        screen.begin_frame();
        // Only one column left: a two-column glyph must not straddle the edge.
        let used = screen.text(0, 0, "가", Style::default(), 1);
        assert_eq!(used, 0);
    }

    #[test]
    fn clipping_marks_truncation() {
        let mut screen = Screen::new(10, 1);
        screen.begin_frame();
        let used = screen.text_clipped(0, 0, "abcdefgh", Style::default(), 5);
        assert_eq!(used, 5);
        assert_eq!(dump(&screen), "abcd~     \n");
    }

    #[test]
    fn out_of_bounds_writes_are_dropped() {
        let mut screen = Screen::new(4, 2);
        screen.begin_frame();
        screen.put(-1, 0, 'x', Style::default());
        screen.put(0, 9, 'x', Style::default());
        screen.put(9, 0, 'x', Style::default());
        assert_eq!(dump(&screen), "    \n    \n");
    }

    #[test]
    fn flush_emits_only_changed_cells() {
        let mut screen = Screen::new(6, 1);
        let mut sink = Vec::new();

        screen.begin_frame();
        screen.text(0, 0, "abcdef", Style::default(), 6);
        screen.flush(&mut sink).expect("first flush");

        // Same content: the diff has nothing to emit beyond a style reset.
        sink.clear();
        screen.begin_frame();
        screen.text(0, 0, "abcdef", Style::default(), 6);
        screen.flush(&mut sink).expect("idempotent flush");
        assert!(
            !String::from_utf8_lossy(&sink).contains('a'),
            "unchanged frame repainted: {:?}",
            String::from_utf8_lossy(&sink)
        );

        // One changed glyph must appear.
        sink.clear();
        screen.begin_frame();
        screen.text(0, 0, "abZdef", Style::default(), 6);
        screen.flush(&mut sink).expect("diff flush");
        let emitted = String::from_utf8_lossy(&sink);
        assert!(emitted.contains('Z'), "changed glyph missing: {emitted:?}");
        assert!(
            !emitted.contains('f'),
            "unchanged tail repainted: {emitted:?}"
        );
    }

    #[test]
    fn resize_rebuilds_and_invalidates() {
        let mut screen = Screen::new(4, 1);
        let mut sink = Vec::new();
        screen.begin_frame();
        screen.text(0, 0, "abcd", Style::default(), 4);
        screen.flush(&mut sink).expect("flush");

        screen.resize(6, 2);
        assert_eq!((screen.width(), screen.height()), (6, 2));

        sink.clear();
        screen.begin_frame();
        screen.text(0, 0, "abcd", Style::default(), 4);
        screen.flush(&mut sink).expect("flush after resize");
        assert!(
            String::from_utf8_lossy(&sink).contains('a'),
            "resize must force a repaint"
        );
    }
}
