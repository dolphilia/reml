# 0.3 測定・監査・レビュー記録

本章では Phase 1〜4 に共通する測定指標、診断と監査ログの収集方法、レビュー記録フォーマットを定義する。[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) と `docs/notes/llvm-spec-status-survey.md` のフォーマットを継承し、各フェーズの完了条件を定量的に確認できるようにする。

## 0.3.1 指標セット
| カテゴリ | 指標 | 定義 | 収集タイミング | 仕様参照 |
|----------|------|------|----------------|----------|
| 性能 | `parse_throughput` | 10MB ソースの解析時間 (ms) | フェーズごとに最低 3 回計測 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §1.1 |
| 性能 | `memory_peak_ratio` | ピークメモリ / 入力サイズ | 各フェーズ主要マイルストーン後 | 同上 |
| 安全性 | `stage_mismatch_count` | Capability Stage ミスマッチ件数 | CI (PR ごと) | [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) |
| 安全性 | `ffi_ownership_violation` | FFI 所有権警告件数 | CI + 週次レビュー | [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) |
| 安全性 | `iterator.stage.audit_pass_rate` | `typeclass.iterator.stage_mismatch` 診断で必須監査キーが揃った割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §2.4 |
| DX | `diagnostic_regressions` | 診断差分の件数 | PR ごと | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) |
| DX | `error_resolution_latency` | 重大バグの修正までの日数 | 月次 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §2.2 |

- CI 集計スクリプト: `tooling/ci/collect-iterator-audit-metrics.py` を用いて診断 JSON を検査し、結果を `tooling/ci/iterator-audit-metrics.json` に書き出す。`pass_rate` が 1.0 未満の場合は CI 側でブロック条件を設定する。

### macOS 追加指標（Phase 1-8 以降）
| カテゴリ | 指標 | 定義 | 収集タイミング | 計画参照 |
|----------|------|------|----------------|----------|
| CI | `ci_build_time_macos` | `bootstrap-macos` ワークフローにおける `dune build` の実行時間（分） | push/pr ごと | [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) §5 |
| CI | `ci_test_time_macos` | `bootstrap-macos` ワークフローにおける `dune runtest` の実行時間（分） | push/pr ごと | 同上 |
| 品質 | `llvm_verify_macos` | `llvm-as` → `opt -verify` → `llc -mtriple=arm64-apple-darwin` の成否（0=成功,1=失敗） | CI 実行ごと | 同上 |
| 成果物 | `runtime_macho_size` | `libreml_runtime.a` (Mach-O) のファイルサイズ（KB） | 週次 | 同上 |
| 運用 | `macos_runner_queue_time` | GitHub Actions macOS ランナーの待機時間（分） | 週次レビュー | 同上 |

> **補足**: macOS 指標は Linux 指標との比較を想定し、`metrics.json` にターゲット別セクションを設けて記録する。乖離が 15% を超えた場合は `0-4-risk-handling.md` に登録して原因調査を開始する。

### Phase 1-8 実測値（macOS Apple Silicon ARM64）

**測定日**: 2025-10-11
**環境**: macOS 14.x / Apple Silicon (ARM64) / LLVM 18.1.8 / OCaml 5.2.1

| 指標 | 実測値 | 備考 |
|------|--------|------|
| `ci_build_time_macos` | 2.4秒 | `dune build` フルビルド（クリーンビルド後） |
| `ci_test_time_macos` | ~30秒 | `dune runtest` + ランタイムテスト + AddressSanitizer |
| `llvm_verify_macos` | 成功 (0) | ARM64 ターゲットで全サンプル検証成功 |
| `runtime_macho_size` | 56 KB | `libreml_runtime.a` (ARM64 Mach-O) |
| `macos_runner_queue_time` | 未計測 | GitHub Actions の実運用開始後に記録 |

**LLVM IR 検証結果**:
- ターゲット: `arm64-apple-darwin`
- 検証パイプライン: `llvm-as` → `opt -verify` → `llc -mtriple=arm64-apple-darwin`
- 全テストサンプル検証成功（examples/cli/*.reml）

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
- `Diagnostic` オブジェクトの拡張フィールド (`extensions`) は [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) に定義されたキー (`effect.stage.required`, `bridge.stage.actual` など) を使用する。
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

## 0.3.6 最適化パス統計（Phase 3 Week 10-11）

### 実装統計
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| コード規模 | Core IR 総行数 | 5,642行 | ir.ml, desugar.ml, cfg.ml, const_fold.ml, dce.ml, pipeline.ml, ir_printer.ml |
| テスト | テストケース総数 | 42件 | test_core_ir, test_desugar, test_cfg, test_const_fold (26件), test_dce (9件), test_pipeline (7件) |
| テスト | 成功率 | 100% (42/42) | 回帰なし |
| 最適化 | 定数畳み込み式数 | 変動 | パイプラインテストで計測 |
| 最適化 | 削除束縛数 | 変動 | DCEテストで計測 |
| 最適化 | 削除ブロック数 | 変動 | DCEテストで計測 |
| 性能 | ConstFold実行時間 | <0.001秒 | テストケース平均 |
| 性能 | DCE実行時間 | <0.001秒 | テストケース平均 |

### 最適化効果の例
- **定数畳み込み**: `10 + 20` → `30`（26件のテストで検証）
- **死コード削除**: `let x = 42 in 10` → `10`（9件のテストで検証）
- **パイプライン統合**: 不動点反復で複数パスを自動適用（7件のテストで検証）

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `stage_mismatch_count` | 0件 | 0件 | ✅ 達成 |
| テストカバレッジ | 100% | 95%以上 | ✅ 達成 |

## 0.3.7 RuntimeCapability 運用と効果診断ゴールデン

### JSON 管理手順
- Capability Registry は `tooling/runtime/capabilities/` に配置する。デフォルト設定は `default.json`、プラットフォーム差分は `{platform}.json`（例: `linux.json`, `windows.json`）で管理し、コミット時に必ず本節へ変更履歴を追記する。
- JSON フォーマット（暫定）は以下を必須キーとする。`stage` は `experimental` / `beta` / `stable` のいずれか、`capabilities` は `RuntimeCapability` 列挙子文字列、`overrides` はターゲットトリプル別の上書き設定。
  ```json
  {
    "stage": "stable",
    "capabilities": ["io", "panic", "runtime"],
    "overrides": {
      "x86_64-pc-windows-msvc": ["ffi", "process"]
    }
  }
  ```
- JSON の編集手順:
  1. 変更箇所を `tooling/runtime/README.md`（Phase 2-2 で追加予定）に記録し、出典となる仕様 (`docs/spec/3-8-core-runtime-capability.md`) を併記する。
  2. `scripts/validate-runtime-capabilities.sh`（Phase 2-2 で整備）を実行し、スキーマ検証と Stage 解釈トレースの再計算を行う。スクリプトは `reports/runtime-capabilities-validation.json` に `stage_summary`（CLI/JSON/環境変数/Runtime 判定の一覧）を出力し、CI で `jq` フォーマットチェックを通過することを確認する。
  3. 差分を `0-3.9 進捗ログ` に追記し、`stage_summary` から抜粋した Stage 変更点（例: `default.json: beta → stable`）を合わせて記録する。レビュアには JSON とサマリの両方を提示する。
- Stage が変更された場合は、必ず効果診断ゴールデンと `AuditEnvelope` ゴールデンを再生成し、`stage_trace` の差分が Typer/Runtime で一致していることを確認する。
- CLI/環境変数による Stage 指定を検証する場合は、`--cli-stage <stage>` / `--env-stage <stage>` を併用し、`stage_trace` の冒頭エントリ（`cli_option` / `env_var`）と整合を確認する。

### CLI オプション優先度と検証
- Stage 解決は「CLI `--effect-stage` → JSON `--runtime-capabilities` → 環境変数 `REMLC_EFFECT_STAGE`」の優先順を採用し、`RuntimeCapabilityResolver`（Phase 2-2 で導入予定）で一元化する。
- 動作確認フロー:
  1. `remlc examples/effects/demo.reml --effect-stage beta --format=json` を実行し、`Diagnostic.extensions["effect.stage.required"]` が `beta` になることを確認。
  2. 同一コマンドに `--runtime-capabilities tooling/runtime/capabilities/linux.json` を追加し、JSON の `stage` が採用されることを `effect.stage.actual` で確認。
  3. どちらも指定せず `REMLC_EFFECT_STAGE=stable` を設定し、環境変数が採用されることを確認。
  4. 上記 3 ケースで `Diagnostic.extensions["effect.stage_trace"]` に出力される `source` / `stage` / `capability` の配列が CLI 指定 → JSON → 環境変数の順序で記録されていることを確認し、Runtime 側の `AuditEnvelope.metadata.stage_trace` も同一配列であることを `grep` などで突き合わせる。
- 上記 3 ケースの出力を `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden`（新設）でスナップショット化し、`dune runtest compiler/ocaml/tests/test_diagnostics.ml` に統合する。

### 監査ログと CI 指標
- Stage 判定は `RuntimeCapabilityResolver` → `AuditEnvelope` → `tooling/ci/collect-iterator-audit-metrics.py` → `iterator.stage.audit_pass_rate` の順で連携する。各段階で `stage_trace` が欠落した場合は CI を失敗させる。
- `remlc examples/effects/demo.reml --emit-audit --effect-stage beta` を実行し、`AuditEnvelope.metadata.stage_trace` に Typer 判定と Runtime 判定が連続して格納されていることを確認する。監査ゴールデンは `compiler/ocaml/tests/golden/audit/effects-stage.json.golden`（新設）に保存する。
- CI では `tooling/ci/sync-iterator-audit.sh --metrics /tmp/iterator-audit.json --audit compiler/ocaml/tests/golden/audit/effects-stage.json.golden` を実行し、`iterator.stage.audit_pass_rate` が 1.0 であることをゲート条件とする。Stage 判定差分が発生した場合は `stage_trace` の乖離内容を Markdown サマリに追記してレビューへ共有する。
- 監査ログの更新後は `reports/runtime-capabilities-validation.json` の `stage_summary` と `iterator-stage-summary.md`（`sync-iterator-audit.sh` が生成）を本節へリンクする。

### 効果診断ゴールデンの整備
- ゴールデン配置: `compiler/ocaml/tests/golden/diagnostics/effects/`（`*.golden`）に JSON スナップショットを保存し、必須キー `effect.stage.required` / `effect.stage.actual` / `effect.stage.residual` / `effect.stage.source` および `diagnostic.extensions.effect.stage_trace` / `diagnostic.extensions.effect.attribute` / `diagnostic.extensions.effect.residual` を全て検証する。
- 更新手順:
  1. `remlc` を `--format=json --emit-diagnostics` モードで実行し、一時ファイルを生成。
  2. `scripts/update-effects-golden.sh`（Phase 2-2 で追加予定）を用いて対象ゴールデンのみを上書きする。自動プロモートは使用しない。スクリプトでは `stage_trace` の差分を検知し、Typer / Runtime フェーズの順序が正しいかを静的チェックする。
  3. 更新後に `tooling/ci/collect-iterator-audit-metrics.py` を実行し、`iterator.stage.audit_pass_rate` が 1.0 を維持していることを確認する。
  4. 差分と検証結果を本節に追記し、Phase 2-2 の週次レビュー議事録と同期する。
- ゴールデン差分がまだ確認されていない場合や Stage 検証が未完了の場合は、`0-4-risk-handling.md` に TODO を登録して Phase 2-2 の完了条件に含める。

## 0.3.8 LLVM ABI テスト統計（Phase 3 Week 15）

### 実装統計
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| コード規模 | ABI実装総行数 | 211行 | abi.ml/mli（System V ABI判定・LLVM属性設定） |
| コード規模 | テストコード総行数 | 518行 | test_abi.ml（拡張後） |
| テスト | テストケース総数 | 61件 | 既存45件 + 新規16件（境界値9件、エッジケース7件） |
| テスト | 成功率 | 100% (61/61) | 回帰なし |
| カバレッジ | 型サイズテスト | 20件 | プリミティブ9件、タプル8件、エッジケース3件 |
| カバレッジ | ABI判定テスト | 26件 | 戻り値13件、引数13件（境界値・エッジケース含む） |
| カバレッジ | LLVM属性テスト | 6件 | sret 3件、byval 3件 |
| カバレッジ | デバッグ関数テスト | 4件 | 文字列表現検証 |

### ABI判定精度
| 項目 | 詳細 | 検証結果 |
|------|------|----------|
| 16バイト境界 | (i64, i8) 15バイト以下 → DirectReturn/DirectArg | ✅ 正確 |
| 16バイト境界 | (i64, i64) 16バイト → DirectReturn/DirectArg | ✅ 正確 |
| 16バイト境界超過 | (i64, i64, i8) 17バイト超 → SretReturn/ByvalArg | ✅ 正確 |
| ネスト構造 | ((i64, i64), i64) 24バイト → SretReturn/ByvalArg | ✅ 正確 |
| エッジケース | () 空タプル 0バイト → DirectReturn/DirectArg | ✅ 正確 |
| FAT pointer | {data: String, count: i64} 24バイト → SretReturn/ByvalArg | ✅ 正確 |

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `stage_mismatch_count` | 0件 | 0件 | ✅ 達成 |
| テストカバレッジ | 100% | 95%以上 | ✅ 達成 |
| 境界値検証 | 3ケース | 2ケース以上 | ✅ 達成 |

### 技術的発見
- **パディング挙動**: (i64, i8)は実際には16バイトにパディングされ、境界値以下として正しく扱われる
- **ネストタプル**: ((i64, i64), i64)はフラット化され24バイトとして正しくABI判定される
- **関数型**: 現在の実装では関数ポインタ（8バイト）として扱われ、将来的にクロージャ（16バイト）への拡張が必要

## 0.3.9 進捗ログ
- 2025-10-06 / compiler/ocaml / パーサードライバーを `Result<Ast, Diagnostic>` へ移行し、`tests/test_parser.ml` に診断メタデータ検証を追加。`diagnostic_regressions` 指標は `dune test` による差分チェックで監視。
- 2025-10-07 / compiler/ocaml / Phase 3 Week 10-11 完了: Core IR 最適化パス（定数畳み込み、死コード削除、パイプライン統合）を実装。総コード行数: 約5,642行、テスト: 42件全て成功。
- 2025-10-09 / compiler/ocaml / Phase 3 Week 15 完了: ABI判定・属性設定のユニットテスト実装。総テストケース: 61件（既存45件 + 新規16件）、成功率: 100%。16バイト境界の正確な判定を検証済み。
- 2025-10-09 / tooling/ci/docker / `ghcr.io/reml/bootstrap-runtime:dev-local` を linux/amd64 でビルド（所要 ~530 秒、圧縮前 4.09GB）。`scripts/docker/run-runtime-tests.sh` と `scripts/docker/smoke-linux.sh` を実行し、既知の失敗（Let Polymorphism A2、LLVM ゴールデン差分、`basic_interpreter.reml` の構文エラー）を確認。計測値を `tooling/ci/docker/metrics.json` に記録。
- 2025-10-10 / .github/workflows / ランタイム CI 統合完了: `bootstrap-linux.yml` に Valgrind 検証・アーティファクト収集を追加し、Lint/Build/Test/Artifact の 4 ジョブ構成で Phase 1-5 §8 の CI 自動化を達成。
- 2025-10-16 / compiler/ocaml / `compiler/ocaml/scripts/benchmark_typeclass.sh --static-only` を実行し、辞書渡し／モノモルフィゼーションの静的比較レポート (`compiler/ocaml/benchmark_results/static_comparison.json`) を生成。現時点では while/for 未実装のため IR/ビットコード生成がスキップされメトリクスは 0 だが、Phase 3 でループ実装後に再計測予定。
- 2025-10-16 / tooling/ci / `collect-iterator-audit-metrics.py` → `sync-iterator-audit.sh` を手動実行し、`iterator.stage.audit_pass_rate = 1.0` を確認。`/tmp/iterator-summary.md` に生成した Markdown を次回 CI から `reports/` 階層へ保存し、週次で本ドキュメントへ転記する運用を開始。

## 0.3.10 ランタイムテスト統計（Phase 1-5 Week 16）

### CI 統合実装
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| CI ワークフロー | ステップ追加数 | 5件 | Valgrind インストール、ビルド、テスト、Valgrind 検証、アーティファクト収集 |
| テスト | 実行テストケース | 14件 | メモリアロケータ（6件）、参照カウント（8件） |
| テスト | 成功率 | 100% | 全テスト成功（ローカル検証済み） |
| メモリ検証 | Valgrind 統合 | 有効 | `--leak-check=full --error-exitcode=1` で実行 |
| メモリ検証 | AddressSanitizer | 有効 | `DEBUG=1` ビルドで自動有効化 |
| アーティファクト | 保持期間（成功時） | 30日 | `libreml_runtime.a` および `.o` ファイル |
| アーティファクト | 保持期間（失敗時） | 7日 | テストバイナリおよびログファイル |

### メモリ安全性検証
| 項目 | 検証方法 | 結果 |
|------|----------|------|
| リーク検出 | Valgrind `--leak-check=full` | ✅ 0件（全テスト通過） |
| ダングリングポインタ | AddressSanitizer | ✅ 0件（全テスト通過） |
| 二重解放 | AddressSanitizer | ✅ 0件（全テスト通過） |
| 境界チェック | AddressSanitizer | ✅ 0件（全テスト通過） |

### 自動化範囲
- ✅ ランタイムビルド（`make runtime`）
- ✅ 基本テスト実行（`make test`）
- ✅ Valgrind メモリ検証（全テストバイナリ）
- ✅ アーティファクト自動収集（成功時・失敗時）
- ✅ ローカル再現手順のドキュメント化（`runtime/native/README.md`）

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `memory_leak_count` | 0件 | 0件 | ✅ 達成 |
| `test_coverage` | 100% | 95%以上 | ✅ 達成 |
| CI 実行時間（追加分） | 約3-5分 | 10分以内 | ✅ 達成 |

### 今後の課題（Phase 2 以降）
- [ ] Windows 環境での Valgrind 代替（Dr. Memory など）
- [ ] macOS 環境でのメモリリーク検証（leaks コマンド）
- [ ] CI 実行時間の最適化（キャッシュ戦略の改善）
- [ ] クロスプラットフォームでのアーティファクト統合

---

本章で定義した指標とログフォーマットは、計画書全体の共通基盤として扱う。各 Phase 文書はここで定義した指標を利用し、進行状況と品質を定量的に管理する。
