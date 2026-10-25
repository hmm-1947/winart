$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$w = "$root\runtime\target\release\winart.exe"
$fail = 0
function Run($name, $a) {
  & $w @a | Out-Null
  if ($LASTEXITCODE -ne 0) { Write-Output "FAIL $name"; $script:fail++ } else { Write-Output "PASS $name" }
}
Run "WinCalc" @("test","$root\testapp\out\WinCalc.apk","--expect","5","7","*","6","=","+","8","=","/","5","=","C","9","-","4","=")
Run "LayoutTest" @("test","$root\testapp2\out\LayoutTest.apk","--seen","Taps: 2","Tap","Tap")
Run "StringTest" @("test","$root\testapp3\out\StringTest.apk","--seen","STRINGTEST ALL PASS")
Run "ThreadTest" @("test","$root\testapp4\out\ThreadTest.apk","--seen","THREADTEST ALL PASS","--wait","8000")
Run "ThreadTest-gc" @("test","$root\testapp4\out\ThreadTest.apk","--gc","--seen","THREADTEST ALL PASS","--wait","8000")
exit $fail
