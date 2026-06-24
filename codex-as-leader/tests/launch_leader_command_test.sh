#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="${TMPDIR:-/tmp}/codex-as-leader-test-$$"
FAKEBIN="$TMP/bin"
WORKDIR="$TMP/project"
STARTDIR="$TMP/start"
LOG="$TMP/rmux.log"

cleanup() {
  rm -rf "$TMP"
}
trap cleanup EXIT

mkdir -p "$FAKEBIN" "$WORKDIR" "$STARTDIR"

cat >"$WORKDIR/AGENTS.md" <<'AGENTS'
# Target Project Instructions

This marker must come from the target workdir.
AGENTS

cat >"$FAKEBIN/codex" <<'FAKE'
#!/usr/bin/env bash
exit 0
FAKE
chmod +x "$FAKEBIN/codex"

cat >"$FAKEBIN/dclaude" <<'FAKE'
#!/usr/bin/env bash
exit 0
FAKE
chmod +x "$FAKEBIN/dclaude"

cat >"$FAKEBIN/rmux" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail

LOG="${RMUX_TEST_LOG:?}"
if [[ "${1:-}" == "-S" ]]; then
  shift 2
fi

cmd="${1:-}"
shift || true
printf '%s\n' "$cmd $*" >>"$LOG"

case "$cmd" in
  has-session)
    exit 1
    ;;
  new-session)
    printf '%%1\n'
    ;;
  split-window)
    count_file="${RMUX_TEST_SPLIT_COUNT:?}"
    count=0
    if [[ -f "$count_file" ]]; then
      count="$(<"$count_file")"
    fi
    count=$((count + 1))
    printf '%s' "$count" >"$count_file"
    if [[ "$count" -eq 1 ]]; then
      printf '%%2\n'
    else
      printf '%%3\n'
    fi
    ;;
  select-pane|respawn-pane|attach-session|kill-session|kill-server)
    exit 0
    ;;
  list-sessions)
    printf 'codex-as-leader\n'
    ;;
esac
FAKE
chmod +x "$FAKEBIN/rmux"

export PATH="$FAKEBIN:$PATH"
export RMUX_TEST_LOG="$LOG"
export RMUX_TEST_SPLIT_COUNT="$TMP/split-count"
export RMUX_DEMO_SOCKET="$TMP/socket"
export RMUX_WORKGROUP_SESSION="codex-as-leader-test"
export CLAUDE_CMD="dclaude"

(cd "$STARTDIR" && "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null)

leader_cmd="$(grep '^respawn-pane ' "$LOG" | tail -n 1)"

case "$leader_cmd" in
  *"codex --dangerously-bypass-approvals-and-sandbox -C $WORKDIR --add-dir $ROOT"*) ;;
  *)
    echo "leader command did not make target workdir the Codex primary root" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Codex Leader Workgroup"*) ;;
  *)
    echo "leader command did not pass demo leader instructions as the initial prompt" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac
