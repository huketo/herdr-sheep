//! Painting the pasture into a `Screen`.

use crate::herdr::Status;
use crate::layout::{self, Plan, Slot};
use crate::model::{Flock, Sheep};
use crate::render::{display_width, Screen, Style};
use crate::sprite::{self, SPRITE_H, SPRITE_W};
use crate::theme;

/// Width of the name column in the compact list.
const LABEL_COL: i32 = 16;

const KEY_HINTS: &str = "j/k select  enter focus  r refresh  q quit";

pub fn draw(screen: &mut Screen, flock: &Flock, plan: &Plan, t: f32) {
    screen.begin_frame();
    let selected = flock.selected_index(plan);

    // Grass goes down first, and only on rows nothing else will use, so it
    // never shows through a sprite or a label.
    let mut occupied = chrome_rows(screen);
    match plan {
        Plan::Pasture { zones } => {
            for zone in zones {
                mark(&mut occupied, zone.label_y);
                for (_, slot) in &zone.members {
                    for y in (slot.y - 1)..=(slot.y + SPRITE_H) {
                        mark(&mut occupied, y);
                    }
                }
            }
        }
        Plan::Compact { rows, .. } => {
            for line in 0..rows.len() as i32 {
                mark(&mut occupied, layout::HEADER_H + line);
            }
        }
        Plan::Empty => mark(&mut occupied, screen.height() as i32 / 2),
    }
    grass(screen, &occupied);

    header(screen, flock);
    match plan {
        Plan::Pasture { zones } => {
            for zone in zones {
                zone_label(
                    screen,
                    zone.label_y,
                    zone.x,
                    zone.width,
                    zone.status,
                    zone.members.len(),
                );
                for (index, slot) in &zone.members {
                    let sheep = &flock.sheep[*index];
                    sheep_at(screen, sheep, *slot, selected == Some(*index), t);
                }
            }
        }
        Plan::Compact { rows, hidden } => compact(screen, flock, rows, *hidden, selected),
        Plan::Empty => empty(screen, flock),
    }
    footer(screen, flock, selected);
}

/// Rows the header and footer always own.
fn chrome_rows(screen: &Screen) -> Vec<bool> {
    let height = screen.height() as i32;
    let mut occupied = vec![false; height.max(0) as usize];
    mark(&mut occupied, 0);
    for y in (height - layout::FOOTER_H).max(0)..height {
        mark(&mut occupied, y);
    }
    occupied
}

fn mark(occupied: &mut [bool], y: i32) {
    if y >= 0 && (y as usize) < occupied.len() {
        occupied[y as usize] = true;
    }
}

/// Sparse tufts on the rows no sheep stands on, thinning out as they get
/// further from the flock so a tall pane fades to open field instead of noise.
fn grass(screen: &mut Screen, occupied: &[bool]) {
    const TUFTS: [char; 3] = [',', '.', '\''];
    /// One tuft per this many cells directly beside the flock.
    const NEAR: u32 = 9;
    /// How much sparser each further row gets.
    const FADE: u32 = 8;
    /// Sparsest the grass ever gets before it stops entirely.
    const FAR: u32 = 90;

    let width = screen.width() as i32;
    let style = Style::fg(theme::GROUND).dim();
    let mut distance = 0u32;
    for (y, taken) in occupied.iter().enumerate() {
        if *taken {
            distance = 0;
            continue;
        }
        distance += 1;
        let sparsity = NEAR + FADE * (distance - 1);
        if sparsity > FAR {
            continue;
        }
        for x in 0..width {
            // Deterministic per cell, so the grass never shimmers.
            let hash = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
            if hash % sparsity == 0 {
                screen.put(
                    x,
                    y as i32,
                    TUFTS[(hash / 11) as usize % TUFTS.len()],
                    style,
                );
            }
        }
    }
}

fn header(screen: &mut Screen, flock: &Flock) {
    let width = screen.width() as i32;
    let mut x = screen.text(0, 0, sprite::MINI, Style::fg(theme::ACCENT).bold(), width);
    x += screen.text(
        x,
        0,
        " herdr-sheep",
        Style::fg(theme::HEADER).bold(),
        width - x,
    );

    // Right-aligned tally, most urgent first. Falls back to short words, then
    // drops entirely, rather than colliding with the title.
    let tally: Vec<(Status, usize)> = layout::ZONE_ORDER
        .iter()
        .map(|status| (*status, flock.count(*status)))
        .filter(|(_, count)| *count > 0)
        .collect();
    if tally.is_empty() {
        return;
    }

    let cells: Vec<String> = [false, true]
        .into_iter()
        .map(|short| {
            tally
                .iter()
                .map(|(status, count)| format!("{count} {}", tally_word(*status, short)))
                .collect::<Vec<_>>()
        })
        .find_map(|cells| {
            let needed: i32 = cells.iter().map(|cell| display_width(cell) + 2).sum();
            (width - needed > x).then_some(cells)
        })
        .unwrap_or_default();

    let used: i32 = cells.iter().map(|cell| display_width(cell) + 2).sum();
    let mut cursor = width - used + 2;
    for ((status, _), cell) in tally.iter().zip(&cells) {
        cursor += screen.text(
            cursor,
            0,
            cell,
            Style::fg(theme::zone(*status)),
            width - cursor,
        );
        cursor += 2;
    }
}

fn tally_word(status: Status, short: bool) -> &'static str {
    let (_, hint) = layout::zone_name(status);
    if short {
        match status {
            Status::Blocked => "wait",
            Status::Done => "done",
            Status::Working => "run",
            Status::Idle => "idle",
            Status::Unknown => "hmm",
        }
    } else {
        hint
    }
}

fn zone_label(screen: &mut Screen, y: i32, x: i32, width: i32, status: Status, count: usize) {
    let (name, hint) = layout::zone_name(status);
    let color = theme::zone(status);
    let mut cursor = x;
    cursor += screen.text(cursor, y, "~ ", Style::fg(color).dim(), width);
    cursor += screen.text(cursor, y, name, Style::fg(color).bold(), x + width - cursor);
    cursor += screen.text(
        cursor,
        y,
        &format!("  {count} {hint} "),
        Style::fg(color).dim(),
        x + width - cursor,
    );
    // Fill the rest of the divider with grass.
    let remaining = x + width - cursor;
    if remaining > 0 {
        screen.hfill(cursor, y, remaining, '~', Style::fg(theme::GROUND).dim());
    }
}

fn sheep_at(screen: &mut Screen, sheep: &Sheep, slot: Slot, selected: bool, t: f32) {
    let origin_x = sheep.x.round() as i32;
    let origin_y = sheep.y.round() as i32 + sheep.hop();
    let wool = theme::wool(&sheep.view.breed);
    let frame = sprite::frame(sheep.status(), sheep.state_t, sheep.facing_left);

    decorations(screen, sheep, slot, origin_x, origin_y, t);

    for row in 0..SPRITE_H {
        for col in 0..SPRITE_W {
            let Some(glyph) = frame.glyph(col, row) else {
                continue;
            };
            let role = sprite::role_of(glyph);
            let mut style = Style::fg(role.color(wool));
            if selected {
                style = style.bold();
                if role == sprite::Role::Wool {
                    style = Style::fg(theme::ACCENT).bold();
                }
            }
            screen.put(origin_x + col, origin_y + row, glyph, style);
        }
    }

    label(screen, sheep, slot, selected);
}

/// Motion cues that live outside the sprite box.
///
/// Overhead cues use `slot.y - 1`, the row the layout reserved for them, not
/// the drawn origin: a jumping sheep rises into that row and its sparkles must
/// not follow it onto the zone divider.
fn decorations(screen: &mut Screen, sheep: &Sheep, slot: Slot, x: i32, y: i32, t: f32) {
    let overhead = slot.y - 1;
    match sheep.status() {
        Status::Working => {
            // Dust puffs trailing the direction of travel.
            let behind = if sheep.facing_left {
                x + SPRITE_W
            } else {
                x - 1
            };
            let step = if sheep.facing_left { 1 } else { -1 };
            for puff in 0..2 {
                if (t * 6.0 + puff as f32).sin() > -0.3 {
                    screen.put(
                        behind + step * puff,
                        y + SPRITE_H - 1,
                        '.',
                        Style::fg(theme::GROUND).dim(),
                    );
                }
            }
        }
        Status::Blocked => {
            // A question bubble blinking over the head.
            let bright = (t * 1.6).sin() > 0.0;
            let style = if bright {
                Style::fg(theme::ALERT).bold()
            } else {
                Style::fg(theme::ALERT).dim()
            };
            let bubble_x = if sheep.facing_left {
                x + 1
            } else {
                x + SPRITE_W - 4
            };
            screen.text(bubble_x, overhead, "(?)", style, 3);
        }
        Status::Done => {
            // Sparkles beside and above the sheep, twinkling out of phase.
            const SPARKS: [(i32, i32, f32); 3] =
                [(-1, 1, 0.0), (SPRITE_W, 0, 1.1), (SPRITE_W - 2, -1, 2.3)];
            for (dx, dy, phase) in SPARKS {
                let wave = (t * 3.4 + phase + sheep.jitter * 6.0).sin();
                if wave > 0.2 {
                    let glyph = if wave > 0.7 { '*' } else { '+' };
                    let row = if dy < 0 { overhead } else { slot.y + dy };
                    screen.put(x + dx, row, glyph, Style::fg(theme::CHEER).bold());
                }
            }
        }
        Status::Unknown => {
            // Snoring: `z`, `z z`, `z z z`, over and over.
            let visible = 1 + ((t * 0.9 + sheep.jitter * 3.0) as i32).rem_euclid(3);
            for particle in 0..visible {
                screen.put(
                    x + SPRITE_W - 2 + particle * 2,
                    overhead,
                    'z',
                    Style::fg(theme::MUTED).dim(),
                );
            }
        }
        Status::Idle => {
            // Grass tufts to nibble at, on the hoof row.
            let ground = y + SPRITE_H - 1;
            screen.put(x - 1, ground, ',', Style::fg(theme::GROUND).dim());
            screen.put(x + SPRITE_W, ground, '.', Style::fg(theme::GROUND).dim());
        }
    }
}

fn label(screen: &mut Screen, sheep: &Sheep, slot: Slot, selected: bool) {
    let y = slot.y + SPRITE_H;
    let text = format!("{}{}", focus_marker(sheep), sheep.view.label);
    let style = if selected {
        Style::fg(theme::ACCENT).bold().reverse()
    } else {
        Style::fg(theme::wool(&sheep.view.breed))
    };
    // The name follows the sheep, centered under the sprite and kept inside
    // the lane so it never runs into a neighbour.
    let text_w = display_width(&text).min(slot.lane_w);
    let centered = sheep.x.round() as i32 + (SPRITE_W - text_w) / 2;
    let x = centered.clamp(slot.lane_x, slot.lane_x + slot.lane_w - text_w);
    screen.text_clipped(x, y, &text, style, slot.lane_x + slot.lane_w - x);
}

/// Marks the sheep whose pane Herdr currently focuses.
fn focus_marker(sheep: &Sheep) -> &'static str {
    if sheep.view.focused {
        "*"
    } else {
        ""
    }
}

fn compact(
    screen: &mut Screen,
    flock: &Flock,
    rows: &[usize],
    hidden: usize,
    selected: Option<usize>,
) {
    let width = screen.width() as i32;
    for (line, index) in rows.iter().enumerate() {
        let sheep = &flock.sheep[*index];
        let y = layout::HEADER_H + line as i32;
        let wool = theme::wool(&sheep.view.breed);
        let is_selected = selected == Some(*index);

        let mut x = screen.text(
            0,
            y,
            sprite::MINI,
            if is_selected {
                Style::fg(theme::ACCENT).bold()
            } else {
                Style::fg(wool)
            },
            width,
        );
        x += 1;
        let (_, hint) = layout::zone_name(sheep.status());
        x += screen.text(
            x,
            y,
            &format!("{hint:<14} "),
            Style::fg(theme::zone(sheep.status())),
            width - x,
        );
        let style = if is_selected {
            Style::fg(theme::ACCENT).bold().reverse()
        } else {
            Style::fg(wool)
        };
        let label_w = (width - x).min(LABEL_COL);
        let used = screen.text_clipped(x, y, &sheep.view.label, style, label_w);
        x += label_w.max(used) + 1;
        if width - x > 4 {
            screen.text_clipped(
                x,
                y,
                &sheep.view.title,
                Style::fg(theme::MUTED).dim(),
                width - x,
            );
        }
    }

    if hidden > 0 {
        let y = layout::HEADER_H + rows.len() as i32 - 1;
        let text = format!("+{hidden} more (grow the pane)");
        let x = (screen.width() as i32 - display_width(&text)).max(0);
        screen.text(x, y, &text, Style::fg(theme::MUTED).dim(), width);
    }
}

fn empty(screen: &mut Screen, flock: &Flock) {
    let width = screen.width() as i32;
    let y = (screen.height() as i32) / 2;
    let text = if flock.loaded {
        "no agents in this session - start one and it joins the flock"
    } else {
        "listening for the herd..."
    };
    let x = ((width - display_width(text)) / 2).max(0);
    screen.text_clipped(x, y, text, Style::fg(theme::MUTED).dim(), width);
}

fn footer(screen: &mut Screen, flock: &Flock, selected: Option<usize>) {
    let width = screen.width() as i32;
    let height = screen.height() as i32;
    let detail_y = height - 2;
    let hints_y = height - 1;

    if let Some(error) = &flock.error {
        screen.text_clipped(
            0,
            detail_y,
            &format!("! {error}"),
            Style::fg(theme::ERROR),
            width,
        );
    } else if let Some(index) = selected {
        detail(screen, &flock.sheep[index], detail_y, width);
    } else if flock.loaded && !flock.sheep.is_empty() {
        screen.text_clipped(
            0,
            detail_y,
            "press j or k to pick a sheep",
            Style::fg(theme::MUTED).dim(),
            width,
        );
    }

    screen.text_clipped(0, hints_y, KEY_HINTS, Style::fg(theme::MUTED).dim(), width);
}

/// Detail line for the selected sheep: identity, then budget, then title.
fn detail(screen: &mut Screen, sheep: &Sheep, y: i32, width: i32) {
    /// Append one styled chunk, advancing the cursor.
    fn chunk(screen: &mut Screen, x: &mut i32, y: i32, width: i32, text: &str, style: Style) {
        if *x >= width {
            return;
        }
        *x += screen.text(*x, y, text, style, width - *x);
    }

    let gap = Style::fg(theme::MUTED).dim();
    let field = Style::fg(theme::HEADER);
    let mut x = 0;

    chunk(
        screen,
        &mut x,
        y,
        width,
        &sheep.view.pane_id,
        Style::fg(theme::ACCENT),
    );
    chunk(screen, &mut x, y, width, "  ", gap);
    chunk(screen, &mut x, y, width, &sheep.view.workspace, field);
    chunk(screen, &mut x, y, width, "  ", gap);
    // Show the agent kind Herdr detected alongside the model provider it
    // reports, unless they are the same word.
    let breed = if sheep.view.kind.eq_ignore_ascii_case(&sheep.view.breed) {
        sheep.view.breed.clone()
    } else {
        format!("{}/{}", sheep.view.kind, sheep.view.breed)
    };
    chunk(
        screen,
        &mut x,
        y,
        width,
        &breed,
        Style::fg(theme::wool(&sheep.view.breed)),
    );
    if let Some(context) = &sheep.view.context {
        chunk(screen, &mut x, y, width, "  ctx ", gap);
        chunk(screen, &mut x, y, width, context, field);
    }
    if let Some(limit) = &sheep.view.limit {
        chunk(screen, &mut x, y, width, "  ", gap);
        chunk(screen, &mut x, y, width, limit, field);
    }
    if !sheep.view.title.is_empty() && x + 4 < width {
        chunk(screen, &mut x, y, width, "  ", gap);
        screen.text_clipped(x, y, &sheep.view.title, Style::fg(theme::MUTED), width - x);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::{AgentView, Snapshot};

    fn view(pane: &str, status: Status, label: &str) -> AgentView {
        AgentView {
            pane_id: pane.to_string(),
            workspace: "4:bivri".to_string(),
            status,
            label: label.to_string(),
            breed: "claude".to_string(),
            kind: "omp".to_string(),
            title: "some terminal title".to_string(),
            context: Some("31% (306k)".to_string()),
            limit: Some("5h 100%".to_string()),
            focused: false,
        }
    }

    fn flock_of(agents: Vec<AgentView>) -> Flock {
        let mut flock = Flock::default();
        flock.apply(Snapshot {
            agents,
            focused_pane_id: None,
        });
        flock
    }

    /// Render one settled frame and return it as plain text rows.
    fn frame_text(flock: &mut Flock, width: u16, height: u16) -> Vec<String> {
        let mut screen = Screen::new(width, height);
        let mut plan = layout::plan(width as i32, height as i32, &flock.statuses());
        let dt = 1.0 / 30.0;
        let mut t = 0.0;
        for _ in 0..90 {
            plan = layout::plan(width as i32, height as i32, &flock.statuses());
            flock.update(&plan, dt);
            t += dt;
        }
        draw(&mut screen, flock, &plan, t);
        screen.snapshot_text()
    }

    #[test]
    fn pasture_shows_zone_labels_and_sheep_names() {
        let mut flock = flock_of(vec![
            view("w1:p1", Status::Blocked, "deploy"),
            view("w2:p1", Status::Working, "runner"),
            view("w3:p1", Status::Idle, "scout"),
        ]);
        let rows = frame_text(&mut flock, 80, 30);
        let text = rows.join("\n");

        assert!(text.contains("herdr-sheep"), "{text}");
        assert!(text.contains("GATE"), "{text}");
        assert!(text.contains("PADDOCK"), "{text}");
        assert!(text.contains("MEADOW"), "{text}");
        assert!(
            !text.contains("PEN"),
            "empty zones must not be drawn:\n{text}"
        );
        for name in ["deploy", "runner", "scout"] {
            assert!(text.contains(name), "missing sheep {name}:\n{text}");
        }
        // The mascot's prompt face is on screen.
        assert!(text.contains(">_") || text.contains("_<"), "{text}");
    }

    #[test]
    fn a_blocked_sheep_gets_a_question_bubble() {
        let mut flock = flock_of(vec![view("w1:p1", Status::Blocked, "deploy")]);
        let rows = frame_text(&mut flock, 80, 24);
        assert!(
            rows.join("\n").contains("(?)"),
            "blocked sheep must ask for help:\n{}",
            rows.join("\n")
        );
    }

    #[test]
    fn a_sleeping_sheep_snores() {
        let mut flock = flock_of(vec![view("w1:p1", Status::Unknown, "mystery")]);
        let rows = frame_text(&mut flock, 80, 24);
        assert!(rows.join("").contains('z'), "{rows:?}");
    }

    #[test]
    fn selecting_a_sheep_shows_its_details() {
        let mut flock = flock_of(vec![view("w1:p1", Status::Working, "runner")]);
        let plan = layout::plan(80, 24, &flock.statuses());
        flock.move_selection(&plan, 1);
        let text = frame_text(&mut flock, 80, 24).join("\n");

        assert!(text.contains("w1:p1"), "{text}");
        assert!(text.contains("4:bivri"), "{text}");
        assert!(text.contains("31% (306k)"), "{text}");
        assert!(text.contains("some terminal title"), "{text}");
    }

    #[test]
    fn nothing_is_drawn_outside_the_pane() {
        // A crowded flock in a small pane must not paint past the edges; the
        // Screen drops those writes, so this asserts the row geometry instead.
        let mut flock = flock_of(
            (0..8)
                .map(|i| view(&format!("w{i}:p1"), Status::Working, "runnnnnnnnnner"))
                .collect(),
        );
        let rows = frame_text(&mut flock, 40, 20);
        assert_eq!(rows.len(), 20);
        for row in &rows {
            assert!(display_width(row) <= 40, "row overflowed: {row:?}");
        }
    }

    #[test]
    fn a_tiny_pane_falls_back_to_the_compact_list() {
        let mut flock = flock_of(vec![
            view("w1:p1", Status::Working, "runner"),
            view("w2:p1", Status::Blocked, "deploy"),
        ]);
        let text = frame_text(&mut flock, 60, 6).join("\n");
        assert!(text.contains("waiting on you"), "{text}");
        assert!(text.contains("runner"), "{text}");
        assert!(
            !text.contains("PADDOCK"),
            "compact mode has no zones:\n{text}"
        );
    }

    #[test]
    fn an_empty_session_says_so() {
        let mut flock = flock_of(Vec::new());
        let text = frame_text(&mut flock, 60, 12).join("\n");
        assert!(text.contains("no agents"), "{text}");
    }

    #[test]
    fn a_poll_error_replaces_the_detail_line() {
        let mut flock = flock_of(vec![view("w1:p1", Status::Working, "runner")]);
        flock.error = Some("herdr api snapshot failed".to_string());
        let rows = frame_text(&mut flock, 70, 24);
        assert!(
            rows[rows.len() - 2].contains("herdr api snapshot failed"),
            "{rows:?}"
        );
        assert!(rows[rows.len() - 1].contains("q quit"), "{rows:?}");
    }

    #[test]
    fn korean_titles_do_not_overflow_the_pane() {
        let mut agent = view("w1:p1", Status::Working, "러너");
        agent.title = "herdr TUI 양 마스코트 플러그인 만들기".to_string();
        let mut flock = flock_of(vec![agent]);
        let plan = layout::plan(50, 24, &flock.statuses());
        flock.move_selection(&plan, 1);
        for row in frame_text(&mut flock, 50, 24) {
            assert!(display_width(&row) <= 50, "row overflowed: {row:?}");
        }
    }
}
