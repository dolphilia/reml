<#
.SYNOPSIS
    Windows 向け監査サマリー生成スクリプト (Iterator Stage + FFI Bridge)

.DESCRIPTION
    Linux/macOS の sync-iterator-audit.sh の Windows PowerShell 版。
    Iterator Stage と FFI Bridge の監査ログを収集し、pass_rate を検証します。

.PARAMETER MetricsPath
    collect-iterator-audit-metrics.py が生成した JSON ファイルのパス (必須)

.PARAMETER VerifyLogPath
    verify_llvm_ir.sh のログファイルのパス (必須)

.PARAMETER AuditPath
    AuditEnvelope JSON ファイル (単一ファイルまたは JSON Lines 形式)

.PARAMETER OutputPath
    Markdown サマリーの出力先 (既定: reports\iterator-stage-summary.md)

.PARAMETER FfiBridgeMetricsPath
    FFI ブリッジメトリクス JSON (既定: build\ffi-bridge-metrics.json)

.EXAMPLE
    .\Sync-AuditMetrics.ps1 -MetricsPath build\iterator-metrics.json -VerifyLogPath build\verify.log

.EXAMPLE
    .\Sync-AuditMetrics.ps1 `
        -MetricsPath build\iterator-metrics.json `
        -VerifyLogPath build\verify.log `
        -AuditPath build\audit.jsonl `
        -FfiBridgeMetricsPath build\ffi-bridge-metrics.json `
        -OutputPath reports\audit-summary.md

.NOTES
    Phase 2-3 FFI Contract Extension
    Windows MSYS2 環境での監査ログ収集を目的とした PowerShell ラッパー
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidateScript({Test-Path $_ -PathType Leaf})]
    [string]$MetricsPath,

    [Parameter(Mandatory=$true)]
    [ValidateScript({Test-Path $_ -PathType Leaf})]
    [string]$VerifyLogPath,

    [Parameter(Mandatory=$false)]
    [string]$AuditPath,

    [Parameter(Mandatory=$false)]
    [string]$OutputPath = "reports\iterator-stage-summary.md",

    [Parameter(Mandatory=$false)]
    [string]$FfiBridgeMetricsPath = "build\ffi-bridge-metrics.json"
)

$ErrorActionPreference = "Stop"

function Load-JsonFile {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        Write-Error "JSON ファイルが見つかりません: $Path"
    }

    try {
        return Get-Content -Path $Path -Raw | ConvertFrom-Json
    } catch {
        Write-Error "JSON の解析に失敗しました ($Path): $_"
    }
}

function Load-AuditEntries {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        return @()
    }

    $text = Get-Content -Path $Path -Raw -ErrorAction Stop
    $text = $text.Trim()

    if ([string]::IsNullOrEmpty($text)) {
        return @()
    }

    # JSON Lines 形式の解析
    $entries = @()
    try {
        # 単一 JSON オブジェクトとして試行
        $data = $text | ConvertFrom-Json
        if ($data -is [System.Array]) {
            $entries = $data
        } else {
            $entries = @($data)
        }
    } catch {
        # JSON Lines として行ごとに解析
        $lines = $text -split "`n"
        foreach ($line in $lines) {
            $line = $line.Trim()
            if ([string]::IsNullOrEmpty($line)) { continue }

            try {
                $entries += $line | ConvertFrom-Json
            } catch {
                Write-Error "JSON Lines の解析に失敗しました: $_"
            }
        }
    }

    return $entries
}

# ========== メインロジック ==========

Write-Host "Iterator Stage / FFI Bridge 監査サマリー生成" -ForegroundColor Cyan

# メトリクス読み込み
$metrics = Load-JsonFile -Path $MetricsPath
$verifyLogText = Get-Content -Path $VerifyLogPath -Raw

# pass_rate の取得
$passRate = $metrics.pass_rate
$passRateFloat = if ($passRate -ne $null) { [double]$passRate } else { $null }

# verify_llvm_ir ログのステータス判定
$logStatus = "不明"
if ($verifyLogText -match "検証成功|Verification succeeded") {
    $logStatus = "成功"
} elseif ($verifyLogText -match "検証失敗|Verification failed|エラー|失敗") {
    $logStatus = "失敗"
}

# FFI Bridge メトリクス読み込み (オプション)
$ffiBridgeMetrics = $null
if ($metrics.metrics) {
    $candidate = $metrics.metrics | Where-Object { $_.metric -eq "ffi_bridge.audit_pass_rate" } | Select-Object -First 1
    if ($candidate) {
        $ffiBridgeMetrics = $candidate
    }
}
if (Test-Path $FfiBridgeMetricsPath) {
    Write-Host "FFI Bridge メトリクスを読み込み中: $FfiBridgeMetricsPath" -ForegroundColor Gray
    $ffiBridgeMetrics = Load-JsonFile -Path $FfiBridgeMetricsPath
}

# Markdown 出力の生成
$currentDate = Get-Date -Format "yyyy-MM-dd"
$outputLines = @()

$outputLines += "### Iterator Stage Audit サマリー ($currentDate)`n"
$outputLines += "- メトリクスファイル: ``$MetricsPath``"
$outputLines += "- verify ログ: ``$VerifyLogPath`` (判定: $logStatus)"
$outputLines += "- 指標: ``$($metrics.metric)``"
$outputLines += "- 合計: $($metrics.total), 成功: $($metrics.passed), 失敗: $($metrics.failed), pass_rate: $passRate"

# ソースファイル一覧
if ($metrics.sources -and $metrics.sources.Count -gt 0) {
    $outputLines += "- 解析対象ファイル数: $($metrics.sources.Count)"
    foreach ($src in $metrics.sources) {
        $outputLines += "  - ``$src``"
    }
}

# 失敗エントリ
if ($metrics.failures -and $metrics.failures.Count -gt 0) {
    $outputLines += "`n#### 監査必須キーの欠落"
    foreach ($failure in $metrics.failures) {
        $file = $failure.file
        $idx = $failure.index
        $missing = $failure.missing -join ", "
        $outputLines += "- ``$file`` (diagnostic #$idx) → 欠落フィールド: $missing"
    }
} else {
    $outputLines += "`n- 監査必須キー: すべて揃っています 🎉"
}

# Stage トレース検証
$stageTraceMissing = 0
$stageTraceSourceMissing = 0
$stageTraceMismatch = 0
$stageTraceEntries = @()

if ($AuditPath -and (Test-Path $AuditPath)) {
    $auditEntries = Load-AuditEntries -Path $AuditPath

    if ($auditEntries.Count -gt 0) {
        $outputLines += "`n#### Stage トレース検証"

        foreach ($entry in $auditEntries) {
            $index = [Array]::IndexOf($auditEntries, $entry)
            $metadata = if ($entry.metadata) { $entry.metadata } else { $entry }
            $stageTrace = $metadata.stage_trace

            if (-not $stageTrace -or $stageTrace.Count -eq 0) {
                $stageTraceMissing++
                $stageTraceEntries += @{
                    index = $index
                    status = "missing"
                    detail = "stage_trace が存在しません"
                }
                continue
            }

            # typer/runtime ステップの検索
            $typerStep = $stageTrace | Where-Object { $_.source -match "typer" } | Select-Object -First 1
            $runtimeStep = $stageTrace | Where-Object { $_.source -match "runtime" } | Select-Object -First 1

            if (-not $typerStep -or -not $runtimeStep) {
                $stageTraceSourceMissing++
                $stageTraceEntries += @{
                    index = $index
                    status = "incomplete"
                    detail = "typer/runtime の両方のステップが揃っていません"
                    trace = $stageTrace
                }
                continue
            }

            $typerStage = $typerStep.stage
            $runtimeStage = $runtimeStep.stage

            if ($typerStage -ne $runtimeStage) {
                $stageTraceMismatch++
                $stageTraceEntries += @{
                    index = $index
                    status = "mismatch"
                    typer_stage = $typerStage
                    runtime_stage = $runtimeStage
                    trace = $stageTrace
                }
            } else {
                $stageTraceEntries += @{
                    index = $index
                    status = "ok"
                    stage = $typerStage
                    trace = $stageTrace
                }
            }
        }

        $outputLines += "- トレース件数: $($stageTraceEntries.Count), 欠落: $stageTraceMissing, 不足: $stageTraceSourceMissing, 差分: $stageTraceMismatch"
        $outputLines += ""

        foreach ($entry in $stageTraceEntries) {
            $status = $entry.status
            $idx = $entry.index

            switch ($status) {
                "ok" {
                    $outputLines += "- ✅ trace#${idx}: stage=$($entry.stage)"
                }
                "missing" {
                    $outputLines += "- ❌ trace#${idx}: $($entry.detail)"
                }
                "incomplete" {
                    $outputLines += "- ❌ trace#${idx}: $($entry.detail)"
                }
                "mismatch" {
                    $outputLines += "- ❌ trace#${idx}: typer=$($entry.typer_stage) / runtime=$($entry.runtime_stage)"
                }
            }
        }
    }
}

# FFI Bridge メトリクス追加
if ($ffiBridgeMetrics) {
    $outputLines += "`n### FFI Bridge Audit サマリー`n"
    if (Test-Path $FfiBridgeMetricsPath) {
        $outputLines += "- メトリクスファイル: ``$FfiBridgeMetricsPath``"
    } else {
        $outputLines += "- メトリクスファイル: ``$MetricsPath`` (embedded)"
    }

    $ffiPassRate =
        if ($ffiBridgeMetrics.ffi_bridge_pass_rate -ne $null) {
            $ffiBridgeMetrics.ffi_bridge_pass_rate
        } else {
            $ffiBridgeMetrics.pass_rate
        }
    $ffiTotal =
        if ($ffiBridgeMetrics.total_ffi_declarations -ne $null) {
            $ffiBridgeMetrics.total_ffi_declarations
        } else {
            $ffiBridgeMetrics.total
        }
    $ffiPassed =
        if ($ffiBridgeMetrics.passed_ffi_declarations -ne $null) {
            $ffiBridgeMetrics.passed_ffi_declarations
        } else {
            $ffiBridgeMetrics.passed
        }

    $outputLines += "- 指標: ``ffi_bridge.audit_pass_rate``"
    $outputLines += "- FFI 宣言数: $ffiTotal, 検証成功: $ffiPassed, pass_rate: $ffiPassRate"

    # ABI 別の集計
    if ($ffiBridgeMetrics.abi_breakdown) {
        $outputLines += "`n#### ABI 別集計"
        foreach ($abi in $ffiBridgeMetrics.abi_breakdown.PSObject.Properties) {
            $abiName = $abi.Name
            $abiCount = $abi.Value
            $outputLines += "- ``$abiName``: $abiCount 件"
        }
    }

    # 所有権別の集計
    if ($ffiBridgeMetrics.ownership_breakdown) {
        $outputLines += "`n#### 所有権別集計"
        foreach ($ownership in $ffiBridgeMetrics.ownership_breakdown.PSObject.Properties) {
            $ownershipName = $ownership.Name
            $ownershipCount = $ownership.Value
            $outputLines += "- ``$ownershipName``: $ownershipCount 件"
        }
    }
}

# Markdown 出力
$outputDir = Split-Path -Path $OutputPath -Parent
if ($outputDir -and -not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$markdown = ($outputLines -join "`n").TrimEnd() + "`n"

if ($OutputPath -eq "-") {
    Write-Output $markdown
} else {
    Set-Content -Path $OutputPath -Value $markdown -Encoding UTF8 -NoNewline
    Write-Host "監査サマリーを出力しました: $OutputPath" -ForegroundColor Green
}

# 終了コード判定
$exitCode = 0

if ($passRateFloat -eq $null -or $passRateFloat -lt 1.0) {
    Write-Warning "Iterator pass_rate が 1.0 未満です: $passRate"
    $exitCode = 1
}

if ($logStatus -eq "失敗") {
    Write-Warning "LLVM 検証ログがエラーを含んでいます"
    $exitCode = 1
}

if ($stageTraceMissing -gt 0 -or $stageTraceSourceMissing -gt 0 -or $stageTraceMismatch -gt 0) {
    Write-Warning "Stage トレースに不整合があります"
    $exitCode = 1
}

if ($ffiBridgeMetrics) {
    $ffiRawPassRate =
        if ($ffiBridgeMetrics.ffi_bridge_pass_rate -ne $null) {
            $ffiBridgeMetrics.ffi_bridge_pass_rate
        } else {
            $ffiBridgeMetrics.pass_rate
        }
    $ffiPassRateValue = [double]$ffiRawPassRate
    if ($ffiPassRateValue -lt 1.0) {
        Write-Warning "FFI Bridge pass_rate が 1.0 未満です: $ffiPassRateValue"
        $exitCode = 1
    }
}

if ($exitCode -eq 0) {
    Write-Host "✅ すべての監査チェックが成功しました" -ForegroundColor Green
} else {
    Write-Host "❌ 監査チェックに失敗しました (終了コード: $exitCode)" -ForegroundColor Red
}

exit $exitCode
