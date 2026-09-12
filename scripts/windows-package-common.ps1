#requires -Version 5.1
# Shared by the Windows packager, installer and artifact checks. No system changes.
function Write-Utf8File([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}
function Get-Sha256([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Get-ComparableWindowsPath([string]$Path) {
    $full=[IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\?\UNC\',[StringComparison]::OrdinalIgnoreCase)) { return '\\'+$full.Substring(8) }
    if ($full.StartsWith('\\?\')) { return $full.Substring(4) }
    return $full
}
function Assert-RegularDirectory([string]$Path) {
    $cursor = [IO.DirectoryInfo]::new([IO.Path]::GetFullPath($Path))
    while ($null -ne $cursor) {
        if ($cursor.Exists -and ($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Directory traverses a reparse point: $($cursor.FullName)" }
        $cursor = $cursor.Parent
    }
}
function Get-PayloadFiles([string]$Root) {
    $base = [IO.Path]::GetFullPath($Root).TrimEnd('\','/') + [IO.Path]::DirectorySeparatorChar
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($base)
    while ($pending.Count -gt 0) {
        foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Payload contains a reparse point: $($item.FullName)" }
            if ($item.PSIsContainer) { $pending.Push($item.FullName) } else {
                [pscustomobject]@{ Relative=$item.FullName.Substring($base.Length).Replace('\','/'); Absolute=$item.FullName }
            }
        }
    }
}
function Test-SafeArchivePath([string]$Name) {
    if (!$Name -or $Name.Contains('\') -or $Name.Contains(':') -or $Name.StartsWith('/')) { return $false }
    foreach ($part in $Name.TrimEnd('/').Split('/')) {
        if (!$part -or $part -in @('.','..') -or $part -match '[<>"|?*\x00-\x1f]' -or $part.EndsWith('.') -or $part.EndsWith(' ') -or $part -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)') { return $false }
    }
    return $true
}
function Expand-VerifiedZip([string]$Archive, [string]$Destination) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        [long]$total = 0
        foreach ($entry in $zip.Entries) {
            if (!(Test-SafeArchivePath $entry.FullName) -or !$names.Add($entry.FullName.TrimEnd('/'))) { throw "Unsafe or duplicate ZIP entry: $($entry.FullName)" }
            if ((($entry.ExternalAttributes -shr 16) -band 0xf000) -eq 0xa000) { throw 'ZIP symbolic links are not supported' }
            $total += $entry.Length
            if ($total -gt 2GB -or $zip.Entries.Count -gt 10000) { throw 'ZIP exceeds package limits' }
        }
    } finally { $zip.Dispose() }
    [IO.Compression.ZipFile]::ExtractToDirectory($Archive, $Destination)
}
function Assert-Payload([string]$Root) {
    $manifestPath = Join-Path $Root 'MANIFEST.sha256'
    if (!(Test-Path -LiteralPath $manifestPath -PathType Leaf)) { throw 'Missing MANIFEST.sha256' }
    $expected = @{}
    foreach ($line in [IO.File]::ReadAllLines($manifestPath)) {
        if ($line -notmatch '^([a-f0-9]{64})  (.+)$') { throw 'Malformed payload checksum manifest' }
        $hash = $Matches[1]; $relative = $Matches[2]
        if (!(Test-SafeArchivePath $relative) -or $expected.ContainsKey($relative) -or $relative -eq 'MANIFEST.sha256') { throw "Unsafe checksum entry: $relative" }
        $expected[$relative] = $hash
    }
    $actual = @(Get-PayloadFiles $Root | Where-Object Relative -NE 'MANIFEST.sha256')
    if ($actual.Count -ne $expected.Count) { throw 'Payload file inventory differs from checksum manifest' }
    foreach ($file in $actual) {
        if (!$expected.ContainsKey($file.Relative) -or (Get-Sha256 $file.Absolute) -ne $expected[$file.Relative]) { throw "Payload checksum mismatch: $($file.Relative)" }
    }
    foreach ($required in @('BUILD_INFO.json','forge.cmd','bin/forge.exe','bin/ffmpeg.exe','bin/ffprobe.exe','FFMPEG_BUILD.json','install-windows.ps1','windows-package-common.ps1','licenses/LICENSE','licenses/THIRD_PARTY_NOTICES.md','licenses/FFMPEG-LGPL-2.1.txt','licenses/ZLIB-LICENSE.txt','licenses/COPYING.MinGW-w64-runtime.txt','sources/ffmpeg-8.1.2.tar.xz','sources/zlib-1.3.2.tar.xz','sources/build-helpers.sh','sources/build-windows-ffmpeg.ps1')) {
        if (!$expected.ContainsKey($required)) { throw "Required payload file missing: $required" }
    }
    $info = [IO.File]::ReadAllText((Join-Path $Root 'BUILD_INFO.json')) | ConvertFrom-Json
    if ($info.schemaVersion -ne '1' -or $info.target -ne 'x86_64-pc-windows-msvc' -or $info.version -notmatch '^\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?$') { throw 'Unsupported BUILD_INFO.json identity' }
    if ($info.binarySha256 -ne $expected['bin/forge.exe']) { throw 'BUILD_INFO binary hash mismatch' }
    return $info
}
function Invoke-WindowsForgeJson([string]$Launcher, [string[]]$Arguments = @('doctor','--json')) {
    $result = & $Launcher @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Forge failed through public launcher: $Launcher" }
    $value = ($result -join "`n") | ConvertFrom-Json
    if (!$value.ok) { throw "Forge returned an error through public launcher: $Launcher" }
    return $value.data
}
