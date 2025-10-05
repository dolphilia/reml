# 3.3 クロスコンパイル機能実装計画

## 目的
- Phase 3 マイルストーン M3 を達成するため、`notes/cross-compilation-spec-update-plan.md` の Phase A〜C を Reml セルフホスト実装へ組み込み、主要ターゲット (x86_64 Linux/Windows, ARM64 macOS) をサポートする。
- ターゲットプロファイル (`RunConfigTarget`) と `@cfg` キーを整備し、CLI で `reml build --target <profile>` を実行可能にする。

## スコープ
- **含む**: ターゲット構成管理、`TargetCapability` 定義、環境検出、ターゲット別標準ライブラリ配布、CI マトリクス化。
- **含まない**: 新規ターゲット (WASM 等)、高度な最適化。Phase 4 以降で検討。
- **前提**: Phase 2 で Windows x64 サポートが確立し、Phase 1 の x86_64 Linux フローが安定している。

## 作業ブレークダウン

### 1. ターゲットプロファイル設計（51-52週目）
**担当領域**: Phase A - 基本仕様実装

1.1. **RunConfigTarget 定義**
- `notes/cross-compilation-spec-update-plan.md` Phase A に基づくターゲットプロファイル定義
- 3ターゲット: `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`
- プロファイル構造: `{ arch: Arch, os: OS, abi: ABI, features: Vec<Feature> }`
- デフォルトターゲットの決定ロジック（ホスト環境依存）

1.2. **@cfg キー拡張**
- `1-1-syntax.md` への `@cfg` 属性構文追加
- 条件付きコンパイルのパース処理
- ターゲット条件の評価: `@cfg(target_os = "linux")`, `@cfg(target_arch = "x86_64")`
- 複合条件のサポート: `@cfg(all(target_os = "windows", target_arch = "x86_64"))`

1.3. **仕様書更新**
- `1-1-syntax.md` への @cfg 構文追加
- `2-6-execution-strategy.md` へのターゲット実行戦略記載
- サンプルコード追加（ターゲット別実装例）
- 後方互換性の確認

**成果物**: ターゲットプロファイル定義、@cfg 構文仕様、更新済み仕様書

### 2. TargetCapability システム（52-53週目）
**担当領域**: Phase B - 環境推論

2.1. **TargetCapability グループ定義**
- `3-8-core-runtime-capability.md` との統合
- POSIX/Windows/Darwin 固有の Capability 定義
- ターゲット別の Capability 可用性マトリクス
- Stage 要件のターゲット依存性

2.2. **環境推論機能**
- `infer_target_from_env` 関数の実装
- `3-10-core-env.md` に基づく環境変数検出
- ホスト環境の自動検出: OS, Arch, ABI
- フォールバック戦略（検出失敗時のデフォルト）

2.3. **条件付きコンパイル実装**
- @cfg 評価エンジンの実装
- ターゲット条件に基づくコード選択
- 未使用コードの除去（dead code elimination）
- 診断メッセージの統合

**成果物**: `Core.Target.Capability` モジュール、環境推論、条件付きコンパイル

### 3. ビルドコマンド拡張（53-54週目）
**担当領域**: Phase C - CLI統合

3.1. **CLI 引数処理**
- `reml build --target <profile>` の実装
- ターゲットプロファイルの解析とバリデーション
- 複数ターゲットの同時ビルド: `--target linux,windows,macos`
- ターゲット別出力ディレクトリ管理

3.2. **ターゲット固有ビルド処理**
- ターゲット別の LLVM TargetTriple 設定
- DataLayout の切り替え（x86_64 vs ARM64）
- 呼出規約の選択（System V vs Windows x64 vs ARM64）
- リンカオプションの調整

3.3. **標準ライブラリのバンドル**
- ターゲット別標準ライブラリの選択
- ランタイムライブラリのリンク
- プラットフォーム固有API のバインディング
- バイナリ成果物の構成

**成果物**: 拡張 CLI、ターゲット別ビルド処理、ライブラリバンドル機構

### 4. LLVM バックエンド統合（54-55週目）
**担当領域**: コード生成

4.1. **ターゲット別 IR 生成**
- LLVM TargetMachine の初期化（3ターゲット対応）
- ターゲット固有の最適化パス
- DataLayout に基づく型サイズ調整
- アライメント要件の適用

4.2. **ABI 対応**
- System V ABI (x86_64 Linux)
- Windows x64 ABI (MSVC互換)
- ARM64 ABI (macOS)
- 構造体レイアウトの調整

4.3. **リンカ統合**
- ld (Linux), lld (Windows), ld64 (macOS) の選択
- プラットフォーム固有のリンカフラグ
- 依存ライブラリの解決
- 実行可能ファイルの生成

**成果物**: ターゲット別 LLVM IR 生成、ABI 対応、リンカ統合

### 5. ターゲット別標準ライブラリ（55-56週目）
**担当領域**: ライブラリ配布

5.1. **ライブラリビルドシステム**
- 3ターゲット用の標準ライブラリビルド
- クロスコンパイル環境の整備
- ビルド成果物の検証（シンボル、依存関係）
- アーティファクトの圧縮と配布準備

5.2. **プラットフォーム固有実装**
- POSIX システムコール (Linux)
- Win32 API (Windows)
- Darwin 固有 API (macOS)
- FFI ブリッジの実装

5.3. **ライブラリ配布管理**
- ターゲット別アーティファクトの命名規則
- バージョン管理と署名
- ダウンロードとキャッシュ機構
- 依存解決とリンク

**成果物**: ターゲット別標準ライブラリ、配布システム、FFI ブリッジ

### 6. CI マトリクス構築（56-57週目）
**担当領域**: 継続的統合

6.1. **GitHub Actions マトリクス**
- 3ターゲット × ビルド/テストのマトリクス定義
- ホストランナー: Ubuntu (Linux), Windows, macOS
- クロスコンパイルジョブの設定
- アーティファクトのアップロード

6.2. **スモークテスト**
- ターゲット別の基本動作テスト
- Hello World の実行確認
- 標準ライブラリの基本機能テスト
- 実機/VM/エミュレータでの検証

6.3. **CI 最適化**
- ビルドキャッシュ戦略（LLVM, 標準ライブラリ）
- 並列実行の最適化
- タイムアウト設定（ターゲット別）
- 失敗時のログ収集とレポート

**成果物**: CI マトリクス、スモークテスト、キャッシュ戦略

### 7. 検証とテスト（57-58週目）
**担当領域**: 品質保証

7.1. **クロスコンパイル検証**
- 開発環境（macOS/Linux）から3ターゲットへのビルド
- 生成バイナリの実機/VM での動作確認
- ABI 互換性テスト
- ライブラリリンクの検証

7.2. **統合テスト**
- マルチターゲットビルドの自動テスト
- ターゲット切り替えの正確性検証
- 条件付きコンパイルの動作確認
- エッジケース（未対応ターゲット等）

7.3. **性能計測**
- ターゲット別のビルド時間計測
- クロスコンパイルオーバーヘッドの測定
- 生成バイナリサイズの比較
- `0-3-audit-and-metrics.md` への記録

**成果物**: 検証レポート、統合テスト、性能計測

### 8. ドキュメント整備とレビュー（58週目）
**担当領域**: ドキュメント

8.1. **技術文書更新**
- `3-0-phase3-self-host.md` へのクロスコンパイル詳細追記
- ターゲット別ビルドガイドの作成
- トラブルシューティングガイド
- 開発者向けクロスコンパイルのベストプラクティス

8.2. **仕様書反映**
- `1-1-syntax.md` への @cfg 構文最終化
- `3-10-core-env.md` への環境推論機能追記
- `3-8-core-runtime-capability.md` への TargetCapability 追記
- サンプルコードの充実

8.3. **レビュー資料作成**
- M3 マイルストーン達成報告書
- 3ターゲット対応の実証デモ
- 既知の制限事項と TODO リスト
- Phase 3 次タスク（3-4 CodeGen）への引き継ぎ事項

**成果物**: 完全なドキュメント、更新仕様書、レビュー資料

## 成果物と検証
- `reml build --target <profile>` が3ターゲット全てで成功し、生成物が実機または VM で動作すること。
- CI マトリクスが安定稼働し、失敗時はターゲットごとのログが参照可能であること。
- 仕様書（`1-1-syntax.md`, `3-10-core-env.md`, `3-8-core-runtime-capability.md`）が最新状態で、差分が `0-3-audit-and-metrics.md` に記録されること。
- クロスコンパイルのドキュメントが整備され、開発者が再現可能であること。

## リスクとフォローアップ
- ターゲットごとの依存ライブラリが膨大になる可能性があるため、キャッシュ戦略を設計し CI 時間を抑制。
- macOS notarization 等の外部手続きは Phase 4 リリースパイプライン（4-2）で本格対応するため、準備状況を `0-4-risk-handling.md` に記録。
- 環境検出ロジックが複雑になる際は、`guides/runtime-bridges.md` と連携してメンテを容易にする。
- ARM64 macOS の実機テストが困難な場合、エミュレータまたは CI ホストランナーを活用し、制限事項を明記。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [notes/cross-compilation-spec-update-plan.md](../../notes/cross-compilation-spec-update-plan.md)
- [notes/cross-compilation-spec-intro.md](../../notes/cross-compilation-spec-intro.md)
- [1-1-syntax.md](../../1-1-syntax.md)
- [2-6-execution-strategy.md](../../2-6-execution-strategy.md)
- [3-10-core-env.md](../../3-10-core-env.md)
- [3-8-core-runtime-capability.md](../../3-8-core-runtime-capability.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
