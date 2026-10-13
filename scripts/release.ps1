<#
.SYNOPSIS
  Builds a signed Dusty release and publishes it where installed copies look for updates.

.EXAMPLE
  ./scripts/release.ps1 -Version 0.3.0 -Notes "Voice search and a smarter search bar"

  Installed Dusty apps check the update source (Settings > About > Updates; default
  C:\dev\Dusty\releases) every 6 hours, verify the signature and update themselves.
  Upload the same two files (installer + latest.json) to a web server later and point
  the update source at https://.../latest.json.
#>
param(
  [string]$Version,
  [string]$Notes = "",
  [string]$Out = (Join-Path (Split-Path $PSScriptRoot -Parent) "releases"),
  [string]$Key = (Join-Path $HOME ".tauri\dusty.key")
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

if (-not (Test-Path $Key)) { throw "Signing key not found at $Key (create one with: pnpm tauri signer generate -w $Key)" }

if ($Version) {
  Write-Host "Setting version $Version" -ForegroundColor Cyan
  foreach ($f in @("package.json", "src-tauri/tauri.conf.json")) {
    $t = Get-Content $f -Raw
    $t = [regex]::Replace($t, '"version":\s*"[^"]+"', "`"version`": `"$Version`"", 1)
    Set-Content $f $t -NoNewline
  }
  $c = Get-Content "src-tauri/Cargo.toml" -Raw
  $c = [regex]::Replace($c, '(?m)^version = "[^"]+"', "version = `"$Version`"", 1)
  Set-Content "src-tauri/Cargo.toml" $c -NoNewline
}
$v = (Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json).version

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $Key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
Write-Host "Building Dusty $v" -ForegroundColor Cyan
pnpm tauri build
if ($LASTEXITCODE -ne 0) { throw "build failed" }

$exe = "src-tauri/target/release/bundle/nsis/Dusty_${v}_x64-setup.exe"
if (-not (Test-Path "$exe.sig")) { throw "No signature next to $exe (is createUpdaterArtifacts on?)" }
New-Item -ItemType Directory -Force $Out | Out-Null
$name = Split-Path $exe -Leaf
Copy-Item $exe (Join-Path $Out $name) -Force
$manifest = [ordered]@{
  version   = $v
  notes     = $Notes
  pub_date  = (Get-Date).ToUniversalTime().ToString("o")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{ url = $name; signature = (Get-Content "$exe.sig" -Raw).Trim() }
  }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $Out "latest.json") -Encoding utf8
Write-Host "Published Dusty $v to $Out" -ForegroundColor Green
