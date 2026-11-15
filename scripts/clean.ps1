# Delete the build output of every project on this machine.
#
# Why this exists: one `cargo build` of this framework is 13 GB, because
# CEF is 300 MB per profile and every app in `apps/` has its own
# `target/`. Add a `node_modules` per app and a machine fills up in a
# week -- which is not a hypothetical: it filled up while writing this,
# and cargo, `npx tsc` and `cargo doc` all failed with messages that
# named a lint, a type and a doc link rather than the disk.
#
# What it deletes is only ever regenerated output: `target/`,
# `node_modules/`, `.next/`, `dist/`, the registry caches, the
# temporary folders this framework writes. Nothing a person typed.
#
#   pwsh scripts/clean.ps1                 # this project, and say what it freed
#   pwsh scripts/clean.ps1 -All            # every project under the home folder
#   pwsh scripts/clean.ps1 -All -DryRun    # say what it would delete, delete nothing
#   pwsh scripts/clean.ps1 -All -Caches    # also the cargo and npm registry caches
#   pwsh scripts/clean.ps1 -Root D:\work   # somewhere else
#
# `-Caches` is separate because those are downloads rather than builds:
# emptying them is safe and costs the next build its network time.

[CmdletBinding()]
param(
  # Where to look. The project this script lives in, by default.
  [string] $Root = '',
  # Every project under the home folder instead of just this one.
  [switch] $All,
  # Also the package managers' own download caches.
  [switch] $Caches,
  # Say what would go, and delete nothing.
  [switch] $DryRun,
  # Do not ask. Without it, anything over this many gigabytes is
  # confirmed first.
  [switch] $Yes
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

# The folder names that hold build output and nothing else.
#
# A name alone is not enough for two of them: `dist` and `build` are
# also perfectly good source directory names, so those are taken only
# when something beside them says the folder is a project.
$Output = @(
  @{ Name = 'target';       Beside = 'Cargo.toml' },
  @{ Name = 'node_modules'; Beside = 'package.json' },
  @{ Name = '.next';        Beside = 'package.json' },
  @{ Name = '.turbo';       Beside = 'package.json' },
  @{ Name = '.svelte-kit';  Beside = 'package.json' },
  @{ Name = '.parcel-cache'; Beside = 'package.json' },
  @{ Name = '.rustx';       Beside = 'Cargo.toml' },
  @{ Name = 'dist';         Beside = 'package.json' },
  @{ Name = 'dist';         Beside = 'Cargo.toml' },
  @{ Name = '__pycache__';  Beside = '' },
  @{ Name = '.pytest_cache'; Beside = '' },
  @{ Name = '.mypy_cache';  Beside = '' }
)

# Where an installed program keeps its own files.
#
# This matters more than any other rule here: a `node_modules` with a
# `package.json` beside it inside an Electron application's
# `app.asar.unpacked`, or inside a VS Code extension, is that program's
# code and not build output. Deleting it uninstalls the program. A
# first run of this script found five of them.
$Installed = @(
  'AppData\Local\Programs',
  'AppData\Local\Microsoft',
  'AppData\Local\Google',
  'AppData\Local\Discord',
  'AppData\Local\slack',
  'AppData\Roaming\npm',
  'AppData\Roaming\Code',
  'AppData\Roaming\nvm',
  '.vscode',
  '.vscode-insiders',
  '.vscode-server',
  'scoop',
  'AppData\Local\JetBrains'
)

function Installed-Software {
  param([string] $Path)
  foreach ($under in $Installed) {
    $full = Join-Path $env:USERPROFILE $under
    if ($Path.StartsWith($full, 'OrdinalIgnoreCase')) { return $true }
  }
  # An unpacked Electron application, wherever it was installed.
  if ($Path -like '*app.asar.unpacked*' -or $Path -like '*\resources\app\*') { return $true }
  return $false
}

# Folders that are temporary by construction: this framework's own, and
# the engine profiles it writes per application title.
function Temporary-Folders {
  $found = @()
  $temp = $env:TEMP
  if ($temp -and (Test-Path $temp)) {
    $found += Get-ChildItem $temp -Directory -ErrorAction SilentlyContinue |
      Where-Object { $_.Name -like 'rustx-*' -or $_.Name -like 'cargo-install*' }
  }
  $local = $env:LOCALAPPDATA
  if ($local) {
    foreach ($name in @('rustx', 'npm-cache\_cacache', 'esbuild')) {
      $path = Join-Path $local $name
      if (Test-Path $path) { $found += Get-Item $path }
    }
  }
  return $found
}

function Registry-Caches {
  $found = @()
  foreach ($path in @(
      (Join-Path $env:USERPROFILE '.cargo\registry\cache'),
      (Join-Path $env:USERPROFILE '.cargo\registry\src'),
      (Join-Path $env:USERPROFILE '.cargo\git\checkouts'),
      (Join-Path $env:USERPROFILE 'AppData\Local\npm-cache'),
      (Join-Path $env:USERPROFILE 'AppData\Roaming\npm-cache'),
      (Join-Path $env:USERPROFILE '.npm\_cacache')
    )) {
    if (Test-Path $path) { $found += Get-Item $path }
  }
  return $found
}

function Size-Of {
  param([string] $Path)
  # `Measure-Object` over nothing returns an object with no `Sum` at
  # all, and `Set-StrictMode` turns reading that into an error -- so it
  # is held first and asked about, rather than dotted into.
  $measured = Get-ChildItem $Path -Recurse -File -Force -ErrorAction SilentlyContinue |
    Measure-Object -Property Length -Sum
  if (-not $measured) { return 0 }
  $sum = $measured.Sum
  if ($null -eq $sum) { return 0 }
  return [long] $sum
}

function Gb {
  param([long] $Bytes)
  return '{0,8:N2} GB' -f ($Bytes / 1GB)
}

# ------------------------------------------------------------------ find

if ($Root -eq '') {
  $Root = if ($All) { $env:USERPROFILE } else { Split-Path -Parent $PSScriptRoot }
}
if (-not (Test-Path $Root)) {
  Write-Host "rustx clean: $Root is not there." -ForegroundColor Red
  exit 2
}

$free = (Get-PSDrive ((Get-Item $Root).PSDrive.Name)).Free
Write-Host ''
Write-Host "rustx clean" -ForegroundColor Cyan
Write-Host "  looking under $Root"
Write-Host "  free now:$(Gb $free)"
Write-Host ''

$targets = [System.Collections.ArrayList]::new()
$seen = [System.Collections.Generic.HashSet[string]]::new()

function Consider {
  param([string] $Path)
  if (-not (Test-Path $Path)) { return }
  $full = (Resolve-Path -LiteralPath $Path -ErrorAction SilentlyContinue).Path
  if (-not $full) { return }
  if (Installed-Software $full) { return }
  # A `target/` inside a `node_modules` that is already going is
  # counted once, by the outer one.
  foreach ($held in $targets) {
    if ($full.StartsWith($held.Path + [IO.Path]::DirectorySeparatorChar, 'OrdinalIgnoreCase')) { return }
  }
  if (-not $seen.Add($full.ToLowerInvariant())) { return }
  [void] $targets.Add([pscustomobject] @{ Path = $full; Bytes = (Size-Of $full) })
}

# One walk, skipping the insides of anything already matched: a
# recursive search that descends into `node_modules` on a machine with
# twenty of them takes minutes and finds nothing new.
$names = ($Output | ForEach-Object { $_.Name } | Select-Object -Unique)
$stack = [System.Collections.Stack]::new()
$stack.Push($Root)
while ($stack.Count -gt 0) {
  $here = $stack.Pop()
  $children = Get-ChildItem -LiteralPath $here -Directory -Force -ErrorAction SilentlyContinue
  foreach ($child in $children) {
    # Nothing under a hidden system folder, and nothing under .git.
    if ($child.Name -in @('.git', '.svn', '.hg')) { continue }
    if (Installed-Software ($child.FullName + [IO.Path]::DirectorySeparatorChar)) { continue }
    if ($child.Name -in $names) {
      $rule = $Output | Where-Object { $_.Name -eq $child.Name } |
        Where-Object { $_.Beside -eq '' -or (Test-Path (Join-Path $here $_.Beside)) } |
        Select-Object -First 1
      if ($rule) { Consider $child.FullName; continue }
    }
    $stack.Push($child.FullName)
  }
}

foreach ($folder in (Temporary-Folders)) { Consider $folder.FullName }
if ($Caches) { foreach ($folder in (Registry-Caches)) { Consider $folder.FullName } }

if ($targets.Count -eq 0) {
  Write-Host "  nothing to delete." -ForegroundColor Green
  Write-Host ''
  exit 0
}

# ------------------------------------------------------------------ say

$total = ($targets | Measure-Object -Property Bytes -Sum).Sum
foreach ($each in ($targets | Sort-Object -Property Bytes -Descending)) {
  $shown = $each.Path
  if ($shown.StartsWith($Root, 'OrdinalIgnoreCase')) {
    $shown = '.' + $shown.Substring($Root.Length)
  }
  Write-Host "  $(Gb $each.Bytes)  $shown"
}
Write-Host ''
Write-Host "  $($targets.Count) folder(s), $(Gb $total) in all" -ForegroundColor Yellow

if ($DryRun) {
  Write-Host "  -DryRun: nothing was deleted." -ForegroundColor Cyan
  Write-Host ''
  exit 0
}

# Everything here is regenerated by a build, so the only question worth
# asking is whether the person meant to spend that build time.
if (-not $Yes -and ($total / 1GB) -gt 1) {
  Write-Host ''
  $answer = Read-Host "  Delete all of it? Every build will be from scratch. [y/N]"
  if ($answer -notin @('y', 'Y', 'yes')) {
    Write-Host "  nothing was deleted." -ForegroundColor Cyan
    Write-Host ''
    exit 0
  }
}

# ---------------------------------------------------------------- delete

$gone = 0
$kept = @()
foreach ($each in $targets) {
  Remove-Item -LiteralPath $each.Path -Recurse -Force -ErrorAction SilentlyContinue
  if (Test-Path -LiteralPath $each.Path) {
    # A file held open by a running editor, window or watcher. Naming
    # it is the whole point: "some files could not be deleted" is not
    # something anybody can act on.
    $kept += $each.Path
  } else {
    $gone += $each.Bytes
  }
}

$after = (Get-PSDrive ((Get-Item $Root).PSDrive.Name)).Free
Write-Host ''
Write-Host "  freed $(Gb $gone), $(Gb $after) free now" -ForegroundColor Green
if ($kept.Count -gt 0) {
  Write-Host ''
  Write-Host "  still there, held by something that is running:" -ForegroundColor Yellow
  foreach ($path in $kept) { Write-Host "    $path" }
  Write-Host '  close the editor, the window or `rustx dev`, and run this again.'
}
Write-Host ''
exit 0
