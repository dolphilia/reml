# 4.1 Core.Parse パーサーコンビネーター次期計画 (Phase 7〜)

## 背景
- 既存計画（4-1-core-parse-combinator-plan.md）の Phase 0〜6 を完了し、仕様 2.1/2.2 準拠の基本コンビネーター層と回帰テストが Rust ランタイムに整備済み。  
- 次段は「実用的な PEG/Pacrat 実装」に近づけるため、先行ライブラリ（Parsec/Megaparsec/FastParse/nom/chumsky 等）で有効だった機能を取り込み、Core.Parse の操作性・診断・性能を強化する。  
- 仕様との整合を維持しつつ、Lex/OpBuilder/Streaming/Plugin との統合ポイントを段階的に拡張し、Phase4 シナリオ群の回帰を継続する。

## 目的
1) PEG/Pacrat 先行事例で有効な機能（演算子優先度ビルダー、autoWhitespace/Layout、プロファイル/トレース、左再帰ガードなど）を Core.Parse に取り込み、実用性と性能を底上げする。  
2) Lex/OpBuilder/Plugin/Streaming との接合面を拡張し、RunConfig/診断と一貫した契約で運用できる状態を構築する。  
3) サンプル/回帰テストを拡充し、PhaseF/Scenario マトリクスに新機能を反映することで、リグレッションの監視面を強化する。

## スコープ
- **含む**: Core.Parse API 拡張（演算子優先度ビルダー/autoWhitespace/Layout/observe hooks）、左再帰ガイドライン、Packrat/バックトラック計測、Lex ブリッジ強化、OpBuilder/Plugin/Streaming との接合、追加サンプル・回帰テスト、ドキュメント更新。  
- **除外**: OCaml 実装の追随、完全なストリーミング最適化や JIT 化など大規模最適化。必要に応じて TODO/脚注で後続フェーズへ送る。

## 仕様・参照
- `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-2-core-combinator.md`（API 契約の基礎）  
- `docs/guides/core-parse-streaming.md`, `docs/guides/plugin-authoring.md`（接合要件）  
- `docs/notes/core-parse-combinator-survey.md`（Phase 0 調査結果）  
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan.md`（完了済み計画の履歴）  
- `docs/plans/bootstrap-roadmap/4-1-scenario-matrix-plan.md`, `4-1-spec-core-regression-plan.md`（シナリオ/回帰追跡方針）

## フェーズ計画
### Phase 7: 調査結果の統合方針策定
- `docs/notes/core-parse-combinator-survey.md` の調査項目を「高: API 拡張 / 中: ランタイム追加 / 低: 糖衣」に再編し、採用/見送り理由と影響範囲（仕様改訂の有無、破壊的変更の有無）を明文化。性能・安全性（`docs/spec/0-1-project-purpose.md`）に反するものは保留に回す。  
- 導入候補と方針  
  - **高（API 拡張）**:  
    - `makeExprParser` 相当の優先度ビルダー…採用。`chainl1/chainr1` を安全に包む API が不足しており、OpBuilder 互換を確保するため。既存 API へは非破壊で追加。  
    - `recoverWith` バリエーション（`with_default`/`with_context`）…採用。診断一貫性と学習コスト低減を優先。新設 API で既存の `recover` 契約は維持。  
  - **中（ランタイム追加）**:  
    - `autoWhitespace`/`Layout`…条件付き採用。Lex プロファイル共有を尊重しない場合のフォールバックを保持し、`RunConfig.extensions["lex"]` が未設定でも後方互換を担保。  
    - `ParseObserver`/`ParserProfile`（ヒット率/バックトラック/メモ化統計）…採用。性能監視を opt-in で提供し、デフォルトは OFF とする。Packrat キャッシュ鍵との整合を要確認。  
  - **低（糖衣/ガイドライン）**:  
    - `left_recursion_guard` の利用ガイドとサンプル…採用。実装は既存コンビネーター上のラッパーとし、挙動は仕様外の補助として提供。  
    - `lexeme/symbol` 派生の軽量シンタックスシュガー…採用。`RunConfig` 未指定時の空白スキップ挙動を明示するだけで互換性リスクは低。  
  - **保留/見送り（現時点）**:  
    - `resumable parser` / ストリーミング前提の再開 API…保留。`core-parse-streaming` の PoC が未整備で、Stage/Capability 監査との整合リスクがある。  
    - バイトスライス前提の zero-copy 最適化…保留。安全性とクロスプラットフォーム性を満たす検証が不足しており、性能要件を測定してから再検討。  
- 仕様・ドキュメントへの反映方針  
  - `docs/spec/2-2-core-combinator.md` に新設 API（優先度ビルダー、`recoverWith` 拡張、`left_recursion_guard` ガイド）の脚注案を起票し、Phase 8/9/10 で採択可否を判断。  
  - `docs/spec/2-0-parser-api-overview.md` に `RunConfig` での `lex/layout`/`profile` フラグ追加を脚注するドラフトを準備。  
  - 観測系 API は `docs/notes/core-parse-api-evolution.md` に暫定契約を残し、回帰指標（期待集合、バックトラック率）の計測ポイントを明文化してから仕様化する。
- 実施記録  
  - `docs/notes/core-parse-combinator-survey.md#phase-7-統合方針（優先度再編と採否理由）` に Phase 7 の採否テーブルと影響度を集約済み。性能/安全性の判断基準は `docs/spec/0-1-project-purpose.md` に準拠。  
  - 脚注ドラフトの起票先: `docs/spec/2-2-core-combinator.md`（優先度ビルダー/`recoverWith`/左再帰ガード）、`docs/spec/2-0-parser-api-overview.md`（`RunConfig.lex/layout/profile`）。観測系 API は `docs/notes/core-parse-api-evolution.md` に暫定契約を追記する。  
  - シナリオ追加検討: ストリーミング再開 API と zero-copy 最適化は保留扱いとして、`docs/plans/bootstrap-roadmap/4-1-scenario-matrix-plan.md` へ検証条件を登録することを次ステップ候補とする。

### Phase 8: 演算子優先度ビルダー導入
- Phase 7 採否テーブル（`docs/notes/core-parse-combinator-survey.md#phase-7-統合方針（優先度再編と採否理由）`）を前提に、演算子優先度ビルダーを「非破壊追加」として設計する。`committed` フラグの扱いと期待集合抑制を `chainl1/chainr1` の巻き戻し規約に統合する。  
- `makeExprParser` 互換のビルダーを Core.Parse に追加し、`chainl1/chainr1` ベースで優先度テーブルを生成できる API を設計。仕様脚注ドラフトは `docs/spec/2-2-core-combinator.md` に起票し、Phase 10 の観測系 API と一貫した ID/計測ポイントを設定する。  
- `OpBuilder` との互換性を検証し、`RunConfig` 経由で優先度/結合性を外部指定できるかを検討（破壊的変更は避ける）。`RunConfig` 拡張案は `docs/spec/2-0-parser-api-overview.md` の脚注ドラフトに反映。  
- サンプル: `examples/` に演算子優先度の DSL パーサを追加し、既存 `basic_interpreter_combinator.reml` にオプションでビルダー版を併記。  
- 回帰: `phase4-scenario-matrix.csv` にシナリオ行を追加し、期待診断/成功条件を定義。Phase 7 で保留とした zero-copy/ストリーミング再開はシナリオ備考に保留理由を明記して除外する。

#### 実施記録
- `docs/spec/2-2-core-combinator.md` に `expr_builder` ドラフト（`ExprOpLevel`/`ExprCommit` と `operator_table` 上書き経路）を脚注追加し、`chainl1/chainr1` の committed 規約と期待集合抑制の扱いを明文化。  
- `docs/spec/2-0-parser-api-overview.md` の RunConfig 脚注に `extensions["parse"].operator_table` を追加し、OpBuilder/CLI から優先度・結合性を注入する入口を記載。  
- サンプル: `examples/spec_core/chapter2/parser_core/core-parse-precedence-builder-ok.reml`（期待出力: `expected/spec_core/chapter2/parser_core/core-parse-precedence-builder-ok.stdout`）を追加し、`phase4-scenario-matrix.csv` に `CH2-PARSE-801`（resolution=pending、zero-copy/streaming resume 除外理由を備考に記載）を登録。  
- `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml` に `expr_builder` 版を併記（既定は従来チェーンを維持）し、OpBuilder 互換性検証のためのオプション経路を明示。
- Rust 実装: `Parse.expr_builder` を `compiler/rust/runtime/src/parse/combinator.rs` に追加し、`RunConfig.extensions["parse"].operator_table.commit_operators` を尊重する上書きロジックと、`OpTable::to_run_config_extension`（`op_builder.rs`）経由の接続コードを実装。`CH2-PARSE-801` は `compiler/rust/frontend/target/debug/reml_frontend examples/spec_core/chapter2/parser_core/core-parse-precedence-builder-ok.reml（run_id=89811feb-586f-428c-b95b-41263adbda18）` で再実行し、括弧再帰と ternary/infixn を含む入力で `result=33` をゴールデン化して `resolution=ok` を維持。`operator_table` 上書きは `RunConfig.extensions.parse.operator_table` で確認済み。

### Phase 9: autoWhitespace/Layout と Lex ブリッジ強化
- `autoWhitespace`（トリビア共有）と `Layout`（オフサイドルール）を Core.Parse へ導入する設計を決定。`RunConfig.extensions["lex"]` を尊重し、未提供時のフォールバックを整理。  
- `symbol/keyword/lexeme` を新プロファイルに対応させ、コメント/トリビアのスキップ戦略を明文化。`IdentifierProfile` との統合や Bidi/正規化チェックの拡張点を仕様に脚注。  
- テスト: 空白プロファイルの切替、Layout あり/なしを切り替えるユニット/サンプルを追加し、期待診断を更新。

#### 実施記録
- 仕様ドラフト追加: `docs/spec/2-2-core-combinator.md` に `autoWhitespace` / `AutoWhitespaceConfig` / `AutoWhitespaceStrategy` を追記し、RunConfig の lex プロファイル優先・強制プロファイル・無効化フォールバックの 3 戦略を明示。`LayoutProfile` との連携とシンボル側の二重スキップ防止を記述。  
- Lex 側ドラフト: `docs/spec/2-3-lexer.md` に `LayoutProfile` / `layout` / `layout_token` を追加し、`RunConfig.extensions["lex"].layout_profile` を `autoWhitespace` が検出する経路を定義。混在インデント診断とオフサイド無効時のフォールバックを明文化。  
- 今後のテスト: Layout 有効/無効と profile 切替を行うユニット/サンプルを追加し、期待診断に `lex.layout.*` を含める（次フェーズで `examples/spec_core/chapter2` 系へ追加予定）。

### Phase 10: 観測性・性能計測（observe/profile）
- Packrat/バックトラック/左再帰ガードのヒット率やメモ化サイズを収集する `ParseObserver`/`ParserProfile` API を追加し、`RunConfig` で ON/OFF 制御。  
- CLI/診断出力への統合方法を決め、`reports/` へメトリクスを書き出す実験的フラグを実装（デフォルト OFF）。  
- マイクロベンチを作成し、Phase 8/9 の追加機能による性能変化を測定。性能退行がある場合はフォールバック戦略を記録。

#### 実施記録
- Rust ランタイム: `RunConfig.profile` と `extensions["parse"].profile/profile_output` を解釈する観測フラグを追加し、`ParseObserver` → `ParseResult.profile` に Packrat ヒット/ミス、`attempt` 巻き戻し、`recover` 成功、左再帰ガード利用、メモ化エントリ数を集計（`compiler/rust/runtime/src/parse/combinator.rs`）。`profile_output` 指定時は JSON を best-effort で保存し、失敗しても診断へ影響しない。
- API/仕様更新: `ParserProfile` を公開し、`ParseResult` に `profile: Option<ParserProfile>` を追加。`docs/spec/2-2-core-combinator.md` へ B-3 観測/プロファイル脚注、`docs/spec/2-0-parser-api-overview.md` へ RunConfig 拡張の脚注を追記し、Phase10 の利用条件と計測項目を明記。
- テスト: プロファイル無効時の無出力と、Packrat ヒット/ミス＋`attempt` バックトラック計測を確認するユニットテストを `compiler/rust/runtime/tests/parse_combinator.rs` に追加。
- ベンチ: Phase8/9 の演算子チェーンを用いたマイクロベンチ `benchmarks/parse/profile.rs` を新設し、`packrat_only` と `packrat_with_profile` を比較できる Criterion グループ `parse::profile` を登録（`benchmarks/Cargo.toml`）。

### Phase 11: Plugin/Streaming/OpBuilder 連携強化
- Plugin: `docs/guides/plugin-authoring.md` と `docs/spec/3-8-core-runtime-capability.md` を踏まえ、Core.Parse パーサをプラグインから安全に呼び出すための API ガイドラインを追加。署名検証/Stage 整合をチェックリスト化。  
- Streaming: `core-parse-streaming` ガイドに合わせ、`Parser` → `StreamingParser` への変換方針と制約を整理（完全実装は次フェーズでも可）。  
- OpBuilder: 新ビルダー/autoWhitespace を `OpBuilder` DSL に統合するための変更点を洗い出し、回帰テストを更新。互換性リスクを `docs/notes/core-parse-api-evolution.md` に記載。

#### 実施記録
- Plugin: `docs/guides/plugin-authoring.md` に Stage/Capability 署名検証と RunConfig 共有のチェックリストを追加し、`RuntimeBridgeAuditSpec`（`docs/spec/3-8-core-runtime-capability.md`）準拠で `bridge.stage.*` を転写する手順を明文化。`reml plugin verify`（署名検証）と RunConfig 経由の `extensions["parse"].operator_table` 共有を併記。
- Streaming: `docs/guides/core-parse-streaming.md#94-parser-から-streamingparser-への変換指針（phase-11）` を新設し、`Parser<T>` を `StreamDriver` に受け渡す際の Packrat/期待集合/lex/layout/演算子テーブル共有ポリシーと Stage 伝播の扱い、Rust 実装欠落時の TODO を整理。
- OpBuilder/autoWhitespace: RunConfig での演算子テーブル上書き方針とプラグイン設定衝突時の回帰監視案を `docs/notes/core-parse-api-evolution.md#phase-4-1-phase11-pluginstreamingopbuilder-連携メモ` に追記。`phase4-scenario-matrix.csv` への追加と Phase 12 のフォローアップを次ステップとして残した。

### Phase 12: ドキュメント・回帰更新
- 仕様: `docs/spec/2-2-core-combinator.md` に新 API/挙動変更の脚注を追加し、必要に応じて `2-0-parser-api-overview.md` へ概要を追記。  
- ガイド: `docs/guides/plugin-authoring.md`, `core-parse-streaming.md` に新機能の利用例/制約を追記。  
- 回帰: Phase4 シナリオ表と `4-1-spec-core-regression-plan.md` の PhaseF トラッカーへ新シナリオを登録し、完了時にチェックボックスを更新。  
- ハンドオーバー: 未着手/保留項目は `docs/notes/core-parse-api-evolution.md` に TODO として残し、次フェーズの入口を明示。

#### 実施記録
- 仕様更新: `docs/spec/2-2-core-combinator.md` に autoWhitespace/Layout + ParserProfile の回帰登録脚注を追加し、`phase4-scenario-matrix.csv` の `CH2-PARSE-901/902` と PhaseF トラッカーを参照できるようにした。`docs/spec/2-0-parser-api-overview.md` では RunConfig の lex/layout/profile フラグにシナリオ紐付けを追記。
- ガイド更新: `docs/guides/plugin-authoring.md` のチェックリストへ autoWhitespace/Layout 共有と観測フラグの扱いを追加し、`docs/guides/core-parse-streaming.md#94-parser-から-streamingparser-への変換指針（phase-11）` に ParserProfile 出力時の注意と `CH2-PARSE-902` 監視メモを記載。
- 回帰登録: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CH2-PARSE-901`（autoWhitespace/Layout）と `CH2-PARSE-902`（ParserProfile JSON）を追加し、`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` PhaseF チェックリストへ未実施タスクとして転写。`docs/notes/core-parse-api-evolution.md` にサンプル/expected 作成 TODO を残した。

## 成果物と完了条件
- `Core.Parse` に演算子優先度ビルダー、autoWhitespace/Layout、observe/profile API が追加され、Lex/OpBuilder/Plugin/Streaming との接合ガイドが整備されていること。  
- 新規サンプル・ユニット/回帰テストが追加され、Phase4 シナリオ表に反映されていること。  
- 仕様・ガイドが更新され、制約や未対応項目が脚注/TODO で明示されていること。

## 追跡・リスク緩和
- 性能退行は Phase 10 のベンチ結果で確認し、必要ならデフォルト OFF のフラグに戻す。  
- Lex/Layout 導入で互換性リスクがある場合、`RunConfig` の opt-in を維持し、旧挙動を `legacy` プロファイルとして温存する。  
- Plugin/Streaming は安全側に倒し、ステージ整合や署名検証をクリアできない場合は警告診断にとどめ、強制エラー化の是非を次フェーズで判断する。
