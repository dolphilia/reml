# 診断ゴールデン差分ログ（Stage B 運用）

Stage B で実施する診断ゴールデン更新の記録簿です。各バッチごとに変更概要・検証ログ・レビューポイントを残してください。  
更新後は PR 説明と本ファイルの該当節にリンクを記載し、レビュアが差分意図を追跡できるようにします。

---

## テンプレート

```
### YYYY-MM-DD / 担当者: @handle / バッチ: <型エラー|効果・型クラス|CLI補助診断>
- 変更概要:
  - 適用ファイル: compiler/ocaml/tests/golden/diagnostics/… 
  - 追加/更新したフィールド: `codes[]`, `secondary`, `hints`, `audit`, `timestamp`
- 検証ログ:
  - `dune runtest` 結果: ✅ / ❌
  - CI 成果物: <リンク>（`diagnostic-diff.json`, `iterator-stage-summary.md`, など）
  - 補助スクリプト: `scripts/update-diagnostics-golden.sh`, `tooling/ci/collect-diagnostic-diff.py`
- レビューポイント:
  1. `codes[]` の並び・重複チェック
  2. `secondary` に span なしメッセージが無いか
  3. `audit` / `extensions` の必須キー（effects/bridge）有無
  4. `timestamp` フィールドのフォーマット (ISO8601Z)
- 備考 / リスク:
  - 例: V1 クライアント互換の再検証が必要
```

---

## 2025-Q4 バッチログ

### 2025-10-27 / 担当者: _未記入_ / バッチ: 型エラー
- 変更概要: _未記入_
- 検証ログ:
  - `dune runtest`: _未記入_
  - CI 成果物: _未記入_
- レビューポイント: _未記入_
- 備考 / リスク: _未記入_

### 2025-10-27 / 担当者: _未記入_ / バッチ: 効果・型クラス
- 変更概要: _未記入_
- 検証ログ:
  - `dune runtest`: _未記入_
  - CI 成果物: _未記入_
- レビューポイント: _未記入_
- 備考 / リスク: _未記入_

### 2025-10-27 / 担当者: _未記入_ / バッチ: CLI 補助診断
- 変更概要: _未記入_
- 検証ログ:
  - `dune runtest`: _未記入_
  - CI 成果物: _未記入_
- レビューポイント: _未記入_
- 備考 / リスク: _未記入_
