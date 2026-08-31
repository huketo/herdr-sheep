//! herdr-sheep — a Herdr plugin pane that shows every agent in the session as
//! a sheep, doing whatever its state says it is doing.

mod draw;
mod herdr;
mod layout;
mod model;
mod render;
mod sprite;
mod theme;

use std::io::{self, BufWriter, Write};
use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use crossterm::{cursor, execute, terminal};

use herdr::{PollResult, Snapshot, Status};
use layout::Plan;
use model::Flock;
use render::Screen;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_FPS: u64 = 20;
const DEFAULT_POLL_MS: u64 = 800;

/// How close two clicks on the same sheep have to be to count as a double.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Ask the terminal for button reports, in SGR encoding.
///
/// Deliberately not crossterm's `EnableMouseCapture`, which also turns on
/// motion tracking: the pasture only cares about clicks, and every mouse move
/// over the pane would be one more event the render loop has to drain.
const MOUSE_ON: &str = "\x1b[?1000h\x1b[?1006h";
/// Hand the mouse back to the terminal.
const MOUSE_OFF: &str = "\x1b[?1006l\x1b[?1000l";

const USAGE: &str = "\
herdr-sheep — watch your Herdr agents as a flock of ASCII sheep

Usage:
  herdr-sheep                    Run the pasture (this is what the plugin pane does)
  herdr-sheep --open here        Open the pasture over the active Herdr pane
  herdr-sheep --open side        Open the pasture beside it, or close the open one
  herdr-sheep --snapshot [WxH]   Print one frame as plain text and exit
  herdr-sheep --demo [WxH]       Print one frame with one sheep per state
  herdr-sheep --version
  herdr-sheep --help

Options for --snapshot and --demo:
  --seconds N   Seconds of animation to settle before printing (default 3)

Environment:
  HERDR_BIN_PATH        Herdr binary to call back into (set by Herdr)
  HERDR_SHEEP_FPS       Animation frames per second (5-60, default 20)
  HERDR_SHEEP_POLL_MS   Session poll interval in ms (200-10000, default 800)

Keys:
  j / k / arrows   Move between sheep
  enter / f        Focus the selected agent's pane
  r                Poll the session now
  q / esc          Leave the pasture

Mouse:
  click            Select the sheep you clicked
  double click     Focus that agent's pane
";

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run,
    Help,
    Version,
    /// Render one settled frame as text: live session or synthetic states.
    Frame {
        demo: bool,
        width: u16,
        height: u16,
        seconds: u32,
    },
    /// Ask Herdr to put a pasture pane on screen. Backs the plugin's actions.
    Open(herdr::Spot),
}

fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut frame: Option<bool> = None; // Some(demo)
    let mut open: Option<herdr::Spot> = None;
    let mut size: Option<(u16, u16)> = None;
    let mut seconds = 3u32;
    let mut rest = args.into_iter();

    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--version" | "-V" => return Ok(Command::Version),
            "--snapshot" => frame = Some(false),
            "--demo" => frame = Some(true),
            "--open" => {
                let value = rest.next().ok_or("--open needs here or side")?;
                open = Some(match value.as_str() {
                    "here" => herdr::Spot::Here,
                    "side" => herdr::Spot::Side,
                    other => return Err(format!("--open wants here or side, got {other:?}")),
                });
            }
            "--seconds" => {
                let value = rest.next().ok_or("--seconds needs a value")?;
                seconds = value
                    .parse()
                    .map_err(|_| format!("--seconds wants a number, got {value:?}"))?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other:?}")),
            other => {
                let (w, h) = other
                    .split_once(['x', 'X'])
                    .ok_or_else(|| format!("expected a WxH size, got {other:?}"))?;
                let width = w.parse().map_err(|_| format!("bad width in {other:?}"))?;
                let height = h.parse().map_err(|_| format!("bad height in {other:?}"))?;
                size = Some((width, height));
            }
        }
    }

    match (frame, open) {
        (Some(_), Some(_)) => Err("--open does not draw frames; pick one".into()),
        (Some(demo), None) => {
            let (width, height) = size.unwrap_or((88, 30));
            if width < 8 || height < 6 {
                return Err(format!("{width}x{height} is too small to draw anything"));
            }
            Ok(Command::Frame {
                demo,
                width,
                height,
                seconds,
            })
        }
        _ if size.is_some() => Err("a size only makes sense with --snapshot or --demo".into()),
        (None, Some(spot)) => Ok(Command::Open(spot)),
        (None, None) => Ok(Command::Run),
    }
}

fn main() -> std::process::ExitCode {
    let command = match parse_args(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{NAME}: {message}\n\n{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };

    let result = match command {
        Command::Help => {
            print!("{USAGE}");
            Ok(())
        }
        Command::Version => {
            println!("{NAME} {VERSION}");
            Ok(())
        }
        Command::Frame {
            demo,
            width,
            height,
            seconds,
        } => print_frame(demo, width, height, seconds),
        Command::Open(spot) => herdr::open(spot),
        Command::Run => run(),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{NAME}: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn env_number(key: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn frame_interval() -> Duration {
    Duration::from_micros(1_000_000 / env_number("HERDR_SHEEP_FPS", DEFAULT_FPS, 5, 60))
}

fn poll_interval() -> Duration {
    Duration::from_millis(env_number(
        "HERDR_SHEEP_POLL_MS",
        DEFAULT_POLL_MS,
        200,
        10_000,
    ))
}

/// Settle the flock for `seconds` of animation and render one frame.
fn settle(flock: &mut Flock, width: u16, height: u16, seconds: u32) -> Screen {
    let mut screen = Screen::new(width, height);
    let dt = 1.0 / 30.0;
    let steps = (seconds as f32 / dt) as usize;
    let mut plan = Plan::Empty;
    let mut t = 0.0;
    for _ in 0..steps.max(1) {
        plan = layout::plan(width as i32, height as i32, &flock.statuses());
        flock.update(&plan, dt);
        t += dt;
    }
    draw::draw(&mut screen, flock, &plan, t);
    screen
}

fn print_frame(demo: bool, width: u16, height: u16, seconds: u32) -> Result<(), String> {
    let mut flock = Flock::default();
    if demo {
        flock.apply(demo_snapshot());
    } else {
        flock.apply(herdr::fetch()?);
    }
    let screen = settle(&mut flock, width, height, seconds);
    let mut out = io::stdout().lock();
    for row in screen.snapshot_text() {
        let _ = writeln!(out, "{row}");
    }
    Ok(())
}

/// One sheep per state, for documentation screenshots and eyeballing the art.
fn demo_snapshot() -> Snapshot {
    let sheep = [
        (
            "w1:p1",
            Status::Blocked,
            "deploy",
            "codex",
            "approve the migration?",
        ),
        (
            "w2:p1",
            Status::Done,
            "reviewer",
            "claude",
            "review finished",
        ),
        (
            "w3:p1",
            Status::Working,
            "runner",
            "claude",
            "porting the runner boundary",
        ),
        (
            "w4:p1",
            Status::Working,
            "docs",
            "codex",
            "extracting the docs",
        ),
        ("w5:p1", Status::Idle, "scout", "gemini", "waiting for work"),
        (
            "w6:p1",
            Status::Unknown,
            "mystery",
            "amp",
            "unrecognized screen",
        ),
    ];
    Snapshot {
        agents: sheep
            .into_iter()
            .enumerate()
            .map(
                |(index, (pane, status, label, breed, title))| herdr::AgentView {
                    pane_id: pane.to_string(),
                    workspace: format!("{}:demo", index + 1),
                    status,
                    label: label.to_string(),
                    breed: breed.to_string(),
                    kind: "omp".to_string(),
                    title: title.to_string(),
                    context: Some("31% (306k)".to_string()),
                    limit: Some("5h 100%".to_string()),
                    focused: index == 2,
                },
            )
            .collect(),
        focused_pane_id: Some("w3:p1".to_string()),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    None,
    Quit,
    Next,
    Prev,
    Focus,
    Refresh,
}

fn action_for(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('d') => Action::Quit,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => {
            Action::Next
        }
        KeyCode::Char('k')
        | KeyCode::Up
        | KeyCode::BackTab
        | KeyCode::Char('h')
        | KeyCode::Left => Action::Prev,
        KeyCode::Enter | KeyCode::Char('f') => Action::Focus,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}

/// Pane cell a left click landed on. Every other mouse report is ignored, so
/// releases and stray button presses cannot move the selection.
fn click_at(mouse: MouseEvent) -> Option<(i32, i32)> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => Some((mouse.column as i32, mouse.row as i32)),
        _ => None,
    }
}

/// Poll once off the render loop, for the `r` key.
fn refresh_now(tx: Sender<PollResult>) {
    std::thread::spawn(move || {
        let _ = tx.send(herdr::fetch());
    });
}

fn run() -> Result<(), String> {
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout());
    enter_terminal(&mut out).map_err(|err| format!("cannot take over the terminal: {err}"))?;

    let result = pasture(&mut out);

    // Restore the terminal before reporting anything, so an error is readable.
    let _ = leave_terminal(&mut out);
    result.map_err(|err| format!("{err}"))
}

fn enter_terminal(out: &mut impl Write) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
    out.write_all(MOUSE_ON.as_bytes())?;
    out.flush()?;

    // A panic must not leave the pane in raw mode, on the alternate screen, or
    // with the mouse still reporting to a process that is gone.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(MOUSE_OFF.as_bytes());
        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
        previous(info);
    }));
    Ok(())
}

fn leave_terminal(out: &mut impl Write) -> io::Result<()> {
    out.write_all(MOUSE_OFF.as_bytes())?;
    execute!(out, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    out.flush()
}

fn pasture(out: &mut impl Write) -> io::Result<()> {
    let (tx, rx) = mpsc::channel::<PollResult>();
    herdr::spawn_poller(poll_interval(), tx.clone());

    let (width, height) = terminal::size()?;
    let mut screen = Screen::new(width, height);
    let mut flock = Flock::default();
    let mut plan = Plan::Empty;
    let frame = frame_interval();
    let mut t = 0.0f32;
    let mut last = Instant::now();
    let mut next_frame = Instant::now();
    // Sheep and time of the last click, for double-click detection.
    let mut last_click: Option<(String, Instant)> = None;

    loop {
        let timeout = next_frame.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match action_for(key) {
                    Action::Quit => return Ok(()),
                    Action::Next => flock.move_selection(&plan, 1),
                    Action::Prev => flock.move_selection(&plan, -1),
                    Action::Focus => {
                        if flock.selected_index(&plan).is_none() {
                            // Nothing picked yet: go where you are needed most.
                            flock.move_selection(&plan, 1);
                        }
                        if let Some(index) = flock.selected_index(&plan) {
                            herdr::focus_agent(&flock.sheep[index].view.pane_id);
                        }
                    }
                    Action::Refresh => refresh_now(tx.clone()),
                    Action::None => {}
                },
                Event::Mouse(mouse) => {
                    let Some((x, y)) = click_at(mouse) else {
                        continue;
                    };
                    // A click selects; clicking the same sheep again focuses
                    // it, so the mouse can do what enter does.
                    let Some(pane_id) = flock.select_at(&plan, x, y) else {
                        continue;
                    };
                    let now = Instant::now();
                    let again = last_click.take().is_some_and(|(id, at)| {
                        id == pane_id && now.duration_since(at) < DOUBLE_CLICK
                    });
                    if again {
                        herdr::focus_agent(&pane_id);
                    } else {
                        last_click = Some((pane_id, now));
                    }
                }
                Event::Resize(width, height) => screen.resize(width, height),
                _ => {}
            }
            // Drain the rest of the queue before spending a frame.
            continue;
        }

        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(snapshot) => flock.apply(snapshot),
                Err(message) => flock.error = Some(message),
            }
        }

        let now = Instant::now();
        // Clamp so a suspended pane does not teleport the whole flock.
        let dt = (now - last).as_secs_f32().min(0.25);
        last = now;
        t += dt;

        plan = layout::plan(
            screen.width() as i32,
            screen.height() as i32,
            &flock.statuses(),
        );
        flock.update(&plan, dt);
        draw::draw(&mut screen, &flock, &plan, t);
        screen.flush(out)?;
        next_frame = now + frame;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn no_arguments_runs_the_pasture() {
        assert_eq!(parse(&[]), Ok(Command::Run));
    }

    #[test]
    fn snapshot_takes_an_optional_size_and_settle_time() {
        assert_eq!(
            parse(&["--snapshot"]),
            Ok(Command::Frame {
                demo: false,
                width: 88,
                height: 30,
                seconds: 3
            })
        );
        assert_eq!(
            parse(&["--demo", "100x40", "--seconds", "7"]),
            Ok(Command::Frame {
                demo: true,
                width: 100,
                height: 40,
                seconds: 7
            })
        );
    }

    #[test]
    fn bad_arguments_are_rejected_with_a_reason() {
        assert!(parse(&["--nope"]).unwrap_err().contains("--nope"));
        assert!(parse(&["--snapshot", "wide"]).unwrap_err().contains("WxH"));
        assert!(parse(&["--snapshot", "4x4"]).unwrap_err().contains("small"));
        assert!(parse(&["--seconds"]).unwrap_err().contains("--seconds"));
        assert!(parse(&["80x24"]).unwrap_err().contains("--snapshot"));
    }

    #[test]
    fn keys_map_to_the_documented_actions() {
        let press = |code| KeyEvent::new(code, KeyModifiers::NONE);
        assert_eq!(action_for(press(KeyCode::Char('q'))), Action::Quit);
        assert_eq!(action_for(press(KeyCode::Esc)), Action::Quit);
        assert_eq!(action_for(press(KeyCode::Char('j'))), Action::Next);
        assert_eq!(action_for(press(KeyCode::Down)), Action::Next);
        assert_eq!(action_for(press(KeyCode::Char('k'))), Action::Prev);
        assert_eq!(action_for(press(KeyCode::Up)), Action::Prev);
        assert_eq!(action_for(press(KeyCode::Enter)), Action::Focus);
        assert_eq!(action_for(press(KeyCode::Char('r'))), Action::Refresh);
        assert_eq!(action_for(press(KeyCode::Char('z'))), Action::None);
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Quit
        );
        // A modified letter must not trigger its unmodified action.
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)),
            Action::None
        );
    }

    #[test]
    fn only_a_left_press_counts_as_a_click() {
        let event = |kind| MouseEvent {
            kind,
            column: 12,
            row: 7,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            click_at(event(MouseEventKind::Down(MouseButton::Left))),
            Some((12, 7))
        );
        for ignored in [
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Moved,
            MouseEventKind::ScrollDown,
        ] {
            assert_eq!(click_at(event(ignored)), None, "{ignored:?} is not a click");
        }
    }

    #[test]
    fn env_numbers_are_clamped_and_fall_back() {
        assert_eq!(env_number("HERDR_SHEEP_NOT_SET", 20, 5, 60), 20);
        std::env::set_var("HERDR_SHEEP_TEST_FPS", "999");
        assert_eq!(env_number("HERDR_SHEEP_TEST_FPS", 20, 5, 60), 60);
        std::env::set_var("HERDR_SHEEP_TEST_FPS", "nonsense");
        assert_eq!(env_number("HERDR_SHEEP_TEST_FPS", 20, 5, 60), 20);
        std::env::remove_var("HERDR_SHEEP_TEST_FPS");
    }

    #[test]
    fn the_demo_covers_every_state() {
        let statuses: Vec<Status> = demo_snapshot()
            .agents
            .iter()
            .map(|agent| agent.status)
            .collect();
        for status in layout::ZONE_ORDER {
            assert!(statuses.contains(&status), "demo is missing {status:?}");
        }
    }
}
