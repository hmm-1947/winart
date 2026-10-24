$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$sdk = "D:\Android\Sdk"
$bt = "$sdk\build-tools\36.0.0"
$jar = "$sdk\platforms\android-34\android.jar"
$out = "$here\out"
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$out\classes" | Out-Null
$src = Get-ChildItem "$here\src" -Recurse -Filter *.java | ForEach-Object { $_.FullName }
javac --release 8 -classpath $jar -d "$out\classes" $src
$cls = Get-ChildItem "$out\classes" -Recurse -Filter *.class | ForEach-Object { $_.FullName }
& "$bt\d8.bat" --min-api 21 --lib $jar --output $out @cls
& "$bt\aapt2.exe" link -I $jar --manifest "$here\AndroidManifest.xml" --min-sdk-version 21 --target-sdk-version 34 -o "$out\base.apk"
python -c "import zipfile,shutil;shutil.copy(r'$out\base.apk',r'$out\WinCalc.apk');z=zipfile.ZipFile(r'$out\WinCalc.apk','a',zipfile.ZIP_DEFLATED);z.write(r'$out\classes.dex','classes.dex');z.close()"
Write-Output "BUILD_OK $out\WinCalc.apk"
