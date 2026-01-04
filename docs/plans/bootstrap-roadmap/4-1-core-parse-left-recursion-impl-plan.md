# Phase4: Core.Parse Left Recursion 実装計画

## 背景と目的
- `docs/plans/core-parse-improvement/1-5-left-recursion-plan.md`（WS6）で、左再帰を直接書かない方針と安全弁としての `left_recursion` 利用方針を確定した。
- 仕様側の補足（`docs/spec/2-2-core-combinator.md` / `2-6-execution-strategy.md` / `2-5-error.md`）により、
  左再帰は **ガイドラインで回避**し、**混入時の検出と回帰**で安全性を担保することが明文化された。
- Phase4 の spec_core 回帰に戻った際に、`CP-WS6-001/002` を実行可能にし、
  左再帰混入の検出（E4001）と profile 指標の観測を固定できるようにする。

## スコープ
- 対象: Rust フロントエンドの CLI 実行と回帰資産の整備。
- シナリオ:
  - `CP-WS6-001`: 左再帰検出（`RunConfig.left_recursion="off"`）
  - `CP-WS6-002`: 左再帰ガード + profile 指標（`left_recursion_guard_hits`）
- 資産:
  - `examples/spec_core/chapter2/parser_core/core-parse-left-recursion-direct.reml`
  - `examples/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.reml`
  - 期待メモ: `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-direct.expected.md`
  - 期待メモ: `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.expected.md`

## 成果物
- `CP-WS6-001` の CLI 実行ログに `E4001` が含まれ、位置が左再帰定義に一致する。
- `CP-WS6-002` の profile 出力で `left_recursion_guard_hits > 0` を確認できる。
- `phase4-scenario-matrix.csv` に `CP-WS6-001/002` を登録し、`resolution_notes` に実行コマンドとログパスを記録する。

## 実装ステップ
1. **CP-WS6-001（左再帰検出）の実行と確認**
   - 実行コマンド（例）:
     - `compiler/frontend/target/debug/reml_frontend --output json --packrat --left-recursion off examples/spec_core/chapter2/parser_core/core-parse-left-recursion-direct.reml`
   - 期待条件:
     - 診断コードに `E4001` が含まれる。
     - 位置が `expr_left_recursion_direct` 定義の開始付近を指す。
   - 出力ログ: `reports/spec-audit/ch5/logs/spec_core-CP-WS6-001-<timestamp>.diagnostic.json`
   - `E4001` が出ない場合は、左再帰検出パスの診断出力を実装で補う。

2. **CP-WS6-002（profile 指標）の出力確認**
   - 実行コマンド（例）:
    - `compiler/frontend/target/debug/reml_frontend --parse-driver --parse-driver-left-recursion-parser --parse-driver-packrat on --parse-driver-left-recursion on --parse-driver-profile-output expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.profile.json --output json examples/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.reml`
   - 期待条件:
     - `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.profile.json` が生成される。
     - `left_recursion_guard_hits > 0` と `memo_entries > 0` を満たす。
   - `profile_output` が生成されない場合は、`Parse.run` の profile 出力が CLI 実行に反映される経路を実装/修正する。

3. **回帰登録とメモの更新**
   - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CP-WS6-001/002` を登録。
   - `resolution_notes` に実行コマンドとログパスを記録。
   - `expected/...left-recursion-*.expected.md` に実行ログのリンクと確認結果を追記。

## 進捗状況（2025-12-19）
- ✅ サンプル追加済み:
  - `core-parse-left-recursion-direct.reml`
  - `core-parse-left-recursion-slow.reml`
- ✅ 期待メモ追加済み:
  - `core-parse-left-recursion-direct.expected.md`
  - `core-parse-left-recursion-slow.expected.md`
- ✅ CLI 実行ログ採取済み:
  - `reports/spec-audit/ch5/logs/spec_core-CP-WS6-001-20251218T233918Z.diagnostic.json`
  - `reports/spec-audit/ch5/logs/spec_core-CP-WS6-002-20251218T225547Z.diagnostic.json`
- ✅ `profile_output` 生成済み:
  - `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.profile.json`
  - `left_recursion_guard_hits=1` を確認。
- ✅ `phase4-scenario-matrix.csv` の `CP-WS6-001/002` を `ok` へ更新済み。

## 生成経路の確認（2025-12-19）
- `reml_frontend --output json` は **Reml ソースの構文解析のみ**を行い、`examples/...` 内の `Parse.run(...)` を実行しない。
- `--parse-driver` に profile/left_recursion/packrat を渡す CLI オプションを追加し、抽出入力で profile JSON を生成できるようにした。
- 生成例:
  - `compiler/frontend/target/debug/reml_frontend --parse-driver --parse-driver-left-recursion-parser --parse-driver-packrat on --parse-driver-left-recursion on --parse-driver-profile-output expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.profile.json --output json examples/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.reml`

## 補強タスク（完了）
Phase4 で profile 指標を取得するため、`parse-driver` に RunConfig を渡せる経路を追加済み。

1. **CLI オプション追加**（完了）
   - 追加: `--parse-driver-profile-output` / `--parse-driver-left-recursion` / `--parse-driver-packrat` / `--parse-driver-left-recursion-parser`
2. **parse-driver 実行時の RunConfig 反映**（完了）
   - `run_parse_driver_mode` で `run_config` を構築し、`runtime_parse::run_shared` に渡す。
3. **回帰資産の更新**（完了）
   - `core-parse-left-recursion-slow.profile.json` を生成。
   - `phase4-scenario-matrix.csv` の `resolution_notes` を更新。

## 未着手・残タスク（推奨順）
- 現時点の残タスクはなし。次回は Phase4 再実行のログ反映のみを行う。

## 依存関係
- 仕様: `docs/spec/2-2-core-combinator.md`, `docs/spec/2-6-execution-strategy.md`, `docs/spec/2-5-error.md`
- 計画: `docs/plans/core-parse-improvement/1-5-left-recursion-plan.md`
- 既存回帰: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`

## リスクと対策
- **左再帰検出が未実装**: `E4001` を出せない場合、`RunConfig.left_recursion="off"` の検出パスを実装し、診断キーを固定する。（対応済み）
- **profile 出力が出ない**: `Parse.run` の profile 出力経路を CLI が拾うよう調整し、`profile_output` を生成させる。（対応済み）
- **他シナリオへの影響**: `--left-recursion` フラグの既定値や警告が他の Phase4 シナリオに影響しないか確認する。

## 完了判定
- `CP-WS6-001/002` を Phase4 マトリクスへ登録し、`resolution_notes` を記入。
- `E4001` がログに含まれ、`left_recursion_guard_hits` の profile 出力が確認できる。
- `expected/...left-recursion-*.expected.md` が最新ログと整合している。
