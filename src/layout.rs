//! Where every sheep stands.
//!
//! The pasture is divided into zones, one per agent state, ordered by how much
//! they want your attention. Zones with no sheep are not drawn at all, so the
//! space always belongs to agents that exist. When even that does not fit, the
//! whole pasture degrades to a one-line-per-agent list.

use crate::herdr::Status;
use crate::sprite::{BARN_H, BARN_W, SPRITE_H, SPRITE_W};

/// Rows reserved at the bottom: selection detail, then the key hints.
pub const FOOTER_H: i32 = 2;
/// Rows reserved at the top for the header.
pub const HEADER_H: i32 = 1;

/// Narrowest lane a sheep can stand in.
const MIN_LANE: i32 = SPRITE_W + 2;
/// Narrowest lane a running sheep gets, so there is room to actually run.
const MIN_RUN_LANE: i32 = SPRITE_W + 6;
/// Columns kept between the barn and the right edge of the pane.
const BARN_MARGIN: i32 = 2;

/// States whose sheep draw something over their heads: a question bubble,
/// sparkles, or sleep glyphs. Those zones reserve one extra row per sheep row.
const fn has_overhead(status: Status) -> bool {
    matches!(status, Status::Blocked | Status::Done | Status::Unknown)
}

/// Rows between the tops of two stacked sheep rows in a zone.
const fn row_pitch(status: Status) -> i32 {
    // sprite + name label, plus the overhead row when the state uses one.
    SPRITE_H + 1 + has_overhead(status) as i32
}

/// Zones from "needs you now" to "nothing to see".
pub const ZONE_ORDER: [Status; 5] = [
    Status::Blocked,
    Status::Done,
    Status::Working,
    Status::Idle,
    Status::Unknown,
];

/// Zone name and the state it stands for.
pub fn zone_name(status: Status) -> (&'static str, &'static str) {
    match status {
        Status::Blocked => ("GATE", "waiting on you"),
        Status::Done => ("PEN", "done"),
        Status::Working => ("PADDOCK", "working"),
        Status::Idle => ("MEADOW", "idle"),
        Status::Unknown => ("FOLD", "unknown"),
    }
}

/// A sheep's standing room: the lane it owns and the row its hooves are on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub lane_x: i32,
    pub lane_w: i32,
    pub y: i32,
    /// First row the slot owns, which is the overhead cue row when the state
    /// draws one. `top..=y + SPRITE_H` tiles the zone, so it doubles as the
    /// click target.
    pub top: i32,
}

impl Slot {
    /// Columns a sheep can travel inside its lane.
    pub fn travel(&self) -> i32 {
        (self.lane_w - SPRITE_W).max(0)
    }

    /// Lane-centered x, for sheep that are not moving.
    pub fn center_x(&self) -> f32 {
        self.lane_x as f32 + self.travel() as f32 / 2.0
    }

    /// True when the pane cell at `(x, y)` belongs to this slot.
    ///
    /// The whole lane counts, not just the sprite: a sheep wanders inside its
    /// lane, so hit-testing the drawn body would make clicking it a game of
    /// timing.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.lane_x && x < self.lane_x + self.lane_w && y >= self.top && y <= self.y + SPRITE_H
    }
}

#[derive(Debug, Clone)]
pub struct Zone {
    pub status: Status,
    /// Row carrying the zone label.
    pub label_y: i32,
    pub x: i32,
    pub width: i32,
    /// Flock indices in this zone, paired with their slot.
    pub members: Vec<(usize, Slot)>,
}

/// Background the pasture only draws when there are rows to spare, so scenery
/// never costs a sheep its standing room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scenery {
    /// Row carrying the fence along the bottom of the field.
    pub fence_y: i32,
    /// Top-left of the barn, which stands on the fence row.
    pub barn: Option<(i32, i32)>,
}

#[derive(Debug, Clone)]
pub enum Plan {
    /// Full pasture with animated sheep.
    Pasture {
        zones: Vec<Zone>,
        scenery: Option<Scenery>,
    },
    /// One line per agent, used when the pasture cannot fit.
    Compact {
        /// Flock indices in display order, already clipped to the viewport.
        rows: Vec<usize>,
        /// Agents that did not fit.
        hidden: usize,
    },
    /// No agents at all.
    Empty,
}

impl Plan {
    /// Flock indices in display order, for selection movement.
    pub fn order(&self) -> Vec<usize> {
        match self {
            Plan::Pasture { zones, .. } => zones
                .iter()
                .flat_map(|zone| zone.members.iter().map(|(index, _)| *index))
                .collect(),
            Plan::Compact { rows, .. } => rows.clone(),
            Plan::Empty => Vec::new(),
        }
    }

    /// Slot assigned to a flock index, when the plan places sheep.
    pub fn slot_of(&self, index: usize) -> Option<Slot> {
        let Plan::Pasture { zones, .. } = self else {
            return None;
        };
        zones.iter().find_map(|zone| {
            zone.members
                .iter()
                .find(|(member, _)| *member == index)
                .map(|(_, slot)| *slot)
        })
    }

    /// Flock index drawn at pane cell `(x, y)`, for mouse selection.
    pub fn index_at(&self, x: i32, y: i32) -> Option<usize> {
        match self {
            Plan::Pasture { zones, .. } => zones.iter().find_map(|zone| {
                zone.members
                    .iter()
                    .find(|(_, slot)| slot.contains(x, y))
                    .map(|(index, _)| *index)
            }),
            // One agent per row, full width.
            Plan::Compact { rows, .. } => {
                if x < 0 || y < HEADER_H {
                    return None;
                }
                rows.get((y - HEADER_H) as usize).copied()
            }
            Plan::Empty => None,
        }
    }
}

fn lane_count(members: usize, width: i32, min_lane: i32) -> i32 {
    let per_row = (width / min_lane).max(1);
    per_row.min(members as i32).max(1)
}

/// Assign zones and slots for `statuses`, one entry per sheep in flock order.
pub fn plan(width: i32, height: i32, statuses: &[Status]) -> Plan {
    if statuses.is_empty() {
        return Plan::Empty;
    }

    let field_y = HEADER_H;
    let field_h = height - HEADER_H - FOOTER_H;
    if width < MIN_LANE || field_h < 1 + row_pitch(Status::Working) {
        return compact(statuses, height);
    }

    // Group flock indices by zone, keeping flock order inside a zone so a
    // sheep does not hop lanes when an unrelated agent appears.
    let groups: Vec<(Status, Vec<usize>)> = ZONE_ORDER
        .iter()
        .filter_map(|status| {
            let members: Vec<usize> = statuses
                .iter()
                .enumerate()
                .filter(|(_, item)| **item == *status)
                .map(|(index, _)| index)
                .collect();
            (!members.is_empty()).then_some((*status, members))
        })
        .collect();

    let heights: Vec<i32> = groups
        .iter()
        .map(|(status, members)| {
            let min_lane = min_lane_for(*status);
            let cols = lane_count(members.len(), width, min_lane);
            let rows = (members.len() as i32 + cols - 1) / cols;
            1 + rows * row_pitch(*status)
        })
        .collect();

    let needed: i32 = heights.iter().sum();
    if needed > field_h {
        return compact(statuses, height);
    }

    // Rows nothing needs. Scenery gets first call on them, then what is left
    // becomes one blank row between zones, top down.
    let mut spare = field_h - needed;
    let scenery = scenery(width, field_y + field_h, &mut spare);
    let mut y = field_y;
    let mut zones = Vec::with_capacity(groups.len());

    for (zone_index, ((status, members), zone_h)) in groups.into_iter().zip(heights).enumerate() {
        if zone_index > 0 && spare > 0 {
            y += 1;
            spare -= 1;
        }
        let min_lane = min_lane_for(status);
        let cols = lane_count(members.len(), width, min_lane);
        let lane_w = width / cols;
        let pitch = row_pitch(status);
        let overhead = has_overhead(status) as i32;
        // Overhead decorations sit on the row above the sprite, so the first
        // sprite row drops one further below the zone label.
        let first_row = y + 1 + overhead;
        let placed = members
            .into_iter()
            .enumerate()
            .map(|(seat, index)| {
                let col = seat as i32 % cols;
                let row = seat as i32 / cols;
                let sprite_y = first_row + row * pitch;
                (
                    index,
                    Slot {
                        lane_x: col * lane_w,
                        lane_w,
                        y: sprite_y,
                        top: sprite_y - overhead,
                    },
                )
            })
            .collect();

        zones.push(Zone {
            status,
            label_y: y,
            x: 0,
            width,
            members: placed,
        });
        y += zone_h;
    }

    Plan::Pasture { zones, scenery }
}

/// Claim the bottom of the field for background scenery, out of `spare` rows.
///
/// The fence is one row and the barn stands on it, so a barn costs
/// `BARN_H - 1` rows more. `bottom` is one past the last field row.
fn scenery(width: i32, bottom: i32, spare: &mut i32) -> Option<Scenery> {
    if *spare < 1 {
        return None;
    }
    *spare -= 1;
    let fence_y = bottom - 1;

    // A barn needs its own rows and enough width that it does not crowd the
    // lanes it stands beside.
    let barn = (*spare >= BARN_H - 1 && width >= BARN_W + MIN_LANE).then(|| {
        *spare -= BARN_H - 1;
        (width - BARN_W - BARN_MARGIN, fence_y - BARN_H + 1)
    });
    Some(Scenery { fence_y, barn })
}

fn min_lane_for(status: Status) -> i32 {
    if status == Status::Working {
        MIN_RUN_LANE
    } else {
        MIN_LANE
    }
}

fn compact(statuses: &[Status], height: i32) -> Plan {
    let mut ordered: Vec<usize> = Vec::with_capacity(statuses.len());
    for status in ZONE_ORDER {
        ordered.extend(
            statuses
                .iter()
                .enumerate()
                .filter(|(_, item)| **item == status)
                .map(|(index, _)| index),
        );
    }
    let capacity = (height - HEADER_H - FOOTER_H).max(0) as usize;
    let hidden = ordered.len().saturating_sub(capacity);
    ordered.truncate(capacity);
    Plan::Compact {
        rows: ordered,
        hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zones(plan: &Plan) -> &[Zone] {
        match plan {
            Plan::Pasture { zones, .. } => zones,
            other => panic!("expected a pasture plan, got {other:?}"),
        }
    }

    fn scenery_of(plan: &Plan) -> Option<Scenery> {
        match plan {
            Plan::Pasture { scenery, .. } => *scenery,
            other => panic!("expected a pasture plan, got {other:?}"),
        }
    }

    #[test]
    fn no_agents_yields_an_empty_plan() {
        assert!(matches!(plan(80, 24, &[]), Plan::Empty));
    }

    #[test]
    fn zones_are_ordered_by_urgency_and_skip_empties() {
        let statuses = [Status::Idle, Status::Working, Status::Blocked, Status::Idle];
        let plan = plan(80, 30, &statuses);
        let order: Vec<Status> = zones(&plan).iter().map(|zone| zone.status).collect();
        assert_eq!(
            order,
            vec![Status::Blocked, Status::Working, Status::Idle],
            "urgent zones come first and Done/Unknown are absent"
        );
    }

    #[test]
    fn zones_never_overlap_and_stay_inside_the_field() {
        let statuses = [
            Status::Blocked,
            Status::Done,
            Status::Working,
            Status::Working,
            Status::Idle,
            Status::Unknown,
        ];
        let plan = plan(80, 40, &statuses);
        let mut lowest = HEADER_H - 1;
        for zone in zones(&plan) {
            assert!(
                zone.label_y > lowest,
                "zone {zone:?} overlaps the one above"
            );
            lowest = zone.label_y;
            for (_, slot) in &zone.members {
                assert!(slot.y > zone.label_y);
                let bottom = slot.y + SPRITE_H;
                assert!(bottom < 40 - FOOTER_H, "slot {slot:?} runs into the footer");
                assert!(slot.lane_x >= 0 && slot.lane_x + slot.lane_w <= 80);
                lowest = lowest.max(bottom);
            }
        }
    }

    #[test]
    fn every_sheep_gets_exactly_one_slot() {
        let statuses = [Status::Working; 7];
        let plan = plan(80, 40, &statuses);
        let mut seen: Vec<usize> = plan.order();
        seen.sort_unstable();
        assert_eq!(seen, (0..7).collect::<Vec<_>>());
        for index in 0..7 {
            assert!(plan.slot_of(index).is_some());
        }
    }

    #[test]
    fn lanes_in_a_row_do_not_overlap() {
        let statuses = [Status::Idle; 5];
        let plan = plan(80, 40, &statuses);
        let zone = &zones(&plan)[0];
        let mut by_row: std::collections::HashMap<i32, Vec<Slot>> = Default::default();
        for (_, slot) in &zone.members {
            by_row.entry(slot.y).or_default().push(*slot);
        }
        for (_, mut row) in by_row {
            row.sort_by_key(|slot| slot.lane_x);
            for pair in row.windows(2) {
                assert!(
                    pair[0].lane_x + pair[0].lane_w <= pair[1].lane_x,
                    "lanes {:?} and {:?} overlap",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    #[test]
    fn a_lone_runner_gets_the_whole_width_to_run_in() {
        let plan = plan(80, 24, &[Status::Working]);
        let slot = plan.slot_of(0).expect("slot");
        assert_eq!(slot.lane_w, 80);
        assert_eq!(slot.travel(), 80 - SPRITE_W);
    }

    #[test]
    fn running_lanes_are_wider_than_standing_lanes() {
        let running = plan(80, 24, &[Status::Working; 8]);
        let standing = plan(80, 24, &[Status::Idle; 8]);
        assert!(
            running.slot_of(0).unwrap().lane_w > standing.slot_of(0).unwrap().lane_w,
            "runners need room to run"
        );
    }

    #[test]
    fn a_short_pane_falls_back_to_the_compact_list() {
        let statuses = [Status::Working, Status::Idle, Status::Blocked];
        match plan(80, 6, &statuses) {
            Plan::Compact { rows, hidden } => {
                // Urgency order, and only what fits.
                assert_eq!(rows, vec![2, 0, 1]);
                assert_eq!(hidden, 0);
            }
            other => panic!("expected compact, got {other:?}"),
        }
    }

    #[test]
    fn a_narrow_pane_falls_back_to_the_compact_list() {
        assert!(matches!(plan(6, 40, &[Status::Idle]), Plan::Compact { .. }));
    }

    #[test]
    fn compact_reports_what_it_had_to_hide() {
        match plan(80, 6, &[Status::Idle; 10]) {
            Plan::Compact { rows, hidden } => {
                assert_eq!(rows.len(), 3);
                assert_eq!(hidden, 7);
            }
            other => panic!("expected compact, got {other:?}"),
        }
    }

    #[test]
    fn many_agents_in_a_tall_pane_still_fit_the_pasture() {
        let statuses = [Status::Working; 12];
        let plan = plan(120, 40, &statuses);
        assert!(matches!(plan, Plan::Pasture { .. }));
        assert_eq!(plan.order().len(), 12);
    }

    #[test]
    fn scenery_waits_until_there_are_rows_to_spare() {
        let statuses = [Status::Blocked, Status::Working, Status::Idle];
        // Exactly as tall as the zones need: the field belongs to the sheep.
        let tight = (1..60)
            .map(|height| (height, plan(80, height, &statuses)))
            .find(|(_, plan)| matches!(plan, Plan::Pasture { .. }))
            .expect("some height fits the pasture");
        assert_eq!(scenery_of(&tight.1), None, "no spare rows at {}", tight.0);

        // One spare row buys the fence, and it goes on the last field row.
        let fenced = plan(80, tight.0 + 1, &statuses);
        let scenery = scenery_of(&fenced).expect("a fence");
        assert_eq!(scenery.fence_y, tight.0 + 1 - FOOTER_H - 1);
        assert_eq!(scenery.barn, None, "one row is not a barn");

        // The barn needs its own rows on top of the fence row.
        let barn = scenery_of(&plan(80, tight.0 + BARN_H, &statuses))
            .expect("a fence")
            .barn
            .expect("a barn");
        assert_eq!(barn.1 + BARN_H - 1, tight.0 + BARN_H - FOOTER_H - 1);
        assert!(
            barn.0 + BARN_W <= 80,
            "the barn hangs off the pane: {barn:?}"
        );
    }

    #[test]
    fn scenery_never_stands_on_a_sheep() {
        let statuses = [
            Status::Blocked,
            Status::Done,
            Status::Working,
            Status::Idle,
            Status::Unknown,
        ];
        for height in 20..60 {
            let plan = plan(80, height, &statuses);
            let Plan::Pasture { zones, scenery } = &plan else {
                continue;
            };
            let Some(scenery) = scenery else { continue };
            let top = scenery.barn.map_or(scenery.fence_y, |(_, y)| y);
            for zone in zones {
                for (_, slot) in &zone.members {
                    assert!(
                        slot.y + SPRITE_H < top,
                        "scenery {scenery:?} lands on slot {slot:?} at height {height}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_cell_of_a_zone_belongs_to_one_sheep() {
        // Slots tile their zone, so a click anywhere below a zone label picks a
        // sheep, and never two.
        let statuses = [Status::Working; 4];
        let plan = plan(80, 40, &statuses);
        let zone = &zones(&plan)[0];
        let bottom = zone
            .members
            .iter()
            .map(|(_, slot)| slot.y + SPRITE_H)
            .max()
            .unwrap();
        for y in (zone.label_y + 1)..=bottom {
            for x in 0..80 {
                let hits: Vec<usize> = zone
                    .members
                    .iter()
                    .filter(|(_, slot)| slot.contains(x, y))
                    .map(|(index, _)| *index)
                    .collect();
                assert_eq!(hits.len(), 1, "cell ({x}, {y}) hit {hits:?}");
                assert_eq!(plan.index_at(x, y), Some(hits[0]));
            }
        }
    }

    #[test]
    fn clicking_chrome_or_open_field_hits_nothing() {
        let plan = plan(80, 40, &[Status::Blocked, Status::Idle]);
        assert_eq!(plan.index_at(0, 0), None, "the header is not a sheep");
        for zone in zones(&plan) {
            assert_eq!(plan.index_at(2, zone.label_y), None, "{zone:?}");
        }
        assert_eq!(plan.index_at(0, 40 - FOOTER_H), None, "the footer");
        assert_eq!(plan.index_at(-1, 5), None, "outside the pane");
    }

    #[test]
    fn a_click_in_the_compact_list_picks_that_row() {
        let statuses = [Status::Idle, Status::Blocked, Status::Working];
        let plan = plan(80, 6, &statuses);
        let rows = plan.order();
        assert_eq!(plan.index_at(0, HEADER_H), Some(rows[0]));
        assert_eq!(plan.index_at(40, HEADER_H + 2), Some(rows[2]));
        assert_eq!(plan.index_at(0, HEADER_H - 1), None);
        assert_eq!(plan.index_at(0, HEADER_H + 3), None, "past the last row");
    }
}
