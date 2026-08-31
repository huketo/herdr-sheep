# herdr-sheep

A [Herdr](https://herdr.dev) plugin pane that draws every coding agent in the
session as a sheep. Rust binary, no framework: it paints a cell grid and talks
to Herdr through the `herdr` CLI.

[README.md](README.md) is the user-facing doc — keys, install, configuration.
This file is for changing the code.

## The gate

`just ci` is the gate, and it is exactly what CI runs: version lockstep,
`fmt --check`, clippy with `-D warnings`, tests, the mouse smoke test, a
rendered frame. Run it before you call a change done. `just` lists every task.

## Where things live

Each module owns one thing, and the seams are load-bearing:

| Module | Owns | Never does |
| --- | --- | --- |
| `layout` | space: which rows and columns each sheep, zone, and piece of scenery gets | draw anything |
| `draw` | pixels: what goes in the cells layout handed out | decide where things go |
| `model` | motion: where each sheep is this frame, and the selection | know about glyphs |
| `sprite` | art: the sheep's poses and the barn, with their box dimensions | know about panes |
| `render` | the cell grid and the diffing flush to the terminal | know about sheep |
| `herdr` | the CLI bridge: snapshots, focus, opening and closing panes | render or lay out |
| `theme` | colors | anything else |

A new feature that needs space asks `layout` for it and paints it in `draw`. A
feature that adds art puts it in `sprite`, dimensions included, so `layout` can
reserve room without hardcoding numbers.

## Invariants

These are the rules the tests defend. Break one and something looks wrong on
screen rather than failing to compile:

- **Sheep get the space first.** Scenery — the horizon fence, the barn — takes
  only rows left over after every sheep has its slot. A pane exactly as tall as
  its flock draws no scenery at all.
- **Slots tile their zone.** Every cell below a zone label belongs to exactly
  one sheep, which is what makes a click unambiguous. A change to `row_pitch`,
  `Slot::top`, or `Slot::contains` has to keep that true.
- **Art fills its box.** Every sprite row is ASCII, padded to exactly
  `SPRITE_W`, so mirroring is symmetric and a right-facing sheep does not drift
  sideways.
- **Role alphabets stay disjoint.** `sprite::role_of` maps a glyph to wool,
  face, horn, or leg with no overlap, so a frame needs no separate color mask.
- **Everything goes through `Screen`.** Writes to the terminal are `put`,
  `text`, `text_clipped`, and `hfill`, so the diffing flush stays correct and
  wide glyphs keep their two columns. Foreground only: the background is the
  user's terminal theme.
- **Deterministic decoration.** Grass and per-sheep jitter come from hashes of
  position or pane id, never from a clock or an RNG, so nothing shimmers.
- **The three versions agree.** `herdr/install.sh` builds the release download
  URL from the manifest version, so `Cargo.toml`, `herdr-plugin.toml`, and the
  tag say the same thing. `just check-version` enforces it.

## Testing

Test what a user could see. Unit tests render a settled frame and assert on the
text (`draw::tests::frame_text`), walk the layout's geometry, or drive the
animation for seconds of simulated time. `tests/cli.rs` runs the built binary.

- Point `HERDR_BIN_PATH` at a path that does not exist in anything automated.
  A test that reaches the live session is a test that fails on someone else's
  machine.
- The pasture needs a terminal, so the mouse path is covered by
  `scripts/smoke-click.py`: it runs the real binary in a pty against a stub
  `herdr`, decodes the frames, clicks, and checks what the pane did about it.
  Reach for that shape when a change only shows up through real terminal input.

## Working on the live plugin

`herdr plugin link` skips build commands, so a linked checkout stages its own
binary where the manifest points:

```bash
just link       # build release, stage bin/herdr-sheep, link this checkout
just install    # after every code change, then reopen the pane
```

`herdr plugin log --plugin huketo.sheep` is where a failed action's stderr
ends up — that is how you find out an action broke.

## Commits and releases

Commit subjects are [Conventional Commits](https://www.conventionalcommits.org/).
`CHANGELOG.md` and every release body are generated from them, so write the
subject as the release note a user should read, and put the reasoning in the
body. `just release-notes` shows what the next release would say.

Releasing is in [README.md](README.md#releasing).
