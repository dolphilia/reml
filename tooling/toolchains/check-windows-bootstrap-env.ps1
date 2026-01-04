# Reml Windows bootstrap environment checker
#
# このスクリプトは Phase 2-3 で必要となる Windows 向け開発環境の依存関係を確認します。
# 仕様および計画書の参照:
#   - docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md
#   - docs/plans/bootstrap-roadmap/2-6-windows-support.md
#
# 使い方:
#   pwsh.exe -NoLogo -File tooling/toolchains/check-windows-bootstrap-env.ps1
#   pwsh.exe -NoLogo -File tooling/toolchains/check-windows-bootstrap-env.ps1 -OutputJson reports/windows-env-check.json
#
# 出力:
#   - 標準出力に依存関係の有無とバージョンを一覧表示します。
#   - -OutputJson を指定すると、同じ内容を JSON で保存します。

[CmdletBinding()]
param(
    [string]$OutputJson = ""
)

Set-StrictMode -Version Latest

$setupScript = Join-Path $PSScriptRoot 'setup-windows-toolchain.ps1'
if (Test-Path -LiteralPath $setupScript) {
    . $setupScript -NoCheck -Quiet | Out-Null
} else {
    Write-Warning "setup-windows-toolchain.ps1 が見つからないため、PATH と MSVC の自動初期化をスキップします。"
}

function Invoke-VersionCommand {
    param(
        [scriptblock]$Command
    )

    try {
        $output = & $Command
        if ($LASTEXITCODE -ne 0) {
            return ""
        }
        if ($null -eq $output) {
            return ""
        }
        return ($output | Out-String).Trim()
    } catch {
        return ""
    }
}

$checks = @(
    @{
        Name = "Git"
        Commands = @("git")
        Category = "core"
        Required = $true
        MinimumVersion = "2.40"
        VersionCommand = { git --version }
    },
    @{
        Name = "Python"
        Commands = @("python")
        Category = "core"
        Required = $true
        MinimumVersion = "3.10"
        VersionCommand = { python --version }
    },
    @{
        Name = "Pip"
        Commands = @("pip")
        Category = "core"
        Required = $false
        MinimumVersion = "23.0"
        VersionCommand = { pip --version }
    },
    @{
        Name = "Bash (MSYS2/Git)"
        Commands = @("bash")
        Category = "core"
        Required = $true
        MinimumVersion = "4.0"
        VersionCommand = { bash --version | Select-Object -First 1 }
    },
    @{
        Name = "LLVM (clang/llc/opt)"
        Commands = @("clang", "llc", "opt")
        Category = "llvm"
        Required = $true
        MinimumVersion = "18.0"
        VersionCommand = { clang --version | Select-Object -First 1 }
    },
    @{
        Name = "LLVM toolchain (llvm-ar)"
        Commands = @("llvm-ar")
        Category = "llvm"
        Required = $true
        MinimumVersion = "18.0"
        VersionCommand = { llvm-ar --version | Select-Object -First 1 }
    },
    @{
        Name = "MSVC toolchain (cl/link/lib)"
        Commands = @("cl", "link", "lib")
        Category = "msvc"
        Required = $true
        MinimumVersion = "19.38"
        VersionCommand = { cl }
    },
    @{
        Name = "CMake"
        Commands = @("cmake")
        Category = "build"
        Required = $false
        MinimumVersion = "3.27"
        VersionCommand = { cmake --version | Select-Object -First 1 }
    },
    @{
        Name = "Ninja"
        Commands = @("ninja")
        Category = "build"
        Required = $false
        MinimumVersion = "1.11"
        VersionCommand = { ninja --version }
    },
    @{
        Name = "jq"
        Commands = @("jq")
        Category = "tools"
        Required = $false
        MinimumVersion = "1.6"
        VersionCommand = { jq --version }
    },
    @{
        Name = "7zip (7z)"
        Commands = @("7z")
        Category = "tools"
        Required = $false
        MinimumVersion = "22.0"
        VersionCommand = { 7z i | Where-Object { $_ -and $_.Trim() } | Select-Object -First 1 }
    }
)

$results = foreach ($check in $checks) {
    $commands = @()
    $locations = @()
    $present = $true

    foreach ($cmd in $check.Commands) {
        $cmdInfo = $null
        try {
            $cmdInfo = Get-Command $cmd -ErrorAction Stop
        } catch {
            $present = $false
        }

        if ($null -ne $cmdInfo) {
            $commands += $cmd
            $locations += $cmdInfo.Source
        } else {
            $commands += "$cmd (missing)"
        }
    }

    $version = ""
    if ($present -and $check.ContainsKey("VersionCommand") -and $null -ne $check.VersionCommand) {
        $version = Invoke-VersionCommand -Command $check.VersionCommand
    }

    [PSCustomObject]@{
        Name = $check.Name
        Present = $present
        Commands = ($commands -join ", ")
        Locations = if ($locations.Count -gt 0) { $locations -join "; " } else { "" }
        MinimumVersion = $check.MinimumVersion
        DetectedVersion = $version
        Required = $check.Required
        Category = $check.Category
    }
}

$results | Sort-Object -Property @{Expression = "Required"; Descending = $true}, @{Expression = "Category"; Descending = $false}, @{Expression = "Name"; Descending = $false} | Format-Table -AutoSize

if ($OutputJson -and $OutputJson.Trim().Length -gt 0) {
    $jsonPath = Resolve-Path -Path $OutputJson -ErrorAction SilentlyContinue
    if (-not $jsonPath) {
        $jsonPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputJson)
    } else {
        $jsonPath = $jsonPath.ProviderPath
    }

    $results | ConvertTo-Json -Depth 3 | Out-File -FilePath $jsonPath -Encoding utf8
    Write-Host ("JSON report written to {0}" -f $jsonPath)
}
