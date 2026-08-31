//! Bridge to the running Herdr server.
//!
//! Everything goes through the `herdr` binary named by `HERDR_BIN_PATH` so the
//! plugin stays portable across the Unix-socket and Windows named-pipe
//! transports behind `HERDR_SOCKET_PATH`.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use serde::Deserialize;

/// Plugin id, entrypoint, and pane label Herdr knows the pasture by. All three
/// have to match `herdr-plugin.toml`: `PANE_LABEL` comes from the pane title,
/// and is the only handle a running pasture pane can be found by.
const PLUGIN_ID: &str = "huketo.sheep";
const ENTRYPOINT: &str = "pasture";
const PANE_LABEL: &str = "Sheep pasture";

/// Lifecycle state Herdr assigns to a recognized agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Working,
    Idle,
    Done,
    Blocked,
    /// Herdr sees an agent but cannot classify it, or reported a state this
    /// build does not know yet.
    Unknown,
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        // Forward compatibility: a future Herdr state must degrade, not abort.
        Ok(match String::deserialize(de)?.as_str() {
            "working" => Status::Working,
            "idle" => Status::Idle,
            "done" => Status::Done,
            "blocked" => Status::Blocked,
            _ => Status::Unknown,
        })
    }
}

/// `herdr api snapshot`.
#[derive(Debug, Deserialize)]
struct SnapshotEnvelope {
    result: SnapshotResult,
}

#[derive(Debug, Deserialize)]
struct SnapshotResult {
    snapshot: RawSnapshot,
}

/// `herdr pane list`, read only for the labels that identify our own pane.
#[derive(Debug, Deserialize)]
struct PaneListEnvelope {
    result: PaneListResult,
}

#[derive(Debug, Deserialize)]
struct PaneListResult {
    #[serde(default)]
    panes: Vec<PaneRow>,
}

#[derive(Debug, Deserialize)]
struct PaneRow {
    pane_id: String,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSnapshot {
    #[serde(default)]
    agents: Vec<RawAgent>,
    #[serde(default)]
    workspaces: Vec<RawWorkspace>,
    #[serde(default)]
    focused_pane_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawWorkspace {
    workspace_id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    number: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawAgent {
    pane_id: String,
    workspace_id: String,
    agent_status: Status,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    display_agent: Option<String>,
    #[serde(default)]
    terminal_title_stripped: Option<String>,
    #[serde(default)]
    terminal_title: Option<String>,
    #[serde(default)]
    tokens: HashMap<String, String>,
    #[serde(default)]
    focused: bool,
}

/// One agent, flattened into exactly what the pasture needs to draw it.
#[derive(Debug, Clone)]
pub struct AgentView {
    /// Pane hosting the agent. Stable identity for a sheep.
    pub pane_id: String,
    pub workspace: String,
    pub status: Status,
    /// Short name shown under the sheep.
    pub label: String,
    /// Palette key: the model provider when the agent reports one, else its kind.
    pub breed: String,
    /// Agent kind as Herdr classified it.
    pub kind: String,
    /// Terminal title, for the detail line.
    pub title: String,
    /// Context usage as reported by the agent, e.g. `31% (306k)`.
    pub context: Option<String>,
    /// Rate/quota budget as reported by the agent, e.g. `5h 100%`.
    pub limit: Option<String>,
    pub focused: bool,
}

/// A poll of live session state.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub agents: Vec<AgentView>,
    pub focused_pane_id: Option<String>,
}

/// Path of the Herdr binary to call back into.
pub fn herdr_bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

/// First candidate that carries visible text, trimmed.
fn first_non_empty<'a>(candidates: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    candidates
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn workspace_display(workspaces: &HashMap<String, RawWorkspace>, id: &str) -> String {
    match workspaces.get(id) {
        Some(ws) => {
            let label = ws.label.as_deref().unwrap_or("").trim();
            match (ws.number, label.is_empty()) {
                (Some(number), false) => format!("{number}:{label}"),
                (Some(number), true) => format!("{number}"),
                (None, false) => label.to_string(),
                (None, true) => ws.workspace_id.clone(),
            }
        }
        None => id.to_string(),
    }
}

fn parse(stdout: &[u8]) -> Result<Snapshot, String> {
    let envelope: SnapshotEnvelope =
        serde_json::from_slice(stdout).map_err(|err| format!("snapshot parse failed: {err}"))?;
    let raw = envelope.result.snapshot;

    let workspaces: HashMap<String, RawWorkspace> = raw
        .workspaces
        .into_iter()
        .map(|ws| (ws.workspace_id.clone(), ws))
        .collect();

    let agents = raw
        .agents
        .into_iter()
        .map(|agent| {
            let title = first_non_empty([
                agent.terminal_title_stripped.as_deref(),
                agent.terminal_title.as_deref(),
            ])
            .unwrap_or_default();
            let kind = first_non_empty([agent.agent.as_deref(), agent.display_agent.as_deref()])
                .unwrap_or_else(|| "agent".to_string());
            let label = first_non_empty([
                agent.name.as_deref(),
                agent.tokens.get("title").map(String::as_str),
            ])
            .unwrap_or_else(|| kind.clone());
            let breed = agent
                .tokens
                .get("provider")
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| kind.to_ascii_lowercase());

            AgentView {
                workspace: workspace_display(&workspaces, &agent.workspace_id),
                pane_id: agent.pane_id,
                status: agent.agent_status,
                label,
                breed,
                kind,
                title,
                context: agent.tokens.get("context").cloned(),
                limit: agent.tokens.get("limit").cloned(),
                focused: agent.focused,
            }
        })
        .collect();

    Ok(Snapshot {
        agents,
        focused_pane_id: raw.focused_pane_id,
    })
}

/// Run the Herdr CLI and hand back its stdout.
fn call(args: &[&str]) -> Result<Vec<u8>, String> {
    let bin = herdr_bin();
    let output = Command::new(&bin)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| format!("cannot run `{bin} {}`: {err}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.lines().next().unwrap_or("no stderr").trim();
        return Err(format!("`herdr {}` failed: {detail}", args.join(" ")));
    }

    Ok(output.stdout)
}

/// Fetch session state once.
pub fn fetch() -> Result<Snapshot, String> {
    parse(&call(&["api", "snapshot"])?)
}

/// Where `--open` puts the pasture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spot {
    /// Over the active pane, as an overlay Herdr takes back down when the
    /// pasture closes.
    Here,
    /// A split beside the active pane. One pasture at a time: opening it again
    /// closes the one that is already up.
    Side,
}

/// Open the pasture, or close the open one when `spot` is `Side`.
///
/// This is what the plugin's keybinding actions run, so the placement rules
/// live here instead of in a shell script Herdr would have to carry.
pub fn open(spot: Spot) -> Result<(), String> {
    if spot == Spot::Side {
        if let Some(pane_id) = pasture_pane()? {
            call(&["plugin", "pane", "close", &pane_id])?;
            return Ok(());
        }
    }

    let mut args = vec![
        "plugin",
        "pane",
        "open",
        "--plugin",
        PLUGIN_ID,
        "--entrypoint",
        ENTRYPOINT,
    ];
    let caller = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    match spot {
        // Herdr rejects `--target-pane` for an overlay: an overlay always covers
        // the pane it is opened from, which is the pane you pressed the key in.
        Spot::Here => args.extend(["--placement", "overlay"]),
        Spot::Side => {
            args.extend(["--placement", "split", "--direction", "right"]);
            // Name the calling pane, so the split lands beside the pane you
            // pressed the key in rather than wherever focus happens to be.
            if !caller.is_empty() {
                args.extend(["--target-pane", caller.as_str()]);
            }
        }
    }
    call(&args)?;
    Ok(())
}

/// Pane hosting an open pasture, wherever in the session it is.
fn pasture_pane() -> Result<Option<String>, String> {
    let panes: PaneListEnvelope = serde_json::from_slice(&call(&["pane", "list"])?)
        .map_err(|err| format!("pane list parse failed: {err}"))?;
    Ok(panes
        .result
        .panes
        .into_iter()
        .find(|pane| pane.label.as_deref() == Some(PANE_LABEL))
        .map(|pane| pane.pane_id))
}

/// Result of one poll, as delivered to the render loop.
pub type PollResult = Result<Snapshot, String>;

/// Poll session state on a background thread and stream results to the UI.
///
/// The thread ends when the receiver is dropped, which happens on quit.
pub fn spawn_poller(interval: Duration, tx: Sender<PollResult>) {
    thread::spawn(move || loop {
        if tx.send(fetch()).is_err() {
            return;
        }
        thread::sleep(interval);
    });
}

/// Move Herdr's focus to the pane hosting `pane_id`.
///
/// Fire-and-forget: focusing steals the terminal away from this pane, so there
/// is nothing useful left to report into.
pub fn focus_agent(pane_id: &str) {
    let bin = herdr_bin();
    let target = pane_id.to_string();
    thread::spawn(move || {
        let _ = Command::new(bin)
            .args(["agent", "focus", &target])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "id": "cli:api:snapshot",
      "result": {
        "type": "snapshot",
        "snapshot": {
          "focused_pane_id": "w1:p1",
          "workspaces": [
            { "workspace_id": "w4", "label": "bivri", "number": 4 },
            { "workspace_id": "w9", "number": 9 }
          ],
          "agents": [
            {
              "pane_id": "w4:p1",
              "workspace_id": "w4",
              "agent_status": "working",
              "agent": "omp",
              "terminal_title_stripped": "planning",
              "tokens": {
                "provider": "claude",
                "title": "runner boundary",
                "context": "31% (306k)",
                "limit": "5h 100%"
              },
              "focused": false
            },
            {
              "pane_id": "w9:p2",
              "workspace_id": "w9",
              "agent_status": "sleepwalking",
              "name": "reviewer",
              "agent": "codex",
              "focused": true
            }
          ]
        }
      }
    }"#;

    #[test]
    fn parses_agents_and_workspace_labels() {
        let snapshot = parse(SAMPLE.as_bytes()).expect("parse");
        assert_eq!(snapshot.focused_pane_id.as_deref(), Some("w1:p1"));
        assert_eq!(snapshot.agents.len(), 2);

        let first = &snapshot.agents[0];
        assert_eq!(first.status, Status::Working);
        assert_eq!(first.workspace, "4:bivri");
        assert_eq!(first.label, "runner boundary");
        assert_eq!(first.breed, "claude");
        assert_eq!(first.kind, "omp");
        assert_eq!(first.context.as_deref(), Some("31% (306k)"));
    }

    #[test]
    fn unknown_status_and_missing_metadata_degrade() {
        let snapshot = parse(SAMPLE.as_bytes()).expect("parse");
        let second = &snapshot.agents[1];
        // An agent_status Herdr adds later must not break the pasture.
        assert_eq!(second.status, Status::Unknown);
        // Herdr-assigned agent name wins over the kind.
        assert_eq!(second.label, "reviewer");
        // No reported provider, so the kind decides the wool color.
        assert_eq!(second.breed, "codex");
        // Workspace with no label falls back to its number.
        assert_eq!(second.workspace, "9");
        assert_eq!(second.title, "");
    }

    #[test]
    fn rejects_non_snapshot_payloads() {
        assert!(parse(b"{\"error\":{\"code\":\"nope\"}}").is_err());
    }
}
