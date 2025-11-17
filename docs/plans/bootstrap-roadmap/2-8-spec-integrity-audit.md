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

## 具体的な計画

### Rust Frontend パーサ拡張のステップ

Phase 2-8 W37（Chapter 1/2 フォーカス）の 2 週間で Rust Frontend の仕様整合を完了させ、W38 で Streaming メトリクスを CI へ昇格させる。スケジュールと成果物は下表の通り。[^frontend-plan-ref]

| ステップ | 期間 (Sprint W37-38) | 主担当 | 主要成果物 |
| --- | --- | --- | --- |
| 1. AST トップレベル整備 | W37 前半 | Rust Parser WG | `ast.rs`/`module_parser.rs` の diff、`reports/spec-audit/ch1/use_nested-*.json` 更新 |
| 2. 式・効果構文 | W37 後半 | Rust Parser WG + Effects WG | `effect_handler.reml` を受理する実装と診断ログ、`rust-gap SYNTAX-003` 終了メモ |
| 3. module_parser 再実装 | W37→W38 | Parser QA | `tests/parser.rs` 追加、フェーズ別進捗メモ、`rust-gap` 行更新 |
| 4. Streaming/CI 反映 | W38 前半 | Streaming WG | `tests/streaming_metrics.rs` 拡張、CI 実行結果、`docs/spec/0-3-code-style-guide.md` 更新 |

1. **Rust Parser のトップレベル拡張**
   - `compiler/rust/frontend/src/parser/ast.rs` へ `ModuleHeader`、`UseDecl`、`OperationDecl`、`HandlerDecl` を正式に追加し、`Function`/`Effect` から `TypeAnnot` を共有できるように `AnnotationKind` を導入する。`docs/plans/rust-migration/1-0-front-end-transition.md` の AST 対応表を同時に更新する。
   - `module_parser` 序盤で `module` と `use` を確定させ、`docs/spec/1-1-syntax/examples/use_nested.reml` が構文エラーにならない状態を確認する。`cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml` を実行し、`reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` を差し替えて `rust-gap SYNTAX-002` のステータスを `docs/notes/spec-integrity-audit-checklist.md` に反映する。
   - AST 差分は `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` に追記し、`docs/spec/1-1-syntax.md` の脚注から暫定 `use_nested_rustcap.reml` を削除する条件を明示する。
2. **式/ブロック/効果構文の段階的実装**
   - `module_parser` の式パーサを `ExprParser` として分離し、`{ ... }` ブロック、`let`/`var` 代入、`return`、`do`/`perform`、`handle ... with handler { ... }` を Chapter 1 BNF に沿って段階導入する。[^syntax-bnf]
   - ハンドラ構文では `operation log(args, resume) { ... }` を `DeclKind::Handler` と `OperationDecl` に展開し、`resume` の型検証を `TypeAnnot` と共有する。`docs/spec/1-1-syntax/examples/effect_handler.reml` を受理できることを Rust Frontend CLI で再現し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` を保存する。
   - 受理後に `docs/notes/spec-integrity-audit-checklist.md` の `rust-gap SYNTAX-003` をクローズし、`docs/plans/rust-migration/p1-rust-frontend-gap-report.md` へ差分結果を逆参照する。
3. **module_parser の再実装マイルストーン**
   - フェーズ順序: (a) module/use ヘッダ → (b) effect/fn 宣言（戻り値注釈含む）→ (c) block/let/do/handle 構文 → (d) `operation` ブロック＋`resume`。各フェーズ完了時に `compiler/rust/frontend/tests/parser.rs` へテストを追記し、Chumsky ベースの `module_parser` が後方互換であることを証明する。
   - 進捗は `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` に日付別で追記し、`reports/spec-audit/diffs/` の `rust-gap` 表に同じ ID を二重記録する。テストシナリオには `use_nested`、`effect_handler`、および `docs/spec/1-1-syntax.md` サンプルの最小ケースを含める。
   - 差分レビューは `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` の CI ブロッカー条件に沿って承認を得る。
4. **Streaming テストと差分ログ更新**
   - `compiler/rust/frontend/tests/streaming_metrics.rs` に `module_header_acceptance`、`effect_handler_acceptance` などの統合テストを追加し、`StreamFlowState::latest_bridge_signal` の二重 `Option` 問題を含む既知バグを回避する再現ケースを固定化する。[^streaming-guide]
   - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics` を Phase 2-8 CI（`3-0-ci-and-dual-write-strategy.md` で定義）に登録し、成功ログを `reports/spec-audit/ch1/` に保存する。成功後は `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` / `SYNTAX-003-*.md` を `Closed` 扱いに更新する。
   - Streaming 経路で Chapter 1 サンプルが通過したら `docs/spec/1-1-syntax.md` の監査ノートからフォールバック (`*_rustcap.reml`) を削除し、`docs/spec/0-3-code-style-guide.md` の実行手順を Rust Frontend ベースに書き換える。併せて `docs/plans/rust-migration/overview.md` の Phase 1 完了条件も更新する。

[^frontend-plan-ref]: `docs/plans/rust-migration/1-0-front-end-transition.md` §2（Rust Parser WG の担当範囲）および `docs/plans/rust-migration/overview.md` の W37〜W38 スプリント配分。
[^syntax-bnf]: `docs/spec/1-1-syntax.md` §2（Module/Use）、§4（Expressions）、§5（Effects）に記載された BNF。
[^streaming-guide]: `docs/guides/core-parse-streaming.md` §3/§7（Streaming パーサの監査とメトリクス管理）。

---

[^phase27-handshake-2-8]: Phase 2-7 診断パイプライン残課題・技術的負債整理計画の最終成果。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5、`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`、`reports/audit/dashboard/diagnostics.md` に記録された監査ベースラインと差分ログを参照する。
[^phase28-diff-class-footnote]: `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#phase28-diff-class`（2025-11-17 更新）に Chapter 別の差分分類と `rust-gap` 取扱いを整理。
