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
- MIR/JSON へ型情報がどこまで渡るかを確認する。
- Backend の `RemlType` / `type_mapping` が受け取れる型表現の範囲を整理する。
- Runtime に newtype / 合成型のレイアウト前提があるか確認する。
- `examples/docs-examples/spec/` から `type` 宣言を含む `.reml` を抽出し、alias/newtype/sum の内訳を整理する。
  - [ ] Frontend で alias/newtype/sum を保持できる前提を整理する
  - [ ] MIR/JSON の型情報（型名・展開後型・名義型 ID）の有無を確認する
  - [ ] Backend の `RemlType` 受け口と `parse_reml_type` の対応範囲を確認する
  - [ ] Runtime 側の型タグ/レイアウト前提があるかを確認する
  - [ ] `examples/docs-examples/spec/` の `type` 宣言を分類する

### フェーズ 1: IR 方針の決定（alias/newtype/sum）
- alias は **展開後の型を IR に渡す** 方針を既定とする（レイアウト影響なし）。
- newtype は **IR では内側の型へマップ**し、**名義情報はデバッグ/診断メタデータ**として保持する方針を検討する。
- sum 型は **`RemlType::Adt` に落とす**方針を採用し、タグ幅と payload の計算ルールを定義する。
  - [ ] alias の展開タイミングを決める（typeck 後 / MIR 生成時）
  - [ ] newtype の名義情報の保持場所（IR メタデータ or Frontend のみ）を決める
  - [ ] sum 型のタグ幅算出ルール（`ceil(log2(variants))`）を決める
  - [ ] IR レイアウトが変わるケース（newtype の ABI 差分有無）を列挙する

### フェーズ 2: Backend/Runtime 側の整合ポイント整理
- Backend の型マッピングが alias/newtype/sum を受け取れる前提を整理する。
- newtype が Runtime で識別可能である必要があるか確認する（基本は同一レイアウト）。
- 合成型の payload/タグの配置ルールが既存の `TypeMappingContext` と矛盾しないか確認する。
  - [ ] `RemlType` へ alias/newtype/sum を渡す経路を洗い出す
  - [ ] `TypeMappingContext::layout_of` と sum 型の tag/payload 仕様を整合させる
  - [ ] Runtime の ABI 影響がある場合は別計画へ切り出す

### フェーズ 3: docs-examples-audit の整合チェック
- 影響が出る `.reml` を `docs-examples-audit` の検証対象としてマークする。
- IR 形状やレイアウトが変わる場合は、検証手順・期待値を追記する。
  - [ ] alias/newtype/sum の `.reml` を一覧化し、検証優先度を付ける
  - [ ] 変更が必要な場合は `reports/spec-audit/summary.md` に起票メモを残す
  - [ ] 代表ケースの `.reml` を追加または更新する（必要時）

### フェーズ 4: 検証計画
- Frontend の型宣言実体化後に `.reml` の診断が 0 件であることを確認する。
- Backend に影響が出る場合は IR スナップショットで形状を確認する。
  - [ ] `compiler/rust/frontend` のテストで alias/newtype/sum を確認する
  - [ ] Backend へ合成型が降りる場合の IR 形状を確認する

## 受け入れ基準
- alias/newtype/sum の IR 方針が文書化されている。
- Backend/Runtime のレイアウト影響の有無が明記されている。
- `docs-examples-audit` で対象 `.reml` の整合チェックが起票されている。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了
  - [ ] フェーズ 4 完了

## 関連リンク
- `docs/plans/typeck-improvement/1-0-type-decl-realization-plan.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-2-types-Inference.md`
- `docs/spec/1-3-effects-safety.md`
- `compiler/rust/backend/llvm/src/type_mapping.rs`
