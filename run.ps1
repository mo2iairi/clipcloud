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

function Ensure-ClientDependencies {
  Assert-Command -Name 'npm'
  if (-not (Test-Path (Join-Path $ClientDir 'node_modules'))) {
    Push-Location $ClientDir
    try {
      Write-Host '==> Installing client dependencies'
      npm install
      if ($LASTEXITCODE -ne 0) {
        throw 'npm install failed.'
      }
    }
    finally {
      Pop-Location
    }
  }
}

function Start-ServerForeground {
  Assert-Command -Name 'cargo'

  Push-Location $ServerDir
  try {
    if (-not $env:DATABASE_URL -and -not (Test-Path (Join-Path $ServerDir '.env'))) {
      Write-Warning 'DATABASE_URL is not set and server/.env was not found.'
    }

    Write-Host '==> Starting server'
    cargo run
    if ($LASTEXITCODE -ne 0) {
      throw 'cargo run failed.'
    }
  }
  finally {
    Pop-Location
  }
}

function Start-ClientForeground {
  Assert-Command -Name 'cargo'
  Ensure-ClientDependencies

  Push-Location $ClientDir
  try {
    Write-Host '==> Starting desktop client'
    cargo tauri dev
    if ($LASTEXITCODE -ne 0) {
      throw 'cargo tauri dev failed.'
    }
  }
  finally {
    Pop-Location
  }
}

function Start-ServerBackground {
  $command = "Set-Location -LiteralPath '$ServerDir'; cargo run"
  Start-Process powershell -ArgumentList @('-NoExit', '-Command', $command) | Out-Null
}

switch ($Target) {
  'server' { Start-ServerForeground }
  'client' { Start-ClientForeground }
  'all' {
    Start-ServerBackground
    Start-Sleep -Seconds 2
    Start-ClientForeground
  }
}
