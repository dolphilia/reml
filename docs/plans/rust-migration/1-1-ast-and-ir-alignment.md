# 1.1 AST / IR 対応表と検証手順

P1 フロントエンド移植の中核である AST（抽象構文木）および Typed AST / 中間表現 (IR) の対応関係を整理し、Rust 実装で互換性を保証するためのチェックリストを定義する。OCaml 実装 (`compiler/ocaml/src/ast.ml`, `typed_ast.ml`, `core_parse/*.ml`) から抽出した構造を基に、Rust 側で再現すべきデータモデルと検証手順をまとめる。

## 1.1.1 目的
- OCaml 実装と Rust 実装で AST/IR のデータ構造を 1:1 で対応付け、仕様書 `docs/spec/1-1-syntax.md` `docs/spec/1-2-types-Inference.md` の契約を保持する。
- Dual-write 比較時にフィールド差異を特定しやすくし、差分原因（仕様差分・実装差分）を切り分ける。
- バックエンド (P2) や CI (P3) が期待するデータフォーマットを前提に、Rust 側の API 設計を固定する。

## 1.1.2 モジュール分類と対応方針

| 分類 | OCaml モジュール/型 | Rust 側設計の方針 | 備考 |
| --- | --- | --- | --- |
| AST (構文木) | `Ast` (`expr`, `pattern`, `decl` 等) | `crate::syntax::ast::*`（構造体 + `enum`）。`Span` は `u32` オフセット + `NonZeroU32` などを使用 | Span はバイトオフセットを維持し、`StageRequirement` を `enum` 化 |
| Typed AST | `Typed_ast` (`typed_expr`, `typed_pattern` 等) | `crate::semantics::typed::*`。型情報は `Arc<Ty>` を想定 | `dict_ref` 等の参照は `usize` インデックス化を検討 |
| 型表現 | `Types`, `Type_env` | `crate::semantics::types::*`。代数的データ型を `enum` で再現 | 仕様 `1-2` の命名を優先 |
| 制約/ソルバ | `Constraint`, `Constraint_solver` | `crate::semantics::constraints::*`。`Result` で例外を排除 | 発散検知 (`occurs_check_failed`) を同名エラーで保持 |
| パーサ状態 | `Core_parse.State`, `Core_parse_streaming` | `crate::frontend::streaming::*`。Packrat キャッシュを `FxHashMap` 等で再現 | `span_trace` / `packrat_stats` の型を揃える |
| 診断補助 | `Parser_expectation`, `Diagnostic.Builder` | `crate::frontend::diagnostics::*` | `expected_tokens` 生成ロジックを Rust へ移植 |

## 1.1.3 AST ノード対応チェックリスト

| カテゴリ | OCaml 定義 | Rust での対応指針 | 検証方法 |
| --- | --- | --- | --- |
| 位置情報 | `type span = { start; end_ }` | `struct Span { start: u32, end_: u32 }`。`end_` は排他的終端 | AST JSON ダンプ比較（`--emit-ast --format json`） |
| 識別子 | `type ident = { name; span }` | `struct Ident { name: SmolStr, span: Span }`。`SmolStr` 採用を検討 | Dual-write で名前/Span が一致するか |
| 式ノード | `expr_kind` バリアント | `enum ExprKind`。`Pipe`, `PerformCall`, `Lambda` 等を同名再現 | OCaml AST を JSON シリアライズして diff |
| パターン | `pattern_kind` | `enum PatternKind`。`PatConstructor` など optional payload を `Vec` で表現 | 型推論テスト (`test_type_inference.ml`) で束縛一覧を比較 |
| 宣言 | `decl_kind` | `enum DeclKind`。`effect`/`handler` など PoC 項目も含む | `examples/cli/*.reml` をパースして比較 |
| 効果参照 | `effect_reference` | `struct EffectRef { path: Option<ModulePath>, ... }` | 効果構文テストで JSON diff |

### JSON フォーマット方針
- OCaml AST ダンプ（既存テストの `*.json.golden`）を基準に、Rust AST も `serde` で同形式に整形する。
- フィールド順序はキー名アルファベット順で統一し、`null`/未使用フィールドは省略する。省略規則は P0 `0-1-baseline-and-diff-assets.md` の規約に合わせる。
- 違いが許容されるケース（例: `Span` の終端値が UTF-8 サロゲートにより ±1 変動）は `tolerance.md`（必要なら新設）で根拠を明示する。

## 1.1.4 Typed AST / 型情報の整合

- OCaml 側の詳細なフィールド棚卸しは `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` §5 を参照（W2 で更新）。

- `typed_expr` / `typed_pattern` の構造を Rust で `struct TypedExpr { kind, ty, span, dict_refs }` とする。型 (`ty`) は `Arc<Ty>` または `Interned<Ty>` を利用し、共有コストを抑える。
- `constrained_scheme` は `struct Scheme { ty: Ty, constraints: Vec<Constraint> }` とし、汎化 (`forall`) 情報は `SmallVec` 等で表現する。
- `Impl_registry` はスレッドセーフなレジストリ (`RwLock<HashMap<ImplId, ImplEntry>>`) で保持し、dual-write ではレジストリ内容を JSON 化して比較。Rust 側で未解決の場合は `pending_impls` としてログへ。
- 効果分析 (`Type_inference_effect`) のラベル付け (`call_tag_prefixes`, `is_known_ffi_call` 等) は `HashSet<String>` で実装し、Rust 移行後も `collect-iterator-audit-metrics.py --section effects` の計測値が一致することを確認。

## 1.1.5 ストリーミング状態 (`Core_parse_streaming`)

- OCaml 実装で計測しているメトリクスと構造の棚卸しは `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` §6 に集約した。

- `packrat_cache` は OCaml 版の `Parser_expectation.Packrat.t` と同じキー構造（入力オフセット → 期待値/結果）を保持。Rust では `IndexMap<usize, CacheEntry>` を仮採用し、`order` を維持する。
- `span_trace_pairs`（refined span の履歴）は `Vec<(Option<String>, Span)>` として保存し、回収ロジックを Rust へ移植。`parser/streaming_runner_tests.ml` のゴールデンが差分ゼロであることが完了条件。
- `Core_parse.Reply` の状態（`consumed`, `committed`）は Rust でも `enum Reply { Success { consumed: bool, ... }, RecoverableError { ... } }` の形で再現し、診断との結合は `1-2-diagnostic-compatibility.md` の手順に従う。

## 1.1.6 検証パイプライン

1. **AST ダンプ比較**: `remlc --frontend ocaml|rust --emit-ast --format json` を dual-write 実行し、`jq --sort-keys` で整形して差分を出す。差分が生じた場合は `reports/dual-write/front-end/ast-diff-report.md` に結果を集約する。
2. **Typed AST 比較**: 型推論後の `TypedExpr` を JSON/バイナリでダンプし、`type_equal` の結果と併せて比較。ソルバの解決順による順序差は `sorted` 正規化後に比較。
3. **Packrat/Span Trace 検証**: ストリーミングテストを dual-write 実行し、`packrat_stats`（ヒット/ミス件数）と `span_trace` を比較。差分が 5%以上の場合は `1-0-front-end-transition.md` のリスク項目へエスカレーション。
   - Rust 側は `cargo run --quiet --bin poc_frontend -- --emit-parse-debug reports/dual-write/front-end/latest/parse-debug.rust.json <input.reml>` を実行し、生成された `parse_result.packrat_stats` を `tooling/ci/collect-iterator-audit-metrics.py --source <file>` へ渡して OCaml 版と同じメトリクス経路で検証する。
4. **メトリクス同期**: `collect-iterator-audit-metrics.py --section parser --require-success` を Rust 版に対して実行し、OCaml 版との差分が `0.5pt` 以内に収まるか確認。

## 1.1.7 既知の差分と対応

- **Stage Requirement 注釈**: OCaml 版は `StageRequirement_annot` を `ident` ベースで保持している。Rust 版では `StageIdentId` (interned) にする可能性があるため、比較時に名称を正規化するフィルタを実装する。
- **効果参照の正規化**: `Effect_profile.canonicalize_module_path` の挙動差（`::` プレフィックスの扱い）が Rust 標準ライブラリで変化する可能性がある。比較前に `lowercase + trim` をかけ、スタイル差を抑制。
- **浮動小数リテラル**: OCaml は文字列のまま保持するが、Rust で `f64` に直列化する場合丸め誤差が発生する。文字列保存を基本とし、評価は別フェーズ（P2）で扱う。
- **Span のエンコーディング**: Windows 環境で改行コード（CRLF）差分が出る場合は、`0-2-windows-toolchain-audit.md` で定義した行末正規化を適用する。

## 1.1.8 ドキュメント連携
- 本文で決めた対応表は、Rust 実装の `README`（予定）や `docs/spec/1-5-formal-grammar-bnf.md` の脚注更新時に参照する。
- 新しい用語・型名が発生した場合は `appendix/glossary-alignment.md` に追記し、`docs/spec/0-2-glossary.md` との整合を確認する。
- P1 のレビュー結果は `docs/plans/rust-migration/1-0-front-end-transition.md` へフィードバックし、必要なら `docs/notes/core-library-outline.md` などの関連ノートへリンクを追加する。
