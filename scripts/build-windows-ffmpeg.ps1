#requires -Version 7.0
<#! Build Forge's Windows helpers from pinned, unmodified FFmpeg/zlib sources.
    Downloads and compiler installation stay inside WorkDirectory. No PATH is persisted. #>
[CmdletBinding()]
param(
    [string]$WorkDirectory = (Join-Path $PSScriptRoot '../target/windows-toolchain'),
    [ValidateRange(1,64)][int]$Jobs = 8,
    [string]$SevenZip = '7z.exe',
    [switch]$ForceRebuild
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$work = [IO.Path]::GetFullPath($WorkDirectory)
if ($work -match '\s') { throw 'FFmpeg source builds require a work directory without spaces; packaged installs support spaces.' }
New-Item -ItemType Directory -Path $work -Force | Out-Null
$pins = @(
    @{ name='ffmpeg-8.1.2.tar.xz'; url='https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz'; sha256='464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c' },
    @{ name='zlib-1.3.2.tar.xz'; url='https://zlib.net/zlib-1.3.2.tar.xz'; fallbackUrl='https://zlib.net/fossils/zlib-1.3.2.tar.xz'; sha256='d7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3' },
    @{ name='w64devkit-x64-2.9.1.7z.exe'; url='https://github.com/skeeto/w64devkit/releases/download/v2.9.1/w64devkit-x64-2.9.1.7z.exe'; sha256='9208c19755cd4964b7915b9afcf02c66d493a4c870c4b3e83f6c538d9c1237a5' }
)
foreach ($pin in $pins) {
    $archive = Join-Path $work $pin.name
    if (!(Test-Path -LiteralPath $archive)) {
        try { Invoke-WebRequest -Uri $pin.url -OutFile $archive -MaximumRetryCount 3 } catch {
            if (!$pin.ContainsKey('fallbackUrl')) { throw }
            Invoke-WebRequest -Uri $pin.fallbackUrl -OutFile $archive -MaximumRetryCount 3
        }
    }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $pin.sha256) {
        throw "Pinned download SHA-256 mismatch: $archive"
    }
}
$compiler = Join-Path $work 'w64devkit'
if (!(Test-Path -LiteralPath (Join-Path $compiler 'bin/gcc.exe'))) {
    & $SevenZip x (Join-Path $work $pins[2].name) "-o$work" -y
    if ($LASTEXITCODE -ne 0) { throw 'w64devkit extraction failed' }
}
$output = Join-Path $work 'helpers'
New-Item -ItemType Directory -Path (Join-Path $output 'bin'),(Join-Path $output 'licenses'),(Join-Path $output 'sources') -Force | Out-Null
$recipe = @'
#!/bin/sh
set -eu
export SOURCE_DATE_EPOCH=0
work="$1"
jobs="$2"
cd "$work/zlib-1.3.2"
make -f win32/Makefile.gcc -j"$jobs" libz.a
cd "$work/ffmpeg-8.1.2"
sh ./configure --arch=x86_64 --target-os=mingw32 --cc=gcc --cxx=g++ \
  --disable-gpl --disable-nonfree --disable-autodetect --enable-zlib \
  --disable-doc --disable-debug --disable-ffplay --disable-network --disable-x86asm \
  --enable-static --disable-shared --disable-pthreads \
  --extra-cflags=-I../zlib-1.3.2 "--extra-ldflags=-L../zlib-1.3.2 -static -Wl,--no-insert-timestamp"
make -j"$jobs" V=1 ffmpeg.exe ffprobe.exe
'@
$recipePath = Join-Path $work 'build-helpers.sh'
[IO.File]::WriteAllText($recipePath, $recipe.Replace("`r`n", "`n") + "`n", [Text.UTF8Encoding]::new($false))
$recipeHash=(Get-FileHash -LiteralPath $recipePath -Algorithm SHA256).Hash.ToLowerInvariant()
$reuse=$false
$receiptPath=Join-Path $output 'FFMPEG_BUILD.json'
if (!$ForceRebuild -and (Test-Path -LiteralPath $receiptPath)) {
    try {
        $cached=Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        $reuse=$cached.sourceRecipeSha256 -eq $recipeHash
        foreach ($name in @('ffmpeg','ffprobe')) {
            $reuse=$reuse -and (Get-FileHash -LiteralPath (Join-Path $output "bin/$name.exe") -Algorithm SHA256).Hash.ToLowerInvariant() -eq $cached.binaries."$name.exe"
        }
        foreach ($pin in $pins[0..1]) {
            $reuse=$reuse -and (Get-FileHash -LiteralPath (Join-Path $output "sources/$($pin.name)") -Algorithm SHA256).Hash.ToLowerInvariant() -eq $pin.sha256
        }
    } catch { $reuse=$false }
}
$sourceRoot=$null
if (!$reuse) {
    # Never compile a previously edited or partially generated source directory.
    $sourceRoot=Join-Path $work ('build-'+[Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $sourceRoot | Out-Null
    foreach ($name in @('ffmpeg-8.1.2','zlib-1.3.2')) {
        & "$env:SystemRoot\System32\tar.exe" -xf (Join-Path $work "$name.tar.xz") -C $sourceRoot
        if ($LASTEXITCODE -ne 0) { throw "Source extraction failed: $name" }
    }
}
$previousPath = $env:PATH
try {
    $env:PATH = (Join-Path $compiler 'bin') + ';' + $previousPath
    if (!$reuse) {
        & (Join-Path $compiler 'bin/sh.exe') $recipePath ($sourceRoot.Replace('\','/')) $Jobs *> (Join-Path $work 'build.log')
        if ($LASTEXITCODE -ne 0) { throw "FFmpeg build failed; see $work\build.log" }
    }
    $compilerVersion = (& (Join-Path $compiler 'bin/gcc.exe') --version | Select-Object -First 1)
} finally { $env:PATH = $previousPath }
foreach ($name in @('ffmpeg','ffprobe')) {
    if (!$reuse) { Copy-Item -LiteralPath (Join-Path $sourceRoot "ffmpeg-8.1.2/$name.exe") -Destination (Join-Path $output "bin/$name.exe") }
    $version = & (Join-Path $output "bin/$name.exe") -version
    if ($LASTEXITCODE -ne 0 -or ($version -join "`n") -match '--enable-(gpl|nonfree)') { throw "Unexpected helper build: $name" }
    [IO.File]::WriteAllLines((Join-Path $output "licenses/$name-version.txt"), [string[]]$version)
}
if (!$reuse) {
    Copy-Item -LiteralPath (Join-Path $sourceRoot 'ffmpeg-8.1.2/COPYING.LGPLv2.1') -Destination (Join-Path $output 'licenses/FFMPEG-LGPL-2.1.txt')
    Copy-Item -LiteralPath (Join-Path $sourceRoot 'zlib-1.3.2/LICENSE') -Destination (Join-Path $output 'licenses/ZLIB-LICENSE.txt')
}
Copy-Item -LiteralPath (Join-Path $compiler 'COPYING.MinGW-w64-runtime.txt') -Destination (Join-Path $output 'licenses/')
Copy-Item -LiteralPath $recipePath -Destination (Join-Path $output 'sources/')
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $output 'sources/')
foreach ($pin in $pins[0..1]) { Copy-Item -LiteralPath (Join-Path $work $pin.name) -Destination (Join-Path $output 'sources/') }
$receipt = [ordered]@{
    schemaVersion='1'; ffmpegVersion='8.1.2'; zlibVersion='1.3.2'; compiler=$compilerVersion
    downloads=$pins; sourceModified=$false; sourceRecipeSha256=$recipeHash
    configuration='LGPL-only; no autodetected libraries; static zlib; network and external x86 assembly disabled'
    binaries=@{}
}
foreach ($name in @('ffmpeg','ffprobe')) { $receipt.binaries["$name.exe"] = (Get-FileHash -LiteralPath (Join-Path $output "bin/$name.exe") -Algorithm SHA256).Hash.ToLowerInvariant() }
$receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'FFMPEG_BUILD.json') -Encoding utf8
Write-Output "Verified helpers: $output"
