# 0.3 測定・監査・レビュー記録

本章では Phase 1〜4 に共通する測定指標、診断と監査ログの収集方法、レビュー記録フォーマットを定義する。`3-6-core-diagnostics-audit.md` と `notes/llvm-spec-status-survey.md` のフォーマットを継承し、各フェーズの完了条件を定量的に確認できるようにする。

## 0.3.1 指標セット
| カテゴリ | 指標 | 定義 | 収集タイミング | 仕様参照 |
|----------|------|------|----------------|----------|
| 性能 | `parse_throughput` | 10MB ソースの解析時間 (ms) | フェーズごとに最低 3 回計測 | `0-1-project-purpose.md` §1.1 |
| 性能 | `memory_peak_ratio` | ピークメモリ / 入力サイズ | 各フェーズ主要マイルストーン後 | 同上 |
| 安全性 | `stage_mismatch_count` | Capability Stage ミスマッチ件数 | CI (PR ごと) | `3-8-core-runtime-capability.md` |
| 安全性 | `ffi_ownership_violation` | FFI 所有権警告件数 | CI + 週次レビュー | `3-9-core-async-ffi-unsafe.md` |
| DX | `diagnostic_regressions` | 診断差分の件数 | PR ごと | `3-6-core-diagnostics-audit.md` |
| DX | `error_resolution_latency` | 重大バグの修正までの日数 | 月次 | `0-1-project-purpose.md` §2.2 |

## 0.3.2 レポートテンプレート
- **週次レポート**: `reports/week-YYYYMMDD.md`（将来追加予定）に以下の項目を記録する。
  - 主要マイルストーン進捗
  - 指標の最新値
  - リスク/ブロッカー（`0-4-risk-handling.md` へのリンク）
- **フェーズ終了レビュー**: 各 Phase 文書末尾のチェックリストと合わせて、以下を必須記録とする。
  - 指標表（最新値と目標）
  - レビュア署名（Parser/Type/Runtime/Toolchain）
  - 仕様変更一覧（ファイル/節/概要）

## 0.3.3 診断・監査ログ整合性
- `Diagnostic` オブジェクトの拡張フィールド (`extensions`) は `3-6-core-diagnostics-audit.md` に定義されたキー (`effect.stage.required`, `bridge.stage.actual` など) を使用する。
- 監査ログ (`AuditEnvelope`) は JSON Lines 形式で保存し、以下を必須フィールドとする。
  - `metadata.effect.stage.required`
  - `metadata.bridge.reload`
  - `metadata.ffi.ownership`
- ログ検証用に `tools/audit-verify`（将来実装予定）を準備し、CI で `--strict` フラグを用いて検証。

## 0.3.4 レビュア体制
| 領域 | 主担当 | 副担当 | レビュー頻度 |
|------|--------|--------|--------------|
| Parser/Core.Parse | TBD (Phase 1 決定) | TBD | 週次 |
| Type/Effects | TBD | TBD | 週次 |
| Runtime/Capability | TBD | TBD | 隔週 |
| Toolchain/CI | TBD | TBD | 隔週 |

レビュアの割当が変更された場合は、この表と各 Phase 文書のレビュア欄を更新する。担当者が空欄の場合は `0-4-risk-handling.md` にリスクとして記録し、埋めるまでフェーズ進行を停止する。

## 0.3.5 仕様差分追跡
- 仕様ファイルに変更が入った際は、以下の形式で記録する。
  - `YYYY-MM-DD / ファイル:節 / 変更概要 / 参照コミット`
- 記録は Phase ごとにセクションを分け、フェーズ終了時にレビューアが確認する。
- 差分が複数フェーズに跨る場合は、各フェーズで影響範囲を明記し、必要に応じて追加タスクを `0-4-risk-handling.md` に登録する。

---

本章で定義した指標とログフォーマットは、計画書全体の共通基盤として扱う。各 Phase 文書はここで定義した指標を利用し、進行状況と品質を定量的に管理する。
