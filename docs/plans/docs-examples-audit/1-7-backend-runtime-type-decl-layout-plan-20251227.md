# 1.7 Backend/IR 型宣言レイアウト影響整理計画（2025-12-27）

`type` 宣言（alias/newtype/合成型）の実体化に伴い、Backend/IR と Runtime の型表現・レイアウト影響を整理し、`docs-examples-audit` の検証対象と齟齬が出ないように整合計画を立てる。

## 目的
- alias/newtype/合成型の IR 取り回し方針（展開/名義保持/タグ付け）を明確化する。
- Backend/Runtime のレイアウト影響を把握し、必要な整合チェックを `docs-examples-audit` に反映する。
- 仕様・実装・サンプルコードの整合性を維持するための検証手順を用意する。

## 対象範囲
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`
- Frontend: `compiler/frontend`
- Backend: `compiler/backend/llvm`
- Runtime: `compiler/runtime/native`
- サンプル: `examples/docs-examples/spec/`

## 前提・現状
- Backend の `RemlType` は最小構成であり、alias/newtype の名義情報は保持されない。
- sum 型（合成型）は `RemlType::Adt` が存在するが、`parse_reml_type` は生成しない。
- `docs-examples-audit` の `.reml` は Frontend 検証が主目的で、Backend/Runtime のレイアウト検証は限定的。

## 実行計画

### フェーズ 0: 現状の棚卸し
- Frontend の型宣言 AST/型環境の現状を確認する。
  - [ ] `compiler/frontend` の型宣言 AST ノードを列挙し、alias/newtype/sum の表現差分を表にまとめる
  - [ ] 型環境に格納される型定義のキー（型名/モジュールパス/スコープ）と参照箇所を整理する
- MIR/JSON へ型情報がどこまで渡るかを確認する。
  - [ ] MIR 生成時に `type` 宣言がどの構造体へ載るかをトレースする
  - [ ] JSON 出力に型名・展開後型・名義型 ID が含まれるかを確認し、欠けている項目を一覧化する
- Backend の `RemlType` / `type_mapping` が受け取れる型表現の範囲を整理する。
  - [ ] `compiler/backend/llvm/src/type_mapping.rs` の `RemlType` 変種と対応する IR 型を表に整理する
  - [ ] `parse_reml_type` の入力 JSON 仕様と、想定外ケースの扱いを確認する
- Runtime に newtype / 合成型のレイアウト前提があるか確認する。
  - [ ] `compiler/runtime/native` 内の型タグ定義や ABI 前提のコメント/ドキュメントを確認する
  - [ ] newtype を識別する必要の有無を判断するため、ランタイム API 参照箇所を洗い出す
- `examples/docs-examples/spec/` から `type` 宣言を含む `.reml` を抽出し、alias/newtype/sum の内訳を整理する。
  - [ ] `examples/docs-examples/spec/` の `type` 宣言を列挙し、alias/newtype/sum の件数と代表例を記録する
  - [ ] 仕様書（`docs/spec/1-1-syntax.md` など）のサンプルと対応付ける

#### フェーズ 0 調査メモ
- Frontend AST: `compiler/frontend/src/parser/ast.rs` の `TypeDecl` / `TypeDeclBody` が alias/newtype/sum を保持し、`TypeDeclVariantPayload` で record/tuple を表現する。`DeclKind::Type` で `type` 宣言を扱う。
- Parser: `compiler/frontend/src/parser/mod.rs` で `type alias` と `type <name> = new`、`type <name> = ... | ...` を構文解析する。
- 型環境: `compiler/frontend/src/typeck/env.rs` の `TypeDeclBinding` が `name/generics/kind/body/span` を保持し、`TypeEnv` は `IndexMap<String, TypeDeclBinding>` を名前キーで管理する（スコープは `enter_scope` による親チェーン）。
- typeck 登録: `compiler/frontend/src/typeck/driver.rs` の `register_type_decls` が `TypeDeclBody` から `TypeDeclKind` を決定し、sum 型は `TypeConstructorBinding` として variant 名を別管理する。
- MIR/JSON: `compiler/frontend/src/semantics/typed.rs` / `compiler/frontend/src/semantics/mir.rs` に型宣言の保持はなく、`typeck/typed-ast.rust.json` と `typeck/mir.rust.json` には型宣言が出力されない。一方で `parse/ast.rust.json` は AST 由来で `TypeDecl` を含む。
- Backend: `compiler/backend/llvm/src/type_mapping.rs` の `RemlType` は alias/newtype を持たず、`compiler/backend/llvm/src/integration.rs` の `parse_reml_type` はプリミティブ/参照/スライス/Set/文字列のみ対応（未知は `Pointer` にフォールバック）。`RemlType::Adt` はあるがパース経路がない。
- Runtime: `compiler/runtime/native/include/reml_runtime.h` に `REML_TAG_ADT` はあるが newtype 固有タグはなく、現状は型タグ側の前提が最小限。
- examples 内訳（簡易集計, `examples/docs-examples/spec/` 全 130 ブロック）: alias/opaque 99、sum 29、`type alias` 1、newtype 1。newtype は `examples/docs-examples/spec/1-1-syntax/sec_b_4-c.reml` の `type UserId = new { value: i64 }`、sum は `examples/docs-examples/spec/1-2-types-Inference/sec_a_2.reml` / `examples/docs-examples/spec/2-2-core-combinator/sec_c_1-a.reml` などに分布。

### フェーズ 1: IR 方針の決定（alias/newtype/sum）
- alias は **展開後の型を IR に渡す** 方針を既定とする（レイアウト影響なし）。
- newtype は **IR では内側の型へマップ**し、**名義情報はデバッグ/診断メタデータ**として保持する方針を検討する。
- sum 型は **`RemlType::Adt` に落とす**方針を採用し、タグ幅と payload の計算ルールを定義する。
  - [x] alias の展開タイミングを **typeck の識別子解決時**とする（MIR/Backend に別名を残さず、IR での再展開や揺れを避けるため）。
  - [x] newtype の名義情報は **Frontend/JSON メタデータ**で保持し、Backend のレイアウト計算には渡さない。
    - メタデータ項目: `newtype_name`, `module_path`, `type_args`, `underlying_ty`, `decl_span`
  - [x] sum 型のタグ幅は `ceil(log2(variants))` とし、**0 バリアントは不可能型（タグ 0 ビット）**、**1 バリアントはタグ省略**で payload のみを保持する。
  - [x] IR レイアウトが変わるケース（newtype の ABI 差分有無）を列挙し、影響範囲を Backend/Runtime に分類する。
    - Backend 影響: 既定は **なし**（内側の型へ直マップ）。例外は `repr`/ABI 指定や FFI 境界で名義型 ID が必要な場合。
    - Runtime 影響: 既定は **なし**（同一レイアウト）。例外は動的型 ID/シリアライズで名義型を保持する必要が出た場合。

### フェーズ 2: Backend/Runtime 側の整合ポイント整理
- Backend の型マッピングが alias/newtype/sum を受け取れる前提を整理する。
- newtype が Runtime で識別可能である必要があるか確認する（基本は同一レイアウト）。
- 合成型の payload/タグの配置ルールが既存の `TypeMappingContext` と矛盾しないか確認する。
  - [x] `RemlType` へ alias/newtype/sum を渡す経路を洗い出し、必要な JSON フィールドを列挙する
    - 経路: `compiler/backend/llvm/src/integration.rs` の `MirFunctionJson.params` / `return` / `ffi_calls.args` / `ffi_calls.return` が `parse_reml_type` で `RemlType` へ変換される。`MirExprJson.ty` は型トークン文字列だが `parse_reml_type` の入力には未使用。
    - 現行 JSON は文字列トークンのみで、`RemlType::Adt` は生成されない。alias/newtype は Frontend 側で展開済みを前提とし、Backend には渡さない想定。
    - sum 型を渡すには構造化 JSON が必要（例: `{"kind":"adt","tag_bits":2,"variants":[{"kind":"tuple","items":["i64"]}, "unit"]}`）。最低限は `kind/tag_bits/variants` を想定し、型名や `module_path` はデバッグ用途で任意。
  - [x] `TypeMappingContext::layout_of` と sum 型の tag/payload 仕様を整合させ、既存の record/layout ルールとの共通化方針を決める
    - `TypeMappingContext::layout_of` の `RemlType::Adt` は `payload=max(variants)` + `tag_size=ceil(tag_bits/8)` の単純合算で、`align=8` 固定。タグ位置は payload の後ろに付与される想定。
    - `RemlType::RowTuple` はフィールドごとのアラインメントでサイズ計算されるが、record は Runtime 側で `reml_record_t` 配列 ABI を使うため `layout_of` の対象外。
    - 既存ルールとの共通化方針: sum 型は「payload 最大 + tag 末尾」の軽量レイアウトで固定し、record/tuple は別系統（heap オブジェクト）として扱う。alignment を 8 固定のままにするかは、`variants` の最大アラインメントが 8 を超える型が導入された時点で再評価する。
  - [x] Runtime の ABI 影響がある場合は別計画へ切り出し、切り出し条件と担当範囲を明文化する
    - `compiler/runtime/native/include/reml_runtime.h` では `REML_TAG_ADT` が定義済みだが newtype 固有タグは存在しない。現状は newtype を内側型と同一レイアウトで扱う前提に一致する。
    - 別計画に切り出す条件: FFI 境界で名義型 ID が必要になる、もしくはランタイムの動的型検証/シリアライズで newtype/alias を区別する必要が出た場合。
    - 担当範囲: Runtime 側の型タグ拡張・メタデータ格納は `compiler/runtime/native`、Backend 側の型タグ埋め込みは `compiler/backend/llvm`、Frontend 側のメタデータ保持は `compiler/frontend` に切り分ける。

#### フェーズ 2 整理メモ
- Backend は MIR JSON の型トークン文字列のみを `RemlType` に変換するため、sum 型のタグ/variant 情報は JSON で構造化しない限り伝播できない。
- Runtime は ADT タグを持つが newtype 用の識別子はなく、現時点で ABI 変更は不要。

### フェーズ 3: docs-examples-audit の整合チェック
- 影響が出る `.reml` を `docs-examples-audit` の検証対象としてマークする。
- IR 形状やレイアウトが変わる場合は、検証手順・期待値を追記する。
  - [x] alias/newtype/sum の `.reml` を一覧化し、検証優先度（高/中/低）と理由を付与する
  - [x] 変更が必要な場合は `reports/spec-audit/summary.md` に起票メモを残し、追跡 ID を付ける
  - [x] 代表ケースの `.reml` を追加または更新する（必要時）うえで、検証観点（型名/展開/タグ）を明記する

#### フェーズ 3 整合チェック対象一覧
| 優先度 | .reml パス | 種別 | 型宣言（種別） | 理由 |
| --- | --- | --- | --- | --- |
| 高 | examples/docs-examples/spec/1-1-syntax/sec_b_4-c.reml | alias, newtype, sum | Expr(sum), Bytes(alias), UserId(newtype) | コア仕様の newtype/ADT で IR レイアウト影響が出るため |
| 高 | examples/docs-examples/spec/1-1-syntax/sec_b_8_5.reml | alias, sum | ConductorCapabilityRequirement(alias), StageRequirement(sum) | コア仕様の newtype/ADT で IR レイアウト影響が出るため |
| 高 | examples/docs-examples/spec/1-1-syntax/sec_e_2.reml | sum | Option(sum) | コア仕様の newtype/ADT で IR レイアウト影響が出るため |
| 高 | examples/docs-examples/spec/1-1-syntax/sec_g.reml | sum | Expr(sum) | コア仕様の newtype/ADT で IR レイアウト影響が出るため |
| 高 | examples/docs-examples/spec/1-2-types-Inference/sec_a_2.reml | sum | Option(sum), Result(sum) | コア仕様の newtype/ADT で IR レイアウト影響が出るため |
| 低 | examples/docs-examples/spec/1-2-types-Inference/sec_c_8.reml | alias | RunConfigTarget(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/1-2-types-Inference/sec_f.reml | alias | Parser(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/1-2-types-Inference/sec_g.reml | alias | DslCapabilityRequirement(alias), DslStageBounds(alias), DslExportSignature(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/1-4-test-unicode-model/sec_i.reml | alias | UnicodeApi(alias), Collator(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-1-parser-type/sec_a.reml | alias, sum | Parser(alias), Reply(sum), ParseResult(alias), State(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-1-parser-type/sec_b.reml | alias | Input(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-1-parser-type/sec_c.reml | alias | Span(alias), SpanTrace(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-1-parser-type/sec_d.reml | alias | RunConfig(alias), RunConfigExtensions(alias), ParserId(alias), MemoKey(alias), MemoVal(alias), MemoTable(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-1-parser-type/sec_f.reml | alias | ParseError(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-1-parser-type/sec_g.reml | alias | ParseResultWithRest(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-2-core-combinator/sec_b_2.reml | alias, sum | AutoWhitespaceConfig(alias), AutoWhitespaceStrategy(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-2-core-combinator/sec_b_3.reml | alias | ParserProfile(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-2-core-combinator/sec_c_1-a.reml | alias, sum | CstNode(alias), CstChild(sum), Trivia(alias), TriviaKind(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 中 | examples/docs-examples/spec/2-2-core-combinator/sec_c_1-b.reml | alias, sum | ExprOpLevel(alias), ExprBuilderConfig(alias), ExprCommit(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-2-core-combinator/sec_c_2.reml | alias | CstOutput(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-2-core-combinator/sec_c_5.reml | alias, sum | EmbeddedDslSpec(alias), EmbeddedMode(sum), ContextBridge(sum), ContextBridgePayload(alias), ContextBridgeHandler(alias), EmbeddedNode(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 中 | examples/docs-examples/spec/2-2-core-combinator/sec_g_1.reml | alias, sum | ParserMetaKind(sum), ParserMeta(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-3-lexer/sec_d_1.reml | alias | IdentifierProfile(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-3-lexer/sec_e_1.reml | alias | NumericOverflow(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-3-lexer/sec_g_1.reml | alias | ConfigTriviaProfile(alias), CommentPair(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-3-lexer/sec_h_2.reml | alias | LayoutProfile(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-4-op-builder/sec_a_2.reml | alias | Ternary(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-5-error/sec_a.reml | alias, sum | Severity(sum), SeverityHint(sum), ErrorDomain(sum), Expectation(sum), FixIt(sum), Diagnostic(alias), ExpectationSummary(alias), ParseError(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-5-error/sec_c.reml | alias | PrettyOptions(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-6-execution-strategy/sec_b_2.reml | alias | RunConfig(alias), RunConfigExtensions(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-6-execution-strategy/sec_b_2_1.reml | alias | RunConfigTarget(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-6-execution-strategy/sec_b_2_1_a.reml | alias | RunArtifactMetadata(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-6-execution-strategy/sec_c_1.reml | alias | ParserId(alias), MemoKey(alias), MemoVal(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-6-execution-strategy/sec_e_2.reml | sum | TraceEvent(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 中 | examples/docs-examples/spec/2-6-execution-strategy/sec_f.reml | alias, sum | RegexRunConfig(alias), RegexEngineMode(sum), RegexMemoPolicy(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 中 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_1-b.reml | sum | StreamOutcome(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_2.reml | alias | StreamingConfig(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_b_1.reml | alias, sum | DemandHint(alias), FeederYield(sum), Chunk(alias), Await(alias), Closed(alias), FeederError(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_b_2.reml | alias | StreamError(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_c_1.reml | alias | Continuation(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_d.reml | alias, sum | FlowController(alias), FlowPolicy(sum), Demand(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 中 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_e.reml | alias, sum | StreamDiagnosticHook(alias), StreamEvent(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_4.reml | alias | Error(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_4_2.reml | alias | Error(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-13-core-text-pretty/sec_2.reml | alias | CstPrinter(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/3-3-core-text-unicode/sec_10_1.reml | alias, sum | RegexOptions(sum), RegexRunMode(alias), RegexMatch(alias), RegexError(sum), UnicodeClassProfile(alias), UnicodeSet(alias) | ADT のタグ/ペイロード計算の確認が必要なため |
| 中 | examples/docs-examples/spec/3-3-core-text-unicode/sec_3_1.reml | alias, sum | UnicodeError(alias), UnicodeErrorKind(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/3-3-core-text-unicode/sec_4_1_1.reml | alias | IndexCache(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-3-core-text-unicode/sec_4_2.reml | alias | RegexHandle(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-3-core-text-unicode/sec_6.reml | alias | TextBuilder(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/3-3-core-text-unicode/sec_9.reml | sum | Token(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/3-5-core-io-path/sec_3_1.reml | alias | BufferedReader(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 中 | examples/docs-examples/spec/3-5-core-io-path/sec_4_4-a.reml | sum | WatchEvent(sum) | ADT のタグ/ペイロード計算の確認が必要なため |
| 低 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_11.reml | alias | CliDiagnosticEnvelope(alias), CliSummary(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_1_3.reml | alias | EffectsExtension(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_2.reml | alias | ParseDiagnosticOptions(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_3.reml | alias | DiagnosticCatalog(alias), DiagnosticTemplate(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_5.reml | alias | AsyncDiagnosticExtension(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_6.reml | alias | PreludeGuardExtension(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-7-core-config-data/sec_1_1.reml | alias | Manifest(alias), ProjectSection(alias), DslEntry(alias), DslExportRef(alias), BuildSection(alias), RegistrySection(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-7-core-config-data/sec_1_5.reml | alias | ConfigCompatibility(alias) | alias 展開のみでレイアウト影響が限定的なため |
| 低 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_1.reml | alias | Ptr(alias), MutPtr(alias), NonNullPtr(alias), VoidPtr(alias), FnPtr(alias), Span(alias), TaggedPtr(alias) | alias 展開のみでレイアウト影響が限定的なため |

#### フェーズ 3 代表ケース（追加不要・既存サンプルを流用）
- newtype/alias/sum 混在: `examples/docs-examples/spec/1-1-syntax/sec_b_4-c.reml`（`UserId`/`Bytes`/`Expr`）
- コア ADT（sum）: `examples/docs-examples/spec/1-1-syntax/sec_e_2.reml`（`Option`）、`examples/docs-examples/spec/1-2-types-Inference/sec_a_2.reml`（`Option`/`Result`）
- タグ幅/variant 境界: `examples/docs-examples/spec/1-1-syntax/sec_b_8_5.reml`（`StageRequirement`）

### フェーズ 4: 検証計画
- Frontend の型宣言実体化後に `.reml` の診断が 0 件であることを確認する。
- Backend に影響が出る場合は IR スナップショットで形状を確認する。
  - [x] `compiler/frontend` のテストで alias/newtype/sum を確認し、期待診断ゼロの条件を記録する
    - 既存テスト（alias/sum）: `compiler/frontend/tests/typeck_hindley_milner.rs` の `type_alias_generics_expands_without_violation` / `sum_type_constructor_resolves_in_expr` / `sum_type_record_payload_constructor_and_match` は `parse_module` で parser diagnostics 0 を前提とする。
    - newtype の専用テストは未整備のため、`examples/docs-examples/spec/1-1-syntax/sec_b_4-c.reml` の `reml_frontend --emit-diagnostics` を 0 件確認の基準とする。
    - 期待診断ゼロの条件: parser diagnostics 0 かつ typecheck violations 0（type alias 展開/ADT コンストラクタ/パターン解決が正常）。
  - [x] Backend へ合成型が降りる場合の IR 形状を確認し、スナップショット差分の受け入れ基準を定義する
    - 受け入れ対象: `MirFunctionJson.params/return/ffi_calls` の型トークンが `String` から構造化 `adt` へ置換される差分。
    - 受け入れ条件: `kind/tag_bits/variants` が `docs/plans/docs-examples-audit/1-7-backend-runtime-sum-mir-json-draft-20251227.md` の定義と一致し、非 ADT フィールド（関数名/式構造/本体 JSON）が不変であること。
    - 差分拒否条件: `tag_bits` の丸め規則が逸脱する、`payload_layout` が `inline`/`boxed` 以外になる、非 ADT の型トークンが構造化へ移行する。
    - スナップショットの採用単位は `docs-examples-audit` の高優先度 `.reml`（`sec_b_4-c.reml` / `sec_e_2.reml` / `sec_a_2.reml`）に限定する。

## 受け入れ基準
- alias/newtype/sum の IR 方針が文書化されている。
- Backend/Runtime のレイアウト影響の有無が明記されている。
- `docs-examples-audit` で対象 `.reml` の整合チェックが起票されている。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了

## 関連リンク
- `docs/plans/typeck-improvement/1-0-type-decl-realization-plan.md`
- `docs/plans/docs-examples-audit/1-7-frontend-mir-type-token-plan-20251227.md`
- `docs/plans/docs-examples-audit/1-7-backend-mir-type-json-plan-20251227.md`
- `docs/plans/docs-examples-audit/1-7-backend-runtime-sum-mir-json-draft-20251227.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-2-types-Inference.md`
- `docs/spec/1-3-effects-safety.md`
- `compiler/backend/llvm/src/type_mapping.rs`
