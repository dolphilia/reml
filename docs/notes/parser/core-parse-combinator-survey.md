# Core.Parse パーサーコンビネーター調査ノート

## 目的と評価軸
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` Phase 7 で求められている先行事例調査のまとめ。優先度付けの根拠を明文化し、後続フェーズの仕様ドラフトに引用できる形にする。
- 評価軸（`docs/spec/0-1-project-purpose.md` 準拠）  
  - 性能と安全性: Packrat/バックトラック抑制、メモリ/時間オーバーヘッド、左再帰ガード。  
  - API 操作性: 演算子優先度ビルダー、autoWhitespace/Layout、ストリーミング/レジューム可否。  
  - 診断と観測性: 期待集合の質、`cut`/コミット、プロファイル/トレース、エラー復旧戦略。  
  - 実装の拡張余地: Lex/Plugin/Streaming 連携、ゼロコピー入力、ユーザ状態の安全な受け渡し。

## 調査対象の概要
- Haskell: Parsec / Megaparsec（広く評価された文法表現と診断品質。`makeExprParser`, `indentBlock`, `try`/`cut` 相当）。  
- F#: FParsec（`OperatorPrecedenceParser`, `FatalError` によるコミット、ユーザ状態管理）。  
- Scala: FastParse（`cut` と `NoCut` でバックトラック制御、`Whitespace` implicits による autoWhitespace、`traced`）。  
- Rust: nom（ストリーミング/部分入力、`cut`/`context`、ゼロコピー入力、`dbg_dmp` で局所トレース）。  
- Rust: chumsky（`recover_with` 系のエラー復旧、`map_with_span`、`just`/`select` の明示的ラベル、左再帰を避ける `recursive` builder）。  
- Scala: cats-parse（バッファリングしないストリーミング志向、`P0`/`P1` で空入力可否を型で表現、`softProduct` による診断統合）。

## ライブラリ別メモ
### Parsec / Megaparsec
- **演算子優先度**: `makeExprParser` がデファクト。`chainl1/chainr1` を安全に包み、優先度テーブルで誤ったバックトラックを抑制。  
- **空白・レイアウト**: `space`/`lexeme`/`symbol` を通じて「トリビア共有」を一箇所に閉じ込める設計。Megaparsec の `indentBlock` / `IndentOpt` はオフサイドルールをシンプルに記述できる。  
- **エラー診断**: `label` と `hidden` の組み合わせで期待集合を抑制・絞り込み、`fancyError` でリッチなメタデータを注入。`try` による巻き戻しと `region` によるスパン計測が分離されている。  
- **観測性**: `Debug.Trace` 系の `dbg`、`parseTest` が軽量なトレース/プロトタイピングを提供。  
- **示唆**: 優先度ビルダーは `chainl1` 包装＋期待集合抑制の組み合わせが肝。`lexeme/symbol` の糖衣は RunConfig と連動させれば後方互換性を壊さず導入可能。

### FParsec
- **コミット制御**: `FatalError` を返すことで以降のバックトラックを禁止する「強い cut」を提供。これにより誤爆診断を減らしつつ性能を稼いでいる。  
- **優先度ビルダー**: `OperatorPrecedenceParser` が AST ビルダーを内包し、前置/後置/中置を統一的に扱う。  
- **ユーザ状態**: `UserState` をパーサ型に埋め込み、安全に可変状態を扱う。Lex/Plugin 共有に類似。  
- **示唆**: `cut` 強度を `committed` フラグで段階化し、`recover` 時にコミット済みかを診断へ持ち込む設計は Core.Parse の `Reply.committed` と親和性が高い。

### FastParse
- **バックトラック削減**: `cut` と `NoCut` の明示でエラーポイントを固定化し、`|`（choice）が増えても性能を保つ。  
- **autoWhitespace**: `implicit Whitespace` により空白処理を呼び出し側スコープで差し替え可能。`NoTrace`/`traced` で局所的トレースも切替式。  
- **ヒューリスティック診断**: `opaque` で期待集合を上書きし、ユーザ向けメッセージを簡潔にする。  
- **示唆**: RunConfig スコープで whitespace プロファイルを差し替える設計は FastParse と同型。`cut`/`opaque` の併用で過剰期待集合を抑制するパターンを採り入れたい。

### nom
- **ストリーミング/ゼロコピー**: `InputTake/Offset` トレイトでバイトスライスを共有し、部分入力（`Incomplete`）を返せる。Packratではないがメモリアロケーションを抑える。  
- **エラー階層**: `Err::Error`（巻き戻し可）と `Err::Failure`（コミット）を分離し、`context` でエラーパスを積み上げる。`cut` コンビネーターは `Error` を `Failure` へ昇格させるだけの薄い実装。  
- **診断補助**: `dbg_dmp`/`dbg` で局所トレース、`VerboseError` で期待集合を収集。  
- **示唆**: `Err` の2段階モデルは Core.Parse の `consumed/committed` と対応。`context` 相当を `label` と別枠で保持すれば、期待集合とトレースを両立できる。

### chumsky
- **エラー復旧**: `recover_with(skip_then_retry_until([...]))` など複数の復旧戦略をプリセット化。`nested_delimiters` による括弧ペア復旧も提供。  
- **スパンとメタ情報**: `map_with_span` で値と位置を同時に扱い、`then_ignore`/`ignored` でトリビアを明示的に捨てる。  
- **ラベリング**: `just("if").labelled("keyword if")` のように期待集合をユーザ文言に置き換えるパターンが普及している。  
- **示唆**: `recover_with` のプリセット化は Phase 7 の `recoverWith` バリエーションと一致。括弧復旧は `RunConfig.extensions["recover"]` の同期トークン拡充と親和性が高い。

### cats-parse
- **型レベル空入力制御**: `P0`（empty 可）/`P1`（empty 不可）でランタイムエラーを型で防ぐ設計。  
- **診断統合**: `softProduct`/`backtrack` でエラー集合を結合しつつ、不要な期待を抑える。  
- **ストリーミング志向**: chunked 入力を前提とし、完全入力を待たない API を提供。  
- **示唆**: `empty` 可否を型で表現する手法は Parser<T> の静的チェック強化に有用。`softProduct` 的な期待集合マージは期待過多の抑制に役立つ。

## Phase 7 統合方針（優先度再編と採否理由）
- `docs/spec/0-1-project-purpose.md` の性能（線形時間、メモリ2倍以下）・安全性（型/メモリ安全、例外ゼロ）を満たすかを判定軸に、採否と影響度を整理した。  
- 既存 API を壊さず追加できるものは「採用（非破壊）」、挙動変更を伴うものは RunConfig で opt-in にする。仕様改訂が必要な項目は脚注案を Phase 8 以降へ渡す。

### 優先度別採否テーブル
| 区分 | 項目 | 方針 | 性能/安全性の観点 | 仕様・互換性の影響 |
| --- | --- | --- | --- | --- |
| 高(API拡張) | 優先度ビルダー (`makeExprParser` 系) | 採用（非破壊追加） | `chainl1/chainr1` の誤バックトラックを抑止し、O(n) を維持。committed フラグを尊重するため安全性を悪化させない。 | `docs/spec/2-2-core-combinator.md` に API 追加脚注を起草。既存 API へは追加のみで破壊なし。 |
| 高(API拡張) | `recoverWith` バリエーション（`with_default`/`with_context`） | 採用（非破壊追加） | 期待集合を過剰化せず復旧でき、診断品質向上による再試行削減で性能リスクなし。安全性は `committed` を保持する契約で担保。 | `2-2-core-combinator.md` に補助 API 脚注を追加。既存 `recover` 契約は維持。 |
| 中(ランタイム) | autoWhitespace / Layout | 条件付き採用（RunConfig opt-in） | Lex プロファイル欠如時は従来の空白消費のみを維持し、性能退行を防ぐ。オフサイド判定はスパン計測依存で安全性に影響なし。 | `docs/spec/2-0-parser-api-overview.md` に `RunConfig.lex/layout` フラグ脚注ドラフトを作成。互換性リスクは opt-in に限定。 |
| 中(ランタイム) | ParseObserver / ParserProfile（ヒット率/BT/Packrat 統計） | 採用（デフォルト OFF） | 観測処理はオプトイン＋定数倍オーバーヘッドに抑え、Packrat キャッシュ鍵を共有することで安全性を維持。 | 仕様化前に `docs/notes/parser/core-parse-api-evolution.md` へ暫定契約を記録。API 追加は非破壊。 |
| 低(糖衣/ガイド) | `left_recursion_guard` ガイド＋サンプル | 採用（ガイド/ラッパー） | ランタイム挙動は既存コンビネーターの組み合わせのみで安全性影響なし。左再帰検知で無限ループを防ぎ安定性を向上。 | 仕様本体は触らず、`docs/spec/2-2-core-combinator.md` の脚注候補としてガイドを付記。 |
| 低(糖衣/ガイド) | `lexeme/symbol` 派生・`cut/opaque` ラベル糖衣 | 採用（非破壊追加） | 空白スキップを明示することで誤パースを減らし、期待集合の縮約で診断性能を改善。安全性は既存 `RunConfig` デフォルトを尊重して維持。 | `lexeme/symbol` は現行 API 上に追加。`cut/opaque` ガイドは仕様に影響なし。 |

### 保留・見送り（性能/安全性リスクが未精査）
| 項目 | 状態 | 理由と再検討条件 |
| --- | --- | --- |
| Resumable parser（ストリーミング再開 API） | 保留 | `core-parse-streaming` の PoC 未整備。Capability/Stage 監査と整合する再開ポイント設計が必要。Streaming 実装と同時に再評価。 |
| バイトスライス前提の zero-copy 最適化 | 保留 | メモリ安全性・プラットフォーム差異の検証不足。Packrat キャッシュ鍵とエンコーディング整合を測定してから採否決定。 |

## リスクとオープン課題
- 観測系 API のデータ項目（Packrat キャッシュ鍵、バックトラック率）の仕様化は未決定。`effects`/Stage 監査ログと衝突しない形でキーを決める必要がある。  
- レジューム可能なストリーミングパーサ（Ohm/ratchet 系）は `core-parse-streaming` の PoC 待ち。再開 API が Capability Stage と整合するか検証が未了。  
- 左再帰自体の禁止方針は継続するが、ガード/エラーメッセージの糖衣は採用予定。ルール ID との組み合わせで誤検出を避けるチェックが必要。

## 次のアクション
- 上表の採否・影響メモを `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` Phase 7 の実施結果として反映済み。以降のフェーズで脚注化する。  
- `docs/spec/2-2-core-combinator.md` へ優先度ビルダーと `recoverWith` の脚注ドラフトを起票し、`docs/spec/2-0-parser-api-overview.md` に RunConfig の `lex/layout/profile` フラグ案を追加する（Phase 8/9/10 で採用可否を決定）。  
- 観測系 API と recover 同期トークンの暫定契約を `docs/notes/parser/core-parse-api-evolution.md` に追記し、Packrat キャッシュ鍵の整合性テストを計画する。  
- autoWhitespace/Layout の PoC を Rust 実装で作成し、バックトラック率・期待集合差分を `reports/` 配下のマイクロベンチで測定する。  
- ストリーミング再開 API と zero-copy 最適化の検証条件を `docs/plans/bootstrap-roadmap/4-1-scenario-matrix-plan.md` のシナリオに追加する検討を行う。
