# 3.0 Phase 3 — Self-Host Transition

Phase 3 は Reml 自身で Reml コンパイラを実装するセルフホスト移行段階である。OCaml 実装と Reml 実装を並行稼働させながら、Parser → TypeChecker → CodeGen → Runtime の順に置き換え、互換性と性能を保ちながら段階的に移行する。

## 3.0.1 目的
- Parser/TypeChecker/CodeGen/Runtime を Reml で再実装し、Reml ソースから生成されたバイナリが OCaml 実装と同一の LLVM IR（または差異が許容範囲内）になることを確認する。
- `2-7-core-parse-streaming.md` のストリーミング実行モデル、`3-8-core-runtime-capability.md` の Stage 契約を Reml 実装で満たし、セルフホスト後のランタイムが仕様通りに稼働することを検証する。
- コンパイラ内部 API を標準ライブラリ (`Core.*`) と再利用可能な形で整理し、DSL/プラグイン開発者が参照できるベースラインを整える。

## 3.0.2 スコープ境界
- **含む**: Reml 実装の Parser/TypeChecker/CodeGen/Runtime、セルフホスト用ビルドパイプライン、互換性テスト、CI ワークフロー。
- **含まない**: フル JIT、ガベージコレクション、ARM64 など追加ターゲット対応。これらは Phase 4 以降または別計画で扱う。
- **前提条件**: Phase 2 の成果物と性能ベースライン、`notes/llvm-spec-status-survey.md` における決定済み仕様、`0-3-audit-and-metrics.md` の計測フレーム。

## 3.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Parser 移植 | Core.Parse API を Reml で実装し、OCaml 実装と AST を比較 | AST スナップショット比較、`2-7` のストリーミングテスト | Phase 3 開始後 8 週 |
| M2: TypeChecker 移植 | HM/型クラス/効果チェックを Reml で再実装 | 型推論差分テスト、診断比較 | 開始後 16 週 |
| M3: CodeGen 移植 | Core/MIR/LLVM IR 生成を Reml で再現、RC 所有権を準拠実装 | IR 比較テスト、`opt -verify` | 開始後 24 週 |
| M4: ランタイム統合 | Reml 実装で最小ランタイムを構築し、Capability/Stage を反映 | Stage テスト、監査ログ比較 | 開始後 28 週 |
| M5: セルフホストビルド | Reml コンパイラ自身を Reml 実装でビルドし、OCaml 実装と相互ブートストラップ | ビルド再帰テスト | 開始後 32 週 |

## 3.0.4 実装タスク
1. **Reml Parser 実装**
   - `Core.Parse` を利用したコンパイラフロントエンドを構築し、ストリーミング API (`run_stream`, `FlowController`) をテスト。
   - Apple Silicon (arm64) 向けにビルドされたランタイムと組み合わせた統合テストを追加し、macOS での CLI 利用フローを先行検証する。
   - Syntax 拡張や DSL 用フック (`4-7-core-parse-plugin.md`) を考慮したプラグインポイントを設計。
2. **TypeChecker 再実装**
   - `Core.Result`/`Core.Option` ベースで推論ワークフローを書き換え、`effect` タグと Capability Stage 判定を再利用。
   - 型クラス辞書生成を Reml の代数的データ型で表現し、ARM64 macOS 上でのパフォーマンス計測結果を `0-3-audit-and-metrics.md` に反映する。
3. **中間 IR と CodeGen**
   - Reml で Core IR/MIR データ構造を定義し、モノモルフィゼーションを最適化（`notes/llvm-spec-status-survey.md` の未決課題を参照）。
   - LLVM IR 生成で `guides/llvm-integration-notes.md` の型マッピング・ABI を遵守し、`-target arm64-apple-macos12.0 -mcpu=apple-m1` でのコードパスを優先実装する。
4. **ランタイムと Capability**
   - `Core.Runtime` 系モジュールを整理し、Stage/Capability の検証 API (`verify_capability_stage`) を組み込む。
   - macOS 固有のシステム呼び出し（`pthread`, `dispatch`, `CoreFoundation` など）を FFI ラッパーで扱い、RC 操作を自動化するヘルパを提供。
5. **セルフホストビルドパイプライン**
   - OCaml 実装 → Reml 実装 → Reml 自己ビルドの 3 段階 CI を構築し、最初のセルフホスト検証は Apple Silicon ランナーで実施する。
   - ビルド成果物のバイナリ互換性をチェックし、IR/診断ログの差分を自動比較する。
6. **ドキュメントと最適化フィードバック**
   - Reml 実装特有の仕様変更が生じた場合は、`1-0-language-core-overview.md` や `3-0-core-library-overview.md` を更新し、本計画書から脚注を追加。
   - Apple Silicon 上で観測した最適化課題を `notes/llvm-spec-status-survey.md` に追記し、Phase 4 の性能改善テーマを明確化する。

## 3.0.5 測定と検証
- **性能比較**: OCaml 実装との性能差を `0-3-audit-and-metrics.md` で可視化。差異が ±10% を超えた場合はフォローアップを起票。
- **IR 一致率**: LLVM IR の差分（関数単位）を比較し、差異の理由をレビューで承認。
- **診断整合**: 代表的なエラーケースで出力が一致するか比較し、差異がある場合は仕様・ガイドのどちらを更新するか決定。
- **ターゲット継続性**: Apple Silicon 向けセルフホスト成果物を `codesign --verify` および `otool -L` で検証し、Phase 1/2 の成果物と互換であることを確認。

## 3.0.6 リスクとフォローアップ
- **パフォーマンス回帰**: Reml 実装で性能低下が発生した場合は、OCaml 実装を並行維持しつつ最適化タスクを `0-4-risk-handling.md` に登録。
- **GC/メモリ管理**: Reml 実装ではメモリ管理戦略が変化する可能性があるため、Phase 4 でガベージコレクタ導入案を検討。
- **ツールチェーン複雑化**: Reml 実装 + OCaml 実装の二重管理が複雑になるため、CI 自動化の整備を優先する。
- **Apple ツールチェーン更新**: Xcode/Command Line Tools の更新によりセルフホスト成果物がビルド不能になるリスクがあるため、CI で使用するバージョン固定とロールバック手順を `0-4-risk-handling.md` に記録。

---

Phase 3 の完了により、Reml はセルフホスト可能なコンパイラを獲得し、以降のフェーズで完全移行とエコシステム展開を進める基盤が整う。
