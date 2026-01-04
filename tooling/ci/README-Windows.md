# Windows 向け CI ツール

Phase 2-3 FFI Contract Extension における Windows ローカル環境での監査ログ収集と検証を支援するツールです。

## スクリプト一覧

### `Sync-AuditMetrics.ps1`

Linux/macOS の `sync-iterator-audit.sh` の Windows PowerShell 版。Iterator Stage と FFI Bridge の監査ログを収集し、pass_rate を検証します。

#### 使用方法

**基本的な使用例**:

```powershell
.\Sync-AuditMetrics.ps1 `
    -MetricsPath build\iterator-metrics.json `
    -VerifyLogPath build\verify.log
```

**FFI Bridge メトリクスを含む完全な検証**:

```powershell
.\Sync-AuditMetrics.ps1 `
    -MetricsPath build\iterator-metrics.json `
    -VerifyLogPath build\verify.log `
    -AuditPath build\audit.jsonl `
    -FfiBridgeMetricsPath build\ffi-bridge-metrics.json `
    -OutputPath reports\audit-summary.md
```

#### パラメータ

| パラメータ | 必須 | 既定値 | 説明 |
|-----------|------|--------|------|
| `-MetricsPath` | ✅ | - | `collect-iterator-audit-metrics.py` が生成した JSON ファイル |
| `-VerifyLogPath` | ✅ | - | `verify_llvm_ir.sh` のログファイル |
| `-AuditPath` | ❌ | - | AuditEnvelope JSON (単一または JSON Lines 形式) |
| `-OutputPath` | ❌ | `reports\iterator-stage-summary.md` | Markdown サマリーの出力先 |
| `-FfiBridgeMetricsPath` | ❌ | `build\ffi-bridge-metrics.json` | FFI ブリッジメトリクス JSON |

#### 出力

スクリプトは以下を含む Markdown サマリーを生成します:

1. **Iterator Stage 監査結果**
   - pass_rate (合格率)
   - 解析対象ファイル一覧
   - 監査必須キーの欠落チェック
   - Stage トレースの整合性検証

2. **FFI Bridge 監査結果** (オプション)
   - FFI 宣言数と検証成功率
   - ABI 別集計 (system_v, msvc, darwin_aapcs64)
   - 所有権別集計 (borrowed, transferred, reference)

#### 終了コード

- `0`: すべての監査チェックが成功
- `1`: 以下のいずれかが検出された場合
  - pass_rate < 1.0
  - LLVM 検証ログにエラーが含まれる
  - Stage トレースに不整合がある
  - FFI Bridge pass_rate < 1.0

#### 実行例

```powershell
# Phase 2-3 FFI サンプルの検証
cd c:\msys64\home\dolph\reml

# コンパイラビルド (MSYS2 bash)
dune build

# FFI サンプルコンパイル
_build\default\src\main.exe --emit-ir examples\ffi\windows\messagebox.reml
_build\default\src\main.exe --emit-audit examples\ffi\windows\messagebox.reml

# 監査ログ収集 (PowerShell)
.\tooling\ci\Sync-AuditMetrics.ps1 `
    -MetricsPath build\iterator-metrics.json `
    -VerifyLogPath build\verify.log `
    -AuditPath build\audit.jsonl `
    -OutputPath reports\ffi-windows-summary.md
```

## メトリクス JSON スキーマ

### Iterator Metrics (`iterator-metrics.json`)

```json
{
  "metric": "iterator.stage.audit_pass_rate",
  "total": 10,
  "passed": 10,
  "failed": 0,
  "pass_rate": 1.0,
  "sources": ["examples/ffi/windows/messagebox.reml"],
  "failures": []
}
```

### FFI Bridge Metrics (`ffi-bridge-metrics.json`)

```json
{
  "ffi_bridge_pass_rate": 1.0,
  "total_ffi_declarations": 3,
  "passed_ffi_declarations": 3,
  "abi_breakdown": {
    "win64": 3
  },
  "ownership_breakdown": {
    "borrowed": 2,
    "transferred": 1
  }
}
```

## トラブルシューティング

### PowerShell 実行ポリシーエラー

```powershell
.\Sync-AuditMetrics.ps1 : このシステムではスクリプトの実行が無効になっているため、ファイル ... を読み込むことができません。
```

**解決方法**:

```powershell
# 現在のセッションのみ実行を許可
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass

# または、ユーザー全体で許可 (管理者権限不要)
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

### JSON 解析エラー

JSON Lines 形式 (`.jsonl`) を使用している場合、各行が有効な JSON オブジェクトであることを確認してください:

```jsonl
{"metadata": {"stage_trace": [...]}}
{"metadata": {"stage_trace": [...]}}
```

### ファイルパスエラー

Windows では `\` をパス区切り文字として使用します:

```powershell
# ✅ 正しい
-MetricsPath build\iterator-metrics.json

# ❌ 誤り (Unix スタイル)
-MetricsPath build/iterator-metrics.json
```

## 参照資料

- [Phase 2-3 FFI Contract Extension 計画](../../docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md)
- [Windows ローカル環境セットアップ](../../docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md)
- [FFI 仕様](../../docs/spec/3-9-core-async-ffi-unsafe.md)
- [監査・診断仕様](../../docs/spec/3-6-core-diagnostics-audit.md)

---

**作成日**: 2025-10-19
**Phase**: 2-3 FFI Contract Extension
**環境**: Windows 11 (MSYS2) + PowerShell 7
