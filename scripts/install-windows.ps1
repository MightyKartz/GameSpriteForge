#requires -Version 5.1
<#! Install a checksum-verified portable ZIP without administrator rights or symlinks.
    Previous payloads and launcher backups are retained. PATH is never modified. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Archive,
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$Sha256,
    [string]$InstallDirectory = (Join-Path $env:LOCALAPPDATA 'GameSpriteForge'),
    [switch]$AllowDevelopmentBuild
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'windows-package-common.ps1')
$archivePath = (Resolve-Path -LiteralPath $Archive).Path
if ((Get-Sha256 $archivePath) -ne $Sha256.ToLowerInvariant()) { throw 'Archive SHA-256 mismatch; installation was not changed' }
$root = [IO.Path]::GetFullPath($InstallDirectory)
Assert-RegularDirectory $root
New-Item -ItemType Directory -Path $root -Force | Out-Null
$ownerPath = Join-Path $root '.forge-windows-install.json'
$launcher = Join-Path $root 'bin/forge.cmd'
foreach ($path in @($ownerPath,(Join-Path $root '.install.lock'))) {
    if ((Test-Path -LiteralPath $path) -and ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Installer control file is a reparse point: $path" }
}
if ((Test-Path -LiteralPath $launcher) -and !(Test-Path -LiteralPath $ownerPath)) { throw 'Refusing to replace an unmanaged launcher' }
if (Test-Path -LiteralPath $ownerPath) {
    $owner = [IO.File]::ReadAllText($ownerPath) | ConvertFrom-Json
    if ($owner.managedBy -ne 'forge-windows-installer' -or $owner.schemaVersion -ne '1') { throw 'Unrecognized installation ownership file' }
}
$lock = [IO.File]::Open((Join-Path $root '.install.lock'),[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try {
    Assert-RegularDirectory (Join-Path $root 'bin')
    Assert-RegularDirectory (Join-Path $root 'releases')
    Assert-RegularDirectory (Join-Path $root 'backups')
    New-Item -ItemType Directory -Path (Join-Path $root 'bin'),(Join-Path $root 'releases'),(Join-Path $root 'backups') -Force | Out-Null
    $stage = Join-Path $root ('.staging-' + [Guid]::NewGuid().ToString('N'))
    Expand-VerifiedZip $archivePath $stage
    $info = Assert-Payload $stage
    if (!$AllowDevelopmentBuild -and ($info.build.dirty -ne $false -or $info.build.profile -ne 'release' -or @($info.build.features).Count -ne 0)) {
        throw 'Development/dirty/feature builds require explicit -AllowDevelopmentBuild'
    }
    $id = 'v' + $info.version + '-' + $Sha256.Substring(0,16).ToLowerInvariant()
    $payload = Join-Path $root "releases/$id"
    $wrapper = "@echo off`r`nsetlocal DisableDelayedExpansion`r`n`"%~dp0..\releases\$id\bin\forge.exe`" %*`r`nexit /b %errorlevel%`r`n"
    $action = 'installed'
    $backup = $null
    if (Test-Path -LiteralPath $launcher) {
        if ((Get-Item -LiteralPath $launcher -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Launcher is a reparse point' }
        $previous = [IO.File]::ReadAllText($launcher)
        if ($previous -notmatch '(?s)^@echo off\r?\nsetlocal DisableDelayedExpansion\r?\n"%~dp0\.\.\\releases\\(v[0-9A-Za-z.-]+)\\bin\\forge\.exe" %\*\r?\nexit /b %errorlevel%\r?\n$') { throw 'Refusing to overwrite a modified launcher' }
        $action = if ($previous -eq $wrapper) { 'unchanged' } else { 'updated' }
    }
    if (Test-Path -LiteralPath $payload) {
        Assert-RegularDirectory $payload
        $null = Assert-Payload $payload
        if ((Get-Sha256 (Join-Path $payload 'MANIFEST.sha256')) -ne (Get-Sha256 (Join-Path $stage 'MANIFEST.sha256'))) { throw 'Existing payload differs from requested archive' }
        # This is only the fresh, validated extraction below this installation.
        $stageAbsolute = [IO.Path]::GetFullPath($stage)
        if (!$stageAbsolute.StartsWith($root.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($stageAbsolute) -notmatch '^\.staging-[a-f0-9]{32}$') { throw 'Unsafe staging cleanup path' }
        Assert-RegularDirectory $stageAbsolute
        $null = @(Get-PayloadFiles $stageAbsolute)
        Remove-Item -LiteralPath $stageAbsolute -Recurse -Force
    } else {
        # Both resolved paths are verified beneath this installation before moving.
        if (![IO.Path]::GetFullPath($stage).StartsWith($root.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase) -or ![IO.Path]::GetFullPath($payload).StartsWith($root.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Payload paths escaped the installation root' }
        Move-Item -LiteralPath $stage -Destination $payload
    }
    if (!(Test-Path -LiteralPath $ownerPath)) { Write-Utf8File $ownerPath '{"schemaVersion":"1","managedBy":"forge-windows-installer"}' }
    if ($action -ne 'unchanged') {
        $pending = Join-Path $root ('bin/.forge-launcher-' + [Guid]::NewGuid().ToString('N') + '.tmp')
        Write-Utf8File $pending $wrapper
        if (Test-Path -LiteralPath $launcher) {
            $backup = Join-Path $root ('backups/forge-' + [Guid]::NewGuid().ToString('N') + '.cmd')
            [IO.File]::Replace($pending,$launcher,$backup)
        } else { [IO.File]::Move($pending,$launcher) }
    }
    [ordered]@{schemaVersion='1';ok=$true;action=$action;launcher=$launcher;payload=$payload;archiveSha256=$Sha256.ToLowerInvariant();build=$info.build;backupPath=$backup;pathModified=$false} | ConvertTo-Json -Depth 8 -Compress
} finally { $lock.Dispose() }
