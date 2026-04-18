param(
  [ValidateSet('client', 'server', 'all')]
  [string]$Target = 'all'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ClientDir = Join-Path $RepoRoot 'client'
$ServerDir = Join-Path $RepoRoot 'server'

function Assert-Command {
  param([Parameter(Mandatory = $true)][string]$Name)

  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $Name"
  }
}

function Invoke-ClientBuild {
  Write-Host '==> Building desktop client'
  Assert-Command -Name 'npm'
  Assert-Command -Name 'cargo'

  Push-Location $ClientDir
  try {
    if (-not (Test-Path (Join-Path $ClientDir 'node_modules'))) {
      Write-Host '==> Installing client dependencies'
      npm install
      if ($LASTEXITCODE -ne 0) {
        throw 'npm install failed.'
      }
    }

    Write-Host '==> Building frontend assets'
    npm run build
    if ($LASTEXITCODE -ne 0) {
      throw 'npm run build failed.'
    }

    Write-Host '==> Packaging Tauri desktop app'
    cargo tauri build
    if ($LASTEXITCODE -ne 0) {
      throw 'cargo tauri build failed.'
    }
  }
  finally {
    Pop-Location
  }
}

function Invoke-ServerBuild {
  Write-Host '==> Building server release binary'
  Assert-Command -Name 'cargo'

  Push-Location $ServerDir
  try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
      throw 'cargo build --release failed.'
    }
  }
  finally {
    Pop-Location
  }
}

switch ($Target) {
  'client' { Invoke-ClientBuild }
  'server' { Invoke-ServerBuild }
  'all' {
    Invoke-ServerBuild
    Invoke-ClientBuild
  }
}

Write-Host ''
Write-Host 'Build completed.'
