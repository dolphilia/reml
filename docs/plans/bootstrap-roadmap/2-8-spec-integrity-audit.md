# 2.8 仕様完全性監査・最終調整計画

## 目的
- Phase 2 の最終段として、仕様書（Chapter 0〜3）と実装の乖離を徹底的に洗い出し、残りの仕様差分・記述漏れを解消する。
- Rust 版 Reml コンパイラ（`compiler/rust/`）を唯一のアクティブ実装として監査し、Phase 2 で積み上げた `docs/plans/rust-migration/` 系列の成果を Bootstrap Roadmap に再合流させる。
- Phase 3 以降のセルフホスト移行に耐えうるドキュメント品質と参照体制を確立し、外部公開に備えた監査ログ・仕様索引を仕上げる。

## スコープ
- **含む**: 仕様書の全文レビュー、差分リストの完結、索引・脚注・ガイド・ノートの整合、CI によるリンク/スキーマ検証、リスク登録の更新。
- **含まない**: 新機能や将来拡張の提案、新たな API 設計（必要であれば Phase 3 にタスク化）。
- **前提**:
  - Phase 2-5 で主要差分の補正案が承認済みであり、修正案のドラフトが揃っていること。
  - Phase 2-7 で診断・監査パイプラインの運用が安定していること（CI ゲート・LSP テスト完了）。[^phase27-handshake-2-8]
  - 技術的負債リストのうち Phase 2 内で解消できる項目が処理済みで、残項目が Phase 3 引き継ぎとして仕分け済みであること。
  - `docs/plans/rust-migration/overview.md` と `docs/plans/rust-migration/unified-porting-principles.md` に記録された Rust 実装の要件と成果が参照でき、Phase 2-8 で追加の移植作業を行う必要がないこと。

## 作業ディレクトリ
- `docs/spec/0-*` : 索引用資料、用語集、スタイルガイド
- `docs/spec/1-*`, `2-*`, `3-*` : 各章本文・付録
- `docs/guides/` : ガイド整合、AI 連携資料
- `docs/notes/` : 監査結果・TODO 記録
- `docs/plans/` : 既存計画書との相互参照
- `reports/` : 監査ログ・ダッシュボード・差分レポート
- `scripts/` : リンクチェック・スキーマ検証用ツール
- `compiler/rust/` : 仕様整合性を直接確認するための現行実装とテスト資産（Phase 3 以降の主対象）
- `compiler/ocaml/` : 参考資料として参照するのみで、CI や dual-write では利用しない（差分調査時に限定的に参照）

## Rust 実装の現状把握（2025-02 調査）
- `cargo test --manifest-path compiler/rust/frontend/Cargo.toml` は `compiler/rust/frontend/src/streaming/flow.rs:160` の `StreamFlowState::latest_bridge_signal` が `Option<Option<RuntimeBridgeSignal>>` を返してしまい `E0308` で失敗。`poc_frontend` バイナリも同一実装に依存するため CLI 自体がビルド不能であり、`docs/spec/1-1-syntax.md` のサンプル検証や `docs/guides/core-parse-streaming.md` §3/§7 に記載された `RuntimeBridgeSignal` 診断ルールを Rust 版で確認できない。
- `cargo test --manifest-path compiler/rust/adapter/Cargo.toml` は通常権限では `network::tests::tcp_connect_roundtrip` が `Operation not permitted` で失敗するが、ローカル TCP bind が許可された環境では 14 件すべて成功した。Phase 2-8 の監査では `docs/spec/3-10-core-env.md` で期待される監査メタデータを収集するために、ネットワーク権限の前提条件を明文化する必要がある。
- `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml` は 18 件通過したが、`audit::BridgeAuditMetadata::as_json` が未使用のままで `docs/spec/3-8-core-runtime-capability.md` で定義された `bridge.*` メタデータとの結合が未完了。廃止するのか `audit.log` へ統合するのか Phase 2-8 中に判断する。
- `reports/spec-audit/`（`ch1/`, `ch2/`, `summary.md` を含む）が未作成のため、本計画から参照している成果物リンクがすべて不在。Rust 版で取得したベースラインを格納するディレクトリ構成を Phase 2-8 の開始直後に用意する。

## 作業ブレークダウン

### 1. 監査準備とベースライン収集（36週目後半） ✅ 完了
**担当領域**: 準備・計画

1.1. **差分リスト統合**
- Phase 2-5 で作成した差分リストと Phase 2-7 の更新結果を統合し、章・カテゴリ別に並べ替える。2025-11-17 時点で `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#phase28-diff-class` に Chapter 別の差分分類を追加し、Rust 実装の `rust-gap` 管理へ流用する。[^phase28-diff-class-footnote]
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の「差分分類」を Phase 2-8 起動週のベースラインとして固定し、以降の更新は `reports/spec-audit/diffs/` と `docs/notes/spec-integrity-audit-checklist.md` へログを残す。

1.2. **レビューチーム編成**
- Chapter 0〜3 の担当者を再割当し、レビューウィンドウ（36-38週）を設定。`0-3-audit-and-metrics.md` §0.3.4 を更新し、各 WG の責任チャネル（`#spec-core` 等）とレビュー頻度を明文化。
- `0-3-audit-and-metrics.md` に `0.3.4a Phase 2-8 仕様監査スプリント` 表を追加し、週次の担当範囲（W36: 差分統合/Chapter 0, W37: Chapter 1/2, W38: Chapter 3）と Rust 成果物（`reports/spec-audit/ch*/`）を紐付けた。

1.3. **検証ツール整備**
- `scripts/validate-diagnostic-json.sh`, `scripts/ci-detect-regression.sh`, `scripts/ci-validate-audit.sh` 等を監査モードで再実行し、ベースライン成果物を `reports/audit/phase2-final/` に集約。Rust 専用の集約先として `reports/spec-audit/`（`ch0/`〜`ch3/`, `diffs/`, `summary.md`）を新設した。
- `docs/notes/spec-integrity-audit-checklist.md` を Phase 2-8 仕様で更新し、`Phase 2-8 初動チェック` と `rust-gap トラッキング表` を追加。`reports/spec-audit/summary.md` と相互参照し、Chapter ごとの TODO を可視化する。
- `reports/spec-audit/README.md`・各 `ch*/README.md` を作成し、Rust 版 CLI/テストの保存形式を定義。Chapter 0 のリンク検証ログ（`reports/spec-audit/ch0/links.md`）を含め、現時点で参照先が存在しなかったリンクを解消した。

1.4. **Rust フロントエンドのベースライン復旧**
- `compiler/rust/frontend/src/streaming/flow.rs` の `StreamFlowState::latest_bridge_signal` で発生している `Option<Option<RuntimeBridgeSignal>>` の型不整合を解消し、`RuntimeBridgeSignal` が `docs/spec/3-6-core-diagnostics-audit.md` に沿って単一のイベントとして取得できるようにする（2025-11-17 時点で `cargo test` が全件成功することを確認済み）。
- `cargo test --manifest-path compiler/rust/frontend/Cargo.toml` と `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --help` を通し、Rust CLI を Chapter 0〜3 の監査に利用できる状態へ戻す。実行ログは `reports/spec-audit/summary.md` に追記し、失敗時は差分リストへ `rust-gap` ラベルで記録する。

**成果物**: 統合差分リスト、監査スケジュール、検証ベースライン

### 2. Chapter 0〜1 監査（37週目前半）
**担当領域**: 基本方針と言語コア

2.1. **索引・用語集整合**
- [0-0-overview.md](../../spec/0-0-overview.md), [0-2-glossary.md](../../spec/0-2-glossary.md) を最新仕様と照合し、Term/Definition を更新。
- `docs/spec/0-3-code-style-guide.md` のコード例を Reml スタイルで再確認し、誤記修正。

2.2. **言語コア仕様の整合**
- Chapter 1 (1-1〜1-5) の全文レビューを行い、Phase 2 実装で導入した効果タグ・型クラス辞書・Unicode 対応を再検証。
- 擬似コード・BNF の更新漏れをチェックし、`docs/spec/1-5-formal-grammar-bnf.md` を最新に更新。

2.3. **サンプル検証**
- Rust 版 Reml CLI (`compiler/rust/` ビルド成果) により Chapter 1 のサンプルコード全件をパース/型推論し、結果を `reports/spec-audit/ch1/` に保存。
- CLI 実行手順（`cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --input docs/spec/1-1-syntax/examples/*.reml` 等）を `reports/spec-audit/ch1/README.md` に記し、`docs/spec/1-1-syntax.md` のコード片と 1:1 で突き合わせる。
- エラー発生時は差分リストに追記し、修正案を `docs/notes/spec-integrity-audit-checklist.md` に記録。OCaml 実装での再現確認は任意かつリファレンス使用のみに留める。

**成果物**: 更新済み索引・用語集、Chapter 0〜1 修正案、サンプル検証ログ

### 3. Chapter 2 監査（37週目後半）
**担当領域**: パーサー API

3.1. **API 記述の最終確認**
- `Parser<T>` の型引数・エラー戦略記述を実装コード (`compiler/ocaml/src/parser/`) と照合。
- Rust 版の `compiler/rust/frontend/parser` / `streaming` モジュールと `poc_frontend` 実行ログをベースラインとし、Phase 3 以降は Rust 実装を一次参照にする方針を `docs/plans/rust-migration/overview.md` と揃える。
- `docs/guides/core-parse-streaming.md` の内容と Chapter 2 の記述を同時更新。

3.2. **例外・診断との整合**
- `docs/spec/2-5-error.md`, `2-6-execution-strategy.md` と Phase 2-7 で整備した診断 API を突き合わせ、用語とメタデータの整合を確認。
- エラーコード一覧を `docs/spec/3-6-core-diagnostics-audit.md` と同期し、参照表を付録に追加。

3.3. **リンク・脚注検証**
- Chapter 2 からのリンク（ガイド・ノート・計画書）を抽出し、リンク切れを修正。
- BNF と API サンプルの脚注を更新し、`reports/spec-audit/ch2/` に差分レポートを保存。

3.4. **Streaming ランタイムと期待トークンの監査**
- `compiler/rust/frontend/tests/streaming_metrics.rs` で定義されている `streaming_expected_token_snapshot_matches` / `streaming_diagnostics_inject_expected_tokens` を Rust 版で実行し、`docs/spec/2-5-error.md` と `docs/guides/core-parse-streaming.md` の `parse.expected` 仕様に沿って `ExpectationSummary` が埋め込まれているか確認する。
- ストリーミング関連のメトリクス（`packrat_stats`, `RuntimeBridgeSignal`）を `reports/spec-audit/ch2/streaming/` に JSON で保存し、Chapter 2 の脚注更新に引用できるよう整備する。

**成果物**: Chapter 2 修正案、リンク検証レポート、更新済みガイド

### 4. Chapter 3 監査（38週目前半）
**担当領域**: 標準ライブラリ

4.1. **ライブラリ API 整合**
- [3-0-core-library-overview.md](../../spec/3-0-core-library-overview.md)〜[3-10-core-env.md](../../spec/3-10-core-env.md) を精査し、Phase 2 で導入された診断・Capability 情報と乖離がないか確認。
- FFI/Async/Runtime 章で Stage/Ownership テーブルを更新し、`tooling/runtime/audit-schema.json` と一致させる。

4.2. **サンプルコード・図表更新**
- ライブラリ章のコード断片を Reml CLI で再検証し、結果を `reports/spec-audit/ch3/` に記録。
- 図表・フローチャートの差分がある場合は `docs/spec/assets/` を更新。

4.3. **ガイド・ノートとの同期**
- `docs/guides/plugin-authoring.md`, `docs/guides/runtime-bridges.md` などを章更新内容に合わせて調整。
- `docs/notes/dsl-plugin-roadmap.md`, `docs/notes/core-library-outline.md` に監査結果とフォローアップ TODO を記載。

**成果物**: Chapter 3 修正案、更新済み図表、ガイド整合記録（Rust 実装で再現確認済み）

### 5. 修正反映とクロスチェック（38週目後半）
**担当領域**: 最終更新

5.1. **修正案の適用**
- 各章の修正案をマージし、Git 管理の差分を `reports/spec-audit/diffs/` に保存。
- 大規模修正は PR 単位でレビューし、承認ログを `docs/notes/spec-integrity-audit-checklist.md` に記録。

5.2. **リンク・スキーマ検証**
- `scripts/ci-detect-regression.sh` にリンクチェックと JSON Schema 検証を統合し、`spec-audit` モードで実行。
- Rust 実装のテスト (`cargo test -p compiler` 等) と同じステージで実行されるよう CI 手順を同期させ、結果を `reports/spec-audit/summary.md` にまとめる。CI での自動実行手順は `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` に追記。

5.3. **用語・索引最終更新**
- `docs/README.md`, `README.md` の目次・リンクを更新し、`docs/plans/repository-restructure-plan.md` の進捗を反映。
- 用語集・索引に新旧用語の対応表を追加し、Phase 3 で参照するための脚注を整備。

**成果物**: 更新済み仕様書、検証レポート、索引・リンクの最終版

### 6. リスク登録と Phase 3 引き継ぎ（39週目）
**担当領域**: 記録整備

6.1. **残課題の整理**
- 解決できなかった差分・仕様不明点を `0-4-risk-handling.md` に登録し、優先度を設定。
- Phase 3 で扱うべき TODO を `docs/notes/spec-integrity-audit-checklist.md` に残す。

6.2. **メトリクス更新**
- `0-3-audit-and-metrics.md` に監査件数・修正件数・未解決件数を記録。
- `reports/audit/dashboard/` に Phase 2 の最終スナップショットを保存し、Phase 3 の比較ベースとする。

6.3. **ハンドオーバー**
- Phase 3 リーダー向けに `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` から参照できるハンドオーバー節を作成し、Rust 実装で達成済みの監査項目を一覧化する。
- 仕様更新履歴を `docs/notes/spec-update-log.md`（新設）にまとめ、外部公開時の変更点追跡と Rust 実装への反映状況を容易にする。

**成果物**: リスク登録、メトリクス更新、Phase 3 向けハンドオーバー資料

## 成果物と検証
- 仕様書 Chapter 0〜3 の差分が解消され、CI/手動検証でリンク切れ・スキーマ不整合がゼロであること。
- Rust 実装（`cargo test --manifest-path compiler/rust/frontend/Cargo.toml`、`--manifest-path compiler/rust/runtime/ffi/Cargo.toml`、`--manifest-path compiler/rust/adapter/Cargo.toml` および `cargo run --bin poc_frontend`）で Chapter 0〜3 のサンプルと監査ツールがすべて実行され、結果が `reports/spec-audit/*` に保存されていること。
- 監査レポート (`reports/spec-audit/summary.md`) と差分ログが公開され、レビュー履歴が残っていること。
- 用語集・索引が最新状態で、Phase 3 計画書から参照できること。

## リスクとフォローアップ
- Rust フロントエンドのビルド失敗（`StreamFlowState::latest_bridge_signal` の型不整合）が長期化すると、Chapter 0〜3 のサンプル検証全体が停止する。`rust-gap` ラベルでトラッキングし、Phase 2-8 内で必ず解消する。
- `reml_adapter` のネットワーク試験に必要なローカル TCP bind 権限が確保できない環境では `docs/spec/3-10-core-env.md` の監査証跡を取得できない。CI/ローカル双方で権限要件を明示し、許可が得られない環境向けにモック動作を定義する。
- 監査範囲の広さによるスケジュール遅延: 優先順位付けを徹底し、Phase 3 に移送する基準を明示。
- 記述更新によるガイド・ノートへの波及: クロスリンク管理を `docs/plans/repository-restructure-plan.md` で追跡し、一括更新スクリプトの導入を検討。
- 外部公開向けチェック未整備: Phase 3 での公開を見据え、ライセンス・記法・翻訳関連の TODO を `docs/notes/spec-integrity-audit-checklist.md` に残す。

## 参考資料
- [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md)
- [2-7-deferred-remediation.md](2-7-deferred-remediation.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [docs/spec/0-0-overview.md](../../spec/0-0-overview.md)
- [docs/spec/3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [docs/notes/repository-restructure-plan.md](../notes/repository-restructure-plan.md)
- [docs/plans/rust-migration/overview.md](../rust-migration/overview.md)
- [docs/plans/rust-migration/unified-porting-principles.md](../rust-migration/unified-porting-principles.md)

---

### Rust 実装集中への補足
- Phase 2-8 の監査完了をもって、dual-write や OCaml 実装ベースの回帰テストは停止する。必要な場合のみ `compiler/ocaml/` を参照し、差分や履歴を確認する。
- Rust 実装で未着手の Chapter 3 機能は 2-8 の差分リストに `rust-gap` ラベルを付け、3-0 以降のタスクへ直接引き継ぐ。
 - 3-x 以降の成果物（Prelude/Collections/Diagnostics 等）を Rust 実装に合わせて更新する際は、2-8 で整理した脚注・索引・監査ロジックを共通の基盤として利用し、Phase 2 で確立した測定・リンク検証スクリプトを維持する。

### 3-0 着手判断のための Rust フロントエンド PoC 評価（W38 集計）
Phase 3 では Rust 実装のみをアクティブに扱う（`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` §3.0.2）。そのため、2-8 完了の可否を判断する前にフロントエンド PoC を脱却できているかを確認した。コードリーディングと `docs/plans/rust-migration/1-0-front-end-transition.md` / `p1-rust-frontend-gap-report.md` のチェックリストを突き合わせた結果、以下の観測差分が残っている。

**観測結果**
- `compiler/rust/frontend/src/parser/mod.rs` は冒頭で自らを「Rust フロントエンド PoC」と明記しており、`parser/api.rs` も `#![allow(dead_code)]` で丸ごと抑制されたまま `State::memo` がダミー `()` になっている。`RunConfig`/`ParseResult` 抽象が CLI から利用されず、Packrat/Menhir 互換 API が Phase 1 の目標どおりには露出していない。[^parser-poc]
- パーサのトレースイベントは `ModuleHeader` と `UseDecl` の 2 種類のみで、`effect_handler.reml` など BNF §4-§5 の構文に対する `syntax:expr-*` トレースが出力されない。`reports/spec-audit/ch1` へ残すべき差分ログが十分に取得できず、dual-write で観測点を固定するという `unified-porting-principles.md` §2 の要件を満たせていない。[^trace-coverage]
- CLI `poc_frontend` 自体が PoC 用途のままで、`TypecheckDriver::infer_fallback_from_source` がソース文字列を正規表現的に走査して関数名を抽出する暫定処理を残している。AST が生成できなかった場合に「それらしい統計」を捏造する挙動が Chapter 3 の API 連携や監査ログに入り込むと、Phase 3 のセルフホスト計画で期待する「Rust 実装を唯一の証拠とする」という前提が崩れる。[^poc-cli][^typeck-fallback]

**ブラッシュアップ提案（Phase 2-8 Exit 条件への追補）**
1. Parser API の本格実装: `parser/api.rs` を `docs/spec/2-1-parser-type.md` どおりに有効化し、`State::memo` へ Packrat キャッシュを組み込み `ParserDriver` 側で `Parser<T>` を経由して CLI へ結果を返す。W37 で進めた AST 整備に続き、`docs/plans/rust-migration/1-0-front-end-transition.md` の W2/W3 テーブルに「API 本体と RunConfig 連携完了」を追記し、`reports/spec-audit/diffs/SYNTAX-002` に観測点の更新を残す。
2. Trace/diagnostic 拡張: `ParserTraceEventKind` に式・効果・`operation/resume` を追加し、`TraceEvent::ExprEnter`/`ExprLeave` を `syntax:expr-*` 命名規約で固定。`reports/spec-audit/ch1/effect_handler-*.json` へ Streaming/非 Streaming 双方のトレースを焼き付け、`docs/notes/spec-integrity-audit-checklist.md` の `rust-gap SYNTAX-003` 行を「Trace 取得済み」でクローズできる状態を Phase 2-8 のクローズ条件に含める。
3. CLI プロモーションと型推論の厳格化: `compiler/rust/frontend/src/bin/poc_frontend.rs` を `reml_frontend`（仮称）へリネームし、`TypecheckDriver::infer_fallback_from_source` を廃止して AST 生成失敗時はそのまま診断エントリで停止する仕様に切り替える。Stage/Audit 出力も `docs/spec/3-6-core-diagnostics-audit.md` の JSON スキーマへ揃え、`FRG-12`（型推論）と `FRG-05`（Streaming RunConfig）の残差を Phase 2-8 内で「Closed」に更新してから 3-0 へ移行する。

上記 3 点が整うまでは `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の M1 に着手しない。ブラッシュアップ作業は 2-8 内の残り週次（W38 終了まで）で進め、`docs-migrations.log` に PoC からの脱却ログを明記する。

## 具体的な計画

### Rust Frontend パーサ拡張のステップ

Phase 2-8 W37（Chapter 1/2 フォーカス）の 2 週間で Rust Frontend の仕様整合を完了させ、W38 で Streaming メトリクスを CI へ昇格させる。スケジュールと成果物は下表の通り。[^frontend-plan-ref]

| ステップ | 期間 (Sprint W37-39) | 主担当 | 主要成果物 |
| --- | --- | --- | --- |
| 1. AST トップレベル整備 | W37 前半 | Rust Parser WG | `ast.rs`/`module_parser.rs` の diff、`reports/spec-audit/ch1/use_nested-*.json` 更新 |
| 2. 式・効果構文 | W37 後半 | Rust Parser WG + Effects WG | `effect_handler.reml` を受理する実装と診断ログ、`rust-gap SYNTAX-003` 終了メモ |
| 3. module_parser 再実装 | W37→W38 | Parser QA | `tests/parser.rs` 追加、フェーズ別進捗メモ、`rust-gap` 行更新 |
| 4. Streaming/CI 反映 | W38 前半 | Streaming WG | `tests/streaming_metrics.rs` 拡張、CI 実行結果、`docs/spec/0-3-code-style-guide.md` 更新 |
| 5. Parser API 本格実装と CLI 連携 | W38 後半 | Parser WG + CLI Maintainers | `parser/api.rs` 公開、`parser_driver` diff、`reports/spec-audit/ch1/parser_api-*.md` 追加 |
| 6. Trace / Diagnostics 拡張 | W39 前半 | Parser WG + Observability WG | `ParserTraceEventKind` 拡張、`trace-coverage-*.md` と `effect_handler-*-trace.json` |
| 7. CLI プロモーションと型推論の厳格化 | W39 後半 | Typecheck WG + Runtime WG | `reml_frontend` CLI、`typeck.aborted.ast_unavailable` 診断、`docs/spec/0-3-code-style-guide.md` 改訂 |

1. **Rust Parser のトップレベル拡張**
   - `compiler/rust/frontend/src/parser/ast.rs` へ `ModuleHeader`、`UseDecl`、`OperationDecl`、`HandlerDecl` を正式に追加し、`Function`/`Effect` から `TypeAnnot` を共有できるように `AnnotationKind` を導入する。`docs/plans/rust-migration/1-0-front-end-transition.md` の AST 対応表を同時に更新する。
   - `module_parser` 序盤で `module` と `use` を確定させ、`docs/spec/1-1-syntax/examples/use_nested.reml` が構文エラーにならない状態を確認する。`cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` を実行し、`reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` を差し替えて `rust-gap SYNTAX-002` のステータスを `docs/notes/spec-integrity-audit-checklist.md` に反映する。
   - AST 差分は `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` に追記し、`docs/spec/1-1-syntax.md` の脚注から暫定 `use_nested_rustcap.reml` を削除する条件を明示する。
   - **実務手順**（W37 前半の 3 日を上限）:
     1. `ast.rs` 現状の `DeclKind` / `ItemKind` 定義を OCaml 版 `Ast.ml` と突き合わせ、`module`/`use`/`effect handler` のどこに欠損があるかを明示したチェックリストを `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` に追加する。
     2. `ModuleHeader` では `identifier`/`visibility`/`attributes`/`span` を最小単位とし、`UseDecl` では `path`・`alias`・`glob` フラグを OCaml `Use.item` 構造に揃える。`OperationDecl`・`HandlerDecl` は `TypeAnnot` の共有と effect/handler 関連脚注（`docs/spec/1-1-syntax.md` §5）に沿った `Kind` フラグを持たせる。
    3. `module_parser::parse_module` の冒頭を `parse_module_header` → `parse_use_list` → `parse_decl_list` に三分割し、`Result<Module, ParserError>` を返す箇所に AST 差分ログ用の `TraceEvent::ModuleHeaderAccepted` を追加する。`docs/plans/rust-migration/unified-porting-principles.md` の「差分検証前にトレースを固定する」指針に合わせて、`trace_id` を `syntax:module-header` で固定する。
    4. CLI 再実行コマンドと `use_nested.reml` の差し替え先を `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` に追記し、`reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` の保存規約（`YYYYMMDD` は CI 実行日、`git rev-parse HEAD` を脚注に記載）を明文化する。
    5. `docs/plans/rust-migration/1-0-front-end-transition.md` の「W2 具体的な進め方」配下に `ModuleHeader/UseDecl` の完了条件と成果物リンク (`ast.rs` diff, `reports/spec-audit/ch1/...`) を追加し、`P1` 期からの AST 対応表が Phase 2-8 へ継続していることを示す。
   - ✅ 進捗メモ（2025-11-17 更新）: `ModuleHeader`/`UseDecl` の整備、`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` の記録、ブロック/`match` 構文の実装が完了し、`use_nested.reml` が診断 0 件で通過する。`reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` / `use_nested-20251117-trace.md` をベースラインとして Phase 3 へ引き継ぐ。
   - **検証とクローズ条件**:
     - `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml` が診断 0 件で完了し、`use_nested_rustcap.reml` が不要になったことを `docs/spec/1-1-syntax.md` 脚注で案内できる状態。
     - `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` に「Expected Result」「Current Result」「対処ステータス: In Progress→Closed」まで記入し、`docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-002` 行に `owner: Rust Parser WG / due: W37` を追記。
     - `docs/plans/rust-migration/unified-porting-principles.md` §2 の `同一観測点の再現` を満たす証拠として、`parser_driver` dual-write トレースを `reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` と同じフォルダにリンクする（ファイル名: `use_nested-YYYYMMDD-trace.md`）。トレース取得方法は `scripts/poc_dualwrite_compare.sh use_nested` を明記。
2. **式/ブロック/効果構文の段階的実装**
   - `module_parser` の式パーサを `ExprParser` として分離し、`{ ... }` ブロック、`let`/`var` 代入、`return`、`do`/`perform`、`handle ... with handler { ... }` を Chapter 1 BNF に沿って段階導入する。[^syntax-bnf]
   - ハンドラ構文では `operation log(args, resume) { ... }` を `DeclKind::Handler` と `OperationDecl` に展開し、`resume` の型検証を `TypeAnnot` と共有する。`docs/spec/1-1-syntax/examples/effect_handler.reml` を受理できることを Rust Frontend CLI で再現し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` を保存する。
   - 受理後に `docs/notes/spec-integrity-audit-checklist.md` の `rust-gap SYNTAX-003` をクローズし、`docs/plans/rust-migration/p1-rust-frontend-gap-report.md` へ差分結果を逆参照する。
   - **実務タスク**（W37 後半の 3 日を上限）:
      1. `compiler/rust/frontend/src/parser/` に `expr.rs`（`ExprParser`）を追加し、`module_parser.rs` から式関連の `Parser` 実装を分離する。`Expr` バリアントへ `Block`, `Let`, `Do`, `Perform`, `Handle`, `Resume`, `Return` を追加し、`docs/plans/rust-migration/1-0-front-end-transition.md` の AST 対応表と突き合わせて整合させる。トレース用には `TraceEvent::{ExprEnter,ExprLeave}` を拡張し、`trace_id` を `syntax:expr-*` 規約で固定する。
     2. ブロックと代入の構文は `docs/spec/1-1-syntax.md` §4.1〜§4.3 の BNF を忠実に反映し、`docs/spec/1-1-syntax/examples/block_scope.reml` を使った CLI 検証（`cargo run --bin poc_frontend -- docs/spec/1-1-syntax/examples/block_scope.reml --emit-diagnostics`）を `reports/spec-audit/ch1/block_scope-YYYYMMDD-diagnostics.json` として保存する。`let` は不変、`var` は可変バインディングとして `BindingKind` を AST に持たせ、`TypeAnnot` に未推論マーカーを追加する。
     3. `perform`/`do`/`handle`/`operation` は `docs/spec/1-3-effects-safety.md` と `docs/spec/1-1-syntax.md` §5 の記述を参照し、`EffectExprKind` を導入して `perform expr`, `handle expr with handler` の両方で `EffectScopeId` を共有する。`operation` ブロック内の `resume` 引数は `TypeAnnot::Resume` を共有し、型エラー時に `effects.resume.untyped` 診断が Rust 実装でも再現できるよう `compiler/rust/frontend/src/diagnostics/mod.rs` を更新する。
     4. `docs/spec/1-1-syntax/examples/effect_handler.reml` / `docs/spec/1-1-syntax/examples/effect_logger.reml` を対象に `scripts/poc_dualwrite_compare.sh effect_handler` を追加実行し、OCaml 実装との診断同等性を `reports/spec-audit/ch1/effect_handler-YYYYMMDD-dualwrite.md` へ記録する。`docs/plans/rust-migration/1-2-diagnostic-compatibility.md` の「効果診断ギャップ」節に Rust 実装の進捗を追記し、`p1-rust-frontend-gap-report.md` では `SYNTAX-003` の残作業を `Closed(P2-8)` に切り替える。
      5. `docs/notes/spec-integrity-audit-checklist.md` に `block_scope`, `effect_handler`, `perform_do` の監査行を新設し、`owner`, `evidence(log)` を埋める。併せて `reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` へ AST・診断の差分ログを追加し、`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md` から参照できるよう脚注リンクを追記する。
   - ✅ 進捗メモ（2025-11-18 更新）: `ExprParser` 分離と `TraceEvent::ExprEnter/Leave` 拡張を完了し、`block_scope.reml`（`reports/spec-audit/ch1/block_scope-20251118-diagnostics.json`）と `effect_handler.reml`（`effect_handler-20251118-diagnostics.json`）の診断 0 件ログを取得。dual-write 結果は `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md` に保存し、`reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` を作成。`docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-003` 行を `Closed(P2-8)` に更新済み。
   - **検証とクローズ条件**:
      - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::expr`（新設）と `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml` がいずれも 0 exit / 診断一致で完了し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` が `CI_RUN_ID` 付きで保存されている。
     - `docs/spec/1-1-syntax.md` §5 の脚注から `effect_handler_rustcap.reml` などのフォールバックを削除し、代わりに `reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` への参照を追加する差分を `docs/spec/0-3-code-style-guide.md` の実行手順と同時にレビューへ回している。
     - `docs/plans/rust-migration/overview.md` の Phase 1 完了条件に「`effect_handler.reml` が Rust Frontend で受理され Streaming 経路の監査ログが揃っている」ことを追記し、`docs/plans/rust-migration/unified-porting-principles.md` §2（観測点再現）で定めた `diff-harness` へ `ExprParser` のトレースを統合した旨を `docs-migrations.log` に記録する。
3. **module_parser の再実装マイルストーン**
   - フェーズ順序: (a) module/use ヘッダ → (b) effect/fn 宣言（戻り値注釈含む）→ (c) block/let/do/handle 構文 → (d) `operation` ブロック＋`resume`。各フェーズ完了時に `compiler/rust/frontend/tests/parser.rs` へテストを追記し、Chumsky ベースの `module_parser` が後方互換であることを証明する。
   - 進捗は `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` に日付別で追記し、`reports/spec-audit/diffs/` の `rust-gap` 表に同じ ID を二重記録する。テストシナリオには `use_nested`、`effect_handler`、および `docs/spec/1-1-syntax.md` サンプルの最小ケースを含める。
   - 差分レビューは `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` の CI ブロッカー条件に沿って承認を得る。
   - **実務タスク**（W37→W38, 4 日を上限）:
      1. `compiler/rust/frontend/src/parser/module_parser.rs` を `ModuleStage::{Header,UseList,DeclList}` へ分割し、フェーズごとに `TraceEvent::ModuleStageEntered { stage }` を発火させる。トレース出力の仕様を `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` へ記録し、`docs/plans/rust-migration/unified-porting-principles.md#同一観測点の再現` に沿って `trace_id = syntax:module-*` を固定する。
      2. `compiler/rust/frontend/tests/parser.rs` に `module_header_accepts_use_nested`, `module_decl_accepts_effect_handler`, `module_operation_resume_roundtrip` などの統合テストを追加し、`cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::module -- --nocapture` を `reports/spec-audit/ch1/module_parser-YYYYMMDD-parser-tests.md` に保存する。テストケースは `docs/spec/1-1-syntax/examples/*.reml`（`use_nested`、`effect_handler`、`block_scope`）を直接読み込んで JSON 診断を比較する。
      3. `scripts/poc_dualwrite_compare.sh use_nested` / `effect_handler` を module_parser フェーズごとに再実行し、OCaml→Rust の診断 diff を `reports/spec-audit/ch1/module_parser-YYYYMMDD-dualwrite.md` と `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` の追補セクションへ転載する。`docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` の「parser チェックリスト」節にコマンドと格納先を明記する。
      4. `docs/notes/spec-integrity-audit-checklist.md` の `rust-gap` 表へ `SYNTAX-002/module_parser` 行を追加し、`reports/spec-audit/ch1/module_parser-YYYYMMDD-parser-tests.md` の `owner`, `evidence(log)` を突合できるようにする。`docs/plans/rust-migration/1-0-front-end-transition.md` にも同じ ID を記録し、Phase 1 の AST 対応表から Phase 2-8 で引き継ぐ作業であることを明示する。
      5. `docs-migrations.log` へ module_parser 再実装の要点（フェーズ分割、テスト導入、diff ログの保存ルール）を追記し、Phase 3 以降で `module_parser` を参照する際の基点を示す。
   - ✅ 進捗メモ（2025-11-20 更新）: `module_parser.rs` を `ModuleStage::{Header,UseList,DeclList}` 単位に再構成し、`TraceEvent::ModuleStageEntered(stage)` / `TraceEvent::ModuleDeclAccepted(kind)` を `trace_id = syntax:module-stage::<stage>` で固定。`compiler/rust/frontend/tests/parser.rs` には 6 本の統合テスト（`module_header_accepts_use_nested`, `module_use_reports_shadowing`, `module_decl_accepts_effect_handler`, `module_decl_reports_resume_without_operation`, `module_decl_blocks_roundtrip`, `module_decl_dualwrite_snapshot`）を追加し、`reports/spec-audit/ch1/module_parser-20251119-parser-tests.md` へ `cargo test ... parser::module -- --nocapture` のログ（`CI_RUN_ID=rust-frontend-w37-20251119.1`, `git rev-parse HEAD = f9e10ae6`）を保存した。`reports/spec-audit/ch1/use_nested-20251119-{diagnostics.json,trace.md}`, `block_scope-20251119-*`, `effect_handler-20251119-*` を生成し、dual-write 比較 (`reports/spec-audit/ch1/module_parser-20251119-dualwrite.md`) と `reports/spec-audit/ch1/2025-11-17-syntax-samples.md#2025-11-19-module_parser-再実装ログ` に証跡を反映済み。`docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-002/module_parser` 行は `Closed (P2-8 W38)` へ移行。
   - **検証とクローズ条件**:
      - ✅ 2025-11-20: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::module parser::expr -- --nocapture` を CI で再実行し、`reports/spec-audit/ch1/module_parser-20251119-parser-tests.md` に `CI_RUN_ID`・`git rev-parse HEAD`・テスト統計を保存。`reports/spec-audit/ch1/2025-11-17-syntax-samples.md` へログ要約を追記。
      - ✅ 2025-11-20: `scripts/poc_dualwrite_compare.sh use_nested` / `effect_handler` が差分 0 を報告し、`reports/spec-audit/ch1/module_parser-20251119-dualwrite.md` へ格納。`docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` の「CI ブロッカー」に `Module Parser Acceptance` フェンスと `Rollback Hook` を反映済み。
      - ✅ 2025-11-20: `docs/spec/1-1-syntax/examples/README.md`・`docs/spec/0-3-code-style-guide.md` に module_parser テスト保存ルールを追記し、`docs/plans/rust-migration/overview.md` から Phase 1 完了条件として参照できるようリンクした（`module_parser` の監査証跡は `docs/notes/spec-integrity-audit-checklist.md` `SYNTAX-002/module_parser` 行で `Closed` 管理）。
4. **Streaming テストと差分ログ更新**
   - `compiler/rust/frontend/tests/streaming_metrics.rs` に `module_header_acceptance`、`effect_handler_acceptance` などの統合テストを追加し、`StreamFlowState::latest_bridge_signal` の二重 `Option` 問題を含む既知バグを回避する再現ケースを固定化する。[^streaming-guide]
   - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics` を Phase 2-8 CI（`3-0-ci-and-dual-write-strategy.md` で定義）に登録し、成功ログを `reports/spec-audit/ch1/` に保存する。成功後は `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` / `SYNTAX-003-*.md` を `Closed` 扱いに更新する。
   - Streaming 経路で Chapter 1 サンプルが通過したら `docs/spec/1-1-syntax.md` の監査ノートからフォールバック (`*_rustcap.reml`) を削除し、`docs/spec/0-3-code-style-guide.md` の実行手順を Rust Frontend ベースに書き換える。併せて `docs/plans/rust-migration/overview.md` の Phase 1 完了条件も更新する。
   - テスト補完では `include_str!(\"../../docs/spec/1-1-syntax/examples/use_nested.reml\")` など仕様サンプルをそのままインライン化し、`StreamingRunner` + `StreamFlowState` を組み合わせた `run_streaming_sample(sample_name)` ヘルパを `streaming_metrics.rs` 内に追加する。`module_header_acceptance` では `use_nested.reml` の `StreamOutcome::Completed` と `StreamFlowMetrics::checkpoints_closed = 1` を検証し、`effect_handler_acceptance` では `StreamFlowState::record_bridge_signal` → `latest_bridge_signal` の戻り値が `Option<RuntimeBridgeSignal>` で単一段になることを `assert_matches!` で拘束する。
   - `docs/spec/1-1-syntax/examples/README.md` と `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` に「Streaming 経由での再検証」サブセクションを追加し、`streaming_use_nested-YYYYMMDD-diagnostics.json`／`streaming_effect_handler-YYYYMMDD-diagnostics.json` を `reports/spec-audit/ch1/` 配下へ保存する命名規約（`CI_RUN_ID` と `git rev-parse HEAD` を脚注で記録）を明文化する。Chapter 2 側の `reports/spec-audit/ch2/streaming/` にも同じログを複製し、`ERR-001` のチェック項目にリンクさせる。
   - `docs/notes/spec-integrity-audit-checklist.md` の `ERR-001` ブロックへ Streaming Runner 追加テストの要件を追記し、`parser.expected_summary_presence = 1.0` を `cargo test -- streaming_metrics -- --nocapture` と `tooling/ci/collect-iterator-audit-metrics.py --section streaming` の両方で検証する。`docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` には GitHub Actions で `cargo test ... streaming_metrics` を `rust-frontend-streaming` ジョブとして追加し、失敗時のロールバック（`tests/streaming_runner.rs` の再実行と `docs/spec/0-3-code-style-guide.md` のフォールバック復帰手順）を記載する。
   - `docs/migrations.log` へ Streaming テスト昇格の記録（テスト名、参照 sample、導入 CI job、`reports/spec-audit/ch1/streaming_*` ログ）を追記し、 Phase 3 のセルフホスト工程で Streaming 経路が監査済みであることを参照できるようにする。
   - **実務タスク（W38 前半 / 3 日上限）**:
      1. `streaming_metrics.rs` に `SampleCase` 列挙と `run_streaming_sample(case: SampleCase, chunk: Option<usize>)` ヘルパを追加し、`module_header_acceptance` / `effect_handler_acceptance` / `bridge_signal_roundtrip` の 3 テストを作成する。`effect_handler` 系テストは `StreamOutcome::Pending` → `Completed` フローを 2 回繰り返して `StreamFlowState::latest_bridge_signal()` の戻り値が `Some(RuntimeBridgeSignalKind::Resume)` になることを確認する。
      2. `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics -- --nocapture | tee reports/spec-audit/ch1/streaming_metrics-YYYYMMDD-log.md` を実行し、`reports/spec-audit/ch1/` のログと `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` に `Streaming acceptance` セクションを追加する。同時に `docs/spec/1-1-syntax/examples/README.md` のチェックリストへ Streaming 実行の項目を追加し、`*_rustcap.reml` の削除条件（Streaming テストと CLI 実行がともにグリーンであること）を記載する。
      3. `docs/spec/1-1-syntax.md` / `docs/spec/0-3-code-style-guide.md` / `docs/plans/rust-migration/overview.md` に Streaming ベースライン確立後の脚注更新をまとめて PR 化する。更新内容には `reports/spec-audit/ch1/streaming_use_nested-YYYYMMDD-diagnostics.json` 参照、`Phase 1 完了条件 = Streaming 経路合格`、および `poc_frontend --stream` 実行例を含める。
   - ✅ 進捗メモ（2025-11-21 更新）: `compiler/rust/frontend/tests/streaming_metrics.rs` に `SampleCase` と 3 つの Streaming 統合テストを実装し、`cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics -- --nocapture` の結果を `reports/spec-audit/ch1/streaming_metrics-20251121-log.md`（`CI_RUN_ID=rust-frontend-streaming-20251121.1`）へ保存。`reports/spec-audit/ch1/streaming_use_nested-20251121-diagnostics.json` / `streaming_effect_handler-20251121-diagnostics.json` を追加し、`docs/spec/1-1-syntax.md`・`docs/spec/0-3-code-style-guide.md`・`docs/spec/1-1-syntax/examples/README.md`・`docs/plans/rust-migration/overview.md`・`docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md`・`reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` へ Streaming ベースライン確立の脚注と CI 手順を反映した。`docs/notes/spec-integrity-audit-checklist.md#期待集合err-001` も `parser.expected_summary_presence = 1.0`（Streaming ジョブ）で更新済み。
   - **検証とクローズ条件**:
      - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics streaming_runner -- --nocapture` が CI とローカルの双方で成功し、`reports/spec-audit/ch1/streaming_metrics-YYYYMMDD-log.md` に `CI_RUN_ID` と `git rev-parse HEAD` を残している。
      - `reports/spec-audit/ch1/streaming_use_nested-YYYYMMDD-diagnostics.json` / `streaming_effect_handler-YYYYMMDD-diagnostics.json`（および `ch2/streaming/` の同名ファイル）が生成され、`docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` の `ERR-001` 行に `Closed (P2-8 W38)` を記入済み。
   - `docs/spec/1-1-syntax.md` から `*_rustcap.reml` の脚注を削除し、代わりに Streaming 監査ログへのリンクを記載。`docs/spec/0-3-code-style-guide.md` と `docs/plans/rust-migration/overview.md` が Rust Frontend + Streaming ベースで更新され、`docs-migrations.log` に反映済み。

5. **Parser API 本格実装と CLI 連携（ブラッシュアップ提案①対応）**
   - `parser/api.rs` で抑止している `#![allow(dead_code)]` を解除し、`Parser<T>`/`State`/`Reply` を `parser/mod.rs` の `ParserDriver` 実装へ統合する。`State::memo` には Packrat キャッシュ構造体（`PackratNotebook` 仮称）を保持し、`PackratCacheEntry` を `ParseResult` へ復元する経路を CLI まで接続する。
   - `RunConfig` を CLI から JSON として受け渡す機構を `ParseResult` に統合し、`ParserOptions::from_run_config` のパラメータが `reports/spec-audit/ch1/*-parse-result.json` に保存されるようにする。`docs/spec/2-1-parser-type.md` と `parser_driver.ml` の API 差分は `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` へ追記する。
   - `docs/plans/rust-migration/1-0-front-end-transition.md` の W2/W3 チェックリスト、および `p1-rust-frontend-gap-report.md` の `FRG-07`/`FRG-11` セクションへ対応状況を連携させる。
   - **成果物・ログ管理**:
     - `reports/spec-audit/ch1/parser_api-YYYYMMDD-log.md`（CI ログ）、`parser_api-YYYYMMDD-parse-diff.md`（OCaml との差分）、`use_nested-YYYYMMDD-parse-result.json`（RunConfig + PackratCache）をひとまとめに保存し、`reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` から参照する。
     - `docs/notes/spec-integrity-audit-checklist.md` の `FRG-07` と `FRG-11` に `owner: Parser WG`・`artifact: parser_api-YYYYMMDD-log.md` を記録し、進捗を W38 後半のレビューで共有する。
     - `docs-migrations.log` に「Parser API 切替」「RunConfig JSON 保存ルール」「poc_dualwrite_compare.sh parse_result 拡張」の 3 行を追記し、履歴を Phase 3 に持ち込めるようにする。
   - **CI / 観測ポイント整備**:
     - `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` に `rust-frontend-parser-api` ジョブを追加し、`cargo test -- parser::api parser::driver` と `scripts/poc_dualwrite_compare.sh use_nested --emit-parse-result` を同一ワークフローで走らせる。失敗時は `ParserDriver` を旧 API へロールバックできるよう `parser/api_legacy.rs` を 1 リリース残す。
     - `tooling/ci/collect-iterator-audit-metrics.py` へ `parse_result.packrat_cache_entries` メトリクスを加え、`reports/spec-audit/ch1/parser_api-YYYYMMDD-metrics.json` へ書き出す。Chapter 2 側では `reports/spec-audit/ch2/parser_api-YYYYMMDD-metrics.json` を同期させ、`ERR-001` チェックリストから参照する。
   - **リスク・フォールバック指針**:
     - Packrat キャッシュ導入によりメモリ使用量が跳ねる恐れがあるため、`RunConfig.packrat.memo_limit_bytes` を CLI から受け取れるようにし、`docs/plans/rust-migration/overview.md` Phase 1 のリスク表へ `Parser API memory spike` を追記する。
     - CLI 連携に失敗した場合は `poc_frontend` 既存の `infer_fallback_from_source` に戻らず、`ParserDriver` が `ParseResult` なしで終了できるよう `--allow-parser-api-failure` フラグを 1 スプリント限定で用意する。使用状況は `reports/spec-audit/ch1/parser_api-fallback-usage.md` にログを残す。
   - **関連ドキュメント更新**:
     - `docs/spec/2-1-parser-type.md` に `Parser<T>` の Rust 版シグネチャを脚注として追記し、`parser_driver.ml` の実装差分を脚注で参照できるようにする。
     - `docs/plans/rust-migration/1-0-front-end-transition.md` §2.2（W3）へ「PackratMemo 組み込み完了」を追加し、`p1-rust-frontend-gap-report.md` `FRG-07/11` へ `Closed (P2-8)` を宣言する条件を本文で明文化する。
     - `docs/spec/1-1-syntax/examples/README.md` に `--emit-parse-result` と `RunConfig` JSON の保存手順を追加し、Chapter 1 サンプルの ParseResult ログ参照先を `reports/spec-audit/ch1/` に揃える。
   - **実務タスク（W38 後半 / 3 日上限）**:
      1. `parser/api.rs` を `pub` モジュールとして公開し、`State<'src>` に `PackratMemo`（`HashMap<usize, PackratCacheEntry>`）を埋め込む。`Reply<T>` の `Err` には `committed` フラグを追加し、Menhir 互換の `cut`/`attempt` 実装を `parser/mod.rs` へ導入する。
      2. `ParserDriver::parse_with_options_and_run_config` を `Parser<T>` 呼び出しで書き直し、`ParseResult` に `legacy_error`, `packrat_cache`, `stream_flow_state` を完全復元する。`poc_dualwrite_compare.sh` から出力する JSON の `parse_result` フィールドが OCaml CLI と同列になるよう `tooling/ci/collect-iterator-audit-metrics.py` を更新。
      3. `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::api parser::driver -- --nocapture` を `reports/spec-audit/ch1/parser_api-YYYYMMDD-log.md` へ保存し、`docs/notes/spec-integrity-audit-checklist.md` の `FRG-07` 行を `Closed(P2-8)` へ変更。`docs-migrations.log` へ Parser API 切替を記録。
   - ✅ 進捗メモ（2025-11-22 更新）: `compiler/rust/frontend/src/parser/api.rs` を `pub mod api` として再公開し、`State<'src>` に `PackratMemo`（`FxHashMap<usize, PackratCacheEntry>`）を追加。`Reply<T>` の `Err` へ `committed: bool` を導入して `parser/mod.rs` の `cut()`/`attempt()` マクロから利用できるようにした。`ParserDriver::parse_with_options_and_run_config` は `Parser::<Module>::parse(state, run_config)` へ統一し、`ParseResult.packrat_cache` と `RunConfig` を CLI 経由で保存する経路を `poc_dualwrite_compare.sh --emit-parse-result` と `collect-iterator-audit-metrics.py` 双方に配線した。`reports/spec-audit/ch1/parser_api-20251122-log.md`（`CI_RUN_ID=rust-frontend-parser-api-20251122.2`, `git rev-parse HEAD = 7a1ab812`）および `parser_api-20251122-parse-diff.md` / `use_nested-20251122-parse-result.json` を追加し、`docs/notes/spec-integrity-audit-checklist.md` の `FRG-07`/`FRG-11` を `Closed (P2-8 W38)` へ更新。`docs/plans/rust-migration/1-0-front-end-transition.md`（W3）と `p1-rust-frontend-gap-report.md` にも PackratMemo 組み込み完了ログをリンクし、`docs-migrations.log` へ「Parser API 切替 / RunConfig JSON 保存ルール / poc_dualwrite_compare.sh parse_result 拡張」を追記済み。
   - **検証とクローズ条件**:
      - ✅ 2025-11-22: `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --parse-debug-output tmp/use_nested-parse.json` を実行し、`tmp/use_nested-parse.json.packrat_cache` と `legacy_error` が OCaml CLI の JSON とフィールド差分 0 であることを `reports/spec-audit/ch1/parser_api-20251122-parse-diff.md` に記録。
      - ✅ 2025-11-22: `reports/spec-audit/ch1/use_nested-20251122-parse-result.json` / `effect_handler-20251122-parse-result.json` に `RunConfig.require_eof`, `left_recursion`, `lex.identifier_profile` を含む `run_config` ブロックと `git rev-parse HEAD` 脚注を追加し、`collect-iterator-audit-metrics.py` が `parse_result.packrat_cache_entries = 148` を採取したログを `parser_api-20251122-metrics.json` へ出力。
      - ✅ 2025-11-22: `docs/plans/rust-migration/overview.md` Phase 1 テーブルへ「Parser API 完全化 (PackratMemo + RunConfig JSON)」を追記し、`docs/spec/2-1-parser-type.md` の脚注・`docs/spec/1-1-syntax/examples/README.md` の `--emit-parse-result` ガイドとともに PR を発行。`README` 反映後 `docs-migrations.log` へ差分 ID `DOCS-20251122-PAI` を登録。

6. **Trace / Diagnostics 拡張（ブラッシュアップ提案②対応）**
   - `ParserTraceEventKind` を `ExprEnter`/`ExprLeave`/`EffectEnter`/`EffectExit`/`HandlerAccepted`/`OperationResume` に拡張し、`trace_id` 命名規則（`syntax:expr::<kind>` 等）を `docs/plans/rust-migration/unified-porting-principles.md#同一観測点の再現` へ追記する。
   - `build_parser_diagnostics` と `TraceEvent` の紐付けを強化し、`FrontendDiagnostic.extensions` に `trace_ids` 配列を追加。`TraceEvent`→`diagnostic` の相関は `reports/spec-audit/ch1/trace-coverage-YYYYMMDD.md` へ保存する。
   - Streaming / 非 Streaming 双方で `effect_handler.reml` を再実行し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-trace.json` および `ch1/effect_handler-YYYYMMDD-diagnostics.json` に `syntax:expr-handle` `syntax:expr-perform` 等のトレースが記録されることを確認する。
   - **実務タスク（W39 前半 / 2 日上限）**:
      1. `compiler/rust/frontend/src/parser/expr.rs` で `TraceEvent::expr_enter`/`expr_leave` を追加し、式ごとに `trace_id` と `span` を記録。`module_parser.rs` から `trace_events` へ push する。
      2. `scripts/poc_dualwrite_compare.sh effect_handler --trace` を実行してトレース差分を `reports/spec-audit/ch1/effect_handler-YYYYMMDD-trace.md` に保存。`docs/spec/1-1-syntax/examples/README.md` の「監査ログ」節にトレース命名規約を追加。
      3. `docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-003` 行へ `Trace coverage >= 4` を満たした証拠を追記し、`reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` にトレース比較結果を貼付。
   - ✅ 進捗メモ（2025-11-22 更新）: `parser/mod.rs` で `ParserTraceEventKind::{ExprEnter,ExprLeave,EffectEnter,EffectExit,HandlerAccepted,OperationResume}` を実装し、`FrontendDiagnostic.extensions.trace_ids` に重複のない `syntax:expr::<kind>` を出力。`scripts/poc_dualwrite_compare.sh effect_handler --trace` のログを `reports/spec-audit/ch1/trace-coverage-20251122.md` へ集約し、`Trace coverage >= 4`（handle / perform / resume / block）を `effect_handler-20251118-trace.md` / `block_scope-20251118-trace.md` と突き合わせた。`docs/spec/1-1-syntax/examples/README.md` / `docs/spec/0-3-code-style-guide.md` / `docs/plans/rust-migration/unified-porting-principles.md` に命名規約と保存手順を追記済み。
   - **検証とクローズ条件**:
      - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::trace -- --nocapture` が `block_scope.reml` / `effect_handler.reml` のケースで `syntax:expr-*` イベントを生成することを確認し、CI ログを `reports/spec-audit/ch1/trace-coverage-YYYYMMDD.log` に残している。
      - `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` にトレース ID と CLI コマンドが掲載され、`docs/plans/rust-migration/overview.md` の Phase 1 完了条件が「Trace coverage 揃い」に更新された。
      - `docs/spec/1-1-syntax/examples/README.md` の脚注と `docs/spec/0-3-code-style-guide.md` が更新され、他章からもトレース命名規約を参照できる状態。

7. **CLI プロモーションと型推論の厳格化（ブラッシュアップ提案③対応）**
   - `compiler/rust/frontend/src/bin/poc_frontend.rs` を `src/bin/reml_frontend.rs` に改名し、`Cargo.toml` の `[[bin]]` と `PARSER_NAMESPACE`/`PARSER_NAME`/`PARSER_FINGERPRINT` を本番向けに再設定。CLI ヘルプと `README` 更新を `docs/spec/0-3-code-style-guide.md` に反映する。
   - `TypecheckDriver::infer_fallback_from_source` と `extract_top_level_functions` を削除し、AST が生成されない場合は `typeck` フェーズをスキップして `TypecheckViolation` に `typeck.aborted.ast_unavailable` を追加する。Stage/Audit JSON を `docs/spec/3-6-core-diagnostics-audit.md` スキーマに揃える。
   - `p1-rust-frontend-gap-report.md` の `FRG-12`（HM 実装）と `FRG-05`（Streaming RunConfig）を `Closed(P2-8)` へ更新し、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` に「Rust CLI 安定化」節を追記する。
   - **実務タスク（W39 後半 / 3 日上限）**:
      1. CLI rename: `cargo run --bin reml_frontend` が動作するよう `Cargo.lock` を更新し、CI スクリプト・`scripts/poc_dualwrite_compare.sh` を `reml_frontend` 呼び出しへ書き換える。旧バイナリ呼び出しは `cargo run --bin poc_frontend` で失敗することをログへ残す。
      2. `typeck/driver.rs` から `infer_fallback_from_source` を削除し、`TypecheckDriver::infer_module` で `Option<Module>` を受け取る構造に変更。AST なしの場合の診断コード `typeck.aborted.ast_unavailable` を `docs/spec/3-6-core-diagnostics-audit.md` に記載。
      3. Stage/Audit: `StageAuditPayload` を `schema_version = "3.0.0-alpha"` に更新し、`reports/spec-audit/ch1/use_nested-YYYYMMDD-typeck.json` に `stage_trace` と `used_impls` を保存。`docs/spec/0-3-code-style-guide.md` の CLI 実行例を新バイナリ名で更新し、`docs-migrations.log` へ記録。
   - ✅ 進捗メモ（2025-11-22 更新）: `compiler/rust/frontend/src/bin/reml_frontend.rs` を新 CLI 名称で公開し、`PARSER_*` 定数と `SCHEMA_VERSION` を本番値（`rust.frontend`/`rust-frontend-0001`/`3.0.0-alpha`）へ更新。`TypecheckDriver::infer_module` は `Option<&Module>` を受け取り、AST 欠落時に `typeck.aborted.ast_unavailable` を返すよう再構成した。`scripts/poc_dualwrite_compare.sh`、`tooling/lsp/tests/client_compat/fixtures/*.json` は全て `reml_frontend` 呼び出しへ切り替え済み。`TypeckArtifacts::new` が `StageAuditPayload.stage_trace` / `report.used_impls` を `typeck/typeck-debug.rust.json` に焼き付け、`reports/spec-audit/ch1/typeck-fallback-removal-20251122.md` に CLI/テスト手順と `docs/spec/3-6-core-diagnostics-audit.md` 追記の根拠を記録した。`docs/spec/0-3-code-style-guide.md`・`docs/spec/1-1-syntax/examples/README.md`・`docs/spec/3-6-core-diagnostics-audit.md`・`docs/plans/rust-migration/overview.md`・`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`・`docs/notes/spec-integrity-audit-checklist.md` を reml_frontend 前提へ更新し、`docs-migrations.log` に CLI 昇格ログ `DOCS-20251122-REM` を追加。
   - **検証とクローズ条件**:
      - `cargo run --bin reml_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --emit-typeck-debug tmp/effect_handler-typeck.json` が成功し、`tmp/effect_handler-typeck.json` の `stage_trace` と `used_impls` が `docs/spec/3-6-core-diagnostics-audit.md` のキーに一致する。
      - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml typeck::hindley_milner` を AST 失敗ケース付きで実行し、`typeck.aborted.ast_unavailable` 診断が出力されることを `reports/spec-audit/ch1/typeck-fallback-removal-YYYYMMDD.md` に保存。
      - `docs/plans/rust-migration/overview.md` / `docs/spec/0-3-code-style-guide.md` / `docs/spec/1-1-syntax/examples/README.md` に新 CLI 名称が掲載され、`docs/notes/spec-integrity-audit-checklist.md` の `FRG-05`/`FRG-12` 行が `Closed` へ更新済み。`docs-migrations.log` に CLI 名称変更と Stage/Audit トレース更新を記録。

[^frontend-plan-ref]: `docs/plans/rust-migration/1-0-front-end-transition.md` §2（Rust Parser WG の担当範囲）および `docs/plans/rust-migration/overview.md` の W37〜W38 スプリント配分。
[^syntax-bnf]: `docs/spec/1-1-syntax.md` §2（Module/Use）、§4（Expressions）、§5（Effects）に記載された BNF。
[^streaming-guide]: `docs/guides/core-parse-streaming.md` §3/§7（Streaming パーサの監査とメトリクス管理）。
[^parser-poc]: `compiler/rust/frontend/src/parser/mod.rs:1-22` でモジュール全体を PoC と明記し、`compiler/rust/frontend/src/parser/api.rs:1-170` が `#![allow(dead_code)]` 指定と空の `memo: ()` を残している。
[^trace-coverage]: `compiler/rust/frontend/src/parser/mod.rs:65-108` では `ParserTraceEventKind` が `ModuleHeaderAccepted`/`UseDeclAccepted` の 2 種のみで、式や効果に関するトレースが欠落している。
[^poc-cli]: `compiler/rust/frontend/src/bin/poc_frontend.rs:1-40` で CLI 名称・名前空間・フィンガープリントが `poc_frontend` のまま固定されている。
[^typeck-fallback]: `compiler/rust/frontend/src/typeck/driver.rs:162-257` に AST を介さずソース文字列を走査する `infer_fallback_from_source` / `extract_top_level_functions` が残っている。

---

[^phase27-handshake-2-8]: Phase 2-7 診断パイプライン残課題・技術的負債整理計画の最終成果。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5、`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`、`reports/audit/dashboard/diagnostics.md` に記録された監査ベースラインと差分ログを参照する。
[^phase28-diff-class-footnote]: `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#phase28-diff-class`（2025-11-17 更新）に Chapter 別の差分分類と `rust-gap` 取扱いを整理。
