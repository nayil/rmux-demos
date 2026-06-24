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

MEMBER1_CMD="${CLAUDE_CMD:-claude --dangerously-skip-permissions --permission-mode bypassPermissions}"
MEMBER2_CMD="${CLAUDE_CMD:-claude --dangerously-skip-permissions --permission-mode bypassPermissions}"

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

need_zsh_command() {
  local name="$1"
  local name_q

  name_q="$(q "$name")"
  zsh -ic "type $name_q" >/dev/null 2>&1 || {
    echo "missing zsh command/function/alias: $name" >&2
    exit 1
  }
}

check() {
  need rmux
  need codex
  need claude
  need zsh
  echo "rmux, codex, claude and zsh are available"
}

resolve_workdir() {
  local requested="${1:-$WORKDIR_INPUT}"

  if [[ ! -d "$requested" ]]; then
    echo "workdir does not exist: $requested" >&2
    exit 1
  fi

  WORKDIR="$(cd "$requested" && pwd)"
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

leader_command() {
  local member1_target="$1"
  local member2_target="$2"
  local socket_q session_q member1_q member2_q demo_dir_q workdir_q leader_cmd

  socket_q="$(q "$SOCKET")"
  session_q="$(q "$SESSION")"
  member1_q="$(q "$member1_target")"
  member2_q="$(q "$member2_target")"
  demo_dir_q="$(q "$DEMO_DIR")"
  workdir_q="$(q "$WORKDIR")"

  if [[ -n "${CODEX_CMD:-}" ]]; then
    leader_cmd="$CODEX_CMD"
  else
    leader_cmd="codex --dangerously-bypass-approvals-and-sandbox --add-dir $workdir_q"
  fi

  printf "cd %s; export RMUX_DEMO_SOCKET=%s; export RMUX_WORKGROUP_SESSION=%s; export RMUX_MEMBER1_TARGET=%s; export RMUX_MEMBER2_TARGET=%s; export RMUX_WORKDIR=%s; exec env IS_DEMO=1 %s" \
    "$demo_dir_q" "$socket_q" "$session_q" "$member1_q" "$member2_q" "$workdir_q" "$leader_cmd"
}

member_command() {
  local member_cmd="$1"
  local workdir_q

  workdir_q="$(q "$WORKDIR")"
  printf "cd %s; export RMUX_WORKDIR=%s; exec %s" "$workdir_q" "$workdir_q" "$member_cmd"
}

launch() {
  local member1_target leader_target member2_target
  local requested_workdir="${1:-$WORKDIR_INPUT}"

  resolve_workdir "$requested_workdir"
  check
  install -d -m 700 "$SOCKET_DIR"
  SESSION="$(next_session_name "$SESSION_BASE")"

  member1_target="$(rmux_demo new-session -d -P -F '#{pane_id}' -s "$SESSION" -n "$WINDOW" -x 160 -y 42 "$(member_command "$MEMBER1_CMD")")"
  rmux_demo select-pane -t "$member1_target" -T "member1: claude" >/dev/null 2>&1 || true

  member2_target="$(rmux_demo split-window -h -P -F '#{pane_id}' -t "$member1_target" -p 50 "$(member_command "$MEMBER2_CMD")")"
  rmux_demo select-pane -t "$member2_target" -T "member2: claude" >/dev/null 2>&1 || true

  leader_target="$(rmux_demo split-window -v -P -F '#{pane_id}' -t "$member1_target" -p 50 "sleep 3600")"

  rmux_demo respawn-pane -k -t "$leader_target" "$(leader_command "$member1_target" "$member2_target")"
  rmux_demo select-pane -t "$leader_target" -T "leader: codex" >/dev/null 2>&1 || true
  rmux_demo select-pane -t "$leader_target"

  echo "workgroup started"
  echo "socket: $SOCKET"
  echo "session: $SESSION"
  echo "workdir: $WORKDIR"
  echo "member1: $member1_target"
  echo "member2: $member2_target"
  echo "leader:  $leader_target"
  echo
  echo "detach: Ctrl-b then d"
  echo "reattach: ./launch.sh attach $SESSION"
  echo "attaching to rmux session..."
  attach "$SESSION"
}

case "${1:-launch}" in
  launch) launch "${2:-}" ;;
  check) check ;;
  list) list_sessions ;;
  cleanup) cleanup "${2:-}" ;;
  attach) attach "${2:-}" ;;
  /*|.|..|./*|../*) launch "$1" ;;
  *)
    echo "usage: ./launch.sh [launch [WORKDIR]|attach [SESSION]|check|list|cleanup [SESSION]]" >&2
    exit 2
    ;;
esac
