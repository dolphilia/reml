# Phase 1 Draft Outline (Core Language)

フェーズ1で改訂するコア言語章 (`1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`) の草案方針をまとめる。

---

- ✅ **フィードバック反映**
  - `schema` セクションに「継承・テンプレート」「テンプレート関数との統合」のサンプルを追加予定。
  - プラグイン構文 (`package`/`use plugin`) についてバージョン管理や capability の語彙検討を継続。

## 1. `1-1-syntax.md` 追加要素案
- ✅ **スキーマ宣言構文**（`B.6`）
  - 条件付き束縛 (`let … if …`)・`compute`・`requires` を仕様化。
  - 継承/テンプレートと `Schema.template().with(...)` の例を掲載済み。
- ✅ **条件付き束縛・構成**
  - `when` ブロックと糖衣 `let field if cond = ...` の関係を定義。
- ✅ **モジュール / プラグイン構文**（`B.7`）
  - `@plugin` メタデータ・バージョン制約・Capability 宣言を明文化。
- **DSL 拡張ポイントの例示**
  - テンプレート DSL（Web）、ルーティング DSL（Web）、IaC DSL（Cloud）などへの橋渡しを「例」節として示す。

## 2. `1-2-types-Inference.md` 追加要素案
- ✅ **ドメイン型の導入**（`J`）
  - Tensor/Column/Schema/Resource の種と単一化ルールを定義。
  - `SchemaDiff` と `DiffConstraint` の解決手順を記述。
- ✅ **型と効果の接続**
  - `effect` 注釈付き関数型と `EffectSet` の和集合規則を明文化。
  - クラウド/GPU/監査へのマッピングを 1-3 節 `K` と接続。
- ✅ **推論ルールの拡張**
  - Tensor ブロードキャスト、Column メタデータ整合、Resource Capability の制約解決を整理。
  - スキーマ進化の未解決制約を `SchemaEvolutionRequired` として報告する流れを記録。

## 3. `1-3-effects-safety.md` 追加要素案
- **効果分類の拡張**（Draft 追加済 `K`、サンプル追加済）
  - `audit`, `debug`, `runtime`（FFI）, `config`, `io`, `network`, `db` など用途別タグを整理。
  - 効果の合成・制約ルールを図表で提示。
- **FFI / ランタイムガイドライン**
  - クラウド API, GPU アクセラレータ, 組み込み I/O のケーススタディ。
  - `unsafe` ブロックの境界策定、メモリアクセス規約、差分適用の安全確認。
- **ホットリロード / 差分適用**
  - 状態整合性、効果の巻き戻し、エラーハンドリングの手順。
  - `defer` / `audit` 効果と連携するパターン。

---

## 次のアクション
1. 各章のドラフト作業ブランチ（またはメモファイル）を用意し、上記項目に基づいた骨子を作成。
2. 既存テキストに挿入する位置を `notes/phase0-gap-analysis.md` の指摘と照合して決定。
3. 章ごとにサンプルコード／仕様例を収集し、ドラフトへ組み込む準備を進める。
