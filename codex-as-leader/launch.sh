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

DEFAULT_CLAUDE_CMD="dclaude"
MEMBER_CMD="${CLAUDE_CMD:-$DEFAULT_CLAUDE_CMD}"

q() {
  printf "%q" "$1"
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
  ./launch.sh check|list|attach [SESSION]|cleanup [SESSION]|help

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
EOF
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
- Keep responses concise, actionable, and grounded in the project at \$RMUX_WORKDIR.
- Do not take rmux control actions unless the leader explicitly asks.
- If you are blocked, state the missing input or evidence needed.
EOF
}

leader_workflow_prompt() {
  case "$1" in
    simple)
      cat <<'EOF'
Mode-specific leader workflow: simple

Workflow graph:
leader -> developer: implement
developer -> leader: patch/result
leader -> reviewer: review/test
reviewer -> leader: findings
leader -> developer: fix if needed
leader -> final: summarize

Rules:
- Leader coordinates, inspects, and integrates.
- Developer owns implementation; reviewer owns validation.
- Leader must not do the whole task alone unless the work is a tiny control-plane fix.
EOF
      ;;
    classic)
      cat <<'EOF'
Mode-specific leader workflow: classic

Workflow graph:
leader -> architect: boundaries/risks
leader -> frontend: UI/client plan
leader -> backend: API/data plan
architect -> leader: integration constraints
frontend/backend -> leader: implementation notes
leader -> architect: final coherence review
leader -> final: integrated decision

Rules:
- Leader must delegate frontend/backend/architecture work before implementing cross-cutting changes.
- Frontend owns client-facing behavior; backend owns API/data/runtime behavior; architect owns boundaries and risk.
- Leader integrates role outputs and may only do direct edits after ownership is clear.
EOF
      ;;
    product)
      cat <<'EOF'
Mode-specific leader workflow: product

Workflow graph:
leader -> product: scope/acceptance criteria
leader -> designer: UX/flow
leader -> developer: implementation plan
leader -> qa: test matrix
product/designer/developer/qa -> leader: role outputs
leader -> developer: execute scoped changes
leader -> qa: verify
leader -> final: release summary

Rules:
- Product defines what, designer defines experience, developer builds, qa validates.
- Leader must not collapse product, design, implementation, and QA ownership into one role.
- Leader integrates decisions and reports which roles participated.
EOF
      ;;
    advanced)
      cat <<'EOF'
Mode-specific leader workflow: advanced

Workflow graph:
leader -> architect: architecture and sequencing
leader -> interaction: flows/accessibility
leader -> frontend: UI/state implementation
leader -> backend: API/data/runtime implementation
leader -> qa: test strategy
all roles -> leader: constraints/results
leader -> qa + architect: final gates
leader -> final: decision and residual risk

Rules:
- Leader must gather role-specific input before major implementation and before declaring completion.
- Interaction owns flows/accessibility; frontend owns UI/state; backend owns API/data/runtime; architect and qa own final gates.
- Leader integrates, resolves conflicts, and records residual risk.
EOF
      ;;
    *)
      cat <<'EOF'
Mode-specific leader workflow: custom

Workflow graph:
leader -> each member: assign scoped responsibility
members -> leader: result/risk
leader -> selected member(s): implementation/fix
leader -> selected member(s): review/verify
leader -> final: summarize

Rules:
- Leader must name ownership explicitly before asking for work.
- Leader must avoid doing all implementation directly when member panes are available.
- Final summary should state which members contributed and what they owned.
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
  local i

  for i in "${!member_targets[@]}"; do
    role_expr+="#{?#{==:#{pane_id},${member_targets[$i]}},${MEMBER_NAMES[$i]},"
  done
  role_expr+="#{?#{==:#{pane_id},$leader_target},leader,pane}"
  for i in "${!member_targets[@]}"; do
    role_expr+="}"
  done

  border_format="#[align=left]#{?pane_active,#[bold,fg=black,bg=yellow],#[bold,fg=colour250,bg=colour238]} $role_expr #[default]"

  rmux_demo set-option -w -t "$target" pane-active-border-style fg=yellow >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" pane-border-style fg=colour238 >/dev/null 2>&1 || true
  rmux_demo set-option -w -t "$target" pane-border-status top >/dev/null
  rmux_demo set-option -w -t "$target" pane-border-format "$border_format" >/dev/null
}

leader_command() {
  local member_targets="$1"
  local member_roles="$2"
  local socket_q session_q mode_q member1_q member2_q members_q roles_q demo_dir_q workdir_q leader_prompt leader_prompt_q leader_workflow leader_cmd

  socket_q="$(q "$SOCKET")"
  session_q="$(q "$SESSION")"
  mode_q="$(q "$WORKGROUP_MODE")"
  member1_q="$(q "$3")"
  member2_q="$(q "$4")"
  members_q="$(q "$member_targets")"
  roles_q="$(q "$member_roles")"
  demo_dir_q="$(q "$DEMO_DIR")"
  workdir_q="$(q "$WORKDIR")"
  leader_workflow="$(leader_workflow_prompt "$WORKGROUP_MODE")"
  printf -v leader_prompt "Use these additional leader workgroup instructions from %s/AGENTS.md:\n\n%s\n\n%s" \
    "$DEMO_DIR" "$(<"$DEMO_DIR/AGENTS.md")" "$leader_workflow"
  leader_prompt_q="$(q "$leader_prompt")"

  if [[ -n "${CODEX_CMD:-}" ]]; then
    leader_cmd="$CODEX_CMD"
  else
    leader_cmd="codex --dangerously-bypass-approvals-and-sandbox -C $workdir_q --add-dir $demo_dir_q $leader_prompt_q"
  fi

  printf "cd %s; export RMUX_DEMO_SOCKET=%s; export RMUX_WORKGROUP_SESSION=%s; export RMUX_WORKGROUP_MODE=%s; export RMUX_MEMBER_COUNT=%s; export RMUX_MEMBER_TARGETS=%s; export RMUX_MEMBER_ROLES=%s; export RMUX_MEMBER1_TARGET=%s; export RMUX_MEMBER2_TARGET=%s; export RMUX_WORKDIR=%s; exec env IS_DEMO=1 %s" \
    "$demo_dir_q" "$socket_q" "$session_q" "$mode_q" "$MEMBER_COUNT" "$members_q" "$roles_q" "$member1_q" "$member2_q" "$workdir_q" "$leader_cmd"
}

member_command() {
  local member_cmd="$1"
  local prompt="${2:-}"
  local workdir_q prompt_q inner_script inner_script_q

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

  rmux_demo select-pane -t "$target" -T "${MEMBER_NAMES[$index]}: claude" >/dev/null 2>&1 || true
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
  install -d -m 700 "$SOCKET_DIR"
  SESSION="$(next_session_name "$SESSION_BASE")"

  member1_target="$(rmux_demo new-session -d -P -F '#{pane_id}' -s "$SESSION" -n "$WINDOW" -x 160 -y 42 "$(member_command "$MEMBER_CMD" "${MEMBER_ROLE_PROMPTS[0]}")")"
  member_targets+=("$member1_target")
  set_member_title 0 "$member1_target"

  leader_target="$(rmux_demo split-window -v -P -F '#{pane_id}' -t "$member1_target" -p 50 "sleep 3600")"

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
  rmux_demo select-pane -t "$leader_target"

  echo "workgroup started"
  echo "socket: $SOCKET"
  echo "session: $SESSION"
  echo "workdir: $WORKDIR"
  echo "mode: $WORKGROUP_MODE"
  echo "members: $MEMBER_COUNT"
  for i in "${!member_targets[@]}"; do
    echo "${MEMBER_NAMES[$i]}: ${member_targets[$i]}"
  done
  echo "leader:  $leader_target"
  echo
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
