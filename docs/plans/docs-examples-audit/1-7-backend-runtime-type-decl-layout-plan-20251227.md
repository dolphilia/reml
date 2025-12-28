# 1.7 Backend/IR 型宣言レイアウト影響整理計画（2025-12-27）

`type` 宣言（alias/newtype/合成型）の実体化に伴い、Backend/IR と Runtime の型表現・レイアウト影響を整理し、`docs-examples-audit` の検証対象と齟齬が出ないように整合計画を立てる。

## 目的
- alias/newtype/合成型の IR 取り回し方針（展開/名義保持/タグ付け）を明確化する。
- Backend/Runtime のレイアウト影響を把握し、必要な整合チェックを `docs-examples-audit` に反映する。
- 仕様・実装・サンプルコードの整合性を維持するための検証手順を用意する。

## 対象範囲
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`
- Frontend: `compiler/rust/frontend`
- Backend: `compiler/rust/backend/llvm`
- Runtime: `runtime/native`
- サンプル: `examples/docs-examples/spec/`

## 前提・現状
- Backend の `RemlType` は最小構成であり、alias/newtype の名義情報は保持されない。
- sum 型（合成型）は `RemlType::Adt` が存在するが、`parse_reml_type` は生成しない。
- `docs-examples-audit` の `.reml` は Frontend 検証が主目的で、Backend/Runtime のレイアウト検証は限定的。

## 実行計画

### フェーズ 0: 現状の棚卸し
- Frontend の型宣言 AST/型環境の現状を確認する。
  - [ ] `compiler/rust/frontend` の型宣言 AST ノードを列挙し、alias/newtype/sum の表現差分を表にまとめる
  - [ ] 型環境に格納される型定義のキー（型名/モジュールパス/スコープ）と参照箇所を整理する
- MIR/JSON へ型情報がどこまで渡るかを確認する。
  - [ ] MIR 生成時に `type` 宣言がどの構造体へ載るかをトレースする
  - [ ] JSON 出力に型名・展開後型・名義型 ID が含まれるかを確認し、欠けている項目を一覧化する
- Backend の `RemlType` / `type_mapping` が受け取れる型表現の範囲を整理する。
  - [ ] `compiler/rust/backend/llvm/src/type_mapping.rs` の `RemlType` 変種と対応する IR 型を表に整理する
  - [ ] `parse_reml_type` の入力 JSON 仕様と、想定外ケースの扱いを確認する
- Runtime に newtype / 合成型のレイアウト前提があるか確認する。
  - [ ] `runtime/native` 内の型タグ定義や ABI 前提のコメント/ドキュメントを確認する
  - [ ] newtype を識別する必要の有無を判断するため、ランタイム API 参照箇所を洗い出す
- `examples/docs-examples/spec/` から `type` 宣言を含む `.reml` を抽出し、alias/newtype/sum の内訳を整理する。
  - [ ] `examples/docs-examples/spec/` の `type` 宣言を列挙し、alias/newtype/sum の件数と代表例を記録する
  - [ ] 仕様書（`docs/spec/1-1-syntax.md` など）のサンプルと対応付ける

#### フェーズ 0 調査メモ
- Frontend AST: `compiler/rust/frontend/src/parser/ast.rs` の `TypeDecl` / `TypeDeclBody` が alias/newtype/sum を保持し、`TypeDeclVariantPayload` で record/tuple を表現する。`DeclKind::Type` で `type` 宣言を扱う。
- Parser: `compiler/rust/frontend/src/parser/mod.rs` で `type alias` と `type <name> = new`、`type <name> = ... | ...` を構文解析する。
- 型環境: `compiler/rust/frontend/src/typeck/env.rs` の `TypeDeclBinding` が `name/generics/kind/body/span` を保持し、`TypeEnv` は `IndexMap<String, TypeDeclBinding>` を名前キーで管理する（スコープは `enter_scope` による親チェーン）。
- typeck 登録: `compiler/rust/frontend/src/typeck/driver.rs` の `register_type_decls` が `TypeDeclBody` から `TypeDeclKind` を決定し、sum 型は `TypeConstructorBinding` として variant 名を別管理する。
- MIR/JSON: `compiler/rust/frontend/src/semantics/typed.rs` / `compiler/rust/frontend/src/semantics/mir.rs` に型宣言の保持はなく、`typeck/typed-ast.rust.json` と `typeck/mir.rust.json` には型宣言が出力されない。一方で `parse/ast.rust.json` は AST 由来で `TypeDecl` を含む。
- Backend: `compiler/rust/backend/llvm/src/type_mapping.rs` の `RemlType` は alias/newtype を持たず、`compiler/rust/backend/llvm/src/integration.rs` の `parse_reml_type` はプリミティブ/参照/スライス/Set/文字列のみ対応（未知は `Pointer` にフォールバック）。`RemlType::Adt` はあるがパース経路がない。
- Runtime: `runtime/native/include/reml_runtime.h` に `REML_TAG_ADT` はあるが newtype 固有タグはなく、現状は型タグ側の前提が最小限。
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
  - [ ] `RemlType` へ alias/newtype/sum を渡す経路を洗い出し、必要な JSON フィールドを列挙する
  - [ ] `TypeMappingContext::layout_of` と sum 型の tag/payload 仕様を整合させ、既存の record/layout ルールとの共通化方針を決める
  - [ ] Runtime の ABI 影響がある場合は別計画へ切り出し、切り出し条件と担当範囲を明文化する

### フェーズ 3: docs-examples-audit の整合チェック
- 影響が出る `.reml` を `docs-examples-audit` の検証対象としてマークする。
- IR 形状やレイアウトが変わる場合は、検証手順・期待値を追記する。
  - [ ] alias/newtype/sum の `.reml` を一覧化し、検証優先度（高/中/低）と理由を付与する
  - [ ] 変更が必要な場合は `reports/spec-audit/summary.md` に起票メモを残し、追跡 ID を付ける
  - [ ] 代表ケースの `.reml` を追加または更新する（必要時）うえで、検証観点（型名/展開/タグ）を明記する

### フェーズ 4: 検証計画
- Frontend の型宣言実体化後に `.reml` の診断が 0 件であることを確認する。
- Backend に影響が出る場合は IR スナップショットで形状を確認する。
  - [ ] `compiler/rust/frontend` のテストで alias/newtype/sum を確認し、期待診断ゼロの条件を記録する
  - [ ] Backend へ合成型が降りる場合の IR 形状を確認し、スナップショット差分の受け入れ基準を定義する

## 受け入れ基準
- alias/newtype/sum の IR 方針が文書化されている。
- Backend/Runtime のレイアウト影響の有無が明記されている。
- `docs-examples-audit` で対象 `.reml` の整合チェックが起票されている。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [x] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了
  - [ ] フェーズ 4 完了

## 関連リンク
- `docs/plans/typeck-improvement/1-0-type-decl-realization-plan.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-2-types-Inference.md`
- `docs/spec/1-3-effects-safety.md`
- `compiler/rust/backend/llvm/src/type_mapping.rs`
