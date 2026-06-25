#!/usr/bin/env bash
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNAME_S="$(uname -s)"
SOCKET_DIR="${TMPDIR:-/tmp}/rmux-codex-as-leader-${USER:-user}"
if [[ "$UNAME_S" == MINGW* || "$UNAME_S" == MSYS* || "$UNAME_S" == CYGWIN* ]]; then
  DEFAULT_SOCKET='\\.\pipe\rmux-codex-as-leader'
else
  DEFAULT_SOCKET="$SOCKET_DIR/socket"
fi
SOCKET="${RMUX_DEMO_SOCKET:-$DEFAULT_SOCKET}"
SESSION_BASE="${RMUX_WORKGROUP_SESSION:-codex-as-leader}"
SESSION="$SESSION_BASE"
WINDOW="${RMUX_WORKGROUP_WINDOW:-workgroup}"
LAUNCH_CWD="$(pwd)"
WORKDIR_INPUT="${RMUX_WORKDIR:-${WORKDIR:-$LAUNCH_CWD}}"
WORKDIR=""
MEMBER_COUNT_INPUT="${RMUX_MEMBER_COUNT:-}"
MEMBER_COUNT=""
WORKGROUP_MODE_INPUT="${RMUX_WORKGROUP_MODE:-}"
WORKGROUP_MODE=""
MEMBER_NAMES=()
MEMBER_ROLE_TITLES=()
MEMBER_ROLE_PROMPTS=()
ALERT_ROOT=""
ALERT_DIR=""
ALERT_LOG=""
ALERT_INTERVAL="${RMUX_ALERT_INTERVAL:-2}"
ALERT_WATCH="${RMUX_ALERT_WATCH:-1}"
ALERT_COOLDOWN="${RMUX_ALERT_COOLDOWN:-30}"
ALERT_MESSAGE="${RMUX_ALERT_MESSAGE:-0}"
RMUX_THEME="${RMUX_THEME:-dark}"
THEME_ACTIVE_BORDER="fg=yellow"
THEME_INACTIVE_BORDER="fg=colour238"
THEME_ACTIVE_LABEL="#[bold,fg=black,bg=yellow]"
THEME_INACTIVE_LABEL="#[bold,fg=colour250,bg=colour238]"
THEME_STATUS_STYLE="fg=colour250,bg=colour236"
THEME_STATUS_LEFT_STYLE="#[bold,fg=black,bg=yellow]"
THEME_STATUS_RIGHT_STYLE="#[fg=colour250,bg=colour236]"

DEFAULT_CLAUDE_CMD="dfclaude"
MEMBER_CMD="${CLAUDE_CMD:-$DEFAULT_CLAUDE_CMD}"

q() {
  printf "%q" "$1"
}

apply_theme() {
  case "$RMUX_THEME" in
    dark|"")
      THEME_ACTIVE_BORDER="fg=yellow"
      THEME_INACTIVE_BORDER="fg=colour238"
      THEME_ACTIVE_LABEL="#[bold,fg=black,bg=yellow]"
      THEME_INACTIVE_LABEL="#[bold,fg=colour250,bg=colour238]"
      THEME_STATUS_STYLE="fg=colour250,bg=colour236"
      THEME_STATUS_LEFT_STYLE="#[bold,fg=black,bg=yellow]"
      THEME_STATUS_RIGHT_STYLE="#[fg=colour250,bg=colour236]"
      ;;
    light)
      THEME_ACTIVE_BORDER="fg=blue"
      THEME_INACTIVE_BORDER="fg=colour250"
      THEME_ACTIVE_LABEL="#[bold,fg=white,bg=blue]"
      THEME_INACTIVE_LABEL="#[bold,fg=colour238,bg=colour250]"
      THEME_STATUS_STYLE="fg=colour238,bg=colour255"
      THEME_STATUS_LEFT_STYLE="#[bold,fg=white,bg=blue]"
      THEME_STATUS_RIGHT_STYLE="#[fg=colour238,bg=colour255]"
      ;;
    mono)
      THEME_ACTIVE_BORDER="fg=white"
      THEME_INACTIVE_BORDER="fg=colour244"
      THEME_ACTIVE_LABEL="#[bold,fg=black,bg=white]"
      THEME_INACTIVE_LABEL="#[bold,fg=white,bg=colour238]"
      THEME_STATUS_STYLE="fg=white,bg=black"
      THEME_STATUS_LEFT_STYLE="#[bold,fg=black,bg=white]"
      THEME_STATUS_RIGHT_STYLE="#[fg=white,bg=black]"
      ;;
    *)
      echo "unknown RMUX_THEME: $RMUX_THEME" >&2
      echo "supported themes: dark, light, mono" >&2
      exit 2
      ;;
  esac
}

agent_name_from_command() {
  local cmd="$1"
  local first="${cmd%% *}"
  local base="${first##*/}"

  case "$base" in
    claude|dclaude|dfclaude) printf "claude" ;;
    codex|codex-*) printf "codex" ;;
    *) printf "%s" "$base" ;;
  esac
}

role_badge_colors() {
  local role="$1"
  local marker="${2:-}"
  local fg="white"
  local bg="colour244"

  case "$marker" in
    x)
      printf "white colour196"
      return
      ;;
    ok)
      printf "black colour46"
      return
      ;;
    "?")
      printf "black colour51"
      return
      ;;
    "!")
      printf "black colour220"
      return
      ;;
  esac

  case "$role" in
    leader)
      fg="black"; bg="colour220" ;;
    developer)
      fg="white"; bg="colour34" ;;
    reviewer)
      fg="white"; bg="colour33" ;;
    frontend)
      fg="white"; bg="colour39" ;;
    backend)
      fg="white"; bg="colour28" ;;
    architect)
      fg="white"; bg="colour93" ;;
    interaction)
      fg="white"; bg="colour201" ;;
    qa)
      fg="white"; bg="colour160" ;;
    member*)
      fg="white"; bg="colour244" ;;
  esac

  if [[ "$RMUX_THEME" == "light" && "$role" == "member"* ]]; then
    fg="black"; bg="colour252"
  elif [[ "$RMUX_THEME" == "mono" ]]; then
    if [[ "$role" == "leader" ]]; then
      fg="black"; bg="white"
    else
      fg="white"; bg="colour238"
    fi
  fi

  printf "%s %s" "$fg" "$bg"
}

role_badge_format() {
  local role="$1"
  local label="$2"
  local marker="${3:-}"
  local fg bg

  read -r fg bg <<<"$(role_badge_colors "$role" "$marker")"
  if [[ -n "$marker" ]]; then
    label="$marker $label"
  fi

  printf "#[bold]#[fg=%s]#[bg=%s] %s #[default]" "$fg" "$bg" "$label"
}

rmux_demo() {
  rmux -S "$SOCKET" "$@"
}

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command in PATH: $1" >&2
    exit 1
  }
}

need_bash_command() {
  local name="$1"
  local name_q

  name_q="$(q "$name")"
  bash -ic "type $name_q" >/dev/null 2>&1 || {
    echo "missing bash command/function/alias: $name" >&2
    exit 1
  }
}

check() {
  need rmux
  need codex
  need bash
  if [[ -z "${CLAUDE_CMD:-}" ]]; then
    need_bash_command "$DEFAULT_CLAUDE_CMD"
  fi
  echo "rmux, codex and bash are available"
}

usage() {
  print_help >&2
}

print_help() {
  cat <<'EOF'
usage:
  ./launch.sh [launch [WORKDIR] [N]]
  ./launch.sh [launch [WORKDIR] --mode MODE]
  ./launch.sh check|list|attach [SESSION]|alerts [SESSION]|cleanup [SESSION]|help

modes:
  simple    developer, reviewer
            Small changes, bugfixes, scripts, and focused implementation.
  classic   frontend, backend, architect
            Standard product work with client, server, and architecture review.
  product   product, designer, developer, qa
            Requirement-to-delivery work with scope, UX, implementation, and QA.
  advanced  frontend, interaction, backend, architect, qa
            Complex cross-module features with dedicated UX and quality review.

priority:
  mode overrides N. If mode is set, its role list determines member count.
  Modes with more than three members use a two-row member grid above the leader.

alerts:
  Member approval/confirmation alerts are written under alerts/<session>/.
  Use ./launch.sh alerts [SESSION] [--all|--follow] to read the alert log.

visual options:
  RMUX_THEME=dark|light|mono sets border/status colors. Default: dark.
  RMUX_ALERT_INTERVAL controls alert polling seconds. Default: 2.
  RMUX_ALERT_COOLDOWN controls repeated alert messages per session. Default: 30.
  RMUX_ALERT_MESSAGE=1 enables transient rmux alert messages. Default: 0.
EOF
}

safe_session_name() {
  local name="$1"
  name="${name//\//_}"
  name="${name// /_}"
  printf "%s" "$name"
}

alert_root() {
  local socket_parent

  case "$SOCKET" in
    \\\\.*|*.pipe)
      printf "%s/alerts" "$SOCKET_DIR"
      ;;
    *)
      socket_parent="$(dirname "$SOCKET")"
      printf "%s/alerts" "$socket_parent"
      ;;
  esac
}

alert_dir_for_session() {
  local session_name="$1"
  printf "%s/%s" "$(alert_root)" "$(safe_session_name "$session_name")"
}

init_alert_log() {
  ALERT_ROOT="$(alert_root)"
  ALERT_DIR="$(alert_dir_for_session "$SESSION")"
  ALERT_LOG="$ALERT_DIR/alerts.log"
  install -d -m 700 "$ALERT_DIR"
  touch "$ALERT_LOG"
}

alert_state_path() {
  local name="$1"
  printf "%s/%s.waiting" "$ALERT_DIR" "$(safe_session_name "$name")"
}

alerts() {
  local session_name="${1:-$SESSION_BASE}"
  local mode="${2:-}"
  local log_path

  if [[ "$session_name" == "--all" || "$session_name" == "--follow" || "$session_name" == "-f" ]]; then
    mode="$session_name"
    session_name="$SESSION_BASE"
  fi

  log_path="$(alert_dir_for_session "$session_name")/alerts.log"
  if [[ ! -f "$log_path" ]]; then
    echo "no alerts for session: $session_name"
    return 0
  fi

  case "$mode" in
    --all)
      cat "$log_path"
      ;;
    --follow|-f)
      tail -n 20 -f "$log_path"
      ;;
    "")
      tail -n 20 "$log_path"
      ;;
    *)
      echo "unknown alerts option: $mode" >&2
      echo "usage: ./launch.sh alerts [SESSION] [--all|--follow]" >&2
      exit 2
      ;;
  esac
}

resolve_workdir() {
  local requested="${1:-$WORKDIR_INPUT}"

  if [[ ! -d "$requested" ]]; then
    echo "workdir does not exist: $requested" >&2
    exit 1
  fi

  WORKDIR="$(cd "$requested" && pwd)"
}

resolve_member_count() {
  local requested="${1:-$MEMBER_COUNT_INPUT}"

  if [[ ! "$requested" =~ ^[1-9][0-9]*$ ]]; then
    echo "member count must be a positive integer: $requested" >&2
    exit 1
  fi

  MEMBER_COUNT="$requested"
}

mode_names() {
  case "$1" in
    simple) printf "%s\n" developer reviewer ;;
    classic) printf "%s\n" frontend backend architect ;;
    product) printf "%s\n" product designer developer qa ;;
    advanced) printf "%s\n" frontend interaction backend architect qa ;;
    *)
      echo "unknown mode: $1" >&2
      echo "run ./launch.sh help to list modes" >&2
      exit 1
      ;;
  esac
}

role_title() {
  case "$1" in
    developer) printf "Developer" ;;
    reviewer) printf "Reviewer" ;;
    frontend) printf "Frontend Engineer" ;;
    backend) printf "Backend Engineer" ;;
    architect) printf "Software Architect" ;;
    product) printf "Product Lead" ;;
    designer) printf "Product Designer" ;;
    interaction) printf "Interaction Designer" ;;
    qa) printf "QA Engineer" ;;
    member*) printf "Generalist Member" ;;
    *) printf "%s" "$1" ;;
  esac
}

role_responsibilities() {
  case "$1" in
    developer)
      printf "%s\n" \
        "- Implement scoped changes with simple, maintainable code." \
        "- Surface technical risks and missing requirements early." \
        "- Run focused self-checks before handing work back."
      ;;
    reviewer)
      printf "%s\n" \
        "- Challenge assumptions, edge cases, and regression risk." \
        "- Propose concrete tests and acceptance checks." \
        "- Review implementation plans and diffs for correctness."
      ;;
    frontend)
      printf "%s\n" \
        "- Own UI behavior, component boundaries, and browser-facing state." \
        "- Keep interactions responsive, accessible, and visually consistent." \
        "- Verify important flows in realistic desktop and mobile contexts."
      ;;
    backend)
      printf "%s\n" \
        "- Own APIs, data flow, persistence, concurrency, and runtime failure modes." \
        "- Keep interfaces explicit and backwards-compatible where practical." \
        "- Define server-side tests and operational checks."
      ;;
    architect)
      printf "%s\n" \
        "- Own system shape, module boundaries, integration points, and tradeoffs." \
        "- Identify coupling, migration risk, and long-term maintenance concerns." \
        "- Keep recommendations pragmatic and tied to the current codebase."
      ;;
    product)
      printf "%s\n" \
        "- Own user goals, scope boundaries, success criteria, and prioritization." \
        "- Turn vague requests into testable acceptance criteria." \
        "- Call out unnecessary scope and unresolved product decisions."
      ;;
    designer)
      printf "%s\n" \
        "- Own information architecture, visual hierarchy, and end-to-end usability." \
        "- Make interaction states, copy, and layout choices concrete." \
        "- Keep the experience coherent for the target audience."
      ;;
    interaction)
      printf "%s\n" \
        "- Own task flow, micro-interactions, accessibility, and user feedback." \
        "- Check that controls, labels, and states are understandable in context." \
        "- Reduce friction without adding decorative complexity."
      ;;
    qa)
      printf "%s\n" \
        "- Own test strategy, regression scope, boundary cases, and release confidence." \
        "- Translate requirements and risks into executable checks." \
        "- Report gaps clearly with reproduction steps and expected results."
      ;;
    *)
      printf "%s\n" \
        "- Collaborate with the leader on assigned analysis or implementation tasks." \
        "- Keep answers concise, concrete, and grounded in observed evidence." \
        "- Ask for clarification when blocked."
      ;;
  esac
}

role_prompt() {
  local name="$1"
  local title="$2"
  local responsibilities

  responsibilities="$(role_responsibilities "$name")"
  cat <<EOF
You are the $name member in a live rmux workgroup.

Role: $title

Responsibilities:
$responsibilities

Collaboration rules:
- Wait for leader instructions before starting new work.
- Members own code, configuration, command execution, and verification work in their assigned scope.
- Keep responses concise, actionable, and grounded in the project at \$RMUX_WORKDIR.
- Do not take rmux control actions unless the leader explicitly asks.
- If you are blocked, state the missing input or evidence needed.
EOF
}

member_command_with_auto_mode() {
  local cmd="$1"
  local first="${cmd%% *}"
  local base="${first##*/}"

  case "$base" in
    claude|dclaude|dfclaude) ;;
    *)
      printf "%s" "$cmd"
      return
      ;;
  esac

  case " $cmd " in
    *" --permission-mode "*|*" --dangerously-skip-permissions "*|*" --allow-dangerously-skip-permissions "*)
      printf "%s" "$cmd"
      ;;
    *)
      printf "%s --permission-mode auto" "$cmd"
      ;;
  esac
}

leader_operating_contract() {
  cat <<'EOF'
Leadership operating contract

Leader mission:
- You are the team driver and inspector, not the default implementer.
- Your job is to analyze, assign ownership, collect member outputs, integrate decisions, verify quality, and report accountability.
- The leader owns orchestration, sequencing, conflict resolution, checkpoint gates, and final accountability.

Source of truth:
- Use RMUX_MEMBER_TARGETS as the source of truth for role-to-pane routing.
- Do not assume pane order equals role. Resolve a role target from RMUX_MEMBER_TARGETS before sending work.
- Use RMUX_MEMBER_ROLES to understand each member's title and expected specialty.

Task intake protocol:
- When the user gives a task, first analyze the problem before delegating or implementing.
- Restate the concrete goal in one sentence.
- Identify work domains: product, UX, frontend, backend, architecture, QA, ops, docs, scripts, or other relevant areas.
- Identify unknowns, risks, and likely files/systems to inspect.
- Build role responsibility mapping: active role -> task responsibility -> expected output.
- Mark irrelevant roles as skipped and state why.

Delegation protocol:
- Build an ownership map before implementation: role -> owned scope -> expected output.
- Send role-scoped requests to the relevant member panes for all implementation work.
- Ask for small, concrete outputs: risks, plan, patch notes, tests, review findings, or acceptance criteria.
- When waiting for members, check RMUX_ALERT_LOG or run ./launch.sh alerts "$RMUX_WORKGROUP_SESSION" before assuming they are still working.

Mandatory short-cycle protocol:
- You MUST run every non-trivial task through: Plan -> Delegate -> Wait/Collect -> Integrate -> Verify -> Report.
- Plan: write the current ownership map and the next role-scoped requests.
- Delegate: send work to the relevant role targets resolved from RMUX_MEMBER_TARGETS.
- Wait/Collect: capture member output or state that a role did not respond.
- Integrate: combine member outputs and resolve conflicts explicitly.
- Verify: run or request checks owned by the relevant role.
- Report: state role contributions, skipped roles, blocked work, and residual risk.

Checkpoint gates:
- Implementation gate: You MUST complete one delegation cycle before any implementation work, and implementation must be done by member agents.
- Major-change gate: before broad edits, architecture changes, UI/API contract changes, or test strategy changes, you MUST consult the owning role(s).
- Final-answer gate: before final answer, you MUST collect review/verification from the relevant role(s), or report blocked if that is impossible.

Leader hard boundary:
- You MUST NOT write code.
- You MUST NOT edit configuration.
- You MUST NOT run implementation commands that modify project files.
- You MUST NOT take over member-owned implementation work.
- All code/config/file changes must be delegated to member agents.
- Do not merge multiple specialties into one vague "member" assignment when named roles exist.
- Keep leader work focused on orchestration, integration, conflict resolution, and final accountability.
- If all relevant members are blocked or unavailable, stop and report blocked status to the user.

Final report contract:
- Report role contributions before final answer.
- State which roles responded, which did not, and which decisions were made by the leader.
- Summarize delegated implementation, verification, unresolved risks, and blocked member-owned work.
EOF
}

leader_workflow_prompt() {
  case "$1" in
    simple)
      cat <<'EOF'
Mode-specific context: simple

Role ownership:
- developer -> implementation plan, code changes, local self-checks
- reviewer -> critique, edge cases, regression tests, final review
- leader -> scope, delegation, integration, verification summary

Consultation order:
1. Ask developer first for implementation plan or code-change notes.
2. Ask reviewer before final answer for risks, tests, and regression checks.

Mode skip rule:
- Reviewer may be skipped only when the task has no code, config, command, or test impact.
EOF
      ;;
    classic)
      cat <<'EOF'
Mode-specific context: classic

Role ownership:
- frontend -> UI/client state and browser behavior
- backend -> APIs/data/runtime behavior
- architect -> boundaries/risks/sequencing
- leader -> ownership map, delegation, integration, final decision

Consultation order:
1. Ask architect for boundaries, coupling risks, and sequencing when the task is cross-cutting.
2. Ask frontend for UI/client impact when user-facing behavior, state, layout, or browser logic is involved.
3. Ask backend for API/data/runtime impact when server behavior, persistence, scripts, or integrations are involved.
4. After implementation, ask architect for coherence review if both frontend and backend scopes changed.

Mode skip rule:
- If a role is irrelevant, leader must state the exclusion reason in the ownership map.
EOF
      ;;
    product)
      cat <<'EOF'
Mode-specific context: product

Role ownership:
- product -> user goal, scope boundary, acceptance criteria, priority
- designer -> UX flow, information architecture, copy, visual hierarchy
- developer -> implementation plan, code changes, technical risks
- qa -> test matrix, boundary cases, regression confidence
- leader -> tradeoff resolution, integration, final readiness decision

Consultation order:
1. Ask product to clarify scope and acceptance criteria before implementation.
2. Ask designer for flow and usability impact before changing user-facing behavior.
3. Ask developer for implementation plan and risk.
4. Ask qa for test matrix before final answer.

Mode skip rule:
- If the task is purely technical, leader may skip product/designer only after stating why.
EOF
      ;;
    advanced)
      cat <<'EOF'
Mode-specific context: advanced

Role ownership:
- architect -> architecture, sequencing, integration boundaries, long-term risk
- interaction -> flows/accessibility/user feedback
- frontend -> UI/state implementation and browser behavior
- backend -> APIs/data/runtime implementation
- qa -> test strategy/regression gates
- leader -> role routing, conflict resolution, final accountability

Consultation order:
1. Ask architect for architecture and sequencing before major implementation.
2. Ask interaction for flow, accessibility, and feedback impact when user behavior changes.
3. Ask frontend and backend for their owned implementation plans where relevant.
4. Ask qa for test strategy before declaring completion.
5. Return to architect and qa for final gates when multiple modules changed.

Mode skip rule:
- Interaction may be skipped only when no user-facing flow, accessibility, or feedback behavior changes.
EOF
      ;;
    *)
      cat <<'EOF'
Mode-specific context: custom

Role ownership:
- each member -> explicitly assigned scoped responsibility
- leader -> assign ownership, avoid duplicate/conflicting work, integrate results

Consultation order:
1. Name each available member and assign a scoped responsibility before implementation.
2. Ask at least one member for implementation or analysis and at least one member for review/verification when there are two or more members.
3. Reassign or take over only after stating why the original owner is skipped.

Mode skip rule:
- Do not refer to generic members by number only; attach each member number to a concrete responsibility.
EOF
      ;;
  esac
}

configure_members() {
  local requested_mode="$1"
  local requested_count="$2"
  local name title
  local i

  MEMBER_NAMES=()
  MEMBER_ROLE_TITLES=()
  MEMBER_ROLE_PROMPTS=()

  if [[ -n "$requested_mode" ]]; then
    WORKGROUP_MODE="$requested_mode"
    while IFS= read -r name; do
      [[ -n "$name" ]] || continue
      MEMBER_NAMES+=("$name")
    done < <(mode_names "$requested_mode")
    MEMBER_COUNT="${#MEMBER_NAMES[@]}"
  elif [[ -n "$requested_count" ]]; then
    resolve_member_count "$requested_count"
    WORKGROUP_MODE="custom"
    for ((i = 1; i <= MEMBER_COUNT; i += 1)); do
      MEMBER_NAMES+=("member$i")
    done
  else
    WORKGROUP_MODE="simple"
    while IFS= read -r name; do
      [[ -n "$name" ]] || continue
      MEMBER_NAMES+=("$name")
    done < <(mode_names "$WORKGROUP_MODE")
    MEMBER_COUNT="${#MEMBER_NAMES[@]}"
  fi

  for name in "${MEMBER_NAMES[@]}"; do
    title="$(role_title "$name")"
    MEMBER_ROLE_TITLES+=("$title")
    MEMBER_ROLE_PROMPTS+=("$(role_prompt "$name" "$title")")
  done
}

cleanup() {
  local session="${1:-}"
  local sessions=()
  local selected
  local i

  install -d -m 700 "$SOCKET_DIR"
  if [[ -n "$session" ]]; then
    rmux_demo kill-session -t "$session" >/dev/null 2>&1 || true
    if ! rmux_demo list-sessions >/dev/null 2>&1; then
      rmux_demo kill-server >/dev/null 2>&1 || true
    fi
  else
    while IFS= read -r selected; do
      sessions+=("$selected")
    done < <(rmux_demo list-sessions -F '#{session_name}' 2>/dev/null || true)

    case "${#sessions[@]}" in
      0)
        echo "no rmux sessions found on socket: $SOCKET" >&2
        exit 1
        ;;
      1)
        cleanup "${sessions[0]}"
        return
        ;;
    esac

    if [[ ! -t 0 ]]; then
      echo "multiple rmux sessions found; pass one explicitly:" >&2
      for selected in "${sessions[@]}"; do
        echo "  ./launch.sh cleanup $selected" >&2
      done
      exit 1
    fi

    echo "Select rmux session to clean up:"
    for i in "${!sessions[@]}"; do
      printf "  %d) %s\n" "$((i + 1))" "${sessions[$i]}"
    done

    while true; do
      printf "session number: "
      read -r selected
      if [[ "$selected" =~ ^[0-9]+$ ]] && (( selected >= 1 && selected <= ${#sessions[@]} )); then
        cleanup "${sessions[$((selected - 1))]}"
        return
      fi
      echo "invalid selection: $selected" >&2
    done
  fi
}

list_sessions() {
  install -d -m 700 "$SOCKET_DIR"
  if ! rmux_demo list-sessions -F '#{session_name}' 2>/dev/null; then
    echo "no rmux sessions found on socket: $SOCKET" >&2
    exit 1
  fi
}

attach() {
  local requested="${1:-}"
  local sessions=()
  local selected
  local i

  if [[ -n "$requested" ]]; then
    exec rmux -S "$SOCKET" attach-session -t "$requested"
  fi

  while IFS= read -r selected; do
    sessions+=("$selected")
  done < <(rmux_demo list-sessions -F '#{session_name}' 2>/dev/null || true)
  case "${#sessions[@]}" in
    0)
      echo "no rmux sessions found on socket: $SOCKET" >&2
      exit 1
      ;;
    1)
      exec rmux -S "$SOCKET" attach-session -t "${sessions[0]}"
      ;;
  esac

  if [[ ! -t 0 ]]; then
    echo "multiple rmux sessions found; pass one explicitly:" >&2
    for selected in "${sessions[@]}"; do
      echo "  ./launch.sh attach $selected" >&2
    done
    exit 1
  fi

  echo "Select rmux session to attach:"
  for i in "${!sessions[@]}"; do
    printf "  %d) %s\n" "$((i + 1))" "${sessions[$i]}"
  done

  while true; do
    printf "session number: "
    read -r selected
    if [[ "$selected" =~ ^[0-9]+$ ]] && (( selected >= 1 && selected <= ${#sessions[@]} )); then
      exec rmux -S "$SOCKET" attach-session -t "${sessions[$((selected - 1))]}"
    fi
    echo "invalid selection: $selected" >&2
  done
}

session_exists() {
  rmux_demo has-session -t "$1" >/dev/null 2>&1
}

next_session_name() {
  local base="$1"
  local candidate="$base"
  local n=2

  while session_exists "$candidate"; do
    candidate="$base-$n"
    ((n += 1))
  done

  printf "%s" "$candidate"
}

configure_pane_titles() {
  local leader_target="$1"
  local target="$SESSION:$WINDOW"
  local role_expr=""
  local border_format
  local role_label
  local state_marker
  local role_badge
  local leader_badge
  local i

  for i in "${!member_targets[@]}"; do
    role_label="${MEMBER_NAMES[$i]}"
    state_marker=""
    if [[ -n "$ALERT_DIR" && -f "$(alert_state_path "$role_label")" ]]; then
      state_marker="$(<"$(alert_state_path "$role_label")")"
      state_marker="${state_marker:-!}"
    fi
    role_badge="$(role_badge_format "$role_label" "$role_label" "$state_marker")"
    role_expr+="#{?#{==:#{pane_id},${member_targets[$i]}},$role_badge,"
  done
  leader_badge="$(role_badge_format "leader" "leader" "")"
  role_expr+="#{?#{==:#{pane_id},$leader_target},$leader_badge,#[fg=colour244] pane #[default]}"
  for i in "${!member_targets[@]}"; do
    role_expr+="}"
  done

  border_format="#[align=left]#{?pane_active,#[bold]#[fg=black]#[bg=colour220] > #[default] , }$role_expr"

  rmux_demo set-option -w -t "$target" pane-active-border-style "$THEME_ACTIVE_BORDER" >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" pane-border-style "$THEME_INACTIVE_BORDER" >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" pane-border-lines heavy >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" pane-border-status top >/dev/null
  rmux_demo set-option -w -t "$target" pane-border-format "$border_format" >/dev/null
}

configure_status_line() {
  local target="$SESSION:$WINDOW"
  local roster=""
  local alerts_label
  local workdir_label
  local status_left
  local status_right
  local i

  for i in "${!MEMBER_NAMES[@]}"; do
    if [[ -n "$roster" ]]; then
      roster+=","
    fi
    roster+="${MEMBER_NAMES[$i]}"
  done

  alerts_label="${ALERT_LOG:-none}"
  workdir_label="$WORKDIR"
  status_left="#[bold]#[fg=black]#[bg=colour220] RMUX #[default]#[fg=colour220]#[bg=colour236] mode:$WORKGROUP_MODE #[default]#[fg=colour45]#[bg=colour236] members:$MEMBER_COUNT #[default]#[fg=colour250]#[bg=colour236] roster:$roster "
  status_right="#[bold]#[fg=black]#[bg=colour46] orchestrator-only #[default]#[fg=colour220]#[bg=colour236] alerts:$alerts_label #[default]#[fg=colour250]#[bg=colour236] workdir:$workdir_label "

  rmux_demo set-option -w -t "$target" status on >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-interval 0 >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-left-length 160 >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-right-length 260 >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" automatic-rename off >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" allow-rename off >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-style "$THEME_STATUS_STYLE" >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-left-style "$THEME_STATUS_LEFT_STYLE" >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-right-style "$THEME_STATUS_RIGHT_STYLE" >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-left "$status_left" >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" status-right "$status_right" >/dev/null 2>&1 || true
}

leader_command() {
  local member_targets="$1"
  local member_roles="$2"
  local socket_q session_q mode_q member1_q member2_q members_q roles_q demo_dir_q workdir_q alert_dir_q alert_log_q leader_prompt leader_prompt_q leader_contract leader_workflow leader_cmd

  socket_q="$(q "$SOCKET")"
  session_q="$(q "$SESSION")"
  mode_q="$(q "$WORKGROUP_MODE")"
  member1_q="$(q "$3")"
  member2_q="$(q "$4")"
  members_q="$(q "$member_targets")"
  roles_q="$(q "$member_roles")"
  demo_dir_q="$(q "$DEMO_DIR")"
  workdir_q="$(q "$WORKDIR")"
  alert_dir_q="$(q "$ALERT_DIR")"
  alert_log_q="$(q "$ALERT_LOG")"
  leader_contract="$(leader_operating_contract)"
  leader_workflow="$(leader_workflow_prompt "$WORKGROUP_MODE")"
  printf -v leader_prompt "%s\n\n%s\n\nRmux operation appendix from %s/AGENTS.md:\n\n%s\n\nPassive member alerts:\n- Member approval/confirmation alerts are written to \$RMUX_ALERT_LOG.\n- When waiting for members, run ./launch.sh alerts \"\$RMUX_WORKGROUP_SESSION\" or inspect \$RMUX_ALERT_LOG before assuming they are still working.\n- Alert logs are scoped per rmux session under \$RMUX_ALERT_DIR." \
    "$leader_contract" "$leader_workflow" "$DEMO_DIR" "$(<"$DEMO_DIR/AGENTS.md")"
  leader_prompt_q="$(q "$leader_prompt")"

  if [[ -n "${CODEX_CMD:-}" ]]; then
    leader_cmd="$CODEX_CMD"
  else
    leader_cmd="codex --dangerously-bypass-approvals-and-sandbox -C $workdir_q --add-dir $demo_dir_q $leader_prompt_q"
  fi

  printf "cd %s; export RMUX_DEMO_SOCKET=%s; export RMUX_WORKGROUP_SESSION=%s; export RMUX_WORKGROUP_MODE=%s; export RMUX_MEMBER_COUNT=%s; export RMUX_MEMBER_TARGETS=%s; export RMUX_MEMBER_ROLES=%s; export RMUX_MEMBER1_TARGET=%s; export RMUX_MEMBER2_TARGET=%s; export RMUX_ALERT_DIR=%s; export RMUX_ALERT_LOG=%s; export RMUX_WORKDIR=%s; exec env IS_DEMO=1 %s" \
    "$demo_dir_q" "$socket_q" "$session_q" "$mode_q" "$MEMBER_COUNT" "$members_q" "$roles_q" "$member1_q" "$member2_q" "$alert_dir_q" "$alert_log_q" "$workdir_q" "$leader_cmd"
}

member_command() {
  local member_cmd="$1"
  local prompt="${2:-}"
  local workdir_q prompt_q inner_script inner_script_q

  member_cmd="$(member_command_with_auto_mode "$member_cmd")"
  workdir_q="$(q "$WORKDIR")"
  if [[ -n "$prompt" ]]; then
    prompt_q="$(q "$prompt")"
    printf -v inner_script "cd %s; export RMUX_WORKDIR=%s; %s %s" "$workdir_q" "$workdir_q" "$member_cmd" "$prompt_q"
  else
    printf -v inner_script "cd %s; export RMUX_WORKDIR=%s; %s" "$workdir_q" "$workdir_q" "$member_cmd"
  fi
  inner_script_q="$(q "$inner_script")"
  printf "exec bash -lic %s" "$inner_script_q"
}

set_member_title() {
  local index="$1"
  local target="$2"
  local agent

  agent="$(agent_name_from_command "$MEMBER_CMD")"
  rmux_demo select-pane -t "$target" -T "${MEMBER_NAMES[$index]}: $agent" >/dev/null 2>&1 || true
}

alert_line_from_capture() {
  local line
  local lines=()
  local alert_regex='(Do you want to proceed\?|Would you like to .*\?|Allow .*\?|Approve .*\?|approval required|permission required|Permission required|Press Enter to continue|\[[yY]/[nN]\]|\[[yY]/N\]|\([yY]/[nN]\)|是否继续[？?]|是否允许[？?]|是否授权[？?]|继续吗[？?]?|^[[:space:]]*(BLOCKED|ERROR|FAILED|EXCEPTION|DONE|COMPLETED):|^[[:space:]]*(阻塞|报错|失败|完成|已完成)：)'

  while IFS= read -r line; do
    lines+=("$line")
    if (( ${#lines[@]} > 25 )); then
      lines=("${lines[@]:1}")
    fi
  done

  for line in "${lines[@]}"; do
    if [[ "$line" =~ $alert_regex ]]; then
      printf "%s" "$line"
      return 0
    fi
  done
  return 1
}

alert_marker_from_line() {
  local line="$1"

  if [[ "$line" =~ ^[[:space:]]*(BLOCKED|ERROR|FAILED|EXCEPTION): || "$line" =~ ^[[:space:]]*(阻塞|报错|失败)： ]]; then
    printf "x"
  elif [[ "$line" =~ ^[[:space:]]*(DONE|COMPLETED): || "$line" =~ ^[[:space:]]*(完成|已完成)： ]]; then
    printf "ok"
  elif [[ "$line" =~ (Would\ you\ like\ to|是否继续|继续吗) ]]; then
    printf "?"
  else
    printf "!"
  fi
}

active_alert_names() {
  local names=()
  local name
  local marker

  for name in "${MEMBER_NAMES[@]}"; do
    if [[ -f "$(alert_state_path "$name")" ]]; then
      marker="$(<"$(alert_state_path "$name")")"
      marker="${marker:-!}"
      names+=("$marker $name")
    fi
  done

  local IFS=","
  printf "%s" "${names[*]}"
}

refresh_alert_visuals() {
  local leader_target="$1"
  local active_names
  local now last_path last

  configure_pane_titles "$leader_target"
  active_names="$(active_alert_names)"
  if [[ "$ALERT_MESSAGE" == "1" && -n "$active_names" ]]; then
    now="$(date '+%s')"
    last_path="$ALERT_DIR/display-message.last"
    last=0
    if [[ -f "$last_path" ]]; then
      last="$(<"$last_path")"
    fi
    if (( now - last >= ALERT_COOLDOWN )); then
      printf '%s\n' "$now" >"$last_path"
      rmux_demo display-message -t "$SESSION:$WINDOW" "ALERT: $active_names" >/dev/null 2>&1 || true
    fi
  fi
}

start_alert_watcher() {
  local leader_target="$1"
  local watch_names=("${MEMBER_NAMES[@]}")
  local watch_targets=("${member_targets[@]}")

  if [[ "$ALERT_WATCH" == "0" ]]; then
    return
  fi

  (
    MEMBER_NAMES=("${watch_names[@]}")
    member_targets=("${watch_targets[@]}")

    while rmux_demo has-session -t "$SESSION" >/dev/null 2>&1; do
      local changed=0
      local i name target capture line state_path timestamp

      for i in "${!member_targets[@]}"; do
        name="${MEMBER_NAMES[$i]}"
        target="${member_targets[$i]}"
        state_path="$(alert_state_path "$name")"
        capture="$(rmux_demo capture-pane -p -t "$target" -S -80 2>/dev/null || true)"

        if line="$(alert_line_from_capture <<<"$capture")"; then
          local marker
          marker="$(alert_marker_from_line "$line")"
          if [[ ! -f "$state_path" ]]; then
            timestamp="$(date '+%Y-%m-%dT%H:%M:%S%z')"
            printf '%s\t%s\t%s\t%s\t%s\n' "$timestamp" "$marker" "$name" "$target" "$line" >>"$ALERT_LOG"
            printf '%s\n' "$marker" >"$state_path"
            changed=1
          elif [[ "$(<"$state_path")" != "$marker" ]]; then
            printf '%s\n' "$marker" >"$state_path"
            changed=1
          fi
        elif [[ -f "$state_path" ]]; then
          rm -f "$state_path"
          changed=1
        fi
      done

      if (( changed )); then
        refresh_alert_visuals "$leader_target"
      fi
      sleep "$ALERT_INTERVAL"
    done
  ) >/dev/null 2>&1 &

  printf '%s\n' "$!" >"$ALERT_DIR/watcher.pid"
}

split_member_pane() {
  local index="$1"
  local target="$2"
  local direction="$3"
  local percent="$4"

  CREATED_MEMBER_TARGET="$(rmux_demo split-window "$direction" -P -F '#{pane_id}' -t "$target" -p "$percent" "$(member_command "$MEMBER_CMD" "${MEMBER_ROLE_PROMPTS[$index]}")")"
  member_targets+=("$CREATED_MEMBER_TARGET")
  set_member_title "$index" "$CREATED_MEMBER_TARGET"
}

create_member_panes() {
  local top_row_targets=("$member1_target")
  local columns
  local remaining_columns
  local percent
  local split_target
  local i
  local column

  if (( MEMBER_COUNT <= 3 )); then
    for ((i = 2; i <= MEMBER_COUNT; i += 1)); do
      split_member_pane "$((i - 1))" "$member1_target" -h 50
    done
    return
  fi

  columns=$(((MEMBER_COUNT + 1) / 2))
  split_target="$member1_target"
  for ((i = 2; i <= columns; i += 1)); do
    remaining_columns=$((columns - i + 2))
    percent=$((100 * (remaining_columns - 1) / remaining_columns))
    split_member_pane "$((i - 1))" "$split_target" -h "$percent"
    split_target="$CREATED_MEMBER_TARGET"
    top_row_targets+=("$CREATED_MEMBER_TARGET")
  done

  column=0
  for ((i = columns + 1; i <= MEMBER_COUNT; i += 1)); do
    split_member_pane "$((i - 1))" "${top_row_targets[$column]}" -v 50
    column=$((column + 1))
  done
}

launch() {
  local member_targets=()
  local member_targets_string=""
  local member_roles_string=""
  local member1_target leader_target member_target
  local leader_percent
  local requested_workdir="$WORKDIR_INPUT"
  local requested_member_count="$MEMBER_COUNT_INPUT"
  local requested_mode="$WORKGROUP_MODE_INPUT"
  local workdir_seen=0
  local count_seen=0
  local arg name
  local i

  while (( $# > 0 )); do
    arg="$1"
    case "$arg" in
      --help|-h)
        print_help
        return
        ;;
      --mode)
        shift || {
          echo "--mode requires a value" >&2
          exit 2
        }
        requested_mode="${1:-}"
        if [[ -z "$requested_mode" ]]; then
          echo "--mode requires a value" >&2
          exit 2
        fi
        ;;
      --mode=*)
        requested_mode="${arg#--mode=}"
        if [[ -z "$requested_mode" ]]; then
          echo "--mode requires a value" >&2
          exit 2
        fi
        ;;
      *)
        if [[ "$arg" =~ ^[1-9][0-9]*$ ]]; then
          if (( count_seen )); then
            usage
            exit 2
          fi
          requested_member_count="$arg"
          count_seen=1
        else
          if (( workdir_seen )); then
            usage
            exit 2
          fi
          requested_workdir="$arg"
          workdir_seen=1
        fi
        ;;
    esac
    shift || true
  done

  resolve_workdir "$requested_workdir"
  configure_members "$requested_mode" "$requested_member_count"
  check
  apply_theme
  install -d -m 700 "$SOCKET_DIR"
  SESSION="$(next_session_name "$SESSION_BASE")"
  init_alert_log

  member1_target="$(rmux_demo new-session -d -P -F '#{pane_id}' -s "$SESSION" -n "$WINDOW" -x 160 -y 42 "$(member_command "$MEMBER_CMD" "${MEMBER_ROLE_PROMPTS[0]}")")"
  member_targets+=("$member1_target")
  set_member_title 0 "$member1_target"

  leader_percent=50
  if (( MEMBER_COUNT > 3 )); then
    leader_percent=38
  fi
  leader_target="$(rmux_demo split-window -v -P -F '#{pane_id}' -t "$member1_target" -p "$leader_percent" "sleep 3600")"

  create_member_panes

  for i in "${!member_targets[@]}"; do
    if [[ -n "$member_targets_string" ]]; then
      member_targets_string+=" "
    fi
    if [[ -n "$member_roles_string" ]]; then
      member_roles_string+=","
    fi
    name="${MEMBER_NAMES[$i]}"
    member_targets_string+="$name=${member_targets[$i]}"
    member_roles_string+="$name=${MEMBER_ROLE_TITLES[$i]}"
  done

  rmux_demo respawn-pane -k -t "$leader_target" "$(leader_command "$member_targets_string" "$member_roles_string" "${member_targets[0]:-}" "${member_targets[1]:-}")"
  rmux_demo select-pane -t "$leader_target" -T "leader: codex" >/dev/null 2>&1 || true
  configure_pane_titles "$leader_target"
  configure_status_line
  start_alert_watcher "$leader_target"
  rmux_demo select-pane -t "$leader_target"

  echo "workgroup started"
  echo "socket: $SOCKET"
  echo "session: $SESSION"
  echo "workdir: $WORKDIR"
  echo "mode: $WORKGROUP_MODE"
  echo "members: $MEMBER_COUNT"
  echo "theme: $RMUX_THEME"
  echo "alerts: $ALERT_LOG"
  echo "guard: leader orchestrates only; members own code/config changes"
  for i in "${!member_targets[@]}"; do
    echo "${MEMBER_NAMES[$i]}: ${member_targets[$i]}"
  done
  echo "leader:  $leader_target"
  echo
  echo "alerts: ./launch.sh alerts $SESSION"
  echo "follow alerts: ./launch.sh alerts $SESSION --follow"
  echo "detach: Ctrl-b then d"
  echo "reattach: ./launch.sh attach $SESSION"
  echo "attaching to rmux session..."
  attach "$SESSION"
}

COMMAND="${1:-launch}"
case "$COMMAND" in
  launch) shift || true; launch "$@" ;;
  check) check ;;
  help|-h|--help) print_help ;;
  list) list_sessions ;;
  alerts) alerts "${2:-}" "${3:-}" ;;
  cleanup) cleanup "${2:-}" ;;
  attach) attach "${2:-}" ;;
  /*|.|..|./*|../*) launch "$@" ;;
  *)
    if [[ "$COMMAND" =~ ^[1-9][0-9]*$ ]]; then
      launch "$@"
    else
      usage
      exit 2
    fi
    ;;
esac
