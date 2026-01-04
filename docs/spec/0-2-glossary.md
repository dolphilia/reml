# 0.2 用語集

Reml 仕様書で繰り返し登場する専門用語と概念をまとめた。各項目は仕様書内の該当セクションへのリンクを添え、詳細な定義や背景を参照できるようにしている。

## 言語コアと型システム

- **Hindley–Milner 型推論 (HM)**: Reml の型推論は Hindley–Milner 系 (Algorithm W) を採用し、サブタイピングを導入せずに合一ベースで多相型を導出する。[1-2 型システムと推論](1-2-types-Inference.md) に設計意図と制約が整理されている。
- **ランク1多相 (Rank-1 Polymorphism)**: 多相はトップレベル `let` など 1 階層でのみ量化され、高ランク多相は将来的な拡張扱い。[1-2 型システムと推論](1-2-types-Inference.md) では一般化タイミングが明示されている。
- **型スキーム (Type Scheme)**: `∀a1 … an. τ` 形式で量化された型。Reml では一般化された束縛をスキームとして保存し、呼び出しごとに具体化する。[1-2 型システムと推論](1-2-types-Inference.md) 参照。
- **トレイト (Trait)**: Haskell の typeclass に相当する静的ディスパッチ機構で、演算子や汎用 API の解決に使われる。例として `Add` や `Zero` が [1-2 型システムと推論](1-2-types-Inference.md) で紹介されている。
- **コヒーレンスと孤児規則**: トレイト実装の一貫性を保つため「定義元モジュールか対象型のモジュールでのみ `impl` を書ける」という孤児規則を課し、重複解決を禁止する規則。[1-2 型システムと推論](1-2-types-Inference.md) に採用理由が記載されている。
- **値制限 (Value Restriction)**: `let` 束縛の右辺が純粋な式の場合だけ型一般化を許す規則。効果を含む式は単相にとどめて安全性を確保する。[1-2 型システムと推論](1-2-types-Inference.md) C.3 を参照。
- **代数的データ型 (ADT)**: `type Expr = | Int | Add` のようなバリアント型。コンストラクタは関数として型付けされ、パターンマッチの基盤となる。[1-2 型システムと推論](1-2-types-Inference.md) A.2 に基本形が示される。
- **ニュータイプ (Newtype)**: 既存型へ零コストで別名を与える `type Name = new T` 構文。暗黙変換を避けつつ静的な区別を付けられる。[1-2 型システムと推論](1-2-types-Inference.md) A.4 参照。
- **双方向型付け (Bidirectional Typing)**: 明示注釈がある場合に推論と検査を往復させ、エラー位置の精度を高める戦略。[1-2 型システムと推論](1-2-types-Inference.md) C.4 で推奨されている。

## 効果システムと安全性
- **代数的効果 (Algebraic Effects)**: `perform` と `handle` によって副作用を構造化し、ハンドラで挙動を差し替える仕組み。[1-3 効果と安全性](1-3-effects-safety.md) および [1-2 型システムと推論](1-2-types-Inference.md#c-6-効果行とハンドラの型付け実験段階) に実験段階の仕様がある。
- **効果タグ (Effect Tag, Σ)**: `A -> B ! {io, panic}` の `{…}` に記録される効果集合。関数や Capability が引き起こし得る副作用を明示し、静的検証に利用する。[1-3 効果と安全性](1-3-effects-safety.md) を参照。
- **効果行 (Effect Row)**: `!ε` のような変数を含む効果集合で、行多相を使って呼び出し側に残余効果を伝搬させる仕組み。[1-2 型システムと推論](1-2-types-Inference.md#c-6-効果行とハンドラの型付け実験段階) に一般化条件が示される。
- **効果ハンドラ (Effect Handler)**: `handle comp with handler` 構文で、特定効果を捕捉・再解釈するパターン。捕捉できなかった効果は残余として再び `Σ` に残る。[1-2 型システムと推論](1-2-types-Inference.md#c-6-効果行とハンドラの型付け実験段階) を参照。
- **`@pure` 属性**: 関数や DSL エクスポートに副作用がないことを示す注釈。効果タグと Capability チェックの整合性を確保するため、[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) でも参照される。

## パーサー実行モデル
- **パーサーコンビネーター**: 小さなパーサ関数を合成して大きな構文解析器を組み立てる手法。Reml の `Core.Parse` 章全体（[2-0 標準パーサーAPI 概要](2-0-parser-api-overview.md)）が前提とする設計思想。
- **`Parser<T>`**: `fn(&mut State) -> Reply<T>` という関数型で表現されるパーサの基本単位。[2-1 パーサ型](2-1-parser-type.md) に入出力モデルが定義されている。
- **`Reply<T>`**: 成功/失敗と「入力を消費したか」「コミット済みか」を 2 ビットで保持する戻り値。[2-1 パーサ型](2-1-parser-type.md) で 4 状態の意味論が説明される。
- **`RunConfig`**: Packrat の有無、左再帰処理、`require_eof`、ロケールなど実行時オプションを集約した設定構造体。エクステンションフック `RunConfig.extensions` もここに含まれ、[2-1 パーサ型](2-1-parser-type.md) D 節で解説される。
- **DemandHint**: ストリーミング実行時に次の入力バッチに必要なサイズ・優先度を伝えるヒント構造体。`Pending` 継続とともに返却され、`Feeder` がバックプレッシャー制御を行う指針となる。[2-7 ストリーミング実行](2-7-core-parse-streaming.md#feeder-demandhint) を参照。
- **FlowController**: ストリーミングランナーが `resume`／`pump` の進行管理に利用する制御ハンドル。`DemandHint` と組み合わせてチャンク投入タイミングを決め、`RunConfig.extensions["stream"]` とも連携する。[2-7 ストリーミング実行](2-7-core-parse-streaming.md#flow-controller) 参照。
- **Packrat パース**: 入力位置とパーサ ID をキーとするメモ化でバックトラックを高速化する戦略。[2-6 実行戦略](2-6-execution-strategy.md) がメモテーブルの利用方針を示す。
- **左再帰サポート**: Packrat と組み合わせた seed-growing だが、仕様として左再帰文法の直接記述は想定しない。`precedence` / `chainl1` への変換を前提とし、`left_recursion` は無限再帰の安全弁として扱う。[2-6 実行戦略](2-6-execution-strategy.md) を参照。
- **トランポリン (Trampoline)**: 再帰的なパーサ合成をループに変換し、末尾再帰のスタック消費を抑えるテクニック。[2-6 実行戦略](2-6-execution-strategy.md) に最適化理由が記載される。
- **`cut` / コミット**: ある地点以降の失敗を `committed=true` にして代替パスを試さないよう指示するコンビネーター。[2-5 エラーハンドリング](2-5-error.md) と [2-1 パーサ型](2-1-parser-type.md#e-コミットと消費の意味論) で使用例が示される。
- **期待集合 (Expected Set)**: エラー発生時に「何が来るはずだったか」を報告するためのシンボル集合。[2-5 エラーハンドリング](2-5-error.md) で診断メッセージ整形と統合される。
- **`recover` 戦略**: 特定の失敗から入力位置を進めつつ再解析を試みるためのコンビネーター群。[2-5 エラーハンドリング](2-5-error.md) D 節が設計ガイドラインを持つ。
- **`Span`**: ソース上の開始/終了位置を保持する構造体で、AST に位置情報を付与するために使われる。[2-1 パーサ型](2-1-parser-type.md#c-スパンとトレース) を参照。
- **`SpanTrace`**: 成功した部分パースの履歴を収集するオプション機能。`RunConfig.trace` 有効時に診断補助として利用される。[2-1 パーサ型](2-1-parser-type.md#c-スパンとトレース) 参照。
- **`Diagnostic`**: エラーや警告を構造化して保持する報告単位。[2-5 エラーハンドリング](2-5-error.md) と [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) でメッセージ整形と監査連携が規定される。
- **Pratt パーサー**: 演算子の結合力を binding power で管理し、前置/中置/後置演算子を宣言的に処理する手法。Reml の `precedence` ビルダーは Pratt 法と連鎖畳み込みのハイブリッド実装を採用し、[2-4 演算子優先度ビルダー](2-4-op-builder.md) に設計理由が記載される。
- **結合力 (Binding Power)**: Pratt パーサーが演算子の優先順位を比較するために用いる数値。高い binding power を持つ演算子ほど強く右項を結び付け、[2-4 演算子優先度ビルダー](2-4-op-builder.md) でレベル順に調整される。
- **Fixity（結合方向）**: 演算子が左結合 (`infixl`)、右結合 (`infixr`)、非結合 (`infixn`) などどのように束縛されるかを表す属性。DSL では `:infix_left` / `:infix_right` / `:infix_nonassoc` / `:prefix` / `:postfix` / `:ternary` のような Fixity シンボルで記述し、[1-5 形式文法 §2.1](1-5-formal-grammar-bnf.md#21-opbuilder-dsl) でトークンが定義される。[2-4 演算子優先度ビルダー](2-4-op-builder.md) の `level` 宣言で実際の挙動が指定される。
- **OpBuilder DSL**: `Core.Parse.OpBuilder` が提供する宣言的な優先度テーブル記法。`builder.level(priority, :fixity, ["token", ...])` で複数演算子と結合方向をまとめて定義し、内部で `precedence` API と同じ AST を構築する。`FixitySymbol` の構文は [1-5 形式文法 §2.1](1-5-formal-grammar-bnf.md#21-opbuilder-dsl) に、意味論は [2-4 演算子優先度ビルダー](2-4-op-builder.md#b-使い方api-と-dsl) に記載される。
- **seed-growing 左再帰**: Packrat メモ化と組み合わせて左再帰規則を安全に展開する手法。`RunConfig.left_recursion="auto"` が必要に応じて適用し、左再帰が混入した場合の安全弁として利用する。[2-6 実行戦略](2-6-execution-strategy.md#c-メモ化packratと左再帰) に挙動が説明される。

## Unicode とテキスト処理
- **Unicode 3層モデル (Byte / Char / Grapheme)**: Reml はバイト列・Unicode スカラー値・拡張書記素クラスタの 3 レイヤで文字を扱い、API ごとに適切な粒度を選択する。[1-4 Unicode 文字モデル](1-4-test-unicode-model.md) 参照。
- **Unicode スカラー値 (コードポイント)**: UTF-8 で表現される単一の Unicode スカラー。`Char` 型が対応し、位置情報や比較の基本単位となる。[1-4 Unicode 文字モデル](1-4-test-unicode-model.md) 参照。
- **拡張書記素クラスタ (Extended Grapheme Cluster)**: ユーザーが 1 文字と認識する複合文字。列数算出や `column` 情報はこの単位で計測する。[1-4 Unicode 文字モデル](1-4-test-unicode-model.md) を参照。
- **Unicode 正規化 (NFC/NFD/NFKC/NFKD)**: 等価な文字列表現を統一する正規化形式。仕様では対応必須の正規化セットとして [0-1 プロジェクト目的](0-1-project-purpose.md#31-unicode対応の充実) と [3-3 Core Text & Unicode](3-3-core-text-unicode.md) に記載がある。
- **XID_Start / XID_Continue**: Unicode 標準が定義する識別子開始/継続文字カテゴリ。識別子の構文規則として [1-1 構文](1-1-syntax.md#a3-識別子とキーワード) で採用されている。
- **GraphemeIndex / CpIndex**: `Input` が保持する書記素・コードポイント境界キャッシュ。高速な位置計算やバックトラックに利用され、[2-1 パーサ型](2-1-parser-type.md#b-入力モデル-input) で説明される。

## ランタイムと Capability
- **Capability Registry**: GC や IO などランタイム機能を Capability として登録・照会する中心レジストリ。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) が API と責務を定義する。
- **Capability Handle**: 各 Capability 実装を表す不透明ハンドル。`CapabilityHandle::Io` などのバリアントで分岐し、Registry 経由で取得する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#11-capabilityhandle-のバリアント) を参照。
- **Capability Stage**: `Experimental/Beta/Stable` の成熟度を示すメタデータ。`verify_capability_stage` で下限を検証し、`Diagnostic.extensions["effects"]` や `AuditEnvelope.metadata` に要求/実際の Stage を記録する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#capability-stage-contract) および [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#24-stage-差分プリセット-effectdiagnostic) 参照。
- **StageRequirement**: Capability や Runtime Bridge が満たすべき Stage 条件を表す列挙。`Exact` と `AtLeast` を持ち、Stage 順序 `Experimental < Beta < Stable` で検証される。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#capability-stage-contract) に仕様がある。
- **RuntimeBridge**: 外部ランタイムやホットリロード対象を Reml 実行系へ接続する契約。`RuntimeBridgeDescriptor` に Stage・Capability・ターゲット整合性を記録し、`bridge.stage.*` 診断と連携する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#10-runtime-bridge-契約) を参照。
- **RuntimeBridgeRegistry**: 登録済み Runtime Bridge を管理するレジストリ。`register_bridge`／`acquire_bridge` を通じて Stage と Capability を検証し、監査テンプレートを提供する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#101-runtimebridgeregistry-とメタデータ) 参照。
- **RuntimeBridgeReloadSpec**: ホットリロード互換ブリッジが公開する差分形式・ロールバック方針。`RuntimeBridgeReloadDiagnostics` と共に `bridge.reload` 監査イベントへ情報を供給する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#103-ホットリロード契約) 参照。
- **SecurityCapability**: Capability の署名検証、許可、隔離レベルを管理するセキュリティ用ハンドル。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#12-セキュリティモデル) が構造体と検証手順を示す。
- **RuntimeCapability / TargetCapability**: 実行環境が備える命令セットやクロックなどの機能一覧。CI や `Core.Env` が環境適合性を確認するために利用し、[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#13-プラットフォーム情報と能力) に列挙がある。
- **SandboxProfile**: Capability 利用時に課すリソース制限を記述する共通プロファイル。`SecurityCapability` と連携して監査方針を適用する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#12-セキュリティモデル) 参照。
- **RunConfigTarget**: `RunConfig.extensions["target"]` に格納されるターゲットプロファイルで、OS/ABI/Capability 情報をまとめて `@cfg` 条件分岐へ渡す構造体。[2-6 実行戦略](2-6-execution-strategy.md#b-2-runconfig-のコアスイッチ) に項目一覧が記載される。
- **PlatformInfo**: 実行中プラットフォームの OS・アーキテクチャ・利用可能能力を報告する構造体。`platform_info()` が返し、Capability Registry と整合させて最適化や制限判断を行う。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#13-プラットフォーム情報と能力) 参照。

## 監査・移行関連
- **Rust Frontend / `poc_frontend`**: `compiler/frontend` の Rust 版 Reml フロントエンド。`cargo run --manifest-path compiler/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics <sample.reml>` を Phase 2-8 以降の標準検証コマンドとし、`docs/plans/rust-migration/overview.md` / `.../unified-porting-principles.md` で運用上の優先度と成功指標を定義する。
- **FFI/Core Prelude 回帰カテゴリー**: Phase4 シナリオマトリクスで `FFI-CORE-PRELUDE-001` などのテスト群を追跡する分類。`docs/spec/3-1-core-prelude-iteration.md` で定義される Core Prelude API と `docs/spec/3-6-core-diagnostics-audit.md` の Stage/Capability 契約を `reml_runtime_ffi` 経由で再現できるかを `compiler/frontend/tests/core_iter_*.rs` の snapshot（`cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_effects` など）で検証する。capability shim の維持期間中は Phase4 KPI（spec_core/practical）と連動し、Stage mismatch が復活した場合は `phase4-scenario-matrix.csv` の `resolution` を `impl_fix` へ戻す。 
- **spec-audit ディレクトリ**: `reports/spec-audit/` に配置した Chapter 別の監査ログ。`ch0`〜`ch3` で CLI 出力やリンクチェックを保存し、`diffs/` に `rust-gap` メモ、`summary.md` にコマンド履歴を記録する。[docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md](../plans/bootstrap-roadmap/2-8-spec-integrity-audit.md) の作業ブレークダウンを参照。
- **`rust-gap` ラベル**: Rust Frontend がまだ受理できない構文や API 差分を示す指標。`reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` のようなログとセットで `docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` に登録し、Phase 3 引き継ぎタスクへ接続する。

## 所有権とリソース管理
- **所有権 (Ownership)**: 値とそのメモリ資源を解放する責務を持つ主体。Core.Ffi では `Ownership::Borrowed` / `Owned` / `Transferred` を通じて移譲パターンを明示し、効果章では RC ポインタ操作と `Result` 伝播で責務を追跡する。[1-3 効果と安全性](1-3-effects-safety.md#m5-所有権とリソース管理), [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界)
- **借用 (Borrow)**: 所有権を保持したまま参照を一時的に貸し出す操作。`Cell`/`Ref` API は `Borrow<T>` を返し、FFI では `Ownership::Borrowed` が呼び出し期間を限定する。[3-2 Core Collections](3-2-core-collections.md#32-cellt-ref), [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界)
- **共有借用 (Shared Borrow)**: 読み取り専用で複数同時に許可される借用。`Borrow<T>` は `effect {readonly}` と整合し、Capability 監査で競合のない観測として扱われる。[3-2 Core Collections](3-2-core-collections.md#32-cellt-ref), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#diagnostic-presets)
- **可変借用 (Mutable Borrow)**: 書き込み可能だが同時に 1 つに制限される借用。`BorrowMut<T>` は `effect {mut}` と `rc` の両方を要求し、診断で競合検出の対象となる。[3-2 Core Collections](3-2-core-collections.md#32-cellt-ref), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#diagnostic-ffi-contract)
- **借用検査 (Borrow Checker)**: 所有権と借用の一貫性を静的に検証する仕組み。FFI と Runtime Bridge では `ffi.contract.*` / `bridge.ownership.*` 診断を通じて違反を可視化し、`Ownership::Borrowed` と `BorrowMut<T>` の競合を監査ログへ記録する。[3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#diagnostic-ffi-contract), [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界)
- **ライフタイム (Lifetime `'a`)**: 参照や借用が有効な区間を明示する注釈。FFI ポインタは `'a` に相当するライフタイムを `Ownership::Borrowed` で表現し、恒常的に保持する場合は `Transferred` と `release` ハンドラで延長する。[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界)
- **所有権移動 (Move Semantics)**: 所有権を別の束縛へ移し、移動元を未初期化状態にする操作。`Ownership::Transferred` や `Result` の戻り値が移動後の責務を明示し、Capability 監査は解放漏れや二重解放を検知する。[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#diagnostic-bridge)
- **非安全ブロック (unsafe block)**: `unsafe { … }` 内で行う未検証操作。効果タグ `unsafe` と連携し、`Core.Unsafe` 章で列挙された API と監査ポリシーに従って使用する。[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#22-効果タグと-unsafe-境界), [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#4-2-監査された-unsafe-操作)
- **解放責務 (Drop)**: 所有権を保持する主体がリソース解放処理を実行する責務。FFI では `release` ハンドラを登録して `Ownership::Transferred` の終端を保証し、Runtime Bridge は `bridge.ownership.*` 監査キーで解放結果を報告する。[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#diagnostic-bridge)

## 診断と監査
- **`DiagnosticDomain`**: 診断メッセージを構文/型/ターゲットなどの領域別に分類する列挙型。語彙は `Syntax` / `Parser` / `Type` / `Effect` / `Runtime` / `Config` / `Manifest` / `Target` / `Security` / `Plugin` / `Cli` / `Lsp` / `Other(Str)` を基本とし、`Other` には `snake_case` の補助識別子を併記する。[3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#12-診断ドメイン-diagnosticdomain) で正式定義され、CLI や LSP のフィルタリングに利用される。Phase 2-5 DIAG-003 Step5 で仕様・ガイド・監査ログの語彙整合が完了している。
- **`AuditEnvelope`**: 診断に付随する監査情報（`audit_id`、`change_set`、Capability との紐付けなど）を保持する構造体。[3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#11-auditenvelope) を参照。
- **`AuditSink`**: 監査ログの出力先を抽象化した関数型で、CLI/LSP/リモート送信など複数のシンクを統一インターフェースで扱う。[3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#3-監査ログ出力) が API を示す。
- **`Stage` (Experimental/Beta/Stable)**: 診断・Capability・効果拡張がどの安定段階にあるかを記録する列挙。未成熟機能の扱いをツール側が調整するため、[3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#13-効果診断拡張-effects) と [3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) で共有される。

## 非同期実行と FFI
- **`Future<T>` / `Poll<T>`**: 非同期計算を表すコア抽象で、ポーリングによって `Ready` か `Pending` を返す。Reml の `Core.Async` は [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#1-coreasync-の枠組み) で型と挙動を規定する。
- **`SchedulerHandle` / `Task`**: 非同期ランタイムのスケジューラを指すハンドルと、そこで実行されるジョブのラッパ。Capability Registry から取得し、[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#1-coreasync-の枠組み) で使用法が示される。
- **バックプレッシャー (Backpressure)**: チャネルやストリームで過剰なデータを抑制する制御ポリシー。`BackpressurePolicy` と `OverflowPolicy` の設定は DSL オーケストレーションで重要となり、[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#14-dslオーケストレーション支援-api) に種類と制約が定義される。
- **`ExecutionPlan`**: `conductor` DSL の実行戦略・スケジューリング・エラー伝播方針をまとめた構造体。[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#14-dslオーケストレーション支援-api) がフィールドと整合チェックを説明する。
- **`Codec`**: DSL 間通信で使うシリアライズ/デシリアライズ契約。`encode`/`decode`/`validate` を持ち、監査や互換性チェックに利用される。[3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#141-codec-契約) を参照。
- **`RetryPolicy`**: `retry` コンビネータが失敗時の再試行回数・バックオフ戦略を管理する設定。`BackoffStrategy` と組み合わせて [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md#12-高度な非同期パターン) に定義される。

## エラー処理と診断システム
- **Diagnostic**: エラーや警告を構造化して保持する報告単位。位置情報、期待集合、FixIt 提案、監査メタデータを含む。[2-5 エラーハンドリング](2-5-error.md) と [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) で詳細が定義される。
- **ParseError**: パーサが失敗時に生成する集約データ。最遠位置、期待集合、文脈情報、コミット状態を含む「素の事実」として扱われ、`Diagnostic` への変換により表示用データを生成する。[2-5 エラーハンドリング](2-5-error.md) を参照。
- **期待集合 (Expected Set)**: エラー発生時に「何が来るはずだったか」を報告するためのシンボル集合。具体トークン・キーワード・ルール名・文字クラスを優先順位付きで保持し、診断メッセージ生成に利用される。[2-5 エラーハンドリング](2-5-error.md) を参照。
- **FixIt**: IDE 用の「その場で直せる」提案。`Insert`（挿入）、`Replace`（置換）、`Delete`（削除）の種類があり、LSP と連携して自動修正候補を提示する。[2-5 エラーハンドリング](2-5-error.md) で詳細が説明される。
- **Severity / SeverityHint**: 診断の重要度（Error/Warning/Note）と推奨アクション（Rollback/Retry/Ignore/Escalate）を示すメタデータ。運用環境での自動対応方針の決定に利用される。[2-5 エラーハンドリング](2-5-error.md) を参照。
- **最遠位置原則 (Farthest-First)**: パーサエラーの合成時に、より遠い失敗位置を採用し、同位置ならコミット状態を優先、それでも同列なら期待集合を和集合する規則。高品質なエラー報告の基盤となる。[2-5 エラーハンドリング](2-5-error.md) B-2 節を参照。

## 型システムと推論関連
- **値制限 (Value Restriction)**: `let` 束縛の右辺が純粋な式の場合だけ型一般化を許す規則。効果を含む式は単相にとどめて安全性を確保する。[1-2 型システムと推論](1-2-types-Inference.md) C.3 を参照。
- **双方向型付け (Bidirectional Typing)**: 明示注釈がある場合に推論と検査を往復させ、エラー位置の精度を高める戦略。[1-2 型システムと推論](1-2-types-Inference.md) C.4 で推奨されている。
- **一般化 (Generalization)**: トップレベル `let` 束縛などで型変数を全称量化子で束縛し、多相型スキームを生成する処理。ランク1多相の制約下で適用タイミングが制御される。[1-2 型システムと推論](1-2-types-Inference.md) を参照。
- **具体化 (Instantiation)**: 型スキームから新鮮な型変数を生成し、呼び出し時に具体的な型を割り当てる処理。Hindley-Milner 推論の核心となる操作。[1-2 型システムと推論](1-2-types-Inference.md) を参照。
- **合一 (Unification)**: 2つの型を等しくするための代入を計算するアルゴリズム。Algorithm W の中核として型推論エラーの発生源を特定する。[1-2 型システムと推論](1-2-types-Inference.md) を参照。

## 実行時システムと Capability
- **Capability Registry**: GC や IO などランタイム機能を Capability として登録・照会する中心レジストリ。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) が API と責務を定義する。
- **Capability Handle**: 各 Capability 実装を表す不透明ハンドル。`CapabilityHandle::Io` などのバリアントで分岐し、Registry 経由で取得する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#11-capabilityhandle-のバリアント) を参照。
- **Capability Stage**: `Experimental/Beta/Stable` の成熟度を示すメタデータ。`verify_capability_stage` で下限を検証し、`AuditEnvelope.metadata` や `Diagnostic.extensions["effects"]` に要求/実際の Stage を記録する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#capability-stage-contract) と [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md#24-stage-差分プリセット-effectdiagnostic) を参照。
- **StreamSignal**: ストリーミングランナーが Runtime Bridge へ送る状態通知オブジェクト。`kind`（Pending/Resume/Error/Completed）・`demand_hint`・`backpressure`・`stage_required`/`stage_actual` を含み、`RuntimeBridgeHandle::stream_signal` を通じて Stage 監査 (`bridge.stage.backpressure`) と `effects.contract.stage_mismatch` の整合チェックに利用する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#105-ストリーミング-signal-ハンドラ)、[docs/guides/runtime/runtime-bridges.md](../guides/runtime/runtime-bridges.md#105-ストリーミング-flow-signal-と-runtime-bridge-連携) を参照。
- **SecurityCapability**: Capability の署名検証、許可、隔離レベルを管理するセキュリティ用ハンドル。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#12-セキュリティモデル) が構造体と検証手順を示す。
- **RuntimeCapability / TargetCapability**: 実行環境が備える命令セットやクロックなどの機能一覧。CI や `Core.Env` が環境適合性を確認するために利用し、[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#13-プラットフォーム情報と能力) に列挙がある。
- **SandboxProfile**: Capability 利用時に課すリソース制限を記述する共通プロファイル。`SecurityCapability` と連携して監査方針を適用する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md#12-セキュリティモデル) 参照。

## Unicode とテキスト処理
- **Unicode 3層モデル (Byte / Char / Grapheme)**: Reml はバイト列・Unicode スカラー値・拡張書記素クラスタの 3 レイヤで文字を扱い、API ごとに適切な粒度を選択する。[1-4 Unicode 文字モデル](1-4-test-unicode-model.md) 参照。
- **Unicode スカラー値 (コードポイント)**: UTF-8 で表現される単一の Unicode スカラー。`Char` 型が対応し、位置情報や比較の基本単位となる。[1-4 Unicode 文字モデル](1-4-test-unicode-model.md) 参照。
- **拡張書記素クラスタ (Extended Grapheme Cluster)**: ユーザーが 1 文字と認識する複合文字。列数算出や `column` 情報はこの単位で計測する。[1-4 Unicode 文字モデル](1-4-test-unicode-model.md) を参照。
- **Unicode 正規化 (NFC/NFD/NFKC/NFKD)**: 等価な文字列表現を統一する正規化形式。仕様では対応必須の正規化セットとして [0-1 プロジェクト目的](0-1-project-purpose.md#31-unicode対応の充実) と [3-3 Core Text & Unicode](3-3-core-text-unicode.md) に記載がある。
- **XID_Start / XID_Continue**: Unicode 標準が定義する識別子開始/継続文字カテゴリ。識別子の構文規則として [1-1 構文](1-1-syntax.md#a3-識別子とキーワード) で採用されている。
- **GraphemeIndex / CpIndex**: `Input` が保持する書記素・コードポイント境界キャッシュ。高速な位置計算やバックトラックに利用され、[2-1 パーサ型](2-1-parser-type.md#b-入力モデル-input) で説明される。

## 標準ライブラリと反復子
- **`Iter<T>`**: 遅延評価される単方向列。不変データ構造と親和性が高く、`|>` パイプと組み合わせた宣言的データフローを実現する。[3-1 Core Prelude & Iteration](3-1-core-prelude-iteration.md) で詳細が定義される。
- **`Collector<T, C>`**: `Iter` の終端操作で利用するビルダインターフェイス。`Vec`/`Set`/`Map` 等の収集先を抽象化し、`with_capacity` や `reserve` でメモリ効率を制御する。[3-1 Core Prelude & Iteration](3-1-core-prelude-iteration.md) を参照。
- **遅延評価 (Lazy Evaluation)**: `Iter` チェーンが終端操作まで評価されない仕組み。メモリ効率と必要時計算を両立し、大量データ処理で威力を発揮する。[3-1 Core Prelude & Iteration](3-1-core-prelude-iteration.md) を参照。
- **短絡型 (Try Types)**: `Result<T, E>` や `Option<T>` など、`?` 演算子による早期リターンをサポートする型。`Core.Prelude` が定義する `Try` トレイトにより実装される。[3-1 Core Prelude & Iteration](3-1-core-prelude-iteration.md) を参照。

## 標準ライブラリ支援モジュール
- **Core.Test**: DSL のスナップショット/ゴールデン/ファジングを標準化するテスト基盤。`TestError` と監査イベントで失敗理由を集約する。[3-11 Core Test](3-11-core-test.md) を参照。
- **Core.Cli**: DSL 用 CLI を宣言的に構築するモジュール。`CliSpec` と `CliValues` を分離し、診断出力を統一する。[3-12 Core Cli](3-12-core-cli.md) を参照。
- **Core.Text.Pretty**: プリティプリンタの `Doc` コンビネータとレイアウト規則を提供し、フォーマッタやコード生成の基盤とする。[3-13 Core Text Pretty](3-13-core-text-pretty.md) を参照。
- **Core.Lsp**: LSP の基本型と JSON-RPC ヘルパを提供し、DSL 作者が最小実装を構築できるようにする。[3-14 Core Lsp](3-14-core-lsp.md) を参照。
- **Core.Doc**: ドキュメントコメント抽出とレンダリング、Doctest 実行の標準 API を提供する。[3-15 Core Doc](3-15-core-doc.md) を参照。

## ネットワーク
- **Core.Net**: HTTP/TCP/UDP/URL の最小 API と `effect {net}` を定義する標準ライブラリモジュール。[3-17 Core Net](3-17-core-net.md) を参照。
- **Url / UrlParts**: URL の正規化済み構造とビルド用の構造体。`scheme`/`authority`/`path` の不変条件を保証する。[3-17 Core Net](3-17-core-net.md) を参照。
- **NetError / HttpError / UrlError**: ネットワーク、HTTP、URL 解析の失敗を表すエラー型。`diagnostic_key` と `Result` で失敗経路を明確化する。[3-17 Core Net](3-17-core-net.md) を参照。
- **TcpStream / TcpListener**: TCP の接続ストリームとリスナー。`Core.IO.Reader/Writer` と互換の読み書き契約を持つ。[3-17 Core Net](3-17-core-net.md) を参照。
- **UdpSocket**: UDP の送受信ソケットと `Datagram` 形式の受信バッファ。[3-17 Core Net](3-17-core-net.md) を参照。

## DSL・エコシステム・ツール
- **DSL (Domain-Specific Language)**: `Core.Parse` を使って特定領域向けの言語を宣言的に構築するアプローチ。プロジェクト全体が DSL ファーストを掲げ、[0-1 プロジェクト目的](0-1-project-purpose.md#32-エコシステム統合とdslファーストアプローチ) に背景がまとめられる。
- **Conductor パターン**: 複数の DSL を組み合わせてパイプライン化するための構文で、`conductor` ブロックとして宣言する。[1-1 構文](1-1-syntax.md#b11-dslエントリーポイント宣言) と [guides/conductor-pattern.md](../guides/dsl/conductor-pattern.md) に運用指針がある。
- **`@dsl_export` 属性**: DSL を外部に公開するエントリを示し、カテゴリや必要 Capability、許容効果をメタデータとして付与する注釈。[1-1 構文](1-1-syntax.md#b11-dslエントリーポイント宣言) で要件が規定される。
- **`RunConfig.extensions`**: パーサ実行時に LSP やランタイム設定などモジュール固有のオプションを渡すための連想配列。[2-1 パーサ型](2-1-parser-type.md#d-実行設定-runconfig-とメモ) に既定ネームスペースが整理される。
- **`remlc`**: Reml コンパイラ CLI。ターゲットトリプル指定やツールチェーン取得のコマンドライン例が [README](README.md#ビルド--ターゲット指定例) に記載される。
- **`@cfg` 条件分岐**: ターゲットや Capability に応じてコードを条件コンパイルする属性。`RunConfig.extensions["target"]` や CI 環境変数と連携する手順が [README](README.md#ビルド--ターゲット指定例) と [3-10 Core Env](3-10-core-env.md) で説明される。

## パフォーマンスと最適化
- **Packrat パース**: 入力位置とパーサ ID をキーとするメモ化でバックトラックを高速化する戦略。[2-6 実行戦略](2-6-execution-strategy.md) がメモテーブルの利用方針を示す。
- **左再帰サポート**: Packrat と組み合わせた seed-growing だが、仕様として左再帰文法の直接記述は想定しない。`precedence` / `chainl1` への変換を前提とし、`left_recursion` は無限再帰の安全弁として扱う。[2-6 実行戦略](2-6-execution-strategy.md) を参照。
- **トランポリン (Trampoline)**: 再帰的なパーサ合成をループに変換し、末尾再帰のスタック消費を抑えるテクニック。[2-6 実行戦略](2-6-execution-strategy.md) に最適化理由が記載される。
- **seed-growing**: 左再帰パーサで用いる最適化手法。初期値から段階的に結果を成長させ、不動点に到達したら終了する。メモ化と組み合わせて左再帰混入時の安全弁として動作する。[2-6 実行戦略](2-6-execution-strategy.md) を参照。
- **メモ化 (Memoization)**: パーサ結果をキャッシュして同一入力位置での再計算を避ける最適化。Packrat パーシングの基盤技術として用いられる。[2-6 実行戦略](2-6-execution-strategy.md) を参照。

## セキュリティと安全性
- **安全境界 (Safety Boundary)**: `unsafe` ブロックや FFI 呼び出しなど、未定義動作を引き起こし得る操作を明示的に囲む境界。内部で発生した効果はブロック全体に付与される。[1-3 効果と安全性](1-3-effects-safety.md) を参照。
- **Bidi 制御文字攻撃**: Unicode の双方向制御文字を悪用したコード偽装攻撃。識別子内への混入を検出し、`E6001` エラーとして報告する。[2-5 エラーハンドリング](2-5-error.md) H 節を参照。
- **confusable 文字**: 見た目が類似した異なる Unicode 文字による混同攻撃。`W6101` 警告として検出し、正規文字への置換を提案する。[2-5 エラーハンドリング](2-5-error.md) H 節を参照。
- **サンドボックス (Sandbox)**: プラグインや外部コードの実行を制限された環境で行う仕組み。CPU・メモリ・ネットワークアクセスを制御し、セキュリティを確保する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) 6.1 節を参照。
- **署名検証 (Signature Verification)**: Capability やプラグインの真正性を暗号学的署名で検証する仕組み。改ざんや偽装を防ぎ、信頼できる実行環境を提供する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) を参照。

## 構文と解析技術
- **空成功 (Empty Success)**: 入力を消費せずに成功するパーサの状態。`many` などの繰り返しコンビネータで無限ループを引き起こすため、ライブラリが検出してエラーを生成する。[2-2 コア・コンビネータ](2-2-core-combinator.md) A-4 節を参照。
- **消費と非消費 (Consumed / Unconsumed)**: パーサが入力位置を進めたかどうかの状態。消費した場合は失敗時にバックトラックしない。[2-1 パーサ型](2-1-parser-type.md) を参照。
- **先読み (Lookahead)**: 入力を消費せずに成功可否を判定する技法。分岐予告や曖昧性解消に利用され、`lookahead` コンビネータで実現される。[2-2 コア・コンビネータ](2-2-core-combinator.md) A-6 節を参照。
- **演算子優先度 (Operator Precedence)**: 式解析で演算子の結合順序を制御する仕組み。宣言的な優先度テーブルで左/右結合や非結合を指定できる。[2-4 演算子優先度ビルダー](2-4-op-builder.md) を参照。
- **非結合演算子 (Non-associative Operator)**: `a < b < c` のような連鎖を禁止する演算子。連鎖時は専用エラー `E2001` を生成し、`(a < b) && (b < c)` の置換を提案する。[2-5 エラーハンドリング](2-5-error.md) D-2 節を参照。
- **Unicode 識別子プロファイル (Unicode Identifier Profile)**: `IdentifierProfile` が示す UAX #31 ベースの識別子許容集合。既定の `DefaultId` は `unicode` プロファイルで、`RunConfig.extensions["lex"].identifier_profile` から切り替えられる。後方互換が必要な場合は `ascii-compat` を指定して Phase 1 系ツールと同じ ASCII 限定挙動を復元する。詳細は [2-3-lexer.md](2-3-lexer.md#d-1-プロファイル) と [1-1-syntax.md](1-1-syntax.md#a3-識別子とキーワード) を参照。`lexer.identifier_profile_unicode` KPI は CI で Unicode プロファイル運用率を監視する。

## プラグインと拡張性
- **プラグイン (Plugin)**: Reml の機能を動的に拡張するモジュール。Capability システムと統合され、セキュリティポリシーの下で安全に実行される。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) 6 節を参照。
- **プラグインメタデータ (Plugin Metadata)**: プラグインの ID、バージョン、必要 Capability、署名などの情報。登録時に検証され、互換性チェックに利用される。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) を参照。
- **拡張フック (Extension Hook)**: `RunConfig.extensions` を通じて提供されるプラグイン連携ポイント。LSP 設定やランタイムオプションを渡すための仕組み。[2-1 パーサ型](2-1-parser-type.md) D 節を参照。
- **互換性検証 (Compatibility Verification)**: プラグインや DSL の依存関係と効果契約をチェックし、安全な組み合わせかを判定する仕組み。`DslCompatibilityReport` で結果を報告する。[3-8 Core Runtime & Capability](3-8-core-runtime-capability.md) 7 節を参照。

## データモデルと型安全性
- **代数的データ型 (ADT)**: `type Expr = | Int | Add` のようなバリアント型。コンストラクタは関数として型付けされ、パターンマッチの基盤となる。[1-2 型システムと推論](1-2-types-Inference.md) A.2 に基本形が示される。
- **ニュータイプ (Newtype)**: 既存型へ零コストで別名を与える `type Name = new T` 構文。暗黙変換を避けつつ静的な区別を付けられる。[1-2 型システムと推論](1-2-types-Inference.md) A.4 参照。
- **パターン網羅性 (Pattern Exhaustiveness)**: `match` 式でのバリアント網羅チェック。欠落パターンは警告やエラーとして報告され、FixIt で補完案を提示する。[2-5 エラーハンドリング](2-5-error.md) J-3 節を参照。
- **構造的型システム (Structural Type System)**: 名前ではなく構造で型の互換性を判定する方式。レコード型やトレイト実装で部分的に採用される。[1-2 型システムと推論](1-2-types-Inference.md) を参照。
- **型エイリアス (Type Alias)**: `type alias Bytes = [u8]` のような既存型への別名定義。型安全性は維持せず、純粋に記述の簡略化を目的とする。[1-2 型システムと推論](1-2-types-Inference.md) を参照。
