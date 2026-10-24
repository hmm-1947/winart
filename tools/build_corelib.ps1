$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$src = "$root\aosp\libcore_src"
$work = "$root\corelib\out"
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$work\classes" | Out-Null
$icu = "$root\aosp\external\icu\android_icu4j"
$sp = "$src\ojluni;$src\luni;$src\libart;$src\dalvik;$root\corelib\stubs;$icu\src\main\java;$icu\libcore_bridge\src\java"
$roots = Get-Content "$root\corelib\roots.txt" | Where-Object { $_.Trim() -ne "" } | ForEach-Object {
  $r = $_.Trim().Replace(".", "/")
  foreach ($d in @("$src\ojluni", "$src\luni", "$src\libart", "$src\dalvik", "$root\corelib\stubs", "$icu\src\main\java", "$icu\libcore_bridge\src\java")) {
    $p = "$d\$r.java"
    if (Test-Path $p) { $p; break }
  }
}
$a = @("-source", "17", "-target", "17", "-Xlint:-options", "-nowarn", "-proc:none", "-Xmaxerrs", "5000", "-encoding", "UTF-8", "--patch-module", "java.base=$sp", "-d", "$work\classes") + $roots
$a | ForEach-Object { '"' + $_.Replace('\', '/') + '"' } | Set-Content "$work\args.txt" -Encoding ASCII
cmd /c "javac @$work\args.txt > $work\javac.log 2>&1"
Write-Output "JAVAC_DONE"
