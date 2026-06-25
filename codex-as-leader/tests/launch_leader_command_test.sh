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
    if [[ -n "${RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST:-}" ]]; then
      count_file="${RMUX_TEST_HAS_SESSION_COUNT_FILE:?}"
      count=0
      if [[ -f "$count_file" ]]; then
        count="$(<"$count_file")"
      fi
      count=$((count + 1))
      printf '%s' "$count" >"$count_file"
      if (( count == 2 )); then
        exit 0
      fi
    fi
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
  capture-pane)
    printf '%s\n' "${RMUX_TEST_CAPTURE_PANE_OUTPUT:-}"
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

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup status on' "$LOG"; then
  echo "launcher did not enable a stable status line" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup status-interval 0' "$LOG"; then
  echo "launcher did not disable periodic status redraws" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup status-left-length 160' "$LOG" || ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup status-right-length 260' "$LOG"; then
  echo "launcher did not reserve enough room for the visual status line" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup automatic-rename off' "$LOG" || ! grep -Fq 'set-option -w -t codex-as-leader-test:workgroup allow-rename off' "$LOG"; then
  echo "launcher did not disable automatic window renaming" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'status-left #[bold]#[fg=black]#[bg=colour220] RMUX' "$LOG" || ! grep -Fq 'mode:simple' "$LOG" || ! grep -Fq 'members:2' "$LOG"; then
  echo "status line did not include mode, member count, and leader guard" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'status-right #[bold]#[fg=black]#[bg=colour46] orchestrator-only' "$LOG"; then
  echo "status line did not include a visible orchestrator guard badge" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'pane-border-lines heavy' "$LOG"; then
  echo "launcher did not request strong pane border lines" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq '#[bold]#[fg=white]#[bg=colour34] developer #[default]' "$LOG" || ! grep -Fq '#[bold]#[fg=black]#[bg=colour220] leader #[default]' "$LOG"; then
  echo "pane border labels did not include visible role color badges" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'dclaude\ --permission-mode\ auto' "$LOG"; then
  echo "default Claude member command did not enable auto permission mode" >&2
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

for expected in \
  "Mode-specific context: simple" \
  "Role ownership" \
  "developer -> implementation plan, code changes, local self-checks" \
  "reviewer -> critique, edge cases, regression tests, final review" \
  "Consultation order" \
  "developer first for implementation" \
  "reviewer before final answer"; do
  if [[ "$leader_cmd" != *"$expected"* ]]; then
    echo "default launch did not inject lean simple mode context: $expected" >&2
    echo "$leader_cmd" >&2
    exit 1
  fi
done

for expected in \
  "Leadership operating contract" \
  "Leader mission" \
  "You are the team driver and inspector, not the default implementer" \
  "Task intake protocol" \
  "When the user gives a task, first analyze the problem" \
  "Identify work domains" \
  "Build role responsibility mapping" \
  "Build an ownership map before implementation" \
  "Use RMUX_MEMBER_TARGETS as the source of truth" \
  "Do not assume pane order equals role" \
  "Leader hard boundary" \
  "You MUST NOT write code" \
  "You MUST NOT edit configuration" \
  "You MUST NOT run implementation commands that modify project files" \
  "All code/config/file changes must be delegated to member agents" \
  "If all relevant members are blocked or unavailable, stop and report blocked status to the user" \
  "Mandatory short-cycle protocol" \
  "Plan -> Delegate -> Wait/Collect -> Integrate -> Verify -> Report" \
  "You MUST complete one delegation cycle before any implementation work" \
  "Checkpoint gates" \
  "Implementation gate" \
  "Major-change gate" \
  "Final-answer gate"; do
  if [[ "$leader_cmd" != *"$expected"* ]]; then
    echo "default launch did not inject leader operating contract guardrail: $expected" >&2
    echo "$leader_cmd" >&2
    exit 1
  fi
done

case "$leader_cmd" in
  *"Bypass policy"*|*"Gate bypass reason"*|*"tiny control-plane fix"*|*"bypass a member-owned role"*)
    echo "leader prompt should not include bypass policy or tiny-fix implementation escape hatches" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Leadership operating contract"*"Codex Leader Workgroup"*) ;;
  *)
    echo "leader operating contract should be injected before the rmux operation appendix" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Mode-specific context: simple"*"Role ownership"*"Consultation order"*) ;;
  *)
    echo "simple mode prompt should use lean mode-specific context" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Mode-specific leader workflow: simple"*|*"Workflow graph:"*|*"Leader must not do the whole task alone unless"*)
    echo "mode prompt should not repeat global workflow and anti-single-player rules" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"export RMUX_ALERT_DIR=$TMP/alerts/codex-as-leader-test;"*"export RMUX_ALERT_LOG=$TMP/alerts/codex-as-leader-test/alerts.log;"*) ;;
  *)
    echo "default launch did not export session-scoped alert paths" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

if [[ ! -f "$TMP/alerts/codex-as-leader-test/alerts.log" ]]; then
  echo "default launch did not create a session-scoped alert log" >&2
  find "$TMP" -maxdepth 3 -type f -print >&2
  exit 1
fi

if [[ ! -f "$TMP/alerts/codex-as-leader-test/watcher.pid" ]]; then
  echo "default launch did not record the alert watcher pid in the session alert dir" >&2
  find "$TMP" -maxdepth 3 -type f -print >&2
  exit 1
fi

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
  *"Mode-specific context: custom"*"each member -> explicitly assigned scoped responsibility"*"Ask at least one member for implementation or analysis and at least one member for review/verification"*) ;;
  *)
    echo "custom member-count launch did not inject lean custom mode context" >&2
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

if ! grep -Fq 'pane-border-format #[align=left]#{?pane_active,#[bold]#[fg=black]#[bg=colour220] > #[default]' <<<"$border_format_lines" || ! grep -Fq '#[bold]#[fg=white]#[bg=colour244] member1 #[default]' <<<"$border_format_lines"; then
  echo "launcher did not configure strong left-aligned pane role badges" >&2
  printf '%s\n' "$border_format_lines" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(
  export RMUX_THEME="mono"
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

if ! grep -Fq 'pane-active-border-style fg=white' "$LOG" || ! grep -Fq 'pane-border-style fg=colour244' "$LOG"; then
  echo "mono theme did not configure expected border colors" >&2
  cat "$LOG" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(
  export CLAUDE_CMD="codex"
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

if ! grep -Fq 'select-pane -t %1 -T developer: codex' "$LOG"; then
  echo "member pane title did not show the inferred codex agent" >&2
  cat "$LOG" >&2
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
  *"Mode-specific context: classic"*"architect -> boundaries/risks/sequencing"*"Consultation order"*) ;;
  *)
    echo "classic launch did not inject lean classic mode context" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

for expected in \
  "Role ownership" \
  "frontend -> UI/client state and browser behavior" \
  "backend -> APIs/data/runtime behavior" \
  "architect -> boundaries/risks/sequencing" \
  "Ask architect for boundaries"; do
  if [[ "$leader_cmd" != *"$expected"* ]]; then
    echo "classic launch did not inject explicit role ownership text: $expected" >&2
    echo "$leader_cmd" >&2
    exit 1
  fi
done

classic_border_format="$(grep 'pane-border-format' "$LOG" | tail -n 1)"

case "$classic_border_format" in
  *'#{pane_title}'*)
    echo "classic border format should not depend on pane_title" >&2
    echo "$classic_border_format" >&2
    exit 1
    ;;
esac

case "$classic_border_format" in
  *'#{?#{==:#{pane_id},%1},'*' frontend #[default]'*'#{?#{==:#{pane_id},%3},'*' backend #[default]'*'#{?#{==:#{pane_id},%4},'*' architect #[default]'*'#{?#{==:#{pane_id},%2},'*' leader #[default]'*) ;;
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

if ! grep -Fq 'split-window -v -P -F #{pane_id} -t %1 -p 38 sleep 3600' "$LOG"; then
  echo "advanced mode did not reserve a compact fixed leader pane height" >&2
  cat "$LOG" >&2
  exit 1
fi

case "$leader_cmd" in
  *"export RMUX_WORKGROUP_MODE=advanced;"*"export RMUX_MEMBER_COUNT=5;"*) ;;
  *)
    echo "advanced mode did not export its mode-derived member count" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

case "$leader_cmd" in
  *"Mode-specific context: advanced"*"interaction -> flows/accessibility/user feedback"*"qa -> test strategy/regression gates"*) ;;
  *)
    echo "advanced launch did not inject lean advanced mode context" >&2
    echo "$leader_cmd" >&2
    exit 1
    ;;
esac

for expected in \
  "Role ownership" \
  "interaction -> flows/accessibility/user feedback" \
  "qa -> test strategy/regression gates" \
  "Return to architect and qa for final gates"; do
  if [[ "$leader_cmd" != *"$expected"* ]]; then
    echo "advanced launch did not inject explicit advanced ownership text: $expected" >&2
    echo "$leader_cmd" >&2
    exit 1
  fi
done

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

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(
  export RMUX_WORKGROUP_SESSION="codex-as-leader-other"
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

other_leader_cmd="$(grep '^respawn-pane ' "$LOG" | tail -n 1)"

case "$other_leader_cmd" in
  *"export RMUX_ALERT_DIR=$TMP/alerts/codex-as-leader-other;"*"export RMUX_ALERT_LOG=$TMP/alerts/codex-as-leader-other/alerts.log;"*) ;;
  *)
    echo "second session did not get its own alert paths" >&2
    echo "$other_leader_cmd" >&2
    exit 1
    ;;
esac

if [[ ! -f "$TMP/alerts/codex-as-leader-other/alerts.log" ]]; then
  echo "second session did not create its own alert log" >&2
  find "$TMP" -maxdepth 3 -type f -print >&2
  exit 1
fi

for i in $(seq 1 25); do
  printf '2026-06-25T00:00:%02d+0800\tfrontend\t%%1\talert line %02d\n' "$i" "$i"
done >"$TMP/alerts/codex-as-leader-other/alerts.log"
alerts_output="$(RMUX_WORKGROUP_SESSION=codex-as-leader-other "$ROOT/launch.sh" alerts)"

if [[ "$alerts_output" != *"alert line 25"* || "$alerts_output" == *"alert line 01"* ]]; then
  echo "alerts command did not show the latest session-scoped alert entries by default" >&2
  printf '%s\n' "$alerts_output" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"
rm -f "$TMP/has-session-count"
export RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST=1
export RMUX_TEST_HAS_SESSION_COUNT_FILE="$TMP/has-session-count"
export RMUX_TEST_CAPTURE_PANE_OUTPUT="需要用户确认的问题：刷新频率和保留周期"

(
  export RMUX_WORKGROUP_SESSION="codex-as-leader-nonalert"
  export RMUX_ALERT_INTERVAL=0
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

for _ in 1 2 3 4 5 6 7 8 9 10; do
  if [[ -f "$TMP/has-session-count" ]] && (( $(<"$TMP/has-session-count") >= 3 )); then
    break
  fi
  sleep 0.1
done

unset RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST
unset RMUX_TEST_HAS_SESSION_COUNT_FILE
unset RMUX_TEST_CAPTURE_PANE_OUTPUT

if [[ -s "$TMP/alerts/codex-as-leader-nonalert/alerts.log" ]]; then
  echo "alert watcher should not treat ordinary confirmation discussion as an interactive prompt" >&2
  cat "$TMP/alerts/codex-as-leader-nonalert/alerts.log" >&2
  exit 1
fi

if grep -Fq 'select-pane -t %1 -T ! developer: claude' "$LOG"; then
  echo "alert watcher should not mark a member waiting for ordinary confirmation discussion" >&2
  cat "$LOG" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"
rm -f "$TMP/has-session-count"
export RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST=1
export RMUX_TEST_HAS_SESSION_COUNT_FILE="$TMP/has-session-count"
export RMUX_TEST_CAPTURE_PANE_OUTPUT="- If you are blocked, state the missing input or evidence needed."

(
  export RMUX_WORKGROUP_SESSION="codex-as-leader-role-prompt"
  export RMUX_ALERT_INTERVAL=0
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

for _ in 1 2 3 4 5 6 7 8 9 10; do
  if [[ -f "$TMP/has-session-count" ]] && (( $(<"$TMP/has-session-count") >= 3 )); then
    break
  fi
  sleep 0.1
done

unset RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST
unset RMUX_TEST_HAS_SESSION_COUNT_FILE
unset RMUX_TEST_CAPTURE_PANE_OUTPUT

if [[ -s "$TMP/alerts/codex-as-leader-role-prompt/alerts.log" ]]; then
  echo "alert watcher should not treat role prompt blocked guidance as a member alert" >&2
  cat "$TMP/alerts/codex-as-leader-role-prompt/alerts.log" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

(
  export CLAUDE_CMD="claude --permission-mode acceptEdits"
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

if ! grep -Fq 'claude\ --permission-mode\ acceptEdits' "$LOG"; then
  echo "explicit Claude permission mode was not preserved" >&2
  cat "$LOG" >&2
  exit 1
fi

if grep -Fq 'claude\ --permission-mode\ acceptEdits\ --permission-mode\ auto' "$LOG"; then
  echo "launcher should not append auto mode when Claude permission mode is explicit" >&2
  cat "$LOG" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"

cat >"$FAKEBIN/codex-member" <<'FAKE'
#!/usr/bin/env bash
exit 0
FAKE
chmod +x "$FAKEBIN/codex-member"

(
  export CLAUDE_CMD="codex-member"
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

if grep -Fq 'codex-member\ --permission-mode\ auto' "$LOG"; then
  echo "non-Claude member command should not receive Claude auto mode" >&2
  cat "$LOG" >&2
  exit 1
fi

: >"$LOG"
rm -f "$RMUX_TEST_SPLIT_COUNT"
rm -f "$TMP/has-session-count"
export RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST=1
export RMUX_TEST_HAS_SESSION_COUNT_FILE="$TMP/has-session-count"
export RMUX_TEST_CAPTURE_PANE_OUTPUT="Do you want to proceed? [y/N]"

(
  export RMUX_WORKGROUP_SESSION="codex-as-leader-alert"
  export RMUX_ALERT_INTERVAL=0
  cd "$STARTDIR"
  "$ROOT/launch.sh" launch "$WORKDIR" >/dev/null
)

for _ in 1 2 3 4 5 6 7 8 9 10; do
  if grep -Fq '! developer' "$LOG" && [[ -f "$TMP/has-session-count" ]] && (( $(<"$TMP/has-session-count") >= 3 )); then
    break
  fi
  sleep 0.1
done

unset RMUX_TEST_HAS_SESSION_ONCE_AFTER_FIRST
unset RMUX_TEST_HAS_SESSION_COUNT_FILE
unset RMUX_TEST_CAPTURE_PANE_OUTPUT

if ! grep -Fq 'Do you want to proceed? [y/N]' "$TMP/alerts/codex-as-leader-alert/alerts.log"; then
  echo "alert watcher did not write member approval prompt to the session alert log" >&2
  cat "$TMP/alerts/codex-as-leader-alert/alerts.log" >&2
  exit 1
fi

if grep -Fq 'select-pane -t %1 -T ! developer: claude' "$LOG"; then
  echo "alert watcher should not use select-pane to mark alerts because it steals focus" >&2
  cat "$LOG" >&2
  exit 1
fi

if ! grep -Fq 'pane-border-format #[align=left]' "$LOG" || ! grep -Fq '! developer' "$LOG"; then
  echo "alert watcher did not refresh pane border labels with a visible alert marker" >&2
  cat "$LOG" >&2
  exit 1
fi

if grep -Fq 'display-message -t codex-as-leader-alert:workgroup' "$LOG"; then
  echo "alert watcher should not show transient display messages by default because they cause visual flicker" >&2
  cat "$LOG" >&2
  exit 1
fi
