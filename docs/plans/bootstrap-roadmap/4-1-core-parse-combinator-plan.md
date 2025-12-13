# 4.1 Core.Parse パーサーコンビネーター実装計画

## 背景
- `docs/spec/2-0-parser-api-overview.md` / `2-2-core-combinator.md` で定義された `Parser<T>` 公理系（12〜15 個）が現行 Rust 実装に存在しない。`compiler/rust/runtime/src/parse/` は `op_builder` のみを提供し、`Parse.chainl1` などを使うサンプル（例: `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml`）は実行不能。
- Phase4 で `Core.Parse` を利用するシナリオ（Parser/Streaming/Plugin）を regression 追跡するため、仕様準拠のコンビネーター層を Rust ランタイムに新設し、CLI/RunConfig/診断と接続する必要がある。

## 目的
1) 仕様 2.1/2.2 に準拠した `Parser<T>` / `Reply{consumed, committed}` / `RunConfig` 拡張と基本コンビネーター群を Rust で提供する。  
2) `Core.Parse.Lex`・`Core.Parse.Op` との統合点（`symbol/lexeme`、`chainl1` の commit/attempt 規約、Packrat メトリクス）を整備し、診断・監査メタデータを CLI へ反映する。  
3) 代表サンプル（`basic_interpreter_combinator.reml` 等）が CLI で成功する状態を作り、PhaseF トラッカーの未完チェックを閉じられるようにする。

## スコープ
- **含む**: Rust ランタイム `Core.Parse` モジュール新設、コンビネーター実装、最低限の Lex 補助 (`lexeme/symbol`)、`chainl1/chainr1` の結合性保証、`attempt/cut/recover` の commit/rollback 仕様、Packrat/RunConfig 統合、ユニットテスト・サンプル実行確認。  
- **除外**: OCaml 実装の再構成、ストリーミング拡張 (`Core.Parse.Streaming`) の実装。必要なら API スタブ/脚注で後続フェーズへ送る。

## 仕様参照
- 2-1: Parser 型/Reply/ParseResult/RunConfig（特に consumed/committed と Packrat の意味論）  
- 2-2: コアコンビネーター A-1〜A-8 + 派生 (C) の API 契約  
- 2-3: Lex ヘルパ（`lexeme`/`symbol`/空白プロファイル共有）  
- 2-4: OpBuilder 連携（`chainl1` 実装規約）  
- 2-5: エラー・期待集合生成、`cut`/`attempt` の診断挙動  

## 現状ギャップとリスク
- `Parser<T>` 型・`Reply` 型が存在しないため、既存 AST/CLI 経路とどう繋ぐかの設計が必要。  
- `RunConfig.extensions["lex"/"recover"]` を Rust 側でどこまで尊重するか未定義。  
- Packrat/左再帰対策が未実装のため、性能と無限ループ検知（`many` が空成功するケース）の保証がゼロ。  
- 既存 `op_builder` を `Parser` ベースへ移行する際の互換性リスク。

## フェーズ計画
### Phase 1: 型と最小ランナー設計
- `compiler/rust/runtime/src/parse/combinator.rs`（新規）に `Parser<T>` / `Reply<T>` / `ParseResult<T>` / `ParseError` / `ParserId` を定義し、`consumed` / `committed` を型で表現。  
- `ParseState` を新設し、`Input`・位置情報（Byte/Grapheme）・`run_config`・Packrat キャッシュを保持する骨組みを実装。  
- バッチランナー `run(p: Parser<T>, input: &str, cfg: &RunConfig)` を追加し、`require_eof` と `Packrat` ON/OFF を反映する。`RunConfig` 未指定時のデフォルト値も決めておく。  
- 監査メタデータ用に `ParserId` 生成ユーティリティを用意し、`rule` が ID を固定化できるようにする（後続フェーズで利用）。

### Phase 2: コア公理コンビネーター実装（A-1〜A-8）
- A-1〜A-8 の API を仕様どおり実装し、すべて `ParserId` を尊重する。  
  - A-1: `ok/fail/eof/rule/label` の基本。`rule` で ID を固定し、Packrat キーとトレース用に使う。  
  - A-2: 直列・選択 (`then/andThen/skipL/skipR/or/choice`)。`or` は consumed/committed を見て右分岐を抑制。  
  - A-3: `map/cut/cut_here/attempt/recover/trace`。`attempt` は `consumed=false` へ巻き戻し、`cut_here` はゼロ幅コミットを挿入。`recover` で `until` 消費と診断保持。  
  - A-4: 繰り返し (`opt/many/many1/repeat/sepBy/sepBy1/manyTill`)。本体が空成功の場合はエラーにするガードを実装。  
  - A-5: 括り (`between/preceeded/terminated/delimited`) を糖衣として実装。  
  - A-6: 先読み (`lookahead/notFollowedBy`) を非消費で実装し、期待集合を適切に調整。  
  - A-7: `chainl1/chainr1` で左/右結合を保証。内部で `attempt` を使い、中途失敗が外側の `or` に伝播しないようにする。  
  - A-8: `spanned/position` で `Span` を返す。`Input` から Byte/Grapheme を取得し、診断の列情報と一致させる。  
- 消費/コミットの一貫テストを作成（最低: `or` の短絡、`attempt` の巻き戻し、`many` の空成功検知）。

### Phase 3: Lex 補助と空白プロファイル
- `lexeme/keyword/symbol` の糖衣を追加し、`Parser<()>` の空白パーサを受け取れる API にする（`RunConfig.extensions["lex"]` 未対応でも動くようデフォルト実装を持つ）。  
- `Parser::with_space` / `space_id` を実装し、空白パーサに安定 ID を付与。`symbol/keyword` は `with_space` 設定がある場合に自動で空白を消費する。  
- `Core.Parse.Lex` ブリッジが未導入でも動く暫定策として、`RunConfig` からの lex プロファイルを無視する場合は脚注/TODO を残す。将来の `Lex.Bridge` 接続ポイントをコメントで明記。

#### Phase 3 実施時の制約（2026-xx-xx 現在）
- `RunConfig.extensions["lex"].profile/space_id` を `ParseState` でデコードして空白パーサに反映したが、実装は**空白のみ**（`is_whitespace` / ASCII 制限）を連続消費する簡易版。コメント・トリビア構成や Lex プロファイルの詳細は未反映で、後続フェーズで `Core.Parse.Lex.Bridge` に置き換える必要がある。  
- `IdentifierProfile` は `unicode-ident` による XID 判定と単文字レベルの NFC/Bidi チェックのみで、識別子全体の正規化・Bidi ポリシー検証や `identifier` パーサ本体への統合は未対応。識別子処理を導入する際に全体検証へ拡張する。  
- キーワード境界検査は上記簡易チェックに依存するため、Lex 仕様が想定するトリビア共有・コメントスキップが有効になるまでは精度に制約がある。

### Phase 4: エラー・診断統合
- `ParseError` → `Diagnostic` 変換を実装し、`expected_tokens` を `parser.syntax.expected_tokens` と互換な形式で組み立てる（class/keyword/symbol の優先順位も仕様どおり）。  
- `recover` に `until` 走査と診断保持を実装し、`with` の値を返す際の `committed`/`consumed` を明示。  
- Packrat メモ化を導入し、`ParserId` をキーに `RunConfig` で ON/OFF 切替。性能退行を避けるため単体ベンチ or マイクロテストを用意。  
- CLI への統合パス（`reml_frontend`）で `Core.Parse` 診断を既存メッセージと合流させるための橋渡し関数を追加（ファイル配置は後続フェーズで決定）。

#### Phase 4 実装補足（Rust ランタイム反映済み）
- `ParseError` に `expected_tokens` と GuardDiagnostic 変換を追加し、`parser.syntax.expected_tokens` 互換の拡張として CLI/LSP へ橋渡しできるようにした。`parse_result_to_guard_diagnostics` / `parse_errors_to_guard_diagnostics` を `parse::combinator` で提供。  
- `recover` 失敗時の診断を `ParseState` に蓄積し、`run` が結果へ統合する。`recovered` フラグも `ParseResult` に伝搬。  
- Packrat は `ParserId` + バイトオフセットでメモ化し、Reply をクローンして保持する実装。これに伴い `Parser<T>` は `T: Clone` 前提となるため、非 Clone 値を返すパーサーは構築不可（必要ならラップ型で対応）。  
- `eof` / `symbol` / `keyword` のエラーに期待トークンを付加し、期待集合生成の欠落を補正した。

### Phase 5: 例・回帰テスト
- 単体テスト: `compiler/rust/runtime/tests/parse_combinator.rs` を新設。以下を含める:  
  - `or` の短絡（左が consumed/committed の場合は右を試さない）  
  - `attempt` の巻き戻しと `cut/cut_here` のコミット差分  
  - `many` の空成功検知（エラーを返す）  
  - `chainl1/chainr1` の結合順序確認  
  - `recover` が `until` で同期しつつ診断を残すこと  
  - `spanned/position` が期待スパンを返すこと  
- サンプル適用: `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml` を CLI で通し、`expected/` が無ければ生成。`phase4-scenario-matrix.csv` へシナリオを登録し、`resolution_notes` にコマンドを記録。  
- `op_builder` の内部パースを新コンビネータへ移行し、既存 `core-parse` 回帰テスト（opbuilder DSL 含む）を更新。

### Phase 6: 文書・ハンドオーバー
- `docs/spec/2-2-core-combinator.md` に Rust 実装の対応範囲・未対応箇所の脚注を追加し、`2-0-parser-api-overview.md` にも実装状況を一文追記。  
- `docs/guides/core-parse-streaming.md` / `plugin-authoring.md` へ「Rust 実装対応状況」サブセクションを追加し、`RunConfig` との連動制限や TODO を明記。  
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の PhaseF トラッカーに `basic_interpreter_combinator.reml` をチェック項目として追加し、完了時に `[x]` へ更新。  
- 未対応の Lex/Streaming/Plugin 連携は `docs/notes/core-parse-api-evolution.md` に TODO で残し、次フェーズへのハンドオーバー項目として整理。

## 成果物と完了条件
- `compiler/rust/runtime/src/parse/combinator.rs`（新規）＋ `mod.rs` 再エクスポートで `Core.Parse` として公開。  
- ユニットテスト（parse_combinator）とサンプル CLI 実行が CI で緑。`basic_interpreter_combinator.reml` が診断 0 or 期待診断一致。  
- 仕様脚注・ガイド更新済みで、欠落/未対応箇所が明示されていること。

## 追跡・リスク緩和
- Packrat/左再帰・Streaming は段階投入。未対応箇所は `TODO:` 付きで `docs/notes/core-parse-api-evolution.md` にログし、Phase 2-7 へ逆流。  
- Lex ブリッジが不十分な場合は `symbol/lexeme` に暫定実装＋ `RunConfig` 無視の旨を脚注し、後続で `Core.Parse.Lex` の導入を計画する。  
- 期待集合生成が既存 CLI の `ExpectedToken` と食い違う場合、診断キーへの影響を `resolution_notes` に記録し、Phase4 KPI の triage スクリプトで追跡する。
