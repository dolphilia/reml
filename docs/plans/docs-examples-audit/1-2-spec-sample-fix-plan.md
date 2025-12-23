# 1.2 docs/spec サンプル修正計画

`examples/docs-examples/spec/` 配下の `.reml` サンプルで発生した NG を対象に、仕様との整合を保ちながら修正・フォールバックを行うための計画書。

## 対象範囲
- `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` に記載された `.reml`
- `reports/spec-audit/ch0`〜`ch4` に生成した `docs-examples-audit-YYYYMMDD.md` と診断 JSON

## 目的
- 仕様書のコード例が `reml_frontend` の実装と矛盾しない状態を維持する。
- NG サンプルの原因を分類し、修正・代替（rustcap 等）の方針を統一する。
- 更新内容を監査ログと相互参照できる形に残す。

## 入力資料
- 在庫表: `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md`
- 監査サマリ: `reports/spec-audit/summary.md`
- 章別レポート: `reports/spec-audit/ch*/docs-examples-audit-YYYYMMDD.md`
- 診断 JSON: `reports/spec-audit/ch*/*-YYYYMMDD-diagnostics.json`

## 進め方

### 1. NG 分類の付与
- 章別レポートから NG の診断コードと代表メッセージを抽出する。
- 以下のカテゴリを在庫表の備考に追記する（例: `category:syntax`）。
  - `syntax` 仕様上は妥当だが Rust Frontend が未実装
  - `example` 仕様例そのものに誤りがある
  - `staging` 実験段階の機能（Stage/Capability 要件）
  - `fallback` rustcap などの代替サンプルが必要
  - `unknown` 追加調査が必要

### 2. 章単位の修正計画
- NG が多い章を優先し、1 章ずつ修正方針を決める。
- 章ごとに以下を `reports/spec-audit/chX/` にメモとして残す。
  - 代表的な診断コード
  - 修正対象のサンプル一覧
  - 仕様記述との整合メモ

### 3. 修正方針の選択
- **仕様優先**: 仕様の意図が正しい場合は Rust Frontend に合わせない。
- **サンプル修正**: 仕様に準拠しつつ、実装で通る形に書き換え。
- **フォールバック**: `*_rustcap.reml` を追加し、備考に理由と参照を残す。
- **段階明示**: 実験段階の構文は `@unstable` 等で段階を明示し、診断ログと対応付ける。

### 4. 在庫表の更新
- 修正後に再検証を行い、在庫表の「状態」と備考を更新する。
- 変更があれば該当の仕様ドキュメントに注釈を追記し、相互参照を付ける。

### 5. 監査ログの追記
- 実行コマンドと結果を `reports/spec-audit/summary.md` に追記する。
- `docs-migrations.log` は移動・rename が伴う場合に更新する。

## 優先順位の目安
1. 仕様コア（`docs/spec/1-x`）の NG で、他章に波及するもの
2. Parser/Diagnostics など実装の根幹に関わる例
3. Capability/Runtime など外部連携の例

## チェックリスト
- [ ] 在庫表に `category:` と `diag:` が付与されている
- [ ] 修正対象の `.reml` が仕様と矛盾していない
- [ ] 章別メモが `reports/spec-audit/chX/` に残っている
- [ ] `reports/spec-audit/summary.md` に実行記録がある

## TODO
- NG を診断コード別に集計するための補助スクリプトの要否を確認する。
- rustcap サンプルの命名規則を `docs/spec/0-3-code-style-guide.md` に追記するか検討する。
