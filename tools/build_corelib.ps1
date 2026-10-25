$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$src = "$root\aosp\libcore_src"
$work = "$root\corelib\out"
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$work\classes" | Out-Null
$icu = "$root\aosp\external\icu\android_icu4j"
$sp = "$root\corelib\stubs;$src\ojluni;$src\luni;$src\libart;$src\dalvik;$icu\src\main\java;$icu\libcore_bridge\src\java"
$roots = Get-Content "$root\corelib\roots.txt" | Where-Object { $_.Trim() -ne "" } | ForEach-Object {
  $r = $_.Trim().Replace(".", "/")
  foreach ($d in @("$root\corelib\stubs", "$src\ojluni", "$src\luni", "$src\libart", "$src\dalvik", "$icu\src\main\java", "$icu\libcore_bridge\src\java")) {
    $p = "$d\$r.java"
    if (Test-Path $p) { $p; break }
  }
}
$a = @("-XDstringConcat=inline", "-source", "17", "-target", "17", "-Xlint:-options", "-nowarn", "-proc:none", "-Xmaxerrs", "5000", "-encoding", "UTF-8", "--patch-module", "java.base=$sp", "-d", "$work\classes") + $roots
$a | ForEach-Object { '"' + $_.Replace('\', '/') + '"' } | Set-Content "$work\args.txt" -Encoding ASCII
cmd /c "javac @$work\args.txt > $work\javac.log 2>&1"
Write-Output "JAVAC_DONE"
$cls = Get-ChildItem "$work\classes" -Recurse -Filter *.class | ForEach-Object { $_.FullName }
$cls | Set-Content "$work\d8args.txt" -Encoding ASCII
New-Item -ItemType Directory -Force "$work\dex" | Out-Null
cmd /c "D:\Android\Sdk\build-tools\36.0.0\d8.bat --min-api 21 --lib D:\Android\Sdk\platforms\android-34\android.jar --output $work\dex @$work\d8args.txt > $work\d8.log 2>&1"
Copy-Item "$work\dex\classes.dex" "$root\boot\core.dex" -Force
Write-Output "CORE_DEX_DONE"
