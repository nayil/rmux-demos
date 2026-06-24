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
    printf '%%%d\n' "$((count + 1))"
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

if ! grep -Fq 'select-pane -t %1 -T developer: claude' "$LOG"; then
  echo "default launch did not use simple mode developer role" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %3 -T reviewer: claude' "$LOG"; then
  echo "default launch did not use simple mode reviewer role" >&2
  cat "$LOG" >&2
  exit 1
fi

case "$leader_cmd" in
  *"export RMUX_WORKGROUP_MODE=simple;"*"export RMUX_MEMBER_COUNT=2;"*) ;;
  *)
    echo "default launch did not export simple mode with two members" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Mode-specific leader workflow: simple"*"leader -> developer: implement"*"Developer owns implementation; reviewer owns validation"*) ;;
  *)
    echo "default launch did not inject a mode-specific leader workflow" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(cd "$STARTDIR" && "$ROOT/launch.sh" launch "$WORKDIR" 3 >/dev/null)

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

if ! grep -Fq 'select-pane -t %4 -T member3: claude' "$LOG"; then
  echo "launcher did not create and title member3 when member count is 3" >&2
  cat "$LOG" >&2
  exit 1
fi

case "$leader_cmd" in
  *"export RMUX_MEMBER_COUNT=3;"*) ;;
  *)
    echo "leader command did not export RMUX_MEMBER_COUNT=3" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"export RMUX_MEMBER_TARGETS=member1=%1\\ member2=%3\\ member3=%4;"*) ;;
  *)
    echo "leader command did not export all member targets" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"leader -> each member: assign scoped responsibility"*"Leader must name ownership explicitly"*) ;;
  *)
    echo "custom member-count launch did not inject the custom leader workflow" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup pane-border-status top' "$LOG"; then
  echo "launcher did not enable top pane border titles" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup pane-active-border-style fg=yellow' "$LOG"; then
  echo "launcher did not configure active pane border color" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup pane-border-style fg=colour238' "$LOG"; then
  echo "launcher did not configure inactive pane border color" >&2
  cat "$LOG" >&2
  exit 1
fi

border_format_lines="$(grep 'pane-border-format' "$LOG")"
if grep -Fq '#{pane_title}' <<<"$border_format_lines"; then
  echo "pane border format should not depend on pane_title because Claude overwrites it" >&2
  printf '%s\n' "$border_format_lines" >&2
  exit 1
fi

if ! grep -Fq 'pane-border-format #[align=left]#{?pane_active,#[bold,fg=black,bg=yellow],#[bold,fg=colour250,bg=colour238]}' <<<"$border_format_lines"; then
  echo "launcher did not configure highlighted left-aligned pane titles" >&2
  printf '%s\n' "$border_format_lines" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(cd "$STARTDIR" && "$ROOT/launch.sh" launch "$WORKDIR" 8 --mode classic >/dev/null)

leader_cmd="$(grep '^respawn-pane ' "$LOG" | tail -n 1)"

if ! grep -Fq 'select-pane -t %1 -T frontend: claude' "$LOG"; then
  echo "classic mode did not title the first member as frontend" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %3 -T backend: claude' "$LOG"; then
  echo "classic mode did not title the second member as backend" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %4 -T architect: claude' "$LOG"; then
  echo "classic mode did not title the third member as architect" >&2
  cat "$LOG" >&2
  exit 1
fi

if grep -Fq 'member8:' "$LOG"; then
  echo "mode did not take precedence over member count" >&2
  cat "$LOG" >&2
  exit 1
fi

case "$leader_cmd" in
  *"export RMUX_WORKGROUP_MODE=classic;"*"export RMUX_MEMBER_COUNT=3;"*) ;;
  *)
    echo "leader command did not export classic mode with mode-derived member count" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"export RMUX_MEMBER_TARGETS=frontend=%1\\ backend=%3\\ architect=%4;"*) ;;
  *)
    echo "leader command did not export classic role targets" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"export RMUX_MEMBER_ROLES=frontend=Frontend\\ Engineer\\,backend=Backend\\ Engineer\\,architect=Software\\ Architect;"*) ;;
  *)
    echo "leader command did not export classic role descriptions" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

if ! grep -Fq 'Frontend\ Engineer' "$LOG"; then
  echo "frontend member command did not receive a role prompt" >&2
  cat "$LOG" >&2
  exit 1
fi

case "$leader_cmd" in
  *"Mode-specific leader workflow: classic"*"leader -> architect: boundaries/risks"*"Leader must delegate frontend/backend/architecture work"*) ;;
  *)
    echo "classic launch did not inject the classic leader workflow" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

classic_border_format="$(grep 'pane-border-format' "$LOG" | tail -n 1)"

case "$classic_border_format" in
  *'#{pane_title}'*)
    echo "classic border format should not depend on pane_title" >&2
    echo "$classic_border_format" >&2
    exit 1
    ;;
esac

case "$classic_border_format" in
  *'#{?#{==:#{pane_id},%1},frontend,'*'#{?#{==:#{pane_id},%3},backend,'*'#{?#{==:#{pane_id},%4},architect,'*'#{?#{==:#{pane_id},%2},leader,'*) ;;
  *)
    echo "classic border format did not map pane ids to stable role labels" >&2
    echo "$classic_border_format" >&2
    exit 1
    ;;
esac

HELP="$TMP/help.txt"
"$ROOT/launch.sh" help >"$HELP"

if ! grep -Fq 'classic   frontend, backend, architect' "$HELP"; then
  echo "help did not describe classic mode roles" >&2
  cat "$HELP" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(cd "$STARTDIR" && "$ROOT/launch.sh" launch "$WORKDIR" --mode advanced >/dev/null)

leader_cmd="$(grep '^respawn-pane ' "$LOG" | tail -n 1)"

case "$leader_cmd" in
  *"export RMUX_WORKGROUP_MODE=advanced;"*"export RMUX_MEMBER_COUNT=5;"*) ;;
  *)
    echo "advanced mode did not export its mode-derived member count" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Mode-specific leader workflow: advanced"*"leader -> interaction: flows/accessibility"*"leader -> qa: test strategy"*"final gates"*) ;;
  *)
    echo "advanced launch did not inject the advanced leader workflow" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

if ! grep -Fq 'select-pane -t %1 -T frontend: claude' "$LOG"; then
  echo "advanced mode did not title frontend" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %3 -T interaction: claude' "$LOG"; then
  echo "advanced mode did not title interaction" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %4 -T backend: claude' "$LOG"; then
  echo "advanced mode did not title backend" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %5 -T architect: claude' "$LOG"; then
  echo "advanced mode did not place architect in the second member row" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'select-pane -t %6 -T qa: claude' "$LOG"; then
  echo "advanced mode did not place qa in the second member row" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -F 'split-window -v -P -F #{pane_id} -t %1 -p 50' "$LOG" | grep -Fq 'architect'; then
  echo "advanced mode did not split architect below the first member column" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -F 'split-window -v -P -F #{pane_id} -t %3 -p 50' "$LOG" | grep -Fq 'qa'; then
  echo "advanced mode did not split qa below the second member column" >&2
  cat "$LOG" >&2
  exit 1
fi
