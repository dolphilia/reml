---
title: "TODO: collect-iterator audit pass rate 追跡"
status: draft
created: 2025-11-07
---

## TODO

- ✅ **collect-iterator audit pass rate の分母が二重カウントされる問題を修正する**（2025-11-07 `8a5f04c`）  
  - 再現条件: `python3 tooling/ci/collect-iterator-audit-metrics.py --section ffi --source compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-{linux,macos,windows}.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden`  
  - 対処: 診断ループ内で `total += 1` が二度実行されていた箇所を修正し、`ffi_bridge.audit_pass_rate` が `pass_rate=1.0` / `pass_fraction=1.0` を返すことを確認 (`tooling/ci/collect-iterator-audit-metrics.py` 改修)。  
  - `python3 ... --require-success` でゼロ終了になることを再確認済み。

## メモ

- `effects/syntax-constructs.json.golden` は診断スキーマ（diagnostic-v2）対象外のため、`scripts/validate-diagnostic-json.sh` 実行時は引数から除外する運用を今後も継続する。  
- 本 TODO の完了後、`--require-success` を伴う Phase 2 ベースライン測定を再実行し、`docs/plans/rust-migration/0-1-baseline-and-diff-assets.md` のメトリクスを更新する。
