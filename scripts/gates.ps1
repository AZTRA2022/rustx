# The quality gates, run on this machine.
#
# `STABILITY.md` section 6 asks for measurable gates rather than a
# feeling that things are probably fine, and for them to run in CI
# wherever possible. `.github/workflows/ci.yml` is that CI; this script
# is the same gates run locally, and it exists because a gate nobody
# can run before pushing is a gate that is discovered red.
#
# It must be PowerShell and not a shell script: Git Bash puts its own
# `/usr/bin/link` ahead of MSVC's `link.exe` on the `PATH`, so every
# native build started from bash fails at the linker. That is not a
# preference, it is the reason this file has the extension it has.
#
# Usage:
#
#   pwsh scripts/gates.ps1                 # every gate
#   pwsh scripts/gates.ps1 -Only test      # one gate
#   pwsh scripts/gates.ps1 -Offline        # no network (this machine)
#   pwsh scripts/gates.ps1 -List           # what the gates are
#
# Every gate prints PASS, FAIL or SKIP with a reason. A SKIP is not a
# pass: the summary counts it separately and says so, because section 6
# also says that when something cannot be tested, the limitation is
# documented rather than quietly treated as success.

[CmdletBinding()]
param(
  # Run only the gates whose names match one of these.
  [string[]] $Only = @(),
  # Do not run anything that needs the network.
  [switch] $Offline,
  # Print the gates and exit.
  [switch] $List,
  # Where cargo builds. A separate directory by default, so that running
  # the gates does not fight an editor or an app for the lock on
  # `target/`.
  [string] $TargetDir = ''
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if ($TargetDir -ne '') {
  $env:CARGO_TARGET_DIR = $TargetDir
}

# No GPU is assumed anywhere. An app that cannot find one otherwise
# fails in a way that looks like a bug in the app.
$env:RUSTX_SOFTWARE = '1'

$script:Results = [System.Collections.ArrayList]::new()

function Add-Result {
  param([string] $Name, [string] $State, [string] $Note)
  [void] $script:Results.Add([pscustomobject] @{
    Name  = $Name
    State = $State
    Note  = $Note
  })
  $colour = switch ($State) {
    'PASS' { 'Green' }
    'FAIL' { 'Red' }
    default { 'Yellow' }
  }
  $line = "  {0,-5} {1}" -f $State, $Name
  if ($Note -ne '') { $line += "  -- $Note" }
  Write-Host $line -ForegroundColor $colour
}

function Test-Wanted {
  param([string] $Name)
  # `powershell -File gates.ps1 -Only fmt,clippy` hands the whole
  # `fmt,clippy` over as one string rather than as two, so the commas
  # are split here as well. Otherwise the selection silently matches
  # nothing and the script reports that all zero gates passed, which is
  # the most misleading thing it could possibly say.
  $wanted = @()
  foreach ($one in $Only) {
    foreach ($part in ($one -split ',')) {
      if ($part.Trim() -ne '') { $wanted += $part.Trim() }
    }
  }
  if ($wanted.Count -eq 0) { return $true }
  foreach ($one in $wanted) {
    if ($Name -like "*$one*") { return $true }
  }
  return $false
}

# Run a command and hand back its exit code and its output, without
# letting a native program's stderr be mistaken for a PowerShell error.
function Invoke-Gate {
  param([string] $Command)
  $output = & cmd /c "$Command 2>&1"
  return [pscustomobject] @{
    Code   = $LASTEXITCODE
    Output = ($output | Out-String)
  }
}

# Is a program on the PATH and actually usable? `rustup` can list a
# component as installed while its binary is missing from the
# toolchain, which is the state of `rustfmt` on the machine this was
# written on, so asking rustup is not enough -- the program has to run.
function Test-Usable {
  param([string] $Program, [string] $Arguments = '--version')
  $probe = Invoke-Gate "$Program $Arguments"
  return ($probe.Code -eq 0)
}

$Offline_flag = if ($Offline) { '--offline' } else { '' }

# ----------------------------------------------------------------- gates

$Gates = @(
  @{
    Name = 'fmt'
    What = 'Every Rust file is formatted as rustfmt would format it.'
    Run  = {
      if (-not (Test-Usable 'rustfmt')) {
        return @{ State = 'SKIP'; Note = 'rustfmt is not usable on this toolchain; CI runs it' }
      }
      $got = Invoke-Gate "cargo fmt --all --check"
      if ($got.Code -eq 0) { return @{ State = 'PASS'; Note = '' } }
      $files = ([regex]::Matches($got.Output, '(?m)^Diff in ')).Count
      return @{ State = 'FAIL'; Note = "$files file(s) differ; run cargo fmt --all" }
    }
  },
  @{
    Name = 'clippy'
    What = 'No clippy warning anywhere in the workspace, tests included.'
    Run  = {
      if (-not (Test-Usable 'cargo-clippy')) {
        return @{ State = 'SKIP'; Note = 'clippy is not installed' }
      }
      $got = Invoke-Gate "cargo clippy $Offline_flag --workspace --all-targets -- -D warnings"
      if ($got.Code -eq 0) { return @{ State = 'PASS'; Note = '' } }
      $warnings = ([regex]::Matches($got.Output, '(?m)^(warning|error)(\[|:)')).Count
      return @{ State = 'FAIL'; Note = "$warnings diagnostic(s)" }
    }
  },
  @{
    Name = 'build'
    What = 'The whole workspace compiles, every target.'
    Run  = {
      $got = Invoke-Gate "cargo check $Offline_flag --workspace --all-targets"
      if ($got.Code -eq 0) { return @{ State = 'PASS'; Note = '' } }
      return @{ State = 'FAIL'; Note = 'see cargo output above' }
    }
  },
  @{
    Name = 'test'
    What = 'Every test in every crate passes.'
    Run  = {
      $got = Invoke-Gate "cargo test $Offline_flag --workspace"
      $passed = 0
      $failed = 0
      foreach ($match in [regex]::Matches($got.Output, 'test result: \w+\. (\d+) passed; (\d+) failed')) {
        $passed += [int] $match.Groups[1].Value
        $failed += [int] $match.Groups[2].Value
      }
      if ($got.Code -eq 0) {
        return @{ State = 'PASS'; Note = "$passed test(s)" }
      }
      return @{ State = 'FAIL'; Note = "$failed failed of $($passed + $failed)" }
    }
  },
  @{
    Name = 'docs'
    What = 'Every doc comment builds, and every intra-doc link resolves.'
    Run  = {
      $env:RUSTDOCFLAGS = '-D warnings'
      $got = Invoke-Gate "cargo doc $Offline_flag --workspace --no-deps"
      Remove-Item Env:\RUSTDOCFLAGS -ErrorAction SilentlyContinue
      if ($got.Code -eq 0) { return @{ State = 'PASS'; Note = '' } }
      return @{ State = 'FAIL'; Note = 'a doc comment or a doc link is broken' }
    }
  },
  @{
    Name = 'package'
    What = 'Every publishable crate packages, and carries its own readme.'
    Run  = {
      $crates = @(
        'rustx-dom', 'rustx-compiler', 'rustx-os', 'rustx-macros',
        'rustx-engine', 'rustx', 'rustx-cli', 'rustx-lsp'
      )
      $missing = @()
      foreach ($crate in $crates) {
        if (-not (Test-Path "crates/$crate/README.md")) { $missing += $crate }
      }
      # `cargo package` for a crate whose dependencies are not published
      # yet cannot resolve them, which is expected and not a defect
      # while the workspace sits on an unpublished version. The part
      # that is always checkable is whether each crate would ship a
      # readme at all: `readme.workspace` resolves per manifest, so a
      # crate without its own file renders an empty page on crates.io
      # and `cargo package` only warns about it.
      if ($missing.Count -gt 0) {
        return @{ State = 'FAIL'; Note = "no README.md in: $($missing -join ', ')" }
      }
      $got = Invoke-Gate "cargo package $Offline_flag -p rustx-dom --no-verify --allow-dirty"
      if ($got.Code -eq 0) { return @{ State = 'PASS'; Note = 'readmes present; rustx-dom packages' }  }
      return @{ State = 'FAIL'; Note = 'cargo package failed for rustx-dom' }
    }
  },
  @{
    Name = 'deps'
    What = 'Every dependency is permissively licensed and free of known advisories.'
    Run  = {
      # Two tools, because they answer two different questions.
      # `cargo deny` reads `deny.toml` and checks licences, duplicate
      # versions, banned crates and where each crate came from.
      # `cargo audit` checks the RustSec database, which is the one
      # that moves without anything in this repository changing.
      $haveDeny  = Test-Usable 'cargo' 'deny --version'
      $haveAudit = Test-Usable 'cargo' 'audit --version'
      if (-not $haveDeny -and -not $haveAudit) {
        return @{
          State = 'SKIP'
          Note  = 'neither is installed; cargo install cargo-deny cargo-audit'
        }
      }
      if (-not (Test-Path 'deny.toml')) {
        return @{ State = 'FAIL'; Note = 'deny.toml is missing from the repository root' }
      }

      $absent = @()

      if ($haveDeny) {
        # Offline, cargo-deny uses the advisory database already on
        # this machine rather than fetching it.
        $flag = if ($Offline) { '--offline' } else { '' }
        $got = Invoke-Gate "cargo deny $flag check"
        if ($got.Code -ne 0) {
          # Its last line is `advisories ok, bans FAILED, licenses ok,
          # sources ok`, which says which half of the check is red.
          $summary = [regex]::Match($got.Output, '(?m)^(advisories .+, sources \w+)$')
          $note = if ($summary.Success) { $summary.Groups[1].Value } else { 'cargo deny check failed' }
          return @{ State = 'FAIL'; Note = $note }
        }
      } else {
        $absent += 'cargo-deny'
      }

      $crates = ''
      if ($haveAudit) {
        # `-n` is "do not fetch the advisory database", for the same
        # reason as the flag above.
        $flag = if ($Offline) { '-n' } else { '' }
        $got = Invoke-Gate "cargo audit $flag"
        if ($got.Code -ne 0) {
          $found = ([regex]::Matches($got.Output, '(?m)^ID:\s')).Count
          $note = if ($found -gt 0) { "$found advisory/advisories" } else { 'cargo audit failed' }
          return @{ State = 'FAIL'; Note = $note }
        }
        $scanned = [regex]::Match($got.Output, 'vulnerabilities \((\d+) crate')
        if ($scanned.Success) { $crates = "$($scanned.Groups[1].Value) crates" }
      } else {
        $absent += 'cargo-audit'
      }

      # One of the two having run is not this gate having run. Section
      # 6 again: a check that did not happen is not a check that
      # passed, so it is a SKIP with the reason, not a PASS.
      if ($absent.Count -gt 0) {
        return @{
          State = 'SKIP'
          Note  = "$($absent -join ' and ') not installed; what ran passed"
        }
      }
      return @{ State = 'PASS'; Note = ("licences, bans, sources, advisories" + $(if ($crates -ne '') { "; $crates" } else { '' })) }
    }
  },
  @{
    Name = 'editor'
    What = 'The VS Code extension typechecks, builds and passes its tests.'
    Run  = {
      if (-not (Test-Path 'editors/vscode/node_modules')) {
        return @{ State = 'SKIP'; Note = 'editors/vscode/node_modules is absent; npm ci first' }
      }
      Push-Location 'editors/vscode'
      try {
        $types = Invoke-Gate 'npx tsc --noEmit'
        if ($types.Code -ne 0) { return @{ State = 'FAIL'; Note = 'tsc --noEmit failed' } }
        $built = Invoke-Gate 'node build.mjs'
        if ($built.Code -ne 0) { return @{ State = 'FAIL'; Note = 'build.mjs failed' } }
        # The pattern, and not the directory: `node --test test` is read
        # as a module path by Node 24 and fails before a test runs. TAP is
        # asked for so that the counts below are parsed from a stable
        # format rather than from the human reporter.
        $tested = Invoke-Gate 'node --test --test-reporter=tap "test/**/*.test.mjs"'
        if ($tested.Code -ne 0) {
          $failed = ([regex]::Match($tested.Output, '(?m)^# fail (\d+)'))
          $note = if ($failed.Success) { "$($failed.Groups[1].Value) test(s) failed" } else { 'node --test failed' }
          return @{ State = 'FAIL'; Note = $note }
        }
        $passed = [regex]::Match($tested.Output, '(?m)^# pass (\d+)')
        $note = if ($passed.Success) { "$($passed.Groups[1].Value) test(s)" } else { '' }
        return @{ State = 'PASS'; Note = $note }
      } finally {
        Pop-Location
      }
    }
  },
  @{
    Name = 'site'
    What = 'The documentation site typechecks and builds every page.'
    Run  = {
      if (-not (Test-Path 'site/node_modules')) {
        return @{ State = 'SKIP'; Note = 'site/node_modules is absent; npm ci first' }
      }
      Push-Location 'site'
      try {
        $types = Invoke-Gate 'npx tsc --noEmit'
        if ($types.Code -ne 0) { return @{ State = 'FAIL'; Note = 'tsc --noEmit failed' } }
        $built = Invoke-Gate 'npx next build'
        if ($built.Code -ne 0) { return @{ State = 'FAIL'; Note = 'next build failed' } }
        $pages = [regex]::Match($built.Output, 'Generating static pages using .* \((\d+)/\1\)')
        $note = if ($pages.Success) { "$($pages.Groups[1].Value) pages" } else { '' }
        return @{ State = 'PASS'; Note = $note }
      } finally {
        Pop-Location
      }
    }
  }
)

if ($List) {
  Write-Host ''
  Write-Host 'The gates:' -ForegroundColor Cyan
  foreach ($gate in $Gates) {
    Write-Host ("  {0,-8} {1}" -f $gate.Name, $gate.What)
  }
  Write-Host ''
  exit 0
}

# ------------------------------------------------------------------- run

Write-Host ''
Write-Host 'rustx quality gates' -ForegroundColor Cyan
if ($env:CARGO_TARGET_DIR) {
  Write-Host "  target directory: $($env:CARGO_TARGET_DIR)"
}
if ($Offline) {
  Write-Host '  offline: nothing that needs the network will run'
}
Write-Host ''

foreach ($gate in $Gates) {
  if (-not (Test-Wanted $gate.Name)) { continue }
  $answer = & $gate.Run
  Add-Result -Name $gate.Name -State $answer.State -Note $answer.Note
}

# --------------------------------------------------------------- summary

$passed  = @($script:Results | Where-Object { $_.State -eq 'PASS' }).Count
$failed  = @($script:Results | Where-Object { $_.State -eq 'FAIL' }).Count
$skipped = @($script:Results | Where-Object { $_.State -eq 'SKIP' }).Count

Write-Host ''
if ($script:Results.Count -eq 0) {
  # A selection that matched nothing is a mistake, not a clean run.
  Write-Host 'No gate matched the selection. Try -List.' -ForegroundColor Red
  exit 2
}
if ($failed -gt 0) {
  Write-Host "$failed gate(s) failed, $passed passed, $skipped skipped." -ForegroundColor Red
  Write-Host 'Not shippable.' -ForegroundColor Red
  exit 1
}
if ($skipped -gt 0) {
  # Deliberately not a pass. A gate that did not run has not been met,
  # and saying otherwise is the thing STABILITY.md section 6 forbids.
  Write-Host "$passed gate(s) passed; $skipped could not run here." -ForegroundColor Yellow
  Write-Host 'Every gate that ran is green. The skipped ones remain unverified.' -ForegroundColor Yellow
  exit 0
}
Write-Host "All $passed gates passed." -ForegroundColor Green
exit 0
