# 6.0 Phase 6 — 移行完了と運用体制

Phase 6 は Reml セルフホスト実装（Rust 版）を正式版として採用し、OCaml 実装を参照専用のアーカイブへ移行する段階である。互換性確認、リリース体制、エコシステム支援を整え、Reml 開発者コミュニティがセルフホスト版のみを前提に活動できるようにする。

> **注記**: 旧 Phase 4 の計画書（例: `4-1-multitarget-compatibility-verification.md`）は Phase 6 への移行に合わせて `6-x` 系へリネーム済みである（例: `6-1-multitarget-compatibility-verification.md`）。過去の議事録やログでは旧名称が使われているため、本書では必要に応じて「旧 4-x（現 6-x）」という形で併記する。

## 6.0.1 目的
- Reml セルフホスト実装の出力が Phase 3 で確立した仕様ベースのゴールデンテスト・監査メトリクスと一致することを証明し、**x86_64 Linux を正式版のターゲット**として正式なリリースパイプラインへ組み込む。
- [5-3-developer-toolchain.md](../../spec/5-3-developer-toolchain.md) に沿ったツールチェーン整備（パッケージ管理、CI/CD、診断集計）を完了させる。
- OCaml 実装は LTS 運用を行わず、参照用コードとしてアーカイブする。文書には参考リンクを残しつつ、CI/配布/dual-write から完全に切り離す。

## 6.0.2 スコープ境界
- **含む**: 出力一致検証、ローリングリリース戦略、ドキュメント更新、エコシステム（パッケージ、プラグイン、CI テンプレート）への周知、**マルチターゲット（x86_64 Linux/Windows + ARM64 macOS）の正式サポート確立**。
- **含まない**: 追加ターゲット（WASM/WASI/他アーキテクチャ）の正式サポート、JIT/最適化高度化。これらは別計画または次期フェーズへ引き継ぐ。
- **前提条件**: Phase 3 のセルフホスト成果（マルチターゲット対応完了）、`0-3-audit-and-metrics.md` での性能/診断ベンチマーク、`0-4-risk-handling.md` の未解決リスク一覧。

## 6.0.2a 作業ディレクトリ
- `tooling/ci`, `.github/workflows/` : マルチターゲット CI と成果物検証
- `tooling/release` : 署名・notarization・配布スクリプト
- `compiler/ocaml/`, `runtime/native` : セルフホスト成果物の最終ビルド
- `docs/spec/`, `docs/guides/`, `docs/notes/` : ドキュメント更新とリスク記録
- `examples/` : 出力比較・回帰テストで使用するサンプル

## 6.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: 出力一致サインオフ | LLVM IR / バイナリ / 診断の差分が承認閾値内に収束 | 自動差分レポート + レビュア承認記録 | Phase 6 開始後 6 週 |
| M2: リリースパイプライン | セルフホスト実装を CI/CD に組み込み、署名付き成果物を配布 | CI 成果物レビュー、署名確認 | 開始後 10 週 |
| M3: エコシステム移行 | パッケージマネージャ、プラグイン、ガイドをセルフホスト前提に更新 | `README.md` / ガイド更新、コミュニティ告知 | 開始後 14 週 |
| M4: 旧実装アーカイブ | OCaml 実装を参照専用ブランチへ移行し、セルフホスト版のみを配布対象とするアナウンス発行 | `docs/guides/` 追加資料、リスク確認 | 開始後 18 週 |

## 6.0.4 実装タスク

> **ターゲット方針**: Phase 6 の正式リリースは **x86_64 Linux を第一ターゲット**とし、Windows x64 と ARM64 macOS を公式サポート対象として同時リリースする。配布優先度は Linux > Windows > macOS とする。

1. **マルチターゲット互換性検証**
   - LLVM IR の構造差分（関数単位）を**x86_64 Linux、Windows x64、ARM64 macOS の 3 ターゲット全て**で比較し、差分がある場合は `docs/notes/backend/llvm-spec-status-survey.md` の未決項目に照らして承認または修正。
   - **x86_64 Linux (ELF) 成果物を基準**にし、`llvm-diff` と `dwarfdump` でデバッグ情報・シンボル整合を確認する。
   - Windows (PE) と macOS (Mach-O) の成果物も同様に検証し、ターゲット固有の差異は許容範囲として記録。
   - 診断メッセージの差分を比較し、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) のキーセットに準拠するか確認。
2. **マルチターゲットリリースパイプライン構築**
   - [5-3-developer-toolchain.md](../../spec/5-3-developer-toolchain.md) の手順に従い、ビルド→テスト→署名→配布までを自動化。
   - **3 ターゲット全てのアーティファクト生成を CI で自動化**:
     - x86_64 Linux: `.tar.gz` + 署名
     - Windows x64: `.zip` + オプションでコードサイニング
     - ARM64 macOS: `.tar.gz` + Apple notarization (`codesign`/`notarytool`)
   - 公式リリースノートフォーマット（[5-5-roadmap-metrics.md](../../spec/5-5-roadmap-metrics.md) 参照）でセルフホスト移行の進捗を報告。
3. **ドキュメント更新**
   - `README.md` と [0-0-overview.md](../../spec/0-0-overview.md) にセルフホスト完了と **x86_64 Linux を正式版の第一ターゲット**とする旨を明記。
   - Windows x64 と ARM64 macOS も公式サポート対象として併記し、ダウンロードリンクを提供。
   - `docs/guides/compiler/llvm-integration-notes.md` をアップデートし、Phase 6 以降の拡張計画（WASM/WASI/JIT 等）を付録化。
4. **エコシステム整備**
   - パッケージレジストリ ([5-2-registry-distribution.md](../../spec/5-2-registry-distribution.md)) にセルフホスト版を登録し、**3 ターゲット全てのバイナリを配布**。x86_64 Linux を推奨ターゲットとして明示。OCaml 版を非推奨マーク。
   - プラグイン開発ガイド (`docs/guides/dsl/DSL-plugin.md`) をセルフホスト ABI 前提に更新し、マルチターゲット対応の注意点を追加。
5. **後方互換チェックリストの定義と実施**
   - 後方互換チェックリストを `0-3-audit-and-metrics.md` に追加し、以下を含める:
     - Phase 3 のゴールデンテストと比較した LLVM IR・診断キー・監査ログの一致率（関数単位で 95% 以上／診断キー 100%）
     - 性能差異（Phase 3 ベースライン比 ±5% 以内）
     - 3 ターゲット全てでのエンドツーエンドテスト通過
   - チェックリスト通過後、OCaml 実装を `archive/ocaml-reference` ブランチとして保管し、ドキュメントには参照リンクのみを残す。
6. **サポートポリシー策定**
   - セルフホスト版のみを対象とするサポート期間・更新フローを `0-4-risk-handling.md` に記録し、コミュニティへ周知（OCaml 実装は非サポート扱い）。
   - **マルチターゲット対応の優先度**: Linux > Windows > macOS。新機能はまず Linux で検証し、Windows/macOS へ展開。

## 6.0.5 測定と検証
- **性能リグレッション**: Phase 3 のベースラインと比較して ±5% 以内を目標とする（3 ターゲット全て）。超過時は緩和策を `0-4-risk-handling.md` へ登録。
- **Stage/Capability**: すべての Stage 要件が `Stable` 以上に昇格し、監査ログにミスマッチが無いことを確認。
- **診断整合**: 主要エラーケースでのメッセージ差分がゼロ、またはレビュア承認済みの改善のみであること。
- **マルチターゲットリリース検証**:
  - x86_64 Linux: 署名検証、主要ディストリビューション（Ubuntu/Debian/Fedora）での動作確認
  - Windows x64: コードサイニング（オプション）、Windows 10/11 での動作確認
  - ARM64 macOS: Apple Notary Service での notarization (`notarytool submit --wait`)、Gatekeeper 通過確認

## 6.0.6 リスクとフォローアップ
- **エコシステム乗り換え遅延**: サードパーティが移行に遅延した場合に備え、Rust セルフホスト版への移行手順とフォールバックガイドを用意（OCaml 実装へのパッチ提供は行わない）。
- **ツール互換性**: 既存 IDE/CI 連携が想定と異なる場合は、ガイドにトラブルシュートを追加し、`docs/notes/` に TODO ノートを残す。
- **人員負荷**: 移行期のレビュー負荷が高いため、レビュアの割当と休日計画を `0-3-audit-and-metrics.md` のレビュー欄で可視化する。
- **マルチターゲット配布の複雑化**: 3 ターゲット全てのリリース準備により工数増加。CI 自動化を徹底し、手動作業を最小化。
- **ターゲット固有の問題**: 各ターゲットで固有の問題（例: macOS notarization 失敗、Windows Defender 誤検知）が発生した場合の修正フローとリトライ SLA を `0-4-risk-handling.md` に明記する。

---

Phase 6 の完了により、Reml プロジェクトはセルフホスト実装を正式な開発基盤として採用し、以降の最適化・ターゲット拡張・エコシステム成長にフォーカスできる体制が整う。
