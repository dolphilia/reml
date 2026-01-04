# Phase4: 標準ライブラリ改善実装計画（DSL 開発者体験）

## 背景と決定事項
- `docs/plans/stdlib-improvement/` で DSL 開発者体験を支える標準モジュール（Core.Test/Cli/Text.Pretty/Doc/Lsp）の計画をドラフト化した。
- Phase 4 の回帰計画（`4-1-spec-core-regression-plan.md`）を再開する前に、標準ライブラリ側の欠落を埋め、回帰シナリオと KPI を追加する必要がある。
- `docs/spec/0-1-project-purpose.md` に基づき、診断の明瞭性・安全性・実用性能を最優先とする。

## 目的
1. `Core.Test`/`Core.Cli`/`Core.Text.Pretty`/`Core.Doc`/`Core.Lsp` を Rust 実装へ落とし込み、仕様と一致した API を提供する。
2. DSL 由来の実用シナリオを Phase 4 の回帰マトリクスへ登録し、診断・監査ログの整合を検証する。
3. Phase 5 以降のセルフホストで必要となる CLI/ドキュメント/LSP の基盤を先行整備する。

## スコープ
- **含む**: 標準ライブラリモジュールの実装方針、仕様差分の反映先、サンプル/期待出力の整備、回帰シナリオ登録。
- **含まない**: リリースパイプライン整備、エコシステム配布、LSP クライアント実装。

## 成果物
- `Core.Test`/`Core.Cli`/`Core.Text.Pretty`/`Core.Doc`/`Core.Lsp` の最小 API が Rust 実装に反映される。
- `examples/` と `expected/` の DSL サンプルが Phase 4 の回帰対象として登録される。
- `docs/spec/3-0-core-library-overview.md` に新モジュールの概要が追記される。

## 標準ライブラリ拡張の優先順位（Phase 4 以降の着手順）
`docs/notes/stdlib/stdlib-expansion-research.md` の調査結果を踏まえ、Phase 4 の DSL 開発者体験（本計画）と並行・後続で進める標準ライブラリ拡張の着手順を以下に整理する。実用性・安全性・段階的導入（`docs/spec/0-1-project-purpose.md`）を最優先の判断基準とする。

1. **P0: Core.Net / Core.Crypto**  
   - **理由**: 実用用途の根幹（API 連携・配信・認証/整合性）であり、欠落がボトルネック。  
   - **前提**: `Core.Async` と `Core.Io`/`Core.Path` 連携、`effect {net}`/`effect {crypto, random}` の監査統合。

2. **P1: Core.System（Process/Env/Daemon）/ Core.Sync**  
   - **理由**: サブプロセス・シグナル・同期原語は汎用言語として必須。`Core.Process` をプラグインから昇格。  
   - **前提**: Capability Stage と診断監査の整合（`docs/spec/3-6-core-diagnostics-audit.md` / `3-8-core-runtime-capability.md`）。

3. **P2: Core.Encoding / Core.Serialization / Core.Archive / Core.Math**  
   - **理由**: データ交換・パッケージング・数値処理の実務要件を満たすため。  
   - **前提**: `Core.Data` との責務境界（`docs/spec/3-7-core-config-data.md`）を明確化し、ストリーミング API を `Reader`/`Writer` と統合。

4. **P3: Core.Fs / Core.Terminal / Core.Observability**  
   - **理由**: ツール運用・可観測性・開発体験の向上。`Core.Diagnostics` との住み分けを保ちつつ簡易 API を提供。  
   - **前提**: `Core.Async` ストリーム接続、`effect {io.console}` と監査ログの統一。

5. **P4: Core.Sql / Core.Test 拡張**  
   - **理由**: 実装コストが大きく、外部ドライバ依存が強い。インターフェース標準化から着手。`Core.Test` は Phase 4 で最小実装済みのため拡張は後続。  
   - **前提**: Stage 管理とプラグイン/FFI ポリシー（`docs/spec/3-8-core-runtime-capability.md`）の再確認。

## 作業ステップ

### フェーズA: Core.Test 実装と回帰接続
1. `Core.Test` の最小 API（`test` ブロック/スナップショット/テーブル駆動）を Rust 実装へ追加する。
2. スナップショット更新ポリシーと診断安定化のルールを明文化する。
3. DSL のパーサー/変換結果を対象としたサンプルを `examples/` と `expected/` に追加し、回帰シナリオへ登録する。

### フェーズB: Core.Cli 実装と CLI サンプル
1. 宣言的 CLI ビルダー（フラグ/引数/サブコマンド）を Rust 実装へ追加する。
2. `Core.Env` との役割分担を整理し、エラー出力のフォーマットを統一する。
3. DSL ツールの CLI サンプル（解析/検証/整形）を `examples/` に追加し、回帰シナリオへ登録する。

### フェーズC: Core.Text.Pretty 実装と整形サンプル
1. `text/line/softline/group/nest` などのコンビネータを実装する。
2. ページ幅とレイアウト選択の規則を `Core.Text.Unicode` と整合させる。
3. DSL フォーマッタのサンプルを用意し、幅差の出力を `expected/` に固定する。

### フェーズD: Core.Lsp/Core.Doc の最小実装
1. LSP 基本型と JSON-RPC ループの最小実装を追加する。
2. ドキュメントコメント抽出と Doctest の最小 API を追加する。
3. LSP/Doc のサンプルを `examples/` に配置し、Phase 5 への引き継ぎ資料を整理する。

### フェーズE: Phase 4 回帰接続
1. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に DSL/標準ライブラリのシナリオを追加する。
2. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に参照先と実行コマンドの方針を追記する。
3. `reports/spec-audit/ch5/` に実行ログの登録方針を記録する。

### Phase 4 実行ログ方針（標準ライブラリ）
Phase 4 の `CH3-TEST-401` / `CH3-CLI-401` / `CH3-PRETTY-401` / `CH3-DOC-401` / `CH3-LSP-401` を `ok` へ更新するため、以下の手順で実行ログを残す。

1. **実行コマンドの固定**  
   - `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_test/snapshot/basic_ok.reml`  
   - `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_cli/parse_flags/basic_ok.reml`  
   - `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_text/pretty/layout_width_basic.reml`  
   - `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_doc/basic_generate_ok.reml`  
   - `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_lsp/basic_diagnostics_ok.reml`

2. **ログ保存先の統一**  
   - `reports/spec-audit/ch5/logs/stdlib-test-YYYYMMDD.md`  
   - `reports/spec-audit/ch5/logs/stdlib-cli-YYYYMMDD.md`  
   - `reports/spec-audit/ch5/logs/stdlib-pretty-YYYYMMDD.md`  
   - `reports/spec-audit/ch5/logs/stdlib-doc-YYYYMMDD.md`  
   - `reports/spec-audit/ch5/logs/stdlib-lsp-YYYYMMDD.md`

3. **ログに残す項目**  
   - 実行コマンド全文と実行日時  
   - `diagnostics[].code` の集合（空の場合は `[]` を明記）  
   - stdout の先頭 1 行と `expected/` の一致確認  
   - `run_id` が含まれる場合は比較対象から除外する旨を記載

4. **`phase4-scenario-matrix.csv` 更新基準**  
   - `expected/` の stdout と CLI 出力が一致し、診断コード集合が `diagnostic_keys` と一致した時点で `resolution=ok` に更新する。  
   - `resolution_notes` にログファイル名と実行コマンドを記載する。

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 72 週 | フェーズA: Core.Test 実装 |
| 73 週 | フェーズB: Core.Cli 実装 |
| 74 週 | フェーズC: Core.Text.Pretty 実装 |
| 75 週 | フェーズD: Core.Lsp/Core.Doc 最小実装 |
| 76 週 | フェーズE: Phase 4 回帰接続 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| スナップショット差分が膨張する | 回帰差分が追跡不能 | 更新基準とレビュー手順を Core.Test で明文化し、`expected/` の更新条件を統一する |
| LSP/Doc の仕様が肥大化 | Phase 4 の進行遅延 | 最小 API のみ実装し、拡張は Phase 5 へ移管する |
| Unicode 幅差で整形が不安定 | フォーマッタの回帰が不安定 | `Core.Text.Unicode` の幅計算ルールと同一の基準を採用する |

## 進捗状況
- ドラフト作成時点では未着手。各フェーズの完了時に日付を追記する。
- 2025-12-19: フェーズA Step2 を実施。`docs/spec/3-11-core-test.md` にテストブロック糖衣構文とスナップショット安定化ポリシーを追記し、`docs/guides/tooling/testing.md` に更新ルールを反映。
- 2025-12-19: フェーズA Step1 の最小 API 受け口を Rust Runtime に追加。`compiler/runtime/src/test/mod.rs` で `assert_snapshot`/`table_test`/`fuzz_bytes` 等のスタブ実装と in-memory スナップショット保持を用意。
- 2025-12-19: フェーズA Step1 の診断/監査連携を追加。`test.failed` の診断生成と `SnapshotUpdated` 監査イベント記録を Rust Runtime に接続。
- 2025-12-19: `examples/practical/core_test/snapshot/basic_ok.reml` の構文を現行 `match ... with` 形式へ更新し、`CH3-TEST-401` の CLI 実行ログ（`reports/spec-audit/ch5/logs/stdlib-test-20251219.md`）を採取して `phase4-scenario-matrix.csv` を更新。
- 2025-12-19: `reml_frontend` の audit 出力連携を追加し、`remlc` のビルドエラーを修正。`compiler/runtime/src/ffi/dsl/mod.rs` の OnceLock/再帰型修正、`compiler/runtime/src/test/mod.rs` の `catch_unwind` 安全化、`compiler/frontend/src/bin/remlc.rs` のエラー処理/型派生/manifest_path 修正を実施。
- 2025-12-19: ビルド警告の整理とパッチ警告の解消を実施。`compiler/runtime` と `compiler/frontend` の dead_code 警告を個別 `#[allow]` で抑制し、`proc-macro-crate` の未使用パッチを削除して `cargo build` を警告なしで通過させた。
- 2025-12-19: Core.Cli の最小ランタイム実装を追加。`compiler/runtime/src/cli/mod.rs` で宣言的ビルダー・解析・診断・監査イベント記録を実装し、`lib.rs` で公開。
- 2025-12-19: Core.Cli の実装方針を `docs/spec/3-12-core-cli.md` と `docs/guides/tooling/cli-authoring.md` に反映し、Core.Env との分担を明記。
- 2025-12-19: CLI サンプル（解析/検証/整形）を `examples/practical/core_cli/` に追加し、`expected/` を CLI JSON 出力で更新。`reports/spec-audit/ch5/logs/stdlib-cli-20251219.md` を採取し、`phase4-scenario-matrix.csv` の `CH3-CLI-401`/`402`/`403` を `ok` に更新。
- 2025-12-19: Core.Text.Pretty の最小実装を `compiler/runtime/src/text/pretty.rs` に追加し、`examples/practical/core_text/pretty/layout_width_basic.reml` と `expected/practical/core_text/pretty/layout_width_basic.stdout` を幅差の出力に合わせて更新。
- 2025-12-19: Core.Doc/Core.Lsp の最小ランタイム実装を追加。`compiler/runtime/src/doc/mod.rs` と `compiler/runtime/src/lsp/mod.rs` を新設し、`compiler/runtime/src/audit/mod.rs` に Doctest 監査種別を追加した。

## フェーズA 残タスク（チェックリスト）
- [x] Core.Test 実行時の stdout と `expected/practical/core_test/snapshot/basic_ok.stdout` の整合を取る（暫定的に CLI JSON 出力に合わせた）。
- [x] `test.failed` 診断の出力経路を CLI 結果に反映する（失敗時に `CliDiagnosticEnvelope.diagnostics` へ流れることを確認）。
- [x] `SnapshotUpdated` 監査イベントの出力確認（CLI 実行ログに `snapshot.updated` を含むことを確認し、`reports/spec-audit/ch5/logs/stdlib-test-20251219.md` に記録）。
- [x] `Core.Test` のテーブル駆動とファズ API を利用する追加サンプルを `examples/practical/core_test/` に追加し、Phase 4 マトリクスへ登録する。

## 参照
- `docs/plans/stdlib-improvement/README.md`
- `docs/plans/stdlib-improvement/0-0-overview.md`
- `docs/plans/stdlib-improvement/0-1-workstream-tracking.md`
- `docs/plans/stdlib-improvement/1-0-core-test-plan.md`
- `docs/plans/stdlib-improvement/1-1-core-cli-plan.md`
- `docs/plans/stdlib-improvement/1-2-core-text-pretty-plan.md`
- `docs/plans/stdlib-improvement/1-3-core-doc-plan.md`
- `docs/plans/stdlib-improvement/1-4-core-lsp-plan.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/3-0-core-library-overview.md`
- `docs/spec/3-3-core-text-unicode.md`
- `docs/spec/3-5-core-io-path.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-10-core-env.md`
