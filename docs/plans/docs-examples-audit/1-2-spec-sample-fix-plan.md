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
- 修正対象リスト: `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md`

## 進め方

### 1. NG 分類の付与と補足情報の整理
- 章別レポートから NG の診断コードと代表メッセージを抽出する。
- 在庫表の備考に `category:` と `diag_code:` を付与する（不足分のみ追記）。
- 判断が必要なものは `unknown` に分類し、判断理由（参照ファイル）を備考へ短く残す。

### 2. 章単位の修正計画メモ作成
- NG が多い章から順に着手し、1 章ずつ修正方針を固定する。
- 章ごとに `reports/spec-audit/chX/docs-examples-fix-notes-YYYYMMDD.md` を作成する。
- メモには以下を必ず含める。
  - 代表的な診断コードと頻出パターン
  - 修正対象サンプル（コード名・`.reml` パス）
  - 仕様記述との整合メモ（参照箇所を明記）

### 3. 修正方針の選択（テンプレ）
- **仕様優先**: 仕様の意図が正しい場合は Rust Frontend に合わせない（備考に理由と参照）。
- **サンプル修正**: 仕様準拠の範囲で、実装で通る最小構文へ書き換え。
- **フォールバック**: `*_rustcap.reml` を追加し、備考に理由と参照を残す。
- **段階明示**: 実験段階の構文は `@unstable` 等で段階を明示し、診断ログと対応付ける。
- **方針の割り当て**: `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md` の category に従い、差分が出る場合は修正方針を明記して更新する。

### 4. 反映と差分の整理
- `.reml` 修正後に在庫表の「状態」と備考を更新する。
- 変更があれば該当仕様ドキュメントに注釈を追記し、相互参照を付与する。
- ファイル移動・名称変更があれば `docs-migrations.log` を更新する。

### 5. 監査ログの追記
- 実行コマンドと結果を `reports/spec-audit/summary.md` に追記する。
- 章別メモの作成日・対象章を同じエントリ内に記録する。

## 優先順位の目安
1. 仕様コア（`docs/spec/1-x`）の NG で、他章に波及するもの
2. Parser/Diagnostics など実装の根幹に関わる例
3. Capability/Runtime など外部連携の例

## チェックリスト
- [ ] 在庫表に `category:` と `diag_code:` / `diag:` が付与されている
- [ ] `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md` が最新化されている
- [ ] 修正対象の `.reml` が仕様と矛盾していない
- [ ] 章別メモが `reports/spec-audit/chX/` に残っている
- [ ] `reports/spec-audit/summary.md` に実行記録がある

## TODO
- NG を診断コード別に集計するための補助スクリプトの要否を確認する。
- rustcap サンプルの命名規則を `docs/spec/0-3-code-style-guide.md` に追記するか検討する。

## 実施メモ
- 2025-12-26: `docs/spec/2-3-lexer.md` と `examples/docs-examples/spec/2-3-lexer/*.reml` を正準例へ復元（const/型レベル union/let 関数糖衣/集合リテラル/struct・レコード構築）。`reml_frontend --output json` で全サンプル診断 0 件を確認。
