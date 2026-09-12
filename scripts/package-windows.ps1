#requires -Version 7.0
<#! Create a portable Windows ZIP from an already-built CLI and pinned source-built helpers. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Forge,
    [Parameter(Mandatory=$true)][string]$Helpers,
    [Parameter(Mandatory=$true)][string]$Output,
    [switch]$AllowDevelopmentBuild
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'windows-package-common.ps1')
$binary = (Resolve-Path -LiteralPath $Forge).Path
$helperRoot = (Resolve-Path -LiteralPath $Helpers).Path
$zipPath = [IO.Path]::GetFullPath($Output)
if (Test-Path -LiteralPath $zipPath) { throw 'Choose a new package output; existing archives are not overwritten' }
New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($zipPath)) -Force | Out-Null
$stage = Join-Path ([IO.Path]::GetDirectoryName($zipPath)) ('package-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage,(Join-Path $stage 'bin') -Force | Out-Null
$receipt = [IO.File]::ReadAllText((Join-Path $helperRoot 'FFMPEG_BUILD.json')) | ConvertFrom-Json
if ($receipt.schemaVersion -ne '1' -or $receipt.ffmpegVersion -ne '8.1.2' -or $receipt.zlibVersion -ne '1.3.2' -or $receipt.sourceModified -ne $false) { throw 'Expected unmodified pinned FFmpeg 8.1.2 and zlib 1.3.2 source build' }
foreach ($name in @('ffmpeg','ffprobe')) {
    $tool = Join-Path $helperRoot "bin/$name.exe"
    if ((Get-Sha256 $tool) -ne $receipt.binaries."$name.exe") { throw "Helper receipt hash mismatch: $name" }
    $version = (& $tool -version) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $version -notmatch "$name version 8\.1\.2" -or $version -match '--enable-(gpl|nonfree)' -or $version -notmatch '--disable-gpl' -or $version -notmatch '--disable-nonfree') { throw "Unexpected helper configuration: $name" }
}
foreach ($pin in @(
    @{name='ffmpeg-8.1.2.tar.xz';sha256='464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c'},
    @{name='zlib-1.3.2.tar.xz';sha256='d7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3'}
)) {
    $record=@($receipt.downloads | Where-Object name -EQ $pin.name)
    if ($record.Count -ne 1 -or $record[0].sha256 -ne $pin.sha256 -or (Get-Sha256 (Join-Path $helperRoot "sources/$($pin.name)")) -ne $pin.sha256) { throw 'Retained helper source archive does not match the pinned build inputs' }
}
if ((Get-Sha256 (Join-Path $helperRoot 'sources/build-helpers.sh')) -ne $receipt.sourceRecipeSha256) { throw 'Helper source recipe does not match its build receipt' }
Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'bin/forge.exe')
foreach ($name in @('ffmpeg','ffprobe')) { Copy-Item -LiteralPath (Join-Path $helperRoot "bin/$name.exe") -Destination (Join-Path $stage 'bin/') }
foreach ($directory in @('licenses','sources')) { Copy-Item -LiteralPath (Join-Path $helperRoot $directory) -Destination $stage -Recurse }
Copy-Item -LiteralPath (Join-Path $helperRoot 'FFMPEG_BUILD.json') -Destination $stage
foreach ($name in @('LICENSE','THIRD_PARTY_NOTICES.md')) { Copy-Item -LiteralPath (Join-Path $PSScriptRoot "../$name") -Destination (Join-Path $stage 'licenses/') }
foreach ($name in @('install-windows.ps1','windows-package-common.ps1','package-windows.ps1')) { Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $stage }
Write-Utf8File (Join-Path $stage 'forge.cmd') "@echo off`r`nsetlocal DisableDelayedExpansion`r`n`"%~dp0bin\forge.exe`" %*`r`nexit /b %errorlevel%`r`n"
$saved = @{}
foreach ($key in @('FORGE_JOB_STORE','FORGE_PLAN_STORE','GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS','GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS')) { $saved[$key]=[Environment]::GetEnvironmentVariable($key,'Process') }
try {
    $env:FORGE_JOB_STORE=Join-Path $stage '../doctor-jobs'
    $env:FORGE_PLAN_STORE=Join-Path $stage '../doctor-plans'
    $env:GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=Join-Path $stage '../empty-tools'
    $env:GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS='1'
    $doctor = Invoke-WindowsForgeJson (Join-Path $stage 'forge.cmd')
} finally { foreach ($key in $saved.Keys) { [Environment]::SetEnvironmentVariable($key,$saved[$key],'Process') } }
if ($doctor.build.target -ne 'x86_64-pc-windows-msvc') { throw 'The supplied Forge binary is not Windows x64 MSVC' }
if (!$AllowDevelopmentBuild -and ($doctor.build.dirty -ne $false -or $doctor.build.profile -ne 'release' -or @($doctor.build.features).Count -ne 0)) { throw 'Only a clean release with default features may be packaged without -AllowDevelopmentBuild' }
foreach ($name in @('ffmpeg','ffprobe')) {
    $actual = $doctor."$($name)Path"
    if (!$actual -or (Get-ComparableWindowsPath $actual) -ne (Get-ComparableWindowsPath (Join-Path $stage "bin/$name.exe"))) { throw "Public launcher could not discover its own bundled $name.exe" }
}
$info = [ordered]@{
    schemaVersion='1'; version=$doctor.cliVersion; commit=$doctor.build.gitCommit; target=$doctor.build.target
    build=$doctor.build; capabilities=$doctor.capabilities; binarySha256=(Get-Sha256 (Join-Path $stage 'bin/forge.exe'))
    distributionStatus='experimental-unsigned'; authenticodeSigned=$false
    ffmpegVersion=$receipt.ffmpegVersion; zlibVersion=$receipt.zlibVersion; helperBuild='FFMPEG_BUILD.json'
}
Write-Utf8File (Join-Path $stage 'BUILD_INFO.json') (($info | ConvertTo-Json -Depth 10)+"`n")
$manifest = @(Get-PayloadFiles $stage | Sort-Object Relative | ForEach-Object { (Get-Sha256 $_.Absolute)+'  '+$_.Relative })
Write-Utf8File (Join-Path $stage 'MANIFEST.sha256') (($manifest -join "`n")+"`n")
$null = Assert-Payload $stage
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($stage,$zipPath,[IO.Compression.CompressionLevel]::Optimal,$false)
$archiveHash = Get-Sha256 $zipPath
Write-Utf8File ($zipPath+'.sha256') ($archiveHash+'  '+[IO.Path]::GetFileName($zipPath)+"`n")
[ordered]@{schemaVersion='1';ok=$true;archive=$zipPath;sha256=$archiveHash;payload=$stage;build=$info} | ConvertTo-Json -Depth 12
