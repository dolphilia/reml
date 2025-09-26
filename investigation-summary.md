# Reml 仕様分類調査サマリ

## 目的
本ドキュメントは `0-2-project-purpose.md` の原則に基づき、主要仕様が適切な分類（言語仕様／標準パーサーAPI／標準API／エコシステム／ガイド）に配置されているか評価した結果を整理する。

## 評価の観点
- **安全性と性能**: 最優先原則に対して過剰なリスクを露出していないか（`0-2-project-purpose.md:11-36`）。
- **段階的習得と一貫性**: 初学者への導線および学習コストを最小化できているか（`0-2-project-purpose.md:29-36`）。
- **エコシステム統合／DSLファースト**: Chapter 4 や guides に逃がすべき周辺仕様を適切に切り分けているか（`0-2-project-purpose.md:56-64`）。

## 主な調査結果

### 1. 言語仕様への昇格が望ましい文書
- `1-5-formal-grammar-bnf.md:1-146`
  - 完全な言語BNFを収容しており、ガイド扱いのままでは構文仕様の学習経路が分散する。
  - 言語コア仕様（Chapter 1）へ統合することで、構文定義と形式文法を一元化できる。

### 2. 標準APIとしては早期であり公式プラグイン章が適切な仕様
- `5-1-system-plugin.md`（旧 3-11 Core System）
- `5-2-process-plugin.md`（旧 3-12 Core Process）
- `5-3-memory-plugin.md`（旧 3-13 Core Memory）
- `5-4-signal-plugin.md`（旧 3-14 Core Signal）
- `5-5-hardware-plugin.md`（旧 3-15 Core Hardware）
- `5-6-realtime-plugin.md`（旧 3-16 Core RealTime）
  - いずれもドラフト段階で `effect {unsafe}` やプラットフォーム固有機能を広く露出している。
  - 現行の Chapter 3 に置くと、標準API利用者へ安全性リスクを前倒しで課す構成となり、優先原則（安全性・段階的習得）に反する。
  - Chapter 5 の公式プラグインとして隔離し、Capability Registry からは任意登録モジュールとして参照する構成が合理的。

### 3. 現行分類は妥当だが仕様の補強が必要な項目
- `3-9-core-async-ffi-unsafe.md:74-106`
  - チャネル／バックプレッシャ／`Codec` 契約が API として宣言されている一方で、型や失敗モードの詳細はガイド依存（`guides/conductor-pattern.md:54-82`）。
  - 標準APIの責務として、`Codec` や `ExecutionPlan` の正式定義とエラー契約を補う必要がある。
- `3-10-core-env.md:16-134`
  - `TargetProfile` 同期と監査要件が明確であり、標準API分類に適合。文書間リンク整備を継続するのみで可。

## 推奨アクション
1. **形式文法の昇格**
   - 新設した `1-5-formal-grammar-bnf.md` を Chapter 1 の正式節として扱い、`1-1-syntax.md` から参照できるよう維持する。
   - 目次 (`README.md`) と相互参照を更新する。
2. **システム系Capabilityの公式プラグイン整備**
   - Chapter 5 で分離した `5-1`〜`5-6` を公式プラグインとして位置づけ、配布プロセス・審査フローを追記する。
   - `3-8-core-runtime-capability.md` に任意登録扱いである旨と安全審査手順を明記し、Capability Registry からの導線を整理する。
3. **Core.Async チャネルAPIの補強**
   - `Codec` / `Channel` / `ExecutionPlan` の契約・失敗モードを `3-9-core-async-ffi-unsafe.md` 本文に正式記述する。
   - ガイド側 (`guides/conductor-pattern.md`) は運用ベストプラクティスに専念させる。

## 残課題
- Chapter 5 公式プラグイン章のドラフト精査（配布手順や審査要件の追記）。
- Capability Registry 周辺での Security/Audit 流れの再チェック。
- ガイド類と仕様本文の重複チェック（特に DSL 関連資料）。
