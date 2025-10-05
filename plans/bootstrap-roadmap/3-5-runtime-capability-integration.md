# 3.5 ランタイムと Capability 統合計画

## 目的
- Phase 3 マイルストーン M5 で、Reml 実装のランタイムを構築し、`3-8-core-runtime-capability.md` の Stage 契約を反映する。
- ターゲットごとの Capability 差異 (POSIX/Windows/macOS) を `TargetCapability` モデルで吸収し、セルフホスト後のランタイムが仕様通りに動作するよう検証する。

## スコープ
- **含む**: Reml ランタイムモジュール (`Core.Runtime` 系) の実装、Stage/Capability 検証 API、FFI ラッパ、環境検出、監査ログ連携。
- **含まない**: 高度なスケジューラ、マルチスレッド実行、JIT 対応。必要に応じて Phase 4 以降。
- **前提**: Phase 2 の効果システム・FFI 拡張が安定し、クロスコンパイルでターゲット別成果物が生成できる。

## 作業ブレークダウン

### 1. Core.Runtime モジュール設計（67-68週目）
**担当領域**: ランタイム基盤

1.1. **ランタイムAPI整理**
- Phase 1 の C ランタイムを Reml から制御できる API に整理
- メモリ管理: `alloc`, `dealloc`, `rc_inc`, `rc_dec`
- 例外処理: `panic`, `catch`, `rethrow`
- 基本I/O: `print`, `read_line`, `open_file`
- システム情報: `get_env`, `current_time`

1.2. **Capability 判定フック**
- 各 API に Capability チェックを挿入
- `verify_capability_stage` の呼び出しポイント決定
- Stage ミスマッチ時のエラー処理
- 実行時 vs コンパイル時判定の分離

1.3. **モジュール構造**
- `Core.Runtime.Memory`: メモリ管理
- `Core.Runtime.IO`: 入出力
- `Core.Runtime.System`: システム情報・環境変数
- `Core.Runtime.Capability`: Capability 検証

**成果物**: `Core.Runtime.*` モジュール、API仕様書

### 2. Stage 検証システム（68-69週目）
**担当領域**: Stage 契約実装

2.1. **verify_capability_stage 実装**
- `3-8-core-runtime-capability.md` §3.3 の仕様に準拠
- Stage 判定ロジック: `Experimental` < `Preview` < `Stable`
- Capability と Stage のマッピング
- コンパイル時チェック（型システムとの統合）

2.2. **実行時検証**
- 実行時 Capability チェックの実装
- Stage ミスマッチ時のエラーメッセージ
- デバッグモードでの詳細トレース
- リリースビルドでの最適化（チェック省略オプション）

2.3. **型システムとの統合**
- 3-2 の効果システムとの連携
- Effect と CapabilityStage の対応
- 型レベルでの Stage 検証
- 診断メッセージの生成

**成果物**: `verify_capability_stage` 実装、型統合、検証テスト

### 3. TargetCapability 実装（69-70週目）
**担当領域**: ターゲット別対応

3.1. **ターゲット別 Capability 定義**
- POSIX Capabilities (Linux): ファイルシステム、プロセス、シグナル
- Windows Capabilities: Win32 API、レジストリ、サービス
- Darwin Capabilities (macOS): Mach API、Grand Central Dispatch
- 共通 Capability の抽出

3.2. **条件付きコンパイル統合**
- 3-3 の @cfg 機構との連携
- ターゲット別コードの選択
- `@cfg(target_os = "linux")` での Capability 切り替え
- 未サポートターゲットでのコンパイルエラー

3.3. **Capability 可用性マトリクス**
- ターゲット × Capability の可用性表
- Stage 要件のターゲット依存性
- ドキュメント化（表形式）
- 実行時クエリAPI: `is_capability_available`

**成果物**: `Core.Target.Capability` 拡張、可用性マトリクス

### 4. FFI ラッパ実装（70-71週目）
**担当領域**: プラットフォーム統合

4.1. **POSIX FFI ラッパ**
- システムコール: `open`, `read`, `write`, `close`
- プロセス管理: `fork`, `exec`, `wait`
- シグナル: `signal`, `kill`
- 所有権モデルとの統合（ファイルディスクリプタ）

4.2. **Windows FFI ラッパ**
- Win32 API: `CreateFile`, `ReadFile`, `WriteFile`, `CloseHandle`
- プロセス: `CreateProcess`, `WaitForSingleObject`
- レジストリ: `RegOpenKey`, `RegQueryValue`
- Unicode (UTF-16) との変換

4.3. **Darwin FFI ラッパ**
- BSD システムコール（POSIX互換）
- Mach API の限定的サポート
- Core Foundation のブリッジ
- Objective-C ランタイムの基本対応

**成果物**: FFI ラッパモジュール、ターゲット別テスト

### 5. 監査ログ連携（71-72週目）
**担当領域**: 診断・監査

5.1. **AuditEnvelope 統合**
- `3-6-core-diagnostics-audit.md` の AuditEnvelope 形式対応
- Stage 判定結果の記録
- Capability 使用状況のログ
- タイムスタンプとコンテキスト情報

5.2. **監査ログ出力**
- JSON 形式での出力
- CLI フラグ: `--audit-log <file>`
- ログレベルの設定（INFO/WARN/ERROR）
- 構造化ログの活用

5.3. **分析ツール**
- 監査ログの解析ツール
- Stage ミスマッチの検出
- Capability 使用統計
- レポート生成（HTML/Markdown）

**成果物**: 監査ログシステム、分析ツール

### 6. テスト整備（72-73週目）
**担当領域**: 品質保証

6.1. **Stage/Capability テスト**
- 各 Capability の動作確認テスト
- Stage 違反の検出テスト
- ターゲット別テストケース
- エッジケース（未サポート Capability 等）

6.2. **CI マトリクス統合**
- 3-3 のマルチターゲット CI への統合
- 3ターゲット全てでの実行
- 監査ログの自動検証
- 失敗時のデバッグ情報収集

6.3. **統合テスト**
- ランタイム全体の統合テスト
- FFI 呼び出しのテスト
- 効果システムとの整合テスト
- 性能計測（ランタイムオーバーヘッド）

**成果物**: テストスイート、CI統合、性能レポート

### 7. 環境検出とフォールバック（73週目）
**担当領域**: 堅牢性

7.1. **環境検出強化**
- `3-10-core-env.md` の機能活用
- OS バージョンの検出
- 利用可能な Capability の実行時検出
- 機能フラグの設定

7.2. **フォールバック戦略**
- Capability 未サポート時の代替実装
- グレースフルデグラデーション
- ユーザーへの警告メッセージ
- 機能制限モードの提供

7.3. **エラーハンドリング**
- Capability エラーの詳細メッセージ
- リカバリ手順の提示
- ログへの記録
- ユーザーフレンドリーなエラー表示

**成果物**: 環境検出、フォールバック実装

### 8. ドキュメント整備とレビュー（73-74週目）
**担当領域**: ドキュメント

8.1. **技術文書更新**
- `3-0-phase3-self-host.md` へのランタイム詳細追記
- ランタイム API リファレンスの作成
- Capability 使用ガイド
- トラブルシューティング

8.2. **仕様書反映**
- `3-8-core-runtime-capability.md` の実装詳細追記
- `guides/runtime-bridges.md` の更新
- Capability マトリクスの文書化
- サンプルコードの充実

8.3. **レビュー資料作成**
- M5 マイルストーン達成報告書
- Stage/Capability 動作デモ
- 監査ログサンプル
- Phase 3 次タスク（3-6 メモリ管理評価）への引き継ぎ

**成果物**: 完全なドキュメント、更新仕様書、レビュー資料

## 成果物と検証
- `Core.Runtime.*` モジュール群が実装され、CI でランタイムテストが通過すること。
- 各ターゲット（x86_64 Linux/Windows, ARM64 macOS）で Stage/Capability テストが通過し、監査ログに差分が無いこと。
- ランタイム API が Reml で利用でき、セルフホストビルドが実行時に問題なく動作する。
- 仕様ドキュメント（`3-8-core-runtime-capability.md`, `guides/runtime-bridges.md`）がアップデートされ、レビュー済みであること。

## リスクとフォローアップ
- ターゲットごとの条件分岐が増え複雑化する可能性があるため、モジュール分割と自動生成を検討。FFI ラッパの自動生成ツールの導入を評価。
- Stage 要件が未定義の Capability が出現した場合、`0-4-risk-handling.md` へ登録し、仕様策定プロセスを開始。
- 将来の拡張 (WASM/WASI 等) を見据え、Capability モデルの拡張余地を残す。抽象化レイヤの設計を文書化。
- 実行時 Capability チェックのオーバーヘッドが問題になる場合、最適化オプション（チェック省略）を提供し、セキュリティとのトレードオフを明示。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [3-8-core-runtime-capability.md](../../3-8-core-runtime-capability.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)
- [3-10-core-env.md](../../3-10-core-env.md)
- [1-3-effects-safety.md](../../1-3-effects-safety.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [3-3-cross-compilation.md](3-3-cross-compilation.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
