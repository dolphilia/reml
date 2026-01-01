# A. JIT / バックエンド拡張ノート

> 目的：Reml コンパイラの JIT / AOT バックエンドに関する検討事項をメモし、Phase 3 のポータビリティロードマップ（WASM / ARM64 / クラウド）と整合させる。

## 1. 現状メモ
- LLVM を用いたネイティブコード生成は x86_64 (System V / Windows) を優先ターゲットとする。
- WASI / WASM は AOT（`wasm32-wasi` バイナリ）を優先し、JIT の必要性は未評価。
- ARM64 では LLVM の NEON/SVE 最適化を利用できるが、JIT 実行は検証が必要。

## 2. Phase 3 TODO
- [ ] WASM ターゲット向け JIT の要否調査（WASI Preview 2 での JIT 許可状況、AOT との比較）
- [ ] ARM64 NEON / SVE 用の TargetMachine 設定テンプレートと自動検出ロジックの整備
- [ ] GPU アクセラレータ連携時の JIT 生成コード（PTX / Metal Shading Language）検討
- [ ] コンテナ/サーバーレス環境における JIT 実行のセキュリティ評価（`run-as-nonroot`, seccomp, sandbox）

## 3. 参考
- `docs/guides/runtime/runtime-bridges.md` のクラウド/サーバーレスセクション
- `docs/guides/runtime/portability.md` のターゲット戦略チェックリスト
- LLVM ORC JIT / MCJIT のサポート状況

## 4. 調査計画
- [ ] LLVM ORC JIT の WASM 対応状況を確認し、必要なフラグと依存関係を整理
- [ ] ARM64 NEON/SVE 向けの `TargetMachine` 設定例を作成し、ベンチマーク候補（例: JSON パーサ DSL）を選定
- [ ] サーバーレス（AWS Lambda / Cloud Run）での JIT 実行制限を調査し、サンドボックス方針をまとめる
- [ ] `docs/guides/tooling/ci-strategy.md` に JIT ベンチマークジョブを追加する前提条件を列挙
