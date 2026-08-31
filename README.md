# herdr-sheep

*[한국어 README](README.ko.md)*

A [Herdr](https://herdr.dev) plugin pane that turns every coding agent in your
session into a sheep. Sheep run, graze, jump, ask for help, or snore depending
on what their agent is actually doing, so a glance at the pane tells you which
agents need you.

```text
(>_@ herdr-sheep      1 waiting on you  1 done  2 working  1 idle  1 unknown
~ GATE  1 waiting on you ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                   (?)
                                   ,@~~~.
                                  (o_ ~~ )
                                   ''  ''
                                   deploy
                   ,     ', '    .           .        ' .      '     .
~ PEN  1 done ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                        +
                                   ,@~~~. *
                                 +(^_ ~~ )
                                   \'  '/
                                  reviewer
 '             ,           ,          ,    '   ,  . .   '    .           ,
~ PADDOCK  2 working ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                           ,@~~~.                                   ,~~~@.
                          (>_ ~~ )                                 ( ~~ _<)
                           /'  '\                                   /'  '\
                          *runner                                    docs
      '  ,''                       .       .      '    '    ,            , .
~ MEADOW  1 idle ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                 ,~~~@.
                                 ( ~~ )
                               , ''  _< .
                                 scout
~ FOLD  1 unknown ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                        z z z
                                   ,@~~~.
                                  (-_ ~~ )
                                   ~~~~~~
                                  mystery
press j or k to pick a sheep
j/k select  enter focus  r refresh  q quit
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

## Install

Requirements: Herdr 0.8.0 or newer, plus `cargo` on the machine — the plugin is
a Rust binary that Herdr builds at install time.

```bash
herdr plugin install huketo/herdr-sheep
herdr plugin pane open --plugin huketo.sheep --entrypoint pasture
```

Bind it to a key by adding this to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+p"
type = "plugin_action"
command = "huketo.sheep.open-pasture"
description = "sheep pasture"
```

The action opens the pasture in a split beside the pane you pressed the key in.

## Keys

| Key | Does |
| --- | --- |
| `j` `k`, arrows, `Tab` | move between sheep, in zone order |
| `Enter` `f` | focus the selected agent's pane; with nothing selected, focus the most urgent agent |
| `r` | poll the session now instead of waiting for the next tick |
| `q` `Esc` `Ctrl-C` | leave the pasture and close the pane |

Selecting a sheep fills the line above the key hints with its pane id,
workspace, agent kind and provider, context usage, rate budget, and terminal
title.

## Small panes

A pane too short or too narrow for the pasture switches to one line per agent,
still ordered by urgency, and says how many agents did not fit. Grow the pane
and the sheep come back.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `HERDR_SHEEP_FPS` | `20` | animation frames per second, clamped to 5-60 |
| `HERDR_SHEEP_POLL_MS` | `800` | how often to read session state, clamped to 200-10000 |

The plugin reads state with `herdr api snapshot` through `HERDR_BIN_PATH`, and
moves focus with `herdr agent focus`. It writes nothing else and stores no
state.

## Outside a pane

```bash
herdr-sheep --snapshot 100x34    # one frame of your live session, as plain text
herdr-sheep --demo               # one frame with a sheep in every state
herdr-sheep --help
```

`--snapshot` needs a running Herdr server; `--demo` does not.

## Development

```bash
git clone https://github.com/huketo/herdr-sheep
cd herdr-sheep
cargo build --release
herdr plugin link "$PWD"
herdr plugin pane open --plugin huketo.sheep --entrypoint pasture
```

`plugin link` does not run build commands, so rebuild yourself after a change,
then reopen the pane. `cargo test` covers the snapshot parser, zone layout,
sprite mirroring, animation, and the rendered frame.

## License

MIT. See [LICENSE](LICENSE).
