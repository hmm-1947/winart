$ErrorActionPreference = "Continue"
$branch = "android15-release"
$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "aosp"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$repos = @(
  "art",
  "libcore",
  "libnativehelper",
  "dalvik",
  "system/libbase",
  "system/libziparchive",
  "system/logging",
  "system/core",
  "system/unwinding",
  "external/zlib",
  "external/lz4",
  "external/zstd",
  "external/vixl",
  "external/fmtlib",
  "external/tinyxml2",
  "external/expat",
  "external/icu",
  "external/googletest"
)
foreach ($r in $repos) {
  $target = Join-Path $dest $r
  if (Test-Path (Join-Path $target ".git")) { Write-Output "skip $r"; continue }
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
  Write-Output "clone $r"
  git clone --depth 1 --single-branch --branch $branch --filter=blob:none "https://android.googlesource.com/platform/$r" $target
}
Write-Output "FETCH_DONE"
