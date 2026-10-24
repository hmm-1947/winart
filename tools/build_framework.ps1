$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$out = "$root\boot"
$work = "$root\framework\out"
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$work\classes" | Out-Null
New-Item -ItemType Directory -Force $out | Out-Null
$src = Get-ChildItem "$root\framework\src" -Recurse -Filter *.java | ForEach-Object { $_.FullName }
javac --release 8 -Xlint:-options -d "$work\classes" $src
$cls = Get-ChildItem "$work\classes" -Recurse -Filter *.class | ForEach-Object { $_.FullName }
& "D:\Android\Sdk\build-tools\36.0.0\d8.bat" --min-api 21 --lib "D:\Android\Sdk\platforms\android-34\android.jar" --output $work @cls
Copy-Item "$work\classes.dex" "$out\framework.dex" -Force
Write-Output "FRAMEWORK_OK"
