# 付録A: 用語集と索引

## A.1 目的と使い方

本付録は、Reml コンパイラのソースコード解説書（Source Code Commentary）で使用される専門用語と、主要な実装モジュールの参照先をまとめたものです。本書を読み進める中で、未知の用語やモジュール名に遭遇した場合のクイックリファレンスとして活用してください。

本付録は以下の 3 つのセクションで構成されています。

1.  **用語集 (Glossary)**: 言語仕様やコンパイラ設計における重要な用語を定義し、関連する解説章、公式仕様書、および実装コードへのリンクを提供します。
2.  **モジュール索引 (Module Index)**: コンパイラのディレクトリ構造に基づき、各モジュールの役割と解説されている章、対応する仕様書を一覧化しています。
3.  **キーワード索引**: 用語や概念から、それらが詳細に解説されている章を逆引きするための索引です。

## A.2 用語集 (Glossary)

本セクションでは、Reml の言語設計およびコンパイラ実装における重要語彙を五十音順（英字の用語はアルファベット順）で解説します。

### A

#### ADT (Algebraic Data Type / 代数的データ型)

直積（レコード、構造体）と直和（バリアント、列挙体）を組み合わせたデータ型。Reml ではバリアントコンストラクタも関数として扱われます。

- **解説書**: 第7章（型チェックと型推論）
- **仕様**: `docs/spec/1-2-types-Inference.md`
- **実装**: `compiler/frontend/src/typeck`, `compiler/runtime/src/data`

#### AuditEnvelope

診断メッセージや操作ログに付随する監査情報を保持する構造体。`audit_id`、変更セット、Capability の整合性情報などを含みます。

- **解説書**: 第14章（Capability と監査）
- **仕様**: `docs/spec/3-6-core-diagnostics-audit.md`
- **実装**: `compiler/runtime/src/audit/mod.rs`

### B

#### Backpressure (バックプレッシャー)

ストリーム処理や非同期通信において、受信側の処理能力を超えないように送信量を制御する仕組み。Reml では `DemandHint` を用いてフィードバックを行います。

- **解説書**: 第9章（実行パイプライン）
- **仕様**: `docs/spec/3-9-core-async-ffi-unsafe.md`
- **実装**: `compiler/frontend/src/streaming/flow.rs`

#### Binding Power (結合力)

Pratt パーサーにおいて、演算子の優先順位を決定するための数値。数値が大きいほど結合が強く（優先度が高く）なります。

- **解説書**: 第5章（構文解析）
- **仕様**: `docs/spec/2-4-op-builder.md`
- **実装**: `compiler/frontend/src/parser/expr.rs`

### C

#### Capability (ケイパビリティ)

ファイル I/O、ネットワークアクセス、時間計測など、副作用を伴う操作を行うための権限を表すトークン。Reml のセキュリティモデルの中核です。

- **解説書**: 第14章（Capability と監査）
- **仕様**: `docs/spec/3-8-core-runtime-capability.md`
- **実装**: `compiler/runtime/src/capability`

#### Capability Stage

機能の安定度を示すメタデータ。`Experimental`（実験的）、`Beta`（ベータ）、`Stable`（安定）の 3 段階があり、バージョン間の互換性管理に使用されます。

- **解説書**: 第14章（Capability と監査）
- **仕様**: `docs/spec/3-8-core-runtime-capability.md`
- **実装**: `compiler/runtime/src/stage.rs`

#### CST (Concrete Syntax Tree / 具象構文木)

ソースコードの構造を、空白やコメントなども含めて忠実に表現したツリー構造。Reml では `GreenTree` (Rowan など) に近い概念として扱われることがありますが、解析の主役は AST です。

- **解説書**: 第5章（構文解析）
- **実装**: `compiler/frontend/src/parser`

### D

#### DemandHint

ストリーミング実行において、パーサが必要とする次の入力サイズや優先度を `Feeder` に伝えるためのヒント情報。

- **解説書**: 第9章（実行パイプライン）
- **仕様**: `docs/spec/2-7-core-parse-streaming.md`
- **実装**: `compiler/frontend/src/streaming/mod.rs`

#### Diagnostic (診断)

コンパイル時のエラー、警告、情報を構造化したデータ。メッセージだけでなく、発生位置 (`Span`)、修復提案 (`FixIt`)、監査メタデータを含みます。

- **解説書**: 第6章（診断と出力）
- **仕様**: `docs/spec/2-5-error.md`, `docs/spec/3-6-core-diagnostics-audit.md`
- **実装**: `compiler/frontend/src/diagnostic`, `compiler/runtime/src/diagnostics`

### E

#### Effect Row (効果行)

関数が引き起こす可能性のある副作用の集合を表現するもの。`{io, panic}` のように記述され、型システムによって追跡されます。

- **解説書**: 第10章（エフェクトとFFI実行）
- **仕様**: `docs/spec/1-3-effects-safety.md`
- **実装**: `compiler/frontend/src/effects`

### F

#### FFI (Foreign Function Interface)

他のプログラミング言語（主に C や Rust）で記述された関数を呼び出すための仕組み。Reml では非同期実行や所有権管理と密接に統合されています。

- **解説書**: 第17章（FFI とネイティブ連携）
- **仕様**: `docs/spec/3-9-core-async-ffi-unsafe.md`
- **実装**: `compiler/runtime/src/ffi`, `compiler/ffi_bindgen`

#### FixIt

IDE やエディタ向けに提供される、コードの自動修正案。`Diagnostic` に含まれ、`Insert`、`Replace`、`Delete` などの操作を定義します。

- **解説書**: 第6章（診断と出力）
- **仕様**: `docs/spec/2-5-error.md`
- **実装**: `compiler/frontend/src/diagnostic/fixit.rs`

### H

#### HM Inference (Hindley-Milner 型推論)

Reml が採用している型推論アルゴリズム。明示的な型注釈がなくても、文脈から最も一般的な型（主要型）を導出します。

- **解説書**: 第7章（型チェックと型推論）
- **仕様**: `docs/spec/1-2-types-Inference.md`
- **実装**: `compiler/frontend/src/typeck`

### L

#### Lexer (字句解析器)

ソースコードの文字列を、意味のある最小単位（トークン）の列に変換するコンポーネント。

- **解説書**: 第4章（字句解析）
- **仕様**: `docs/spec/2-3-lexer.md`
- **実装**: `compiler/frontend/src/lexer`

### O

#### OpBuilder

演算子の優先順位と結合規則を宣言的に定義するための DSL およびそのビルダ。Pratt パーサーのテーブル生成に使用されます。

- **解説書**: 第5章（構文解析）
- **仕様**: `docs/spec/2-4-op-builder.md`
- **実装**: `compiler/frontend/src/parser/op_builder.rs` (概念的)

### P

#### Packrat Parsing

メモ化（Memoization）を利用してバックトラックのコストを抑える構文解析の手法。Reml のパーサは必要に応じてこの機能を有効化します。

- **解説書**: 第5章（構文解析）
- **仕様**: `docs/spec/2-6-execution-strategy.md`
- **実装**: `compiler/frontend/src/parser`

#### Parser Combinator (パーサコンビネータ)

小さなパーサ関数を組み合わせることで、複雑な文法を解析するパーサを構築する手法。

- **解説書**: 第5章（構文解析）
- **仕様**: `docs/spec/2-0-parser-api-overview.md`
- **実装**: `compiler/frontend/src/parser`

### R

#### RunConfig

パーサの実行時の挙動を制御する設定オブジェクト。Packrat の有効化、左再帰の深さ制限、拡張フック (`extensions`) などを管理します。

- **解説書**: 第13章（ランタイムの全体像）
- **仕様**: `docs/spec/3-7-core-config-data.md`
- **実装**: `compiler/runtime/src/run_config.rs`

#### RuntimeBridge

外部のランタイム環境（ホストアプリケーションなど）と Reml のランタイムを接続するためのインターフェース。

- **解説書**: 第13章（ランタイムの全体像）
- **仕様**: `docs/spec/3-8-core-runtime-capability.md`
- **実装**: `compiler/runtime/src/runtime`

### S

#### Span

ソースコード上の特定の位置（開始バイトと終了バイト）を表す構造体。エラーメッセージの表示や、AST ノードとソースコードの対応付けに使用されます。

- **解説書**: 第4章（字句解析）
- **仕様**: `docs/spec/2-1-parser-type.md`
- **実装**: `compiler/frontend/src/span.rs`

### T

#### Token (トークン)

字句解析によって生成される、ソースコードの最小構成単位。種別 (`TokenKind`)、位置情報 (`Span`)、および必要に応じてリテラル値を保持します。

- **解説書**: 第4章（字句解析）
- **仕様**: `docs/spec/2-3-lexer.md`
- **実装**: `compiler/frontend/src/token.rs`

#### Trait (トレイト)

共通の振る舞い（メソッド）を定義するための仕組み。Haskell の型クラスに類似しており、ポリモーフィズムを実現します。

- **解説書**: 第7章（型チェックと型推論）
- **仕様**: `docs/spec/1-2-types-Inference.md`
- **実装**: `compiler/frontend/src/typeck`

### U

#### Unicode Scalar Value (Unicode スカラー値)

サロゲートペアを除く、すべての Unicode コードポイント。Reml の `Char` 型に対応します。

- **解説書**: 第4章（字句解析）
- **仕様**: `docs/spec/1-4-test-unicode-model.md`
- **実装**: `compiler/frontend/src/unicode.rs`

## A.3 モジュール索引 (Module Index)

以下は、`compiler` ディレクトリ以下の主要モジュールと、本書の解説章との対応表です。

### Frontend (`compiler/frontend`)

| モジュールパス | 章 | 役割 | 対応する仕様 |
| :--- | :--- | :--- | :--- |
| `src/lexer` | 第4章 | 字句解析 | `2-3-lexer.md` |
| `src/token.rs` | 第4章 | トークン定義 | `2-3-lexer.md` |
| `src/unicode.rs` | 第4章 | Unicode 処理 | `1-4-test-unicode-model.md` |
| `src/span.rs` | 第4章, 第6章 | 位置情報 (Span) | `2-1-parser-type.md` |
| `src/parser` | 第5章 | 構文解析 | 2-0 〜 2-4 |
| `src/diagnostic` | 第6章 | 診断・エラー | `2-5-error.md` |
| `src/typeck` | 第7章 | 型検査・推論 | `1-2-types-Inference.md` |
| `src/semantics` | 第8章 | 意味解析 | `1-0-language-core-overview.md` |
| `src/pipeline` | 第9章 | 実行パイプライン | `2-6-execution-strategy.md` |
| `src/streaming` | 第9章 | ストリーミング処理 | `2-7-core-parse-streaming.md` |
| `src/effects` | 第10章 | エフェクト解析 | `1-3-effects-safety.md` |
| `src/ffi_executor.rs` | 第10章 | FFI 実行制御 | `3-9-core-async-ffi-unsafe.md` |

### Runtime (`compiler/runtime`)

| モジュールパス | 章 | 役割 | 対応する仕様 |
| :--- | :--- | :--- | :--- |
| `src/runtime` | 第13章 | ランタイムコア | `3-8-core-runtime-capability.md` |
| `src/capability` | 第14章 | Capability 管理 | `3-8-core-runtime-capability.md` |
| `src/audit` | 第14章 | 監査ログ | `3-6-core-diagnostics-audit.md` |
| `src/collections` | 第15章 | コレクション | `3-2-core-collections.md` |
| `src/text` | 第15章 | テキスト処理 | `3-3-core-text-unicode.md` |
| `src/io` | 第15章 | 入出力 | `3-5-core-io-path.md` |
| `src/diagnostics` | 第16章 | 実行時診断 | `3-6-core-diagnostics-audit.md` |
| `src/dsl` | 第16章 | DSL 基盤 | `3-16-core-dsl-paradigm-kits.md` |
| `src/ffi` | 第17章 | FFI インターフェース | `3-9-core-async-ffi-unsafe.md` |
| `src/lsp` | 第18章 | LSP サポート | `3-14-core-lsp.md` |
| `src/system` | 第18章 | システム連携 | `3-18-core-system.md` |

### Backend & Others

| モジュールパス | 章 | 役割 | 対応する仕様 |
| :--- | :--- | :--- | :--- |
| `compiler/backend/llvm` | 第11章, 第12章 | LLVM コード生成 | - |
| `compiler/adapter` | 第19章 | プラットフォーム抽象化 | - |
| `compiler/ffi_bindgen` | 第20章 | バインディング生成 | - |
| `compiler/xtask` | 第21章 | 開発ツール | - |

## A.4 キーワード索引 (Keyword Index)

### あ行

- **アルゴリズム W**: 第7章
- **依存関係**: 第19章 (Adapter), 第20章 (Bindgen)
- **意味解析**: 第8章
- **エラー回復**: 第5章
- **演算子順位**: 第5章

### か行

- **解決 (Resolution)**: 第8章
- **型推論**: 第7章
- **監査 (Audit)**: 第14章
- **関数**: 第7章, 第8章
- **構文解析**: 第5章

### さ行

- **再帰**: 第5章 (左再帰)
- **字句解析**: 第4章
- **実行パイプライン**: 第9章
- **診断**: 第6章
- **ストリーミング**: 第9章
- **制限 (Constraint)**: 第14章 (Capability)

### た行

- **単一化 (Unification)**: 第7章
- **遅延評価**: 第15章
- **抽象構文木 (AST)**: 第5章, 第8章
- **トークン**: 第4章

### は行

- **バックエンド**: 第11章
- **副作用**: 第10章

### ま行

- **マクロ**: 第5章 (Parser)
- **モジュール**: 第8章 (意味解析)

### ら行

- **ランタイム**: 第13章
- **リテラル**: 第4章 (Token)
- **領域 (Arena)**: 第7章 (型格納)

### 英数字

- **CLI**: 第3章, 第21章
- **FFI**: 第10章, 第17章, 第20章
- **LLVM**: 第11章
- **LSP**: 第18章
- **Packrat**: 第5章
- **Unicode**: 第4章, 第15章
