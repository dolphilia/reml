# PowerShellプロファイルテストスクリプト

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "PowerShellプロファイル機能テスト" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# プロファイル読み込み
. $PROFILE

Write-Host "`n1. 7-Zip PATH確認" -ForegroundColor Yellow
Write-Host "-------------------"
$7zPath = (Get-Command 7z -ErrorAction SilentlyContinue).Source
if ($7zPath) {
    Write-Host "✅ 7z.exe 検出: $7zPath" -ForegroundColor Green
    & 7z | Select-Object -First 2
} else {
    Write-Host "❌ 7z.exe が見つかりません" -ForegroundColor Red
}

Write-Host "`n2. MSVC環境アクティベーション" -ForegroundColor Yellow
Write-Host "------------------------------"
reml-msvc-env

Write-Host "`n3. cl.exe確認" -ForegroundColor Yellow
Write-Host "--------------"
$clPath = (Get-Command cl -ErrorAction SilentlyContinue).Source
if ($clPath) {
    Write-Host "✅ cl.exe 検出: $clPath" -ForegroundColor Green
} else {
    Write-Host "❌ cl.exe が見つかりません（reml-msvc-envを実行してください）" -ForegroundColor Red
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "テスト完了" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan
