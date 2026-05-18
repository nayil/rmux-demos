param(
  [ValidateSet("launch", "check", "cleanup")]
  [string]$Action = "launch"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$DemoDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Socket = if ($env:RMUX_DEMO_SOCKET) { $env:RMUX_DEMO_SOCKET } else { "\\.\pipe\rmux-demo-orchestration" }

$CodexCmd = if ($env:CODEX_CMD) { $env:CODEX_CMD } else { "codex --dangerously-bypass-approvals-and-sandbox" }
$GeminiCmd = if ($env:GEMINI_CMD) { $env:GEMINI_CMD } else { "gemini --skip-trust --approval-mode yolo" }
$GrokCmd = if ($env:GROK_CMD) { $env:GROK_CMD } else { "grok --always-approve" }
$ClaudeCmd = if ($env:CLAUDE_CMD) { $env:CLAUDE_CMD } else { "claude --dangerously-skip-permissions --permission-mode bypassPermissions" }

$CodexGeometry = if ($env:CODEX_GEOMETRY) { $env:CODEX_GEOMETRY } else { "96x26+10+40" }
$GeminiGeometry = if ($env:GEMINI_GEOMETRY) { $env:GEMINI_GEOMETRY } else { "96x26+960+40" }
$GrokGeometry = if ($env:GROK_GEOMETRY) { $env:GROK_GEOMETRY } else { "96x26+10+560" }
$ClaudeGeometry = if ($env:CLAUDE_GEOMETRY) { $env:CLAUDE_GEOMETRY } else { "96x26+960+560" }

function Quote-PowerShellLiteral {
  param([Parameter(Mandatory = $true)][string]$Value)
  $result = [string]::Concat("'", $Value.Replace("'", "''"), "'")
  return $result
}

function Quote-WindowsArgument {
  param([Parameter(Mandatory = $true)][string]$Value)
  $result = [string]::Concat('"', $Value.Replace('"', '\"'), '"')
  return $result
}

function Parse-TerminalGeometry {
  param([Parameter(Mandatory = $true)][string]$Geometry)
  if ($Geometry -match "^([0-9]+)x([0-9]+)\+([0-9]+)\+([0-9]+)$") {
    return [pscustomobject]@{
      Cols = [int]$Matches[1]
      Rows = [int]$Matches[2]
      Left = [int]$Matches[3]
      Top = [int]$Matches[4]
    }
  }
  return [pscustomobject]@{
    Cols = 96
    Rows = 26
    Left = 80
    Top = 80
  }
}

function Test-Command {
  param([Parameter(Mandatory = $true)][string]$Name)
  return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Rmux {
  & rmux -S $Socket @args
  if ($LASTEXITCODE -ne 0) {
    throw "rmux exited with code $LASTEXITCODE"
  }
}

function Invoke-RmuxIgnore {
  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = "SilentlyContinue"
  try {
    & rmux -S $Socket @args *> $null
  } catch {
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
    $global:LASTEXITCODE = 0
  }
}

function Check-Dependencies {
  $missing = @()
  foreach ($command in @("rmux", "claude", "codex", "gemini", "grok")) {
    if (-not (Test-Command $command)) {
      $missing += $command
    }
  }

  if ($missing.Count -gt 0) {
    throw "missing command in PATH: $($missing -join ', ')"
  }

  Write-Host "rmux, claude, codex, gemini and grok are available"
}

function Cleanup-Demo {
  foreach ($session in @("codex", "gemini", "grok")) {
    Invoke-RmuxIgnore kill-session -t $session
  }
  Invoke-RmuxIgnore kill-server
}

function New-AgentSession {
  param(
    [Parameter(Mandatory = $true)][string]$Session,
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][string]$Command
  )

  $agentCommand = New-AgentCommand $Session $Title $Command
  Invoke-Rmux new-session -d -s $Session -n $Title -x 120 -y 34 $agentCommand
  & rmux -S $Socket select-pane -t "${Session}:0.0" -T $Title *> $null
}

function New-AgentCommand {
  param(
    [Parameter(Mandatory = $true)][string]$Session,
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][string]$Command
  )

  $launcherRoot = Join-Path ([System.IO.Path]::GetTempPath()) "rmux-demo-orchestration"
  New-Item -ItemType Directory -Force -Path $launcherRoot | Out-Null
  $launcher = Join-Path $launcherRoot "rmux-agent-$Session.ps1"
  [string]$titleLiteral = Quote-PowerShellLiteral $Title
  [string]$demoDirLiteral = Quote-PowerShellLiteral $DemoDir
  [string]$socketLiteral = Quote-PowerShellLiteral $Socket
  [string]$exitPrefixLiteral = Quote-PowerShellLiteral "[$Title exited with code "
  [string]$exitSuffixLiteral = Quote-PowerShellLiteral "; the rmux pane stays open so the attach window does not disappear.]"
  $scriptLines = [System.Collections.Generic.List[string]]::new()
  $scriptLines.Add('$ErrorActionPreference = "Continue"') | Out-Null
  $scriptLines.Add('$Host.UI.RawUI.WindowTitle = ' + $titleLiteral) | Out-Null
  $scriptLines.Add('Set-Location -LiteralPath ' + $demoDirLiteral) | Out-Null
  $scriptLines.Add('$env:RMUX_DEMO_SOCKET = ' + $socketLiteral) | Out-Null
  $scriptLines.Add($Command) | Out-Null
  $scriptLines.Add('$exitCode = if ($global:LASTEXITCODE -is [int]) { $global:LASTEXITCODE } else { 0 }') | Out-Null
  $scriptLines.Add('if ($exitCode -ne 0) {') | Out-Null
  $scriptLines.Add('  Write-Host ""') | Out-Null
  $scriptLines.Add('  Write-Host ([string]::Concat(' + $exitPrefixLiteral + ', $exitCode, ' + $exitSuffixLiteral + '))') | Out-Null
  $scriptLines.Add('}') | Out-Null
  $script = [string]::Join("`r`n", $scriptLines)
  Set-Content -LiteralPath $launcher -Value $script -Encoding UTF8

  return ([string]::Concat("powershell.exe -NoExit -NoProfile -ExecutionPolicy Bypass -File ", (Quote-WindowsArgument $launcher)))
}

function Open-Terminal {
  param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][string]$Command,
    [Parameter(Mandatory = $true)][string]$Geometry
  )

  $bounds = Parse-TerminalGeometry $Geometry
  $launcherRoot = Join-Path ([System.IO.Path]::GetTempPath()) "rmux-demo-orchestration"
  New-Item -ItemType Directory -Force -Path $launcherRoot | Out-Null
  $slug = ($Title.ToLowerInvariant() -replace '[^a-z0-9_-]+', '-').Trim("-")
  if ([string]::IsNullOrWhiteSpace($slug)) {
    $slug = "terminal"
  }
  $launcher = Join-Path $launcherRoot "rmux-demo-$slug.ps1"
  [string]$titleLiteral = Quote-PowerShellLiteral $Title
  $scriptLines = [System.Collections.Generic.List[string]]::new()
  $scriptLines.Add('$ErrorActionPreference = "Stop"') | Out-Null
  $scriptLines.Add('$Host.UI.RawUI.WindowTitle = ' + $titleLiteral) | Out-Null
  $scriptLines.Add($Command) | Out-Null
  $scriptLines.Add('if ($global:LASTEXITCODE -is [int] -and $global:LASTEXITCODE -ne 0) { exit $global:LASTEXITCODE }') | Out-Null
  $script = [string]::Join("`r`n", $scriptLines)
  Set-Content -LiteralPath $launcher -Value $script -Encoding UTF8

  $wt = Get-Command wt.exe -ErrorAction SilentlyContinue
  if (-not $wt) {
    $wt = Get-Command wt -ErrorAction SilentlyContinue
  }

  if ($wt) {
    $arguments = @(
      "-w", "new",
      "--pos", "$($bounds.Left),$($bounds.Top)",
      "--size", "$($bounds.Cols),$($bounds.Rows)",
      "--title", $Title,
      "powershell.exe", "-NoExit", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $launcher
    ) | ForEach-Object { Quote-WindowsArgument $_ }
    Start-Process -FilePath $wt.Source -ArgumentList ($arguments -join " ") | Out-Null
    Start-Sleep -Milliseconds 250
    return
  }

  $arguments = @(
    "-NoExit", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $launcher
  ) | ForEach-Object { Quote-WindowsArgument $_ }
  Start-Process -FilePath "powershell.exe" -ArgumentList ($arguments -join " ") | Out-Null
}

function Attach-Command {
  param([Parameter(Mandatory = $true)][string]$Session)
  return '$env:RMUX_DEMO_SOCKET = ' + (Quote-PowerShellLiteral $Socket) + '; rmux -S ' + (Quote-PowerShellLiteral $Socket) + ' attach-session -t ' + (Quote-PowerShellLiteral $Session)
}

function Launch-Demo {
  Check-Dependencies
  Cleanup-Demo

  New-AgentSession "codex" "Codex" $CodexCmd
  New-AgentSession "gemini" "Gemini" $GeminiCmd
  New-AgentSession "grok" "Grok" $GrokCmd

  Open-Terminal "Codex agent" (Attach-Command "codex") $CodexGeometry
  Open-Terminal "Gemini agent" (Attach-Command "gemini") $GeminiGeometry
  Open-Terminal "Grok agent" (Attach-Command "grok") $GrokGeometry
  Start-Sleep -Seconds 1

  $claudeCommand =
    'Set-Location -LiteralPath ' + (Quote-PowerShellLiteral $DemoDir) + '; ' +
    '$env:RMUX_DEMO_SOCKET = ' + (Quote-PowerShellLiteral $Socket) + '; ' +
    '$env:RMUX_DEMO_TARGETS = ' + (Quote-PowerShellLiteral "codex:0.0 gemini:0.0 grok:0.0") + '; ' +
    '$env:IS_DEMO = ' + (Quote-PowerShellLiteral "1") + '; ' +
    $ClaudeCmd
  Open-Terminal "Claude orchestrator" $claudeCommand $ClaudeGeometry

  Write-Host "demo started"
  Write-Host "socket: $Socket"
  Write-Host "try in Claude: Send Hi to all agents"
}

switch ($Action) {
  "launch" { Launch-Demo }
  "check" { Check-Dependencies }
  "cleanup" { Cleanup-Demo }
}
