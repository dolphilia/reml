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

## Core.Parse への取り込み候補（優先度メモ）
- **高: API 拡張**  
  - 優先度ビルダー: Parsec/Megaparsec/FParsec のテーブル型 API を下敷きに、`chainl1/chainr1` 包装＋期待集合抑制をセットで導入する。  
  - `recoverWith` プリセット: chumsky の括弧復旧/スキップ再試行を参考に、`RunConfig.extensions["recover"]` と同期トークンを共有する形で追加。  
- **中: ランタイム追加・挙動変更**  
  - autoWhitespace/Layout: FastParse/Megaparsec 型の「スコープで差し替え」設計を採用し、Lex プロファイル未設定時は既存挙動を保持。  
  - 観測/プロファイル: nom の `context`/`dbg`、FastParse の `traced` を参考に、Packrat ヒット率やバックトラック深度を計測する `ParseObserver` を opt-in で提供。  
- **低: 糖衣/ガイドライン**  
  - `cut`/コミット強度のガイド: FParsec の `FatalError` と nom の `Err::Failure` を並記し、`consumed/committed` の使い分けを明文化。  
  - `lexeme/symbol`/`opaque` などのラベル糖衣を追加し、学習コストを下げる。

## リスクとオープン課題
- Packrat とゼロコピー（nom 的アプローチ）は安全性検証が未完。`effects`/Stage 監査と衝突しないキャッシュ鍵設計が必要。  
- レジューム可能なストリーミングパーサ（Ohm/ratchet 系）は `core-parse-streaming` の PoC 待ち。今回は除外し、別途 TODO 化する。  
- 左再帰対応は引き続き禁止方針だが、左再帰検知/ガードの糖衣（エラーメッセージ向上）は導入余地あり。

## 次のアクション
- 本調査を `Phase 7` の優先度再編に反映し、`docs/spec/2-2-core-combinator.md` 脚注案と `docs/notes/core-parse-api-evolution.md` の TODO を更新する。  
- autoWhitespace/Layout と優先度ビルダーについて、Rust 実装の PoC を作成し、バックトラック率と診断差分を測定するマイクロベンチを `reports/` 配下に準備する。  
- `recoverWith` プリセットの設計を `RunConfig.extensions["recover"]` と統一し、同期トークン設定との重複/競合を洗い出す。
