# 5.1 JIT バックエンド調査・設計計画

## 背景
- Reml の JIT 実行は **調査段階**であり、実装は未着手。
- `docs/notes/backend/a-jit.md` と `docs/notes/backend/performance-optimization-research-20251221.md` に TODO が散在しているため、Phase 5 以降のロードマップへ統合する。

## 目的
1. Reml における JIT 実行の **必要性・対象範囲**を整理する。
2. JIT バックエンド候補（Cranelift / LLVM ORC JIT / WASM 経由）を比較し、採用方針を決定する。
3. JIT 実行を許可する **RunConfig/Capability/監査**の設計指針を確定する。

## スコープ
- **含む**:
  - JIT のユースケース整理（`reml run`/DSL 実行/開発モード）
  - Cranelift / LLVM ORC JIT / WASM 経由 JIT の比較調査
  - セキュリティ/サンドボックス/サーバーレス制約の整理
  - JIT 実行時の監査キー・Capability 連携方針の設計
  - JIT 導入時の CI/ベンチマーク前提条件の整理
- **含まない**:
  - JIT バックエンドの本格実装
  - Inline ASM / LLVM IR 直書きのエスケープハッチ実装（別計画）
  - GPU バックエンド本実装

## 成果物
- JIT 実行の採用可否と優先順位の結論メモ
- JIT バックエンド選定結果（Cranelift/LLVM ORC/WASM の比較表）
- RunConfig / Capability / 監査キーの更新方針メモ
- 最小 PoC の仕様と検証条件（実装は別フェーズ）

## 作業フェーズ

### フェーズA: 要件整理
1. `reml run` と DSL 実行の JIT 需要を整理する。
2. JIT が必要なプラットフォーム条件（WASI Preview 2 の可否、AOT との差分）を調査する。

### フェーズB: バックエンド比較
1. Cranelift 採用時の利点/制約を整理する。
2. LLVM ORC JIT の現状、依存、ビルド条件を調査する。
3. WASM + Wasmtime 経由の JIT 実行モデルを評価する。

### フェーズC: 運用・安全設計
1. サーバーレス/コンテナ環境での JIT 制約（`seccomp`, `run-as-nonroot` 等）を調査する。
2. `effect {jit}` / Capability / 監査ログの統合方針を設計する。

### フェーズD: PoC 仕様策定
1. 最小 PoC のコマンド/入出力（例: `reml run --engine jit`）を定義する。
2. CI ベンチマークジョブ導入の前提条件を整理する。

## 作業チェックリスト

### フェーズA: 要件整理
- [ ] JIT ユースケース整理（`reml run` / DSL）
- [ ] WASI Preview 2 と AOT の制約調査

### フェーズB: バックエンド比較
- [ ] Cranelift 比較メモ
- [ ] LLVM ORC JIT 比較メモ
- [ ] WASM/Wasmtime JIT 比較メモ

### フェーズC: 運用・安全設計
- [ ] サーバーレス/コンテナ制約の整理
- [ ] Capability/監査キー設計メモ

### フェーズD: PoC 仕様策定
- [ ] 最小 PoC の仕様定義
- [ ] CI ベンチマーク条件の整理

## 参照
- `docs/notes/backend/a-jit.md`
- `docs/notes/backend/performance-optimization-research-20251221.md`
- `docs/notes/backend/llvm-spec-status-survey.md`
