# Phase 1 Draft Outline (Core Language)

フェーズ1で改訂するコア言語章 (`1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`) の草案方針をまとめる。

---

- ✅ **フィードバック反映**
  - `schema` セクションに「継承・テンプレート」「テンプレート関数との統合」のサンプルを追加予定。
  - プラグイン構文 (`package`/`use plugin`) についてバージョン管理や capability の語彙検討を継続。

## 1. `1-1-syntax.md` 追加要素案
- **スキーマ宣言構文**（Draft 追加済 `B.6`、要追加サンプル）
  - 継承/マージ、条件付きプロパティ (`when env == "prod"` など)。
  - フィードバック: `schema` の再利用性、テンプレート関数との組み合わせ例を追加。
- **条件付き束縛・構成**
  - `let value if condition = ...` や `match config { case ... }` を設定 DSL に転用できる例を追加。
- **モジュール / プラグイン構文**（Draft 追加済 `B.7`）
  - `package` / `export` / `use plugin` の宣言で DSL パッケージを定義。
  - プラグインメタデータ（version, capability）を記述する属性。
- **DSL 拡張ポイントの例示**
  - テンプレート DSL（Web）、ルーティング DSL（Web）、IaC DSL（Cloud）などへの橋渡しを「例」節として示す。

## 2. `1-2-types-Inference.md` 追加要素案
- **ドメイン型の導入**（Draft 追加済 `J`、サンプル追加済）
  - テンソル型（`Tensor<Dim, T>`）、列型（`Column<T>`）、リソースID型（`Resource<Provider, Kind>`）。
  - スキーマ型（`Schema<Record>`）を導入し、フィールドアクセス・差分検証の型ルールを定義。
- **型と効果の接続**
  - `effectful` な関数型（`fn(...) -> Result<T, Error> effect Db` のような表記）を検討。
  - クラウド操作／リアルタイム制約と型の関係を説明する節を新設。
- **推論ルールの拡張**
  - 新型の単一化、既定型（numericsとの相互作用）、制約生成の追加例。
  - スキーマ進化／差分（旧スキーマ vs 新スキーマ）を扱う型推論ガイドライン。

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
