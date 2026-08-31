#!/usr/bin/env bash
# Open the sheep pasture beside the pane that invoked this action.
#
# Bound as a plugin action so it can be reached from a keybinding:
#
#   [[keys.command]]
#   key = "prefix+p"
#   type = "plugin_action"
#   command = "huketo.sheep.open-pasture"
#   description = "sheep pasture"
set -euo pipefail

herdr="${HERDR_BIN_PATH:-herdr}"

args=(plugin pane open --plugin huketo.sheep --entrypoint pasture --placement split --direction right)
if [[ -n "${HERDR_PANE_ID:-}" ]]; then
  args+=(--target-pane "$HERDR_PANE_ID")
fi

exec "$herdr" "${args[@]}"
