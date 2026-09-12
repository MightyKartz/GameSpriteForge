#requires -Version 7.0
<#! Exercise the real installed public launcher, not a payload PATH shortcut. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Archive,
    [Parameter(Mandatory=$true)][string]$Output,
    [string]$PreviousArchive,
    [string]$Godot,
    [switch]$AllowDevelopmentBuild
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'windows-package-common.ps1')
$root=[IO.Path]::GetFullPath($Output)
if (Test-Path -LiteralPath $root) { throw 'Evidence output must be a new directory' }
New-Item -ItemType Directory -Path $root | Out-Null
$package=(Resolve-Path -LiteralPath $Archive).Path
$hash=Get-Sha256 $package
$installation=Join-Path $root 'installation with spaces'
$installer=Join-Path $PSScriptRoot 'install-windows.ps1'
$cases=[Collections.Generic.List[object]]::new()
$saved=@{}
foreach ($key in @('FORGE_JOB_STORE','FORGE_PLAN_STORE','FORGE_GODOT_PATH','GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS','GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS')) { $saved[$key]=[Environment]::GetEnvironmentVariable($key,'Process') }
function Install-Package([string]$File,[string]$Digest,[string]$Directory) {
    $options=@{Archive=$File;Sha256=$Digest;InstallDirectory=$Directory;AllowDevelopmentBuild=$AllowDevelopmentBuild}
    return (& $installer @options | ConvertFrom-Json)
}
function Check-Installed($Installed) {
    $info=Assert-Payload $Installed.payload
    $data=Invoke-WindowsForgeJson $Installed.launcher
    if ($data.cliVersion -ne $info.version -or $data.build.gitCommit -ne $info.commit -or (Get-Sha256 (Join-Path $Installed.payload 'bin/forge.exe')) -ne $info.binarySha256) { throw 'Compiled identity differs from installed BUILD_INFO' }
    foreach ($name in @('ffmpeg','ffprobe')) {
        $actual=$data."$($name)Path"
        if (!$actual -or (Get-ComparableWindowsPath $actual) -ne (Get-ComparableWindowsPath (Join-Path $Installed.payload "bin/$name.exe"))) { throw "Installed launcher used external or missing $name" }
    }
    if ($Godot -and (!$data.godotSupported -or (Get-ComparableWindowsPath $data.godotPath) -ne (Get-ComparableWindowsPath $Godot))) { throw 'Explicit Godot runtime was not detected as supported' }
    return $data
}
try {
    $env:FORGE_JOB_STORE=Join-Path $root 'jobs'
    $env:FORGE_PLAN_STORE=Join-Path $root 'plans'
    $env:GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=Join-Path $root 'empty tools'
    $env:GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS='1'
    New-Item -ItemType Directory -Path $env:GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS | Out-Null
    if ($Godot) { $env:FORGE_GODOT_PATH=(Resolve-Path -LiteralPath $Godot).Path } else { $env:FORGE_GODOT_PATH=$null }
    # Copying the actual archive to a spaced path verifies argument forwarding.
    $spacedPackage=Join-Path $root 'current package.zip'
    Copy-Item -LiteralPath $package -Destination $spacedPackage
    $fresh=Install-Package $spacedPackage $hash $installation
    if ($fresh.action -ne 'installed') { throw 'Fresh install did not report installed' }
    $doctor=Check-Installed $fresh
    $cases.Add(@{name='fresh_install_and_bundled_discovery';passed=$true;launcher=$fresh.launcher})
    $beforeHash=Get-Sha256 $fresh.launcher
    $beforeTime=(Get-Item -LiteralPath $fresh.launcher).LastWriteTimeUtc.Ticks
    $again=Install-Package $spacedPackage $hash $installation
    if ($again.action -ne 'unchanged' -or (Get-Sha256 $again.launcher) -ne $beforeHash -or (Get-Item -LiteralPath $again.launcher).LastWriteTimeUtc.Ticks -ne $beforeTime) { throw 'Reinstall was not idempotent' }
    $null=Check-Installed $again
    $cases.Add(@{name='same_archive_reinstall_preserves_launcher';passed=$true})
    $rejected=$false
    try { $null=Install-Package $spacedPackage ('0'*64) $installation } catch { $rejected=$_.Exception.Message -like '*Archive SHA-256 mismatch*' }
    if (!$rejected -or (Get-Sha256 $fresh.launcher) -ne $beforeHash) { throw 'Bad archive checksum changed active installation' }
    $cases.Add(@{name='bad_checksum_rejected_before_activation';passed=$true})
    $traversal=Join-Path $root 'traversal.zip'
    $zip=[IO.Compression.ZipFile]::Open($traversal,[IO.Compression.ZipArchiveMode]::Create)
    try {
        $writer=[IO.StreamWriter]::new($zip.CreateEntry('../escaped.txt').Open())
        try { $writer.Write('must never be extracted') } finally { $writer.Dispose() }
    } finally { $zip.Dispose() }
    $rejected=$false
    try { $null=Install-Package $traversal (Get-Sha256 $traversal) $installation } catch { $rejected=$_.Exception.Message -like '*Unsafe or duplicate ZIP entry*' }
    if (!$rejected -or (Get-Sha256 $fresh.launcher) -ne $beforeHash -or (Test-Path -LiteralPath (Join-Path $installation 'escaped.txt'))) { throw 'ZIP traversal was not safely rejected' }
    $cases.Add(@{name='zip_traversal_rejected_before_extraction';passed=$true})
    $junctionTarget=Join-Path $root 'junction-target'
    $junction=Join-Path $root 'junction-install-root'
    New-Item -ItemType Directory -Path $junctionTarget | Out-Null
    Write-Utf8File (Join-Path $junctionTarget 'sentinel.txt') 'unchanged'
    New-Item -ItemType Junction -Path $junction -Value $junctionTarget | Out-Null
    $rejected=$false
    try { $null=Install-Package $spacedPackage $hash $junction } catch { $rejected=$_.Exception.Message -like '*Directory traverses a reparse point*' }
    if (!$rejected -or @(Get-ChildItem -LiteralPath $junctionTarget).Count -ne 1) { throw 'Junction install root was not refused without changes' }
    $cases.Add(@{name='junction_root_rejected_without_admin_or_symlink_privilege';passed=$true})
    $originalLauncher=[IO.File]::ReadAllText($fresh.launcher)
    Write-Utf8File $fresh.launcher ($originalLauncher+'rem user modification'+"`r`n")
    $modifiedHash=Get-Sha256 $fresh.launcher
    $rejected=$false
    try { $null=Install-Package $spacedPackage $hash $installation } catch { $rejected=$_.Exception.Message -like '*Refusing to overwrite a modified launcher*' }
    if (!$rejected -or (Get-Sha256 $fresh.launcher) -ne $modifiedHash) { throw 'Modified public launcher was overwritten' }
    Write-Utf8File $fresh.launcher $originalLauncher
    $cases.Add(@{name='modified_launcher_preserved';passed=$true})
    $upgradeKind='previous_executable_archive'
    if ($PreviousArchive) { $previous=(Resolve-Path -LiteralPath $PreviousArchive).Path } else {
        # There was no official Windows release before this installer. Do not fake
        # a different CLI version or commit: change only explicit packaging metadata.
        $upgradeKind='synthetic_previous_packaging_revision_same_executable'
        $fixture=Join-Path $root 'previous-package-fixture'
        Expand-VerifiedZip $package $fixture
        $oldInfo=[IO.File]::ReadAllText((Join-Path $fixture 'BUILD_INFO.json')) | ConvertFrom-Json
        $oldInfo | Add-Member -NotePropertyName packagingTestFixture -NotePropertyValue 'previous-revision-same-compiled-cli'
        Write-Utf8File (Join-Path $fixture 'BUILD_INFO.json') (($oldInfo | ConvertTo-Json -Depth 10)+"`n")
        $manifest=@(Get-PayloadFiles $fixture | Where-Object Relative -NE 'MANIFEST.sha256' | Sort-Object Relative | ForEach-Object { (Get-Sha256 $_.Absolute)+'  '+$_.Relative })
        Write-Utf8File (Join-Path $fixture 'MANIFEST.sha256') (($manifest -join "`n")+"`n")
        $previous=Join-Path $root 'previous packaging fixture.zip'
        [IO.Compression.ZipFile]::CreateFromDirectory($fixture,$previous,[IO.Compression.CompressionLevel]::Optimal,$false)
    }
    $upgradeRoot=Join-Path $root 'upgrade installation with spaces'
    $old=Install-Package $previous (Get-Sha256 $previous) $upgradeRoot
    $null=Check-Installed $old
    $oldIdentity=Assert-Payload $old.payload
    $oldLauncher=[IO.File]::ReadAllBytes($old.launcher)
    $updated=Install-Package $spacedPackage $hash $upgradeRoot
    if ($updated.action -ne 'updated' -or !$updated.backupPath -or !(Test-Path -LiteralPath $old.payload) -or (Get-Sha256 $updated.backupPath) -ne ([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($oldLauncher)).ToLowerInvariant())) { throw 'Upgrade did not preserve old payload and public launcher backup' }
    $null=Check-Installed $updated
    $newIdentity=Assert-Payload $updated.payload
    $binaryChanged=$oldIdentity.binarySha256 -ne $newIdentity.binarySha256
    if ($PreviousArchive -and !$binaryChanged) { $upgradeKind='supplied_previous_packaging_revision_same_executable' }
    $cases.Add(@{name='upgrade_preserves_prior_payload_and_launcher_backup';passed=$true;kind=$upgradeKind;backup=$updated.backupPath;binaryChanged=$binaryChanged;previousBinarySha256=$oldIdentity.binarySha256;currentBinarySha256=$newIdentity.binarySha256;previousBuild=$oldIdentity.build;currentBuild=$newIdentity.build})
    $tamperedRoot=Join-Path $root 'tampered-payload'
    Expand-VerifiedZip $package $tamperedRoot
    Write-Utf8File (Join-Path $tamperedRoot 'licenses/LICENSE') 'changed payload byte inventory'
    $tampered=Join-Path $root 'tampered payload.zip'
    [IO.Compression.ZipFile]::CreateFromDirectory($tamperedRoot,$tampered,[IO.Compression.CompressionLevel]::Optimal,$false)
    $rejected=$false
    try { $null=Install-Package $tampered (Get-Sha256 $tampered) $installation } catch { $rejected=$_.Exception.Message -like '*Payload checksum mismatch*' }
    if (!$rejected -or (Get-Sha256 $fresh.launcher) -ne $beforeHash) { throw 'Inner manifest corruption changed the active launcher' }
    $cases.Add(@{name='inner_payload_checksum_mismatch_rejected';passed=$true})
    # Use helpers reported by the launcher under the empty external search path.
    $media=Join-Path $root 'media smoke'
    New-Item -ItemType Directory -Path $media | Out-Null
    & $doctor.ffmpegPath -hide_banner -loglevel error -f lavfi -i 'testsrc2=size=64x64:rate=8' -t 1 -c:v mpeg4 -y (Join-Path $media 'sample.mp4')
    if ($LASTEXITCODE -ne 0) { throw 'Bundled FFmpeg MP4 encoding failed' }
    & $doctor.ffmpegPath -hide_banner -loglevel error -i (Join-Path $media 'sample.mp4') -vf 'select=not(mod(n\,2))' -vsync 0 -y (Join-Path $media 'frame-%02d.png')
    if ($LASTEXITCODE -ne 0 -or @(Get-ChildItem -LiteralPath $media -Filter 'frame-*.png').Count -ne 4) { throw 'Bundled FFmpeg frame extraction failed' }
    & $doctor.ffmpegPath -hide_banner -loglevel error -i (Join-Path $media 'sample.mp4') -vf fps=8 -y (Join-Path $media 'preview.gif')
    if ($LASTEXITCODE -ne 0) { throw 'Bundled FFmpeg GIF generation failed' }
    $probe=(& $doctor.ffprobePath -v error -show_entries stream=width,height,codec_name -of json (Join-Path $media 'preview.gif')) -join "`n" | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $probe.streams[0].width -ne 64 -or $probe.streams[0].height -ne 64) { throw 'Bundled FFprobe could not read generated GIF' }
    $cases.Add(@{name='bundled_mp4_png_gif_processing';passed=$true;frames=4})
    $summary=[ordered]@{schemaVersion='1';ok=$true;archive=$package;archiveSha256=$hash;doctor=$doctor;upgradeKind=$upgradeKind;cases=$cases;providerRequestsExecuted=0;symlinksRequired=$false;administratorRequired=$false;globalPathModified=$false}
    Write-Utf8File (Join-Path $root 'summary.json') (($summary | ConvertTo-Json -Depth 15)+"`n")
    $summary | ConvertTo-Json -Depth 15
} catch {
    Write-Utf8File (Join-Path $root 'summary.json') ((@{schemaVersion='1';ok=$false;error=$_.Exception.Message;cases=$cases} | ConvertTo-Json -Depth 12)+"`n")
    throw
} finally { foreach ($key in $saved.Keys) { [Environment]::SetEnvironmentVariable($key,$saved[$key],'Process') } }
