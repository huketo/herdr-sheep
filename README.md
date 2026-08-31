# herdr-sheep

*[한국어 README](README.ko.md)*

A [Herdr](https://herdr.dev) plugin pane that turns every coding agent in your
session into a sheep. Sheep run, graze, jump, ask for help, or snore depending
on what their agent is actually doing, so a glance at the pane tells you which
agents need you.

```text
(>_@ herdr-sheep        1 waiting on you  1 done  2 working  1 idle  1 unknown
|- GATE  1 waiting on you ----|----|----|----|----|----|----|----|----|----|--
                                    (?)
                                    ,@~~~.
                                   (o_ ~~ )
                                    ''  ''
                                    deploy
                   ,     ', '    .           .        ' .      '     .       ,
|- PEN  1 done |----|----|----|----|----|----|----|----|----|----|----|----|--
                                         *
                                    ,@~~~. *
                                   (^_ ~~ )
                                    \'  '/
                                   reviewer
 '             ,           ,          ,    '   ,  . .   '    .           ,
|- PADDOCK  2 working ---|----|----|----|----|----|----|----|----|----|----|--
                     ,~~~@.                            ,~~~@.
                    ( ~~ _<)                          ( ~~ _<)
                  .  \'  '/                         .  \'  '/
                    *runner                             docs
      '  ,''                       .       .      '    '    ,            , .
|- MEADOW  1 idle --|----|----|----|----|----|----|----|----|----|----|----|--
                                  ,~~~@.
                                  ( ~~ )
                                , ''  _< .
                                  scout
   . '      .                . , .      '       ,                .   ,,   '
|- FOLD  1 unknown -|----|----|----|----|----|----|----|----|----|----|----|--
                                         z
                                    ,@~~~.
                                   (-_ ~~ )
                                    ~~~~~~
                                   mystery
         ,    .   '            .  ,         ,       ,   '        .   '
.       ,           ,                       .                   ' ,  _____
 .               .                 ,                  .             /     \
                       ,         ,                                  | [+] |
|----|----|----|----|----|----|----|----|----|----|----|----|----|--|_|_|_||--
press j or k to pick a sheep
j/k select  click/enter focus  r refresh  q quit
```

Herdr's mascot is a side-profile ram whose face is a shell prompt, so every
sheep keeps its `@` horn and its `>_` muzzle.

## What the sheep are doing

Sheep stand in a zone per agent state, and the zones are ordered by how much
they want your attention — whatever is at the top of the pane is what to look
at first. Zones with no sheep are not drawn.

| Herdr state | Zone | The sheep |
| --- | --- | --- |
| `blocked` | `GATE` | stands still with a blinking `(?)`, waiting for your approval or answer |
| `done` | `PEN` | jumps, with sparkles |
| `working` | `PADDOCK` | gallops back and forth across its lane, kicking up dust |
| `idle` | `MEADOW` | grazes, head up and down, drifting a little |
| `unknown` | `FOLD` | lies down and snores `z z z` |

A sheep's wool color comes from the model provider its agent reports
(`claude`, `codex`, `gemini`, ...), falling back to the agent kind Herdr
detected. Its name is the Herdr agent name when you set one, otherwise the
short title the agent reports. A `*` prefix marks the pane Herdr currently
focuses.

When a status changes, the sheep walks to its new zone. New agents walk in from
the left edge; agents that exit leave the flock.

## The pasture

Each zone is a fenced paddock, so its divider is a run of fence. A pane with
rows left over after every sheep has its standing room gets a fence along the
horizon too, and one with a few more rows gets a barn standing in that fence.
Scenery never costs a sheep any room: when the pane is exactly as tall as the
flock needs, the fence and the barn are simply not there. Grass grows on the
rows nothing else uses, thinning out with distance from the flock.

## Install

Requirements: Herdr 0.8.0 or newer. Installing downloads a prebuilt binary for
your platform from the matching release, so no Rust toolchain is needed.

```bash
herdr plugin install huketo/herdr-sheep
herdr plugin pane open --plugin huketo.sheep --entrypoint pasture
```

Prebuilt binaries cover macOS (Apple silicon and Intel) and Linux (x86_64 and
aarch64, static musl builds).

## Keys and mouse

| Key | Does |
| --- | --- |
| `j` `k`, arrows, `Tab` | move between sheep, in zone order |
| `Enter` `f` | focus the selected agent's pane; with nothing selected, focus the most urgent agent |
| `r` | poll the session now instead of waiting for the next tick |
| `q` `Esc` `Ctrl-C` | leave the pasture and close the pane |

| Mouse | Does |
| --- | --- |
| click a sheep | select it |
| click it again | focus that agent's pane |

A click anywhere in a sheep's lane counts, not just on the sprite — a sheep
wanders inside its lane, and clicking it should not be a game of timing. The
pasture asks the terminal for button presses only, so moving the mouse across
the pane costs nothing.

Selecting a sheep fills the line above the key hints with its pane id,
workspace, agent kind and provider, context usage, rate budget, and terminal
title.

## Bind it to a key

Two actions, because opening the pasture has two different reasons:

| Action | Opens |
| --- | --- |
| `huketo.sheep.open-here` | an overlay over the pane you pressed the key in. Herdr restores the previous layout and focus when the pasture closes, so this is the "show me the whole flock for a second" key. |
| `huketo.sheep.toggle-side` | a split beside that pane, which stays. Pressing the key again closes the pasture wherever it is, so there is never a second one. |

Add this to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+y"
type = "plugin_action"
command = "huketo.sheep.open-here"
description = "Sheep: pasture over this pane"

# Optional: the same action as a single keypress, next to prefix+y.
[[keys.command]]
key = "f10"
type = "plugin_action"
command = "huketo.sheep.open-here"
description = "Sheep: pasture over this pane"

[[keys.command]]
key = "prefix+shift+y"
type = "plugin_action"
command = "huketo.sheep.toggle-side"
description = "Sheep: toggle pasture side pane"
```

Then `herdr server reload-config`, and `prefix+?` lists the new bindings.

Pick your own keys, but check them against Herdr's defaults first: `prefix+s`
is settings and `prefix+p` is previous-tab, so neither is free. `prefix+y` is
picked here because `y` is free and 양 is Korean for sheep. On a terminal
without the Kitty keyboard protocol — Windows Terminal, for one — a
`ctrl+shift+<letter>` chord arrives without its shift, which is why the direct
binding above is a function key rather than a chord.

Both actions are also reachable from Herdr's command palette and with
`herdr plugin action invoke huketo.sheep.toggle-side`.

## Small panes

A pane too short or too narrow for the pasture switches to one line per agent,
still ordered by urgency, and says how many agents did not fit. Grow the pane
and the sheep come back. Clicking a row selects that agent there too.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `HERDR_SHEEP_FPS` | `20` | animation frames per second, clamped to 5-60 |
| `HERDR_SHEEP_POLL_MS` | `800` | how often to read session state, clamped to 200-10000 |

The plugin reads state with `herdr api snapshot` through `HERDR_BIN_PATH`, moves
focus with `herdr agent focus`, and opens or closes its own pane with
`herdr plugin pane`. It writes nothing else and stores no state.

## Outside a pane

```bash
herdr-sheep --snapshot 100x34    # one frame of your live session, as plain text
herdr-sheep --demo               # one frame with a sheep in every state
herdr-sheep --open side          # toggle the pasture in a side split
herdr-sheep --help
```

`--snapshot` and `--open` need a running Herdr server; `--demo` does not.

## Development

```bash
git clone https://github.com/huketo/herdr-sheep
cd herdr-sheep
just link      # build release, stage bin/herdr-sheep, herdr plugin link .
```

`herdr plugin link` skips build commands, so a linked checkout stages its own
binary into `bin/`, which is where the manifest looks. After a code change, run
`just install` and reopen the pane.

`just` lists every task. The ones worth knowing:

| Task | Does |
| --- | --- |
| `just ci` | everything CI runs: version lockstep, `fmt --check`, clippy with `-D warnings`, tests, the mouse smoke test, a rendered frame |
| `just fmt` | format the tree |
| `just test` | unit tests plus the end-to-end tests in `tests/cli.rs` |
| `just smoke` | pty smoke test: drives a real binary against a stub `herdr`, clicks a sheep, checks the pane's answer |
| `just audit` | `cargo deny check` over `deny.toml` |
| `just changelog` | regenerate `CHANGELOG.md` from the commit history |
| `just demo 120x40` | print one frame at a size |

Unit tests cover the snapshot parser, zone layout and hit-testing, sprite
mirroring, animation, and the rendered frame. `tests/cli.rs` runs the built
binary: the frame it prints, every usage error, and the failure path when Herdr
is not reachable.

## Releasing

Commit subjects are [Conventional Commits](https://www.conventionalcommits.org/).
`CHANGELOG.md` and every release body are generated from them with
[git-cliff](https://git-cliff.org), so a commit subject is a release note —
`just release-notes` prints what the next release would say.

`herdr/install.sh` downloads the release tagged `v<version>` from
`herdr-plugin.toml`, so a release is cut by making those agree:

1. Bump `version` in `Cargo.toml` and `herdr-plugin.toml` to the same value.
2. `just changelog`, and commit the result.
3. `just check-version v<version>`, then push the tag.

The workflow builds every target, attaches the archives with `sha256` sidecars
and signed build provenance, and publishes the release only after all of them
land. Verify an archive with
`gh attestation verify <archive> --repo huketo/herdr-sheep`.

## License

MIT. See [LICENSE](LICENSE).
