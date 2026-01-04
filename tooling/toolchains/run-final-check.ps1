# 最終環境診断ラッパースクリプト
# PATH設定を含めて環境診断を実行

# MSYS2 LLVM
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH

# WinGet Links
$env:PATH = "$env:LOCALAPPDATA\Microsoft\WinGet\Links;" + $env:PATH

# 診断実行
& pwsh -NoLogo -File "$PSScriptRoot\check-windows-bootstrap-env.ps1" -OutputJson "reports\windows-env-check-final.json"
