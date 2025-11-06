# Reml Windows toolchain setup helper
#
# このスクリプトはローカル開発環境および CI で共通利用できるよう、
# PowerShell プロファイルの PATH 初期化と MSVC ツールチェーンの有効化処理を再現します。
# 既定では check-windows-bootstrap-env.ps1 を連続実行し、診断結果を出力します。
#
# 使い方（環境のみ設定）:
#   .\tooling\toolchains\setup-windows-toolchain.ps1 -NoCheck
#   . .\tooling\toolchains\setup-windows-toolchain.ps1 -NoCheck   # 現在のセッションに適用
#
# 使い方（環境設定 + 診断 JSON 出力）:
#   pwsh -NoLogo -File tooling/toolchains/setup-windows-toolchain.ps1 -CheckOutputJson reports/windows-env-check.json
#

[CmdletBinding()]
param(
    [switch]$NoCheck,
    [string]$CheckOutputJson = "",
    [switch]$SkipMsvc,
    [switch]$Quiet
)

Set-StrictMode -Version Latest

function Add-RemlPathFront {
    param([string]$PathEntry)

    if ([string]::IsNullOrWhiteSpace($PathEntry)) {
        return
    }

    $expanded = [System.Environment]::ExpandEnvironmentVariables($PathEntry.Trim())
    $normalized = $expanded.TrimEnd('\')

    if (-not (Test-Path -LiteralPath $normalized -PathType Container)) {
        return
    }

    $segments = @()
    if ($env:PATH) {
        $segments = $env:PATH -split ';'
    }

    $filtered = $segments | Where-Object { $_.TrimEnd('\') -and ($_.TrimEnd('\') -ine $normalized) }
    $env:PATH = [string]::Join(';', @($normalized) + $filtered)
}

function Initialize-RemlPreferredPaths {
    $preferredPaths = @(
        "$env:LOCALAPPDATA\opam\reml-521\bin",
        "$env:LOCALAPPDATA\Microsoft\WinGet\Links",
        'C:\Program Files\Git\cmd',
        'C:\Program Files\Git\bin',
        'C:\msys64\mingw64\bin',
        'C:\Program Files\7-Zip',
        'C:\Program Files\LLVM\bin',
        'C:\llvm\LLVM-19.1.1-Windows-X64\bin',
        'C:\Program Files\CMake\bin'
    )

    foreach ($path in $preferredPaths) {
        Add-RemlPathFront -PathEntry $path
    }
}

function Invoke-RemlMsvcEnvironment {
    param([switch]$Quiet)

    $remlMsvc = Get-Command -Name reml-msvc-env -ErrorAction SilentlyContinue
    if ($remlMsvc) {
        & reml-msvc-env | Out-Null
        if (-not $Quiet) {
            Write-Host "MSVC 環境を reml-msvc-env でアクティベートしました。" -ForegroundColor Green
        }
        return $true
    }

    $vcvarsCandidates = @(
        'C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat',
        'C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat',
        'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
    )

    foreach ($vcvars in $vcvarsCandidates) {
        if (-not (Test-Path -LiteralPath $vcvars)) {
            continue
        }

        try {
            $vcvarsOutput = & cmd /c "`"$vcvars`" && set"
        } catch {
            continue
        }

        if (-not $vcvarsOutput) {
            continue
        }

        foreach ($line in $vcvarsOutput -split "`r?`n") {
            if ($line -match '^([^=]+)=(.*)$') {
                $name = $matches[1]
                $value = $matches[2]
                if ($name -in @('PATH', 'INCLUDE', 'LIB', 'LIBPATH', 'WindowsSDKVersion', 'VSINSTALLDIR', 'VCINSTALLDIR')) {
                    [System.Environment]::SetEnvironmentVariable($name, $value, 'Process')
                }
            }
        }

        if (-not $Quiet) {
            Write-Host ("MSVC 環境を vcvars64.bat でアクティベートしました: {0}" -f (Split-Path -Path $vcvars -Parent)) -ForegroundColor Green
        }
        return $true
    }

    if (-not $Quiet) {
        Write-Warning "MSVC 環境を自動設定できませんでした。Visual Studio Build Tools のインストールと PATH を確認してください。"
    }
    return $false
}

function Invoke-RemlWindowsToolchainSetup {
    param(
        [switch]$SkipMsvc,
        [switch]$Quiet
    )

    Initialize-RemlPreferredPaths
    if (-not $SkipMsvc) {
        Invoke-RemlMsvcEnvironment -Quiet:$Quiet | Out-Null
    }
    Initialize-RemlPreferredPaths
}

Invoke-RemlWindowsToolchainSetup -SkipMsvc:$SkipMsvc -Quiet:$Quiet

if (-not $NoCheck) {
    $checkScript = Join-Path $PSScriptRoot 'check-windows-bootstrap-env.ps1'
    if (-not (Test-Path -LiteralPath $checkScript)) {
        throw "check-windows-bootstrap-env.ps1 が見つかりません。($checkScript)"
    }

    $checkArgs = @{}
    if (-not [string]::IsNullOrWhiteSpace($CheckOutputJson)) {
        $checkArgs['OutputJson'] = $CheckOutputJson
    }

    if ($checkArgs.Count -gt 0) {
        & $checkScript @checkArgs
    } else {
        & $checkScript
    }
}
