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

### 2025-10-27 / 担当者: Codex / バッチ: 型エラー
- 変更概要:
  - 適用ファイル: compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden（差分計測のみ）
  - 追加/更新したフィールド: `schema_version`, `timestamp`, `diagnostic.v2.codes[]`
- 検証ログ:
  - `dune runtest`: 未実施（スクリプト整備フェーズ）
  - CI 成果物: 未実施（差分ツール初期化）
- レビューポイント:
  1. `tooling/ci/collect-diagnostic-diff.py` で `schema_version` が検知されること
  2. `scripts/update-diagnostics-golden.sh --diff` で差分サマリが生成されること
  3. `_actual` ディレクトリがクリーンアップされること
- 備考 / リスク:
  - 次回バッチで実際のゴールデン更新を行う際は `dune runtest` を通じた出力収集が必須

### 2025-10-27 / 担当者: Codex / バッチ: 効果・型クラス
- 変更概要:
  - 適用ファイル: compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden（差分計測のみ）
  - 追加/更新したフィールド: `extensions.effects.stage_trace`, `audit.metadata.stage_trace`
- 検証ログ:
  - `dune runtest`: 未実施（差分計測の下準備）
  - CI 成果物: 未実施（collect-diagnostic-diff 集計検証待ち）
- レビューポイント:
  1. `collect-iterator-audit-metrics.py` が timestamp 欠落を検出するか
  2. `sync-iterator-audit.sh` V2 ステータスが `❌` → `✅` に切り替わるか
  3. `schema_versions` に 2.0.0-draft が記録されるか
- 備考 / リスク:
  - 実際の更新時に Stage トレース欠落が出た場合は `effect.stage_trace` を追記する

### 2025-10-27 / 担当者: Codex / バッチ: CLI 補助診断
- 変更概要:
  - 適用ファイル: scripts/update-diagnostics-golden.sh, tooling/ci/collect-diagnostic-diff.py（新規）
  - 追加/更新したフィールド: CLI 生成物は未更新（パイプライン整備）
- 検証ログ:
  - `dune runtest`: 実行なし
  - CI 成果物: なし（スクリプト導入のみ）
- レビューポイント:
  1. `scripts/update-diagnostics-golden.sh --diff --no-test` がローカルで実行できるか
  2. `collect-diagnostic-diff.py --format markdown` の出力に `変更ファイル数` が表示されるか
  3. `_actual/*.actual.json` が残存しないか
- 備考 / リスク:
  - ゴールデン未更新のため CI 成果物は生成されていない。次回更新時に README への追記が必要
