#!/usr/bin/env bash
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCKET_DIR="${TMPDIR:-/tmp}/rmux-demo-orchestration-${USER:-user}"
SOCKET="${RMUX_DEMO_SOCKET:-$SOCKET_DIR/socket}"

CODEX_CMD="${CODEX_CMD:-codex --dangerously-bypass-approvals-and-sandbox}"
GEMINI_CMD="${GEMINI_CMD:-gemini --skip-trust --approval-mode yolo}"
GROK_CMD="${GROK_CMD:-grok --always-approve}"
CLAUDE_CMD="${CLAUDE_CMD:-claude --dangerously-skip-permissions --permission-mode bypassPermissions}"

CODEX_GEOMETRY="${CODEX_GEOMETRY:-96x26+10+40}"
GEMINI_GEOMETRY="${GEMINI_GEOMETRY:-96x26+960+40}"
GROK_GEOMETRY="${GROK_GEOMETRY:-96x26+10+560}"
CLAUDE_GEOMETRY="${CLAUDE_GEOMETRY:-96x26+960+560}"

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

cleanup() {
  install -d -m 700 "$SOCKET_DIR"
  for session in codex gemini grok; do
    rmux_demo kill-session -t "$session" >/dev/null 2>&1 || true
  done
  rmux_demo kill-server >/dev/null 2>&1 || true
}

check() {
  need rmux
  need codex
  need gemini
  need grok
  need claude
  echo "rmux, claude, codex, gemini and grok are available"
}

open_terminal() {
  local title="$1"
  local command="$2"
  local geometry="$3"

  if command -v mate-terminal >/dev/null 2>&1; then
    mate-terminal --title "$title" --geometry "$geometry" -- bash -lc "$command"
  elif command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --title="$title" --geometry="$geometry" -- bash -lc "$command"
  elif command -v xfce4-terminal >/dev/null 2>&1; then
    xfce4-terminal --title="$title" --geometry="$geometry" --command="bash -lc $(q "$command")"
  elif command -v konsole >/dev/null 2>&1; then
    konsole --new-tab --title "$title" --geometry "$geometry" -e bash -lc "$command"
  elif command -v xterm >/dev/null 2>&1; then
    xterm -T "$title" -geometry "$geometry" -e bash -lc "$command" &
  else
    echo "No supported terminal emulator found. Run manually:"
    echo "$command"
  fi
}

new_agent_session() {
  local session="$1"
  local title="$2"
  local command="$3"

  rmux_demo new-session -d -s "$session" -n "$title" -x 120 -y 34 "$command"
  rmux_demo select-pane -t "$session:0.0" -T "$title" >/dev/null 2>&1 || true
}

launch() {
  check
  install -d -m 700 "$SOCKET_DIR"
  cleanup

  new_agent_session codex Codex "$CODEX_CMD"
  new_agent_session gemini Gemini "$GEMINI_CMD"
  new_agent_session grok Grok "$GROK_CMD"

  local socket_q demo_dir_q
  socket_q="$(q "$SOCKET")"
  demo_dir_q="$(q "$DEMO_DIR")"

  open_terminal "Codex agent" "export RMUX_DEMO_SOCKET=$socket_q; exec rmux -S $socket_q attach-session -t codex" "$CODEX_GEOMETRY"
  open_terminal "Gemini agent" "export RMUX_DEMO_SOCKET=$socket_q; exec rmux -S $socket_q attach-session -t gemini" "$GEMINI_GEOMETRY"
  open_terminal "Grok agent" "export RMUX_DEMO_SOCKET=$socket_q; exec rmux -S $socket_q attach-session -t grok" "$GROK_GEOMETRY"
  sleep 1
  open_terminal "Claude orchestrator" "cd $demo_dir_q; export RMUX_DEMO_SOCKET=$socket_q; export RMUX_DEMO_TARGETS='codex:0.0 gemini:0.0 grok:0.0'; exec $CLAUDE_CMD" "$CLAUDE_GEOMETRY"

  echo "demo started"
  echo "socket: $SOCKET"
  echo "try in Claude: Send Hi to all agents"
}

case "${1:-launch}" in
  launch) launch ;;
  check) check ;;
  cleanup) cleanup ;;
  *)
    echo "usage: ./launch.sh [launch|check|cleanup]" >&2
    exit 2
    ;;
esac
