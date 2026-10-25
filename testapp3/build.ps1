$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$sdk = "D:\Android\Sdk"
$bt = "$sdk\build-tools\36.0.0"
$jar = "$sdk\platforms\android-34\android.jar"
$out = "$here\out"
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$out\classes" | Out-Null
New-Item -ItemType Directory -Force "$out\gen" | Out-Null
& "$bt\aapt2.exe" compile --dir "$here\res" -o "$out\res.zip"
if ($LASTEXITCODE -ne 0) { throw "aapt2 compile failed" }
& "$bt\aapt2.exe" link -I $jar --manifest "$here\AndroidManifest.xml" --min-sdk-version 21 --target-sdk-version 34 --java "$out\gen" -o "$out\base.apk" "$out\res.zip"
if ($LASTEXITCODE -ne 0) { throw "aapt2 link failed" }
$src = Get-ChildItem "$here\src","$out\gen" -Recurse -Filter *.java | ForEach-Object { $_.FullName }
javac --release 8 -encoding UTF-8 -classpath $jar -d "$out\classes" $src
if ($LASTEXITCODE -ne 0) { throw "javac failed" }
$cls = Get-ChildItem "$out\classes" -Recurse -Filter *.class | ForEach-Object { $_.FullName }
& "$bt\d8.bat" --min-api 21 --lib $jar --output $out @cls
if ($LASTEXITCODE -ne 0) { throw "d8 failed" }
python -c "import zipfile,shutil;shutil.copy(r'$out\base.apk',r'$out\StringTest.apk');z=zipfile.ZipFile(r'$out\StringTest.apk','a',zipfile.ZIP_DEFLATED);z.write(r'$out\classes.dex','classes.dex');z.close()"
Write-Output "BUILD_OK $out\StringTest.apk"
