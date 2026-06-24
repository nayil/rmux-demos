#!/usr/bin/env bash
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNAME_S="$(uname -s)"
SOCKET_DIR="${TMPDIR:-/tmp}/rmux-demo-orchestration-${USER:-user}"
if [[ "$UNAME_S" == MINGW* || "$UNAME_S" == MSYS* || "$UNAME_S" == CYGWIN* ]]; then
  DEFAULT_SOCKET='\\.\pipe\rmux-demo-orchestration'
else
  DEFAULT_SOCKET="$SOCKET_DIR/socket"
fi
SOCKET="${RMUX_DEMO_SOCKET:-$DEFAULT_SOCKET}"

CODEX_CMD="${CODEX_CMD:-codex --dangerously-bypass-approvals-and-sandbox}"
GEMINI_CMD="${GEMINI_CMD:-dclaude}"
GROK_CMD="${GROK_CMD:-reasonix chat}"
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
  rm -f "$SOCKET_DIR"/rmux-demo-*.command
  rm -f "$SOCKET_DIR"/rmux-demo-*.ps1
}

check() {
  need rmux
  need codex
  need claude
  echo "rmux, claude, codex  are available"
}

open_terminal() {
  local title="$1"
  local command="$2"
  local geometry="$3"

  if [[ "$UNAME_S" == "Darwin" ]] && command -v open >/dev/null 2>&1 && command -v osascript >/dev/null 2>&1; then
    open_macos_terminal "$title" "$command" "$geometry"
  elif is_windows; then
    open_windows_terminal "$title" "$command" "$geometry"
  elif command -v mate-terminal >/dev/null 2>&1; then
    mate-terminal --title "$title" --geometry "$geometry" -- bash -lc "$command"
  elif command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --title="$title" --geometry="$geometry" -- bash -lc "$command"
  elif command -v xfce4-terminal >/dev/null 2>&1; then
    xfce4-terminal --title="$title" --geometry="$geometry" --command="bash -lc $(q "$command")"
  elif command -v konsole >/dev/null 2>&1; then
    konsole --title "$title" --geometry "$geometry" -e bash -lc "$command" &
  elif command -v xterm >/dev/null 2>&1; then
    xterm -T "$title" -geometry "$geometry" -e bash -lc "$command" &
  else
    echo "No supported terminal emulator found. Run manually:"
    echo "$command"
  fi
}

is_windows() {
  case "$UNAME_S" in
    MINGW* | MSYS* | CYGWIN*) return 0 ;;
    *) return 1 ;;
  esac
}

open_macos_terminal() {
  local title="$1"
  local command="$2"
  local geometry="$3"
  local slug launcher title_q left top right bottom

  slug="$(printf "%s" "$title" | tr "[:upper:]" "[:lower:]" | tr " " "-" | tr -cd "[:alnum:]_-")"
  launcher="$SOCKET_DIR/rmux-demo-${slug:-terminal}.command"
  title_q="$(q "$title")"
  read -r left top right bottom < <(macos_terminal_bounds "$geometry")

  install -d -m 700 "$SOCKET_DIR"
  {
    printf "#!/usr/bin/env bash\n"
    printf "printf '\\\\033]0;%%s\\\\007' %s\n" "$title_q"
    printf "%s\n" "$command"
  } >"$launcher"
  chmod 700 "$launcher"

  open -n -a Terminal "$launcher"
  sleep 0.8
  osascript - "$left" "$top" "$right" "$bottom" <<'APPLESCRIPT'
on run argv
  set leftBound to item 1 of argv as integer
  set topBound to item 2 of argv as integer
  set rightBound to item 3 of argv as integer
  set bottomBound to item 4 of argv as integer

  tell application "Terminal"
    set bounds of front window to {leftBound, topBound, rightBound, bottomBound}
  end tell
end run
APPLESCRIPT
}

macos_terminal_bounds() {
  local geometry="$1"
  local cols rows left top width height

  if [[ "$geometry" =~ ^([0-9]+)x([0-9]+)\+([0-9]+)\+([0-9]+)$ ]]; then
    cols="${BASH_REMATCH[1]}"
    rows="${BASH_REMATCH[2]}"
    left="${BASH_REMATCH[3]}"
    top="${BASH_REMATCH[4]}"
  else
    cols=96
    rows=26
    left=80
    top=80
  fi

  width=$((cols * 8 + 36))
  height=$((rows * 17 + 70))
  printf "%s %s %s %s\n" "$left" "$top" "$((left + width))" "$((top + height))"
}

open_windows_terminal() {
  local title="$1"
  local command="$2"
  local geometry="$3"
  local cols rows left top bash_path

  bash_path="$(windows_bash_path)"
  read -r cols rows left top < <(windows_terminal_geometry "$geometry")

  if command -v wt.exe >/dev/null 2>&1; then
    MSYS2_ARG_CONV_EXCL='*' wt.exe -w new --pos "$left,$top" --size "$cols,$rows" --title "$title" "$bash_path" -lc "$command" >/dev/null 2>&1 &
    return 0
  fi

  open_windows_powershell_terminal "$title" "$command" "$geometry" "$bash_path"
}

windows_bash_path() {
  local bash_path

  bash_path="$(command -v bash)"
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$bash_path"
  else
    printf "%s" "$bash_path"
  fi
}

windows_terminal_geometry() {
  local geometry="$1"
  local cols rows left top

  if [[ "$geometry" =~ ^([0-9]+)x([0-9]+)\+([0-9]+)\+([0-9]+)$ ]]; then
    cols="${BASH_REMATCH[1]}"
    rows="${BASH_REMATCH[2]}"
    left="${BASH_REMATCH[3]}"
    top="${BASH_REMATCH[4]}"
  else
    cols=96
    rows=26
    left=80
    top=80
  fi

  printf "%s %s %s %s\n" "$cols" "$rows" "$left" "$top"
}

open_windows_powershell_terminal() {
  local title="$1"
  local command="$2"
  local geometry="$3"
  local bash_path="$4"
  local cols rows left top width height launcher

  read -r cols rows left top < <(windows_terminal_geometry "$geometry")
  width=$((cols * 8 + 36))
  height=$((rows * 17 + 70))
  launcher="$SOCKET_DIR/rmux-demo-open-window.ps1"

  install -d -m 700 "$SOCKET_DIR"
  cat >"$launcher" <<'POWERSHELL'
param(
  [string]$Title,
  [string]$BashPath,
  [string]$BashCommand,
  [int]$Left,
  [int]$Top,
  [int]$Width,
  [int]$Height
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class RmuxDemoWindow {
  [DllImport("user32.dll")]
  public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool bRepaint);
}
"@

$titleEscape = [char]27 + "]0;" + $Title + [char]7
$wrappedCommand = "printf '$titleEscape'; " + $BashCommand
$process = Start-Process -FilePath $BashPath -ArgumentList @("-lc", $wrappedCommand) -PassThru
for ($i = 0; $i -lt 40 -and $process.MainWindowHandle -eq 0; $i++) {
  Start-Sleep -Milliseconds 100
  $process.Refresh()
}
if ($process.MainWindowHandle -ne 0) {
  [RmuxDemoWindow]::MoveWindow($process.MainWindowHandle, $Left, $Top, $Width, $Height, $true) | Out-Null
}
POWERSHELL

  if command -v powershell.exe >/dev/null 2>&1; then
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$launcher" -Title "$title" -BashPath "$bash_path" -BashCommand "$command" -Left "$left" -Top "$top" -Width "$width" -Height "$height" >/dev/null 2>&1 &
  else
    echo "No supported Windows terminal found. Run manually:"
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
  open_terminal "Claude orchestrator" "cd $demo_dir_q; export RMUX_DEMO_SOCKET=$socket_q; export RMUX_DEMO_TARGETS='codex:0.0 gemini:0.0 grok:0.0'; exec env IS_DEMO=1 $CLAUDE_CMD" "$CLAUDE_GEOMETRY"

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
