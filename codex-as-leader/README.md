# codex-as-leader

Codex runs as the leader of a three-pane rmux workgroup. Claude is member1 and
member2.

Each launch creates one rmux session with this layout:

```text
+----------------------+----------------------+
| member1: claude      | member2: claude      |
|                      |                      |
+----------------------+----------------------+
| leader: codex                               |
|                                            |
+--------------------------------------------+
```

The leader receives rmux context through environment variables and can send
messages to either member, broadcast to both members, and capture their pane
output.

## Requirements

`rmux`, `codex`, `claude`, and `zsh` must be available in `PATH`.

## Run

```bash
./launch.sh check
./launch.sh
```

You can run `./launch.sh` multiple times. If the default session name already
exists, the launcher creates the next available session name, such as
`codex-as-leader-2`.

By default, all agents work in the current directory where you run `launch.sh`.
To make leader, member1, and member2 work in another directory, pass it to
`launch`:

```bash
./launch.sh launch /path/to/project
```

or set `RMUX_WORKDIR`:

```bash
RMUX_WORKDIR=/path/to/project ./launch.sh
```

The member panes start after `cd "$RMUX_WORKDIR"`. The leader keeps this demo
directory as its process cwd so it can load the rmux control instructions, and
the default Codex command receives `--add-dir "$RMUX_WORKDIR"` so it can work
with the target directory without losing workgroup control instructions.

Example command override:

```bash
CLAUDE_CMD="claude --dangerously-skip-permissions --permission-mode bypassPermissions" ./launch.sh
```

If you override `CODEX_CMD`, include the working directory yourself:

```bash
CODEX_CMD="codex --dangerously-bypass-approvals-and-sandbox --add-dir /path/to/project" ./launch.sh launch /path/to/project
```

## Try In Codex

```text
Send Hi to both members
Read both members and summarize what they answered
Ask member1 to propose a plan, ask member2 to review it, then compare their answers
```

## Cleanup

After detaching with `Ctrl-b` then `d`, reattach to the same demo socket:

```bash
./launch.sh attach
```

If the demo socket has multiple sessions, `attach` prompts you to choose one.
You can also attach directly:

```bash
./launch.sh attach codex-as-leader-2
```

List sessions on the demo socket:

```bash
./launch.sh list
```

```bash
./launch.sh cleanup
```

If the demo socket has multiple sessions, `cleanup` prompts you to choose one.
You can also clean up a session directly:

```bash
./launch.sh cleanup codex-as-leader-2
```
