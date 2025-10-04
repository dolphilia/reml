# 3.0 Phase 3 — Self-Host Transition

Phase 3 は Reml 自身で Reml コンパイラを実装するセルフホスト移行段階である。OCaml 実装と Reml 実装を並行稼働させながら、Parser → TypeChecker → CodeGen → Runtime の順に置き換え、互換性と性能を保ちながら段階的に移行する。

## 3.0.1 目的
- Parser/TypeChecker/CodeGen/Runtime を Reml で再実装し、Reml ソースから生成されたバイナリが OCaml 実装と同一の LLVM IR（または差異が許容範囲内）になることを確認する。
- `2-7-core-parse-streaming.md` のストリーミング実行モデル、`3-8-core-runtime-capability.md` の Stage 契約を Reml 実装で満たし、セルフホスト後のランタイムが仕様通りに稼働することを検証する。
- コンパイラ内部 API を標準ライブラリ (`Core.*`) と再利用可能な形で整理し、DSL/プラグイン開発者が参照できるベースラインを整える。

## 3.0.2 スコープ境界
- **含む**: Reml 実装の Parser/TypeChecker/CodeGen/Runtime、**クロスコンパイル機能**（x86_64 Linux/Windows + ARM64 macOS 対応）、セルフホスト用ビルドパイプライン、互換性テスト、CI ワークフロー、**メモリ管理戦略の評価（RC 継続 vs GC 導入）**。
- **含まない**: フル JIT（Phase 4 以降または別計画）。
- **前提条件**: Phase 2 の成果物と性能ベースライン、`notes/llvm-spec-status-survey.md` における決定済み仕様、**`notes/cross-compilation-spec-intro.md` および `notes/cross-compilation-spec-update-plan.md` のクロスコンパイル計画**、`0-3-audit-and-metrics.md` の計測フレーム。

## 3.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Parser 移植 | Core.Parse API を Reml で実装し、OCaml 実装と AST を比較 | AST スナップショット比較、`2-7` のストリーミングテスト | Phase 3 開始後 8 週 |
| M2: TypeChecker 移植 | HM/型クラス/効果チェックを Reml で再実装 | 型推論差分テスト、診断比較 | 開始後 16 週 |
| M3: クロスコンパイル実装 | x86_64 Linux/Windows と ARM64 macOS のターゲット対応とプロファイル管理 | マルチターゲット IR 生成テスト、CI マトリクス | 開始後 20 週 |
| M4: CodeGen 移植 | Core/MIR/LLVM IR 生成を Reml で再現、RC 所有権を準拠実装 | IR 比較テスト、`opt -verify` | 開始後 26 週 |
| M5: ランタイム統合 | Reml 実装で最小ランタイムを構築し、Capability/Stage を反映 | Stage テスト、監査ログ比較 | 開始後 30 週 |
| M6: セルフホストビルド | Reml コンパイラ自身を Reml 実装でビルドし、x86_64 Linux バイナリ生成 | ビルド再帰テスト、マルチターゲット検証 | 開始後 34 週 |

## 3.0.4 実装タスク

> **ターゲット方針**: Phase 3 では **x86_64 Linux を主ターゲット**とし、Windows x64 と ARM64 macOS へのクロスコンパイル機能を実装する。セルフホストコンパイラは開発環境（macOS/Linux）で x86_64 Linux バイナリを生成できることを必須条件とする。

1. **Reml Parser 実装**
   - `Core.Parse` を利用したコンパイラフロントエンドを構築し、ストリーミング API (`run_stream`, `FlowController`) をテスト。
   - x86_64 Linux 向けランタイムと統合テストを優先し、CLI 利用フローを検証。
   - Syntax 拡張や DSL 用フック (`4-7-core-parse-plugin.md`) を考慮したプラグインポイントを設計。
2. **TypeChecker 再実装**
   - `Core.Result`/`Core.Option` ベースで推論ワークフローを書き換え、`effect` タグと Capability Stage 判定を再利用。
   - 型クラス辞書生成（またはモノモルフィゼーション、Phase 2 の決定に従う）を Reml の代数的データ型で表現し、性能計測結果を `0-3-audit-and-metrics.md` に反映。
3. **クロスコンパイル機能の実装**
   - **`notes/cross-compilation-spec-update-plan.md` の Phase A〜C を組み込み**:
     - Phase A: `RunConfigTarget` と `@cfg` キー拡張（`1-1-syntax.md`、`2-6-execution-strategy.md` 更新）
     - Phase B: `TargetCapability` グループと `infer_target_from_env` 拡張（`3-10-core-env.md`、`3-8-core-runtime-capability.md` 更新）
     - Phase C: `reml build --target <profile>` の実装、ターゲット別標準ライブラリ配布基盤
   - 主要ターゲット: **x86_64-unknown-linux-gnu**, **x86_64-pc-windows-msvc**, **aarch64-apple-darwin**
   - CI マトリクスで 3 ターゲット全てのビルド・スモークテストを実行。
4. **中間 IR と CodeGen**
   - Reml で Core IR/MIR データ構造を定義し、モノモルフィゼーションを最適化（`notes/llvm-spec-status-survey.md` の未決課題を参照）。
   - LLVM IR 生成で `guides/llvm-integration-notes.md` の型マッピング・ABI を遵守し、ターゲットごとに適切な DataLayout・呼出規約を適用。
5. **ランタイムと Capability**
   - `Core.Runtime` 系モジュールを整理し、Stage/Capability の検証 API (`verify_capability_stage`) を組み込む。
   - ターゲットごとの Capability 差異（例: POSIX vs Windows API）を `TargetCapability` で表現し、FFI ラッパーで吸収。
6. **メモリ管理戦略の評価**
   - RC 継続 vs GC 導入の評価タスクを追加:
     - RC の性能・メモリリーク検出ツールの整備状況を評価
     - GC 候補（例: Boehm GC、自作世代別 GC）の統合コストを見積もり
     - Phase 3 終了時に方針を決定し、`0-4-risk-handling.md` に記録
7. **セルフホストビルドパイプライン**
   - OCaml 実装 → Reml 実装 → Reml 自己ビルドの 3 段階 CI を構築し、**x86_64 Linux ランナー**で検証を優先実施。
   - クロスコンパイルにより、開発環境（macOS/Linux）から全ターゲット向けバイナリを生成できることを確認。
   - ビルド成果物のバイナリ互換性をチェックし、IR/診断ログの差分を自動比較する。
8. **ドキュメントと仕様フィードバック**
   - Reml 実装特有の仕様変更が生じた場合は、関連する言語仕様書を更新し、本計画書から脚注を追加。
   - クロスコンパイル機能により判明した仕様ギャップを `notes/llvm-spec-status-survey.md` に追記し、Phase 4 の改善テーマを明確化する。

## 3.0.5 測定と検証
- **性能比較**: OCaml 実装との性能差を `0-3-audit-and-metrics.md` で可視化。差異が ±10% を超えた場合はフォローアップを起票。
- **IR 一致率**: LLVM IR の差分（関数単位）を比較し、差異の理由をレビューで承認。
- **診断整合**: 代表的なエラーケースで出力が一致するか比較し、差異がある場合は仕様・ガイドのどちらを更新するか決定。
- **マルチターゲット検証**: x86_64 Linux、Windows x64、ARM64 macOS の 3 ターゲット全てで、セルフホスト成果物が正常にビルド・実行されることを CI で確認。
- **クロスコンパイル正確性**: 開発環境（macOS または Linux）から生成したクロスターゲットバイナリが、実際のターゲット環境（実機または VM/エミュレータ）で動作することを検証。

## 3.0.6 リスクとフォローアップ
- **パフォーマンス回帰**: Reml 実装で性能低下が発生した場合は、OCaml 実装を並行維持しつつ最適化タスクを `0-4-risk-handling.md` に登録。
- **メモリ管理戦略の決定遅延**: RC vs GC の評価が Phase 3 終了までに完了しない場合、Phase 4 への影響大。M6 マイルストーン前（開始後 28 週）までに中間評価を実施し、方針を固める。
- **クロスコンパイル機能の工数超過**: Phase 3 は 34 週と長期のため、M3 クロスコンパイル実装を優先し、遅延リスクを早期検出。
- **ツールチェーン複雑化**: Reml 実装 + OCaml 実装の二重管理が複雑になるため、CI 自動化の整備を優先する。マルチターゲット CI マトリクスによる実行時間増加に対応。
- **Phase 3 の前提条件**: x86_64 Linux と Windows x64 の両方が Phase 2 で完了していること。未完了の場合は Phase 3 開始を延期。

---

Phase 3 の完了により、Reml はセルフホスト可能なコンパイラを獲得し、以降のフェーズで完全移行とエコシステム展開を進める基盤が整う。
