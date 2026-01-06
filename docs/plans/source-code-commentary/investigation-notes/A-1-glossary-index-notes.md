# 調査メモ: 付録A 用語集と索引

## 対象資料

- `docs/plans/source-code-commentary/1-0-structure.md:117-123`
- `docs/plans/source-code-commentary/1-2-module-spec-mapping.md:1-53`
- `docs/spec/0-2-glossary.md:1-120`
- `compiler/README.md:1-19`
- `compiler/frontend/README.md:1-15`
- `compiler/runtime/README.md:1-23`
- `compiler/adapter/README.md:1-26`
- `compiler/backend/README.md:1-23`
- `compiler/ffi_bindgen/README.md:1-27`
- `compiler/xtask/README.md:1-13`

## 目的

- 付録Aの「用語集」と「索引」を、既存の仕様用語と実装モジュールを統合して整理するための基礎情報を集約する。
- モジュール名と機能の対応表（1-0-structure.md の付録A）の要求を満たすため、主要ディレクトリ/モジュールと章番号の対応を確定する。

## 観察メモ

### 付録Aの要求範囲

- 付録Aは「用語集と索引」であり、最低限「モジュール名と機能の対応表」を含める必要がある。
  - `docs/plans/source-code-commentary/1-0-structure.md:117-120`

### 仕様側の用語集

- 仕様書には `docs/spec/0-2-glossary.md` が既に存在し、言語コア/効果/パーサ/ランタイム/診断などの用語が体系的に整理されている。
  - 例: 型推論、効果タグ、Parser<T>、Diagnostic などがカテゴリ別に列挙されている。
  - `docs/spec/0-2-glossary.md:1-120`

### モジュール索引の候補

- フロントエンド/ランタイムの章対応は `1-2-module-spec-mapping.md` が基準になる。
  - `docs/plans/source-code-commentary/1-2-module-spec-mapping.md:1-53`
- 付録Aに載せるべき「主要モジュール」は、各 README に明示されている。
  - `compiler/README.md:5-19`
  - `compiler/frontend/README.md:5-15`
  - `compiler/runtime/README.md:5-23`
  - `compiler/adapter/README.md:5-13`
  - `compiler/backend/README.md:5-6`
  - `compiler/ffi_bindgen/README.md:5-27`
  - `compiler/xtask/README.md:1-13`

## 付録Aドラフトに入れるべき要素（候補）

- 用語集は `docs/spec/0-2-glossary.md` をベースにし、コード解説用に以下を追加する。
  - 実装ファイルへの参照（例: `compiler/frontend/src/typeck/...`）
  - 解説書の章番号（第4章〜第23章）
- 索引は次の2層構造が妥当。
  - 「モジュール索引」: 主要モジュール名/役割/章番号/関連仕様を一覧化。
  - 「キーワード索引」: 用語 → 章番号/関連節の参照表。

## TODO / 確認事項

- 付録Aに「第三者がすぐ引ける」索引形式（五十音/カテゴリ/英字順）のどれを採用するかを決める。
- `compiler/backend/llvm` と `compiler/runtime/native` / `compiler/runtime/ffi` の索引粒度（ディレクトリ単位か README 単位か）を確定する。
- `tooling/` 領域を索引に含める場合、章対応（第21章のみか付録扱いか）の方針を整理する。
