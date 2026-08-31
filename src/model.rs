//! Flock state and the motion that makes a status readable at a glance.
//!
//! Every sheep eases toward a target derived from its slot plus a
//! status-specific motion. Because slots are recomputed each frame, a layout
//! change makes sheep glide to their new lane instead of teleporting.

use crate::herdr::{AgentView, Snapshot, Status};
use crate::layout::{Plan, Slot};
use crate::sprite::SPRITE_W;

/// Cells per second a working sheep covers.
const RUN_SPEED: f32 = 7.0;
/// Easing rate toward the target position. Higher is snappier.
const EASE: f32 = 9.0;
/// Seconds between jumps in the pen.
const JUMP_PERIOD: f32 = 2.2;
/// Seconds a jump stays airborne.
const JUMP_AIR: f32 = 0.45;

#[derive(Debug, Clone)]
pub struct Sheep {
    pub view: AgentView,
    /// Seconds since this sheep entered its current status.
    pub state_t: f32,
    /// Seconds since this sheep joined the flock.
    pub age: f32,
    /// Current drawn position, top-left of the sprite box.
    pub x: f32,
    pub y: f32,
    pub facing_left: bool,
    /// Deterministic per-sheep offset so the flock does not animate in lockstep.
    pub jitter: f32,
    /// True until the first update placed this sheep, so it can walk in.
    fresh: bool,
}

impl Sheep {
    fn new(view: AgentView) -> Self {
        let jitter = hash_unit(&view.pane_id);
        Self {
            view,
            state_t: 0.0,
            age: 0.0,
            x: 0.0,
            y: 0.0,
            facing_left: true,
            jitter,
            fresh: true,
        }
    }

    pub fn status(&self) -> Status {
        self.view.status
    }

    /// Vertical offset of the sprite for this frame, in rows. Only jumping
    /// sheep leave the ground.
    pub fn hop(&self) -> i32 {
        if self.view.status != Status::Done {
            return 0;
        }
        let phase = (self.state_t + self.jitter * JUMP_PERIOD) % JUMP_PERIOD;
        if phase < JUMP_AIR {
            -1
        } else {
            0
        }
    }

    /// Where the sheep wants to be, given its slot.
    fn target(&self, slot: Slot) -> (f32, f32, Option<bool>) {
        let travel = slot.travel() as f32;
        let y = slot.y as f32;
        match self.view.status {
            Status::Working if travel > 0.5 => {
                // Triangle wave: run to the far side of the lane, turn, repeat.
                let period = 2.0 * travel / RUN_SPEED;
                let phase = ((self.age + self.jitter * period) % period) / period;
                let (offset, facing_left) = if phase < 0.5 {
                    (travel * (1.0 - phase * 2.0), true)
                } else {
                    (travel * (phase - 0.5) * 2.0, false)
                };
                (slot.lane_x as f32 + offset, y, Some(facing_left))
            }
            Status::Idle => {
                // A grazing sheep drifts a little and turns around now and then.
                let drift = (self.age * 0.35 + self.jitter * 6.3).sin() * (travel / 2.0).min(2.0);
                let facing_left = ((self.age * 0.09 + self.jitter) % 1.0) < 0.5;
                (slot.center_x() + drift, y, Some(facing_left))
            }
            _ => (slot.center_x(), y, None),
        }
    }

    /// Put a brand-new sheep just off the left edge so it walks into its lane.
    fn place(&mut self, slot: Slot) {
        let (_, ty, _) = self.target(slot);
        self.x = -(SPRITE_W as f32);
        self.y = ty;
        self.facing_left = false;
        self.fresh = false;
    }

    fn step(&mut self, slot: Slot, dt: f32) {
        let (tx, ty, facing) = self.target(slot);
        let k = 1.0 - (-EASE * dt).exp();
        let dx = tx - self.x;
        self.x += dx * k;
        self.y += (ty - self.y) * k;
        self.facing_left = match facing {
            Some(value) => value,
            // While gliding to a new lane, look where you are going; once
            // settled, face left like the mascot.
            None if dx.abs() > 0.6 => dx < 0.0,
            None => true,
        };
    }
}

/// Deterministic value in `[0, 1)` derived from a key.
fn hash_unit(key: &str) -> f32 {
    let hash = key.bytes().fold(2166136261u32, |acc, byte| {
        (acc ^ byte as u32).wrapping_mul(16777619)
    });
    (hash % 1000) as f32 / 1000.0
}

#[derive(Debug, Default)]
pub struct Flock {
    pub sheep: Vec<Sheep>,
    /// Pane id of the selected sheep, if any.
    pub selected: Option<String>,
    /// Pane Herdr currently focuses, for a marker in the label.
    pub focused_pane_id: Option<String>,
    /// Last poll error, shown in the footer until a poll succeeds.
    pub error: Option<String>,
    /// True once a poll has produced a flock, so the UI can say "connecting".
    pub loaded: bool,
}

impl Flock {
    /// Adopt a fresh snapshot, preserving animation state per pane.
    pub fn apply(&mut self, snapshot: Snapshot) {
        let mut previous: Vec<Sheep> = std::mem::take(&mut self.sheep);
        let mut next = Vec::with_capacity(snapshot.agents.len());

        for view in snapshot.agents {
            match previous
                .iter()
                .position(|sheep| sheep.view.pane_id == view.pane_id)
            {
                Some(at) => {
                    let mut sheep = previous.remove(at);
                    if sheep.view.status != view.status {
                        sheep.state_t = 0.0;
                    }
                    sheep.view = view;
                    next.push(sheep);
                }
                None => next.push(Sheep::new(view)),
            }
        }

        // Keep flock order stable so slots do not shuffle between polls.
        next.sort_by(|a, b| {
            a.view
                .workspace
                .cmp(&b.view.workspace)
                .then_with(|| a.view.pane_id.cmp(&b.view.pane_id))
        });

        self.sheep = next;
        self.focused_pane_id = snapshot.focused_pane_id;
        self.error = None;
        self.loaded = true;

        // Drop a selection whose agent is gone.
        if let Some(selected) = &self.selected {
            if !self.sheep.iter().any(|s| &s.view.pane_id == selected) {
                self.selected = None;
            }
        }
    }

    pub fn statuses(&self) -> Vec<Status> {
        self.sheep.iter().map(Sheep::status).collect()
    }

    /// Advance animation by `dt` seconds against the current plan.
    pub fn update(&mut self, plan: &Plan, dt: f32) {
        for (index, sheep) in self.sheep.iter_mut().enumerate() {
            sheep.age += dt;
            sheep.state_t += dt;
            let Some(slot) = plan.slot_of(index) else {
                continue;
            };
            if sheep.fresh {
                sheep.place(slot);
            }
            sheep.step(slot, dt);
        }
    }

    pub fn selected_index(&self, plan: &Plan) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        let index = self
            .sheep
            .iter()
            .position(|sheep| &sheep.view.pane_id == selected)?;
        // Only report a selection the plan actually draws.
        plan.order().contains(&index).then_some(index)
    }

    /// Move the selection by `delta` steps in display order.
    pub fn move_selection(&mut self, plan: &Plan, delta: isize) {
        let order = plan.order();
        if order.is_empty() {
            self.selected = None;
            return;
        }
        let current = self
            .selected_index(plan)
            .and_then(|index| order.iter().position(|item| *item == index));
        let next = match current {
            Some(at) => {
                let len = order.len() as isize;
                (((at as isize + delta) % len + len) % len) as usize
            }
            // No selection yet: enter at the top of the display order, which is
            // the most urgent zone.
            None if delta >= 0 => 0,
            None => order.len() - 1,
        };
        self.selected = Some(self.sheep[order[next]].view.pane_id.clone());
    }

    pub fn count(&self, status: Status) -> usize {
        self.sheep
            .iter()
            .filter(|sheep| sheep.status() == status)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout;

    fn view(pane: &str, status: Status) -> AgentView {
        AgentView {
            pane_id: pane.to_string(),
            workspace: "1:ws".to_string(),
            status,
            label: pane.to_string(),
            breed: "claude".to_string(),
            kind: "omp".to_string(),
            title: "title".to_string(),
            context: None,
            limit: None,
            focused: false,
        }
    }

    fn snapshot(agents: Vec<AgentView>) -> Snapshot {
        Snapshot {
            agents,
            focused_pane_id: None,
        }
    }

    fn settle(flock: &mut Flock, width: i32, height: i32, seconds: f32) -> Plan {
        let mut plan = layout::plan(width, height, &flock.statuses());
        let dt = 1.0 / 30.0;
        for _ in 0..(seconds / dt) as usize {
            plan = layout::plan(width, height, &flock.statuses());
            flock.update(&plan, dt);
        }
        plan
    }

    #[test]
    fn animation_state_survives_a_poll_but_a_status_change_resets_it() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![view("w1:p1", Status::Idle)]));
        settle(&mut flock, 80, 24, 3.0);
        let aged = flock.sheep[0].age;
        assert!(aged > 2.0);

        // Same status: keep aging, keep the pose cycle going.
        flock.apply(snapshot(vec![view("w1:p1", Status::Idle)]));
        assert_eq!(flock.sheep[0].age, aged);
        assert!(flock.sheep[0].state_t > 2.0);

        // Changed status: the new behaviour starts from zero.
        flock.apply(snapshot(vec![view("w1:p1", Status::Done)]));
        assert_eq!(flock.sheep[0].state_t, 0.0);
        assert_eq!(flock.sheep[0].age, aged, "age is lifetime, not state time");
    }

    #[test]
    fn departed_agents_leave_the_flock_and_the_selection() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![
            view("w1:p1", Status::Idle),
            view("w2:p1", Status::Idle),
        ]));
        flock.selected = Some("w2:p1".to_string());

        flock.apply(snapshot(vec![view("w1:p1", Status::Idle)]));
        assert_eq!(flock.sheep.len(), 1);
        assert_eq!(flock.selected, None);
    }

    #[test]
    fn flock_order_is_stable_regardless_of_poll_order() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![
            view("w2:p1", Status::Idle),
            view("w1:p1", Status::Idle),
        ]));
        let first: Vec<String> = flock.sheep.iter().map(|s| s.view.pane_id.clone()).collect();

        flock.apply(snapshot(vec![
            view("w1:p1", Status::Idle),
            view("w2:p1", Status::Idle),
        ]));
        let second: Vec<String> = flock.sheep.iter().map(|s| s.view.pane_id.clone()).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn sheep_settle_inside_their_lane() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![
            view("w1:p1", Status::Blocked),
            view("w2:p1", Status::Idle),
            view("w3:p1", Status::Done),
            view("w4:p1", Status::Unknown),
        ]));
        let plan = settle(&mut flock, 80, 34, 6.0);
        for (index, sheep) in flock.sheep.iter().enumerate() {
            let slot = plan.slot_of(index).expect("slot");
            assert!(
                sheep.x >= slot.lane_x as f32 - 0.5
                    && sheep.x + SPRITE_W as f32 <= (slot.lane_x + slot.lane_w) as f32 + 0.5,
                "{} escaped its lane: x={} lane={:?}",
                sheep.view.pane_id,
                sheep.x,
                slot
            );
            assert!((sheep.y - slot.y as f32).abs() < 0.5);
        }
    }

    #[test]
    fn a_working_sheep_runs_back_and_forth_and_turns_around() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![view("w1:p1", Status::Working)]));
        let mut plan = layout::plan(80, 24, &flock.statuses());
        let travel = plan.slot_of(0).unwrap().travel() as f32;

        let dt = 1.0 / 30.0;
        // Skip the entrance walk so it cannot fake the travel measurement.
        for _ in 0..(30.0 * 3.0) as usize {
            plan = layout::plan(80, 24, &flock.statuses());
            flock.update(&plan, dt);
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut faced_left = false;
        let mut faced_right = false;
        // One full lap of an 80-wide lane takes about 20s at RUN_SPEED.
        for _ in 0..(30.0 * 25.0) as usize {
            plan = layout::plan(80, 24, &flock.statuses());
            flock.update(&plan, dt);
            let sheep = &flock.sheep[0];
            min_x = min_x.min(sheep.x);
            max_x = max_x.max(sheep.x);
            faced_left |= sheep.facing_left;
            faced_right |= !sheep.facing_left;
        }
        assert!(faced_left && faced_right, "the sheep never turned around");
        assert!(min_x >= -0.5, "the sheep left its lane: {min_x}");
        assert!(
            max_x - min_x > travel * 0.8,
            "the sheep barely moved: {min_x}..{max_x} of {travel}"
        );
    }

    #[test]
    fn only_finished_sheep_leave_the_ground() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![
            view("w1:p1", Status::Done),
            view("w2:p1", Status::Working),
        ]));
        let mut airborne = false;
        for step in 0..200 {
            for sheep in &mut flock.sheep {
                sheep.state_t = step as f32 / 30.0;
            }
            airborne |= flock.sheep[0].hop() < 0;
            assert_eq!(flock.sheep[1].hop(), 0, "a working sheep must stay down");
        }
        assert!(airborne, "the finished sheep never jumped");
    }

    #[test]
    fn selection_walks_the_display_order_and_wraps() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![
            view("w1:p1", Status::Idle),
            view("w2:p1", Status::Blocked),
        ]));
        let plan = layout::plan(80, 30, &flock.statuses());

        // Display order is urgency first, so the blocked sheep leads.
        flock.move_selection(&plan, 1);
        assert_eq!(flock.selected.as_deref(), Some("w2:p1"));
        flock.move_selection(&plan, 1);
        assert_eq!(flock.selected.as_deref(), Some("w1:p1"));
        flock.move_selection(&plan, 1);
        assert_eq!(flock.selected.as_deref(), Some("w2:p1"), "wraps forward");
        flock.move_selection(&plan, -1);
        assert_eq!(flock.selected.as_deref(), Some("w1:p1"), "wraps backward");
    }

    #[test]
    fn selection_is_a_no_op_on_an_empty_flock() {
        let mut flock = Flock::default();
        let plan = layout::plan(80, 30, &[]);
        flock.move_selection(&plan, 1);
        assert_eq!(flock.selected, None);
        assert_eq!(flock.selected_index(&plan), None);
    }

    #[test]
    fn a_selection_hidden_by_the_compact_clip_is_not_reported() {
        let mut flock = Flock::default();
        flock.apply(snapshot(
            (0..10)
                .map(|i| view(&format!("w{i}:p1"), Status::Idle))
                .collect(),
        ));
        flock.selected = Some("w9:p1".to_string());
        // A 6-row pane shows 3 rows, so the last agent is clipped away.
        let plan = layout::plan(80, 6, &flock.statuses());
        assert_eq!(flock.selected_index(&plan), None);
    }

    #[test]
    fn counts_report_the_flock_tally() {
        let mut flock = Flock::default();
        flock.apply(snapshot(vec![
            view("w1:p1", Status::Working),
            view("w2:p1", Status::Working),
            view("w3:p1", Status::Blocked),
        ]));
        assert_eq!(flock.count(Status::Working), 2);
        assert_eq!(flock.count(Status::Blocked), 1);
        assert_eq!(flock.count(Status::Done), 0);
    }
}
