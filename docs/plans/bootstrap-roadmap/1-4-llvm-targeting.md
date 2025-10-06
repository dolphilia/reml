# 1.4 LLVM IR 生成とターゲット設定計画

## 目的
- Phase 1 マイルストーン M3 において LLVM IR を確実に生成し、x86_64 Linux (System V ABI) を既定ターゲットとしてコンパイルできるようにする。
- `docs/guides/llvm-integration-notes.md` §5 の ABI 要件と `docs/notes/llvm-spec-status-survey.md` のギャップを解消し、後続フェーズでのマルチターゲット化に備える。

## スコープ
- **含む**: LLVM IR ビルダーの実装、データレイアウト (`e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64`) 設定、関数シグネチャ/ABI マッピング、x86_64 Linux 用ランタイムシンボルとのリンケージ。
- **含まない**: Windows/MSVC や ARM64 向けのターゲットコード生成、JIT 実行、デバッグ情報の高度な最適化。
- **前提**: Core IR が安定しており、型情報が LLVM 型へマッピングできる状態であること。

## 作業ディレクトリ
- `compiler/ocaml/src/codegen`（想定）: LLVM ビルダー、型マッピング、ABI 実装
- `compiler/ocaml/tests/codegen` : IR ゴールデンテスト、`opt`/`llc` 検証結果
- `runtime/native` : ランタイム関数のシンボル確認、ABI 整合
- `tooling/ci` / `.github/workflows/` : LLVM ツールチェーンを利用するビルド/テストフロー
- `docs/notes/llvm-spec-status-survey.md` : ターゲット差分やデータレイアウト情報の記録

## 作業ブレークダウン

### 1. LLVM基盤セットアップ（13週目）
**担当領域**: LLVM統合環境の構築

1.1. **LLVMバインディング選定**
- OCaml LLVM bindings（llvm-ocaml）の調査とインストール
- バージョン固定（LLVM 15以上）と互換性確認
- `opam` 依存関係の設定

1.2. **ビルドシステム統合**
- `dune` でのLLVMライブラリリンク設定
- プラットフォーム別の依存解決スクリプト
- ビルド時のLLVM存在チェック

1.3. **開発環境ドキュメント**
- LLVM インストール手順の文書化
- macOS/Linux での環境構築ガイド
- トラブルシューティング情報の追加

**成果物**: LLVM統合済みビルド環境、環境構築ドキュメント

### 2. 型マッピングシステム（13-14週目）
**担当領域**: Reml型とLLVM型の対応付け

2.1. **プリミティブ型マッピング**
- 整数族（`i8/i16/i32/i64/isize`、`u8/u16/u32/u64/usize`）→ `i8/i16/i32/i64` 等 サイズに応じた LLVM 整数型
- 浮動小数（`f32`, `f64`）→ `float`, `double`
- `Bool` → `i1`
- `Char` → `i32`
- `String`/スライス → `{ ptr, i64 }`
- `unit` → `void` の特殊処理

2.2. **複合型マッピング**
- タプル → 構造体（フィールド順序保証）
- レコード → 名前付き構造体
- 配列・スライス → 固定長配列 / `{ptr,len}`
- 関数型 → 関数ポインタ型（クロージャは `{env_ptr, code_ptr}`）
- 代数的データ型 → `{i32 tag, payload}` のタグ付きユニオン表現

2.3. **型マッピング表の実装**
- `docs/guides/llvm-integration-notes.md` §4のマッピング表をコード化
- 型変換関数 `reml_type_to_llvm: Type -> lltype`
- マッピングの単体テスト（全プリミティブ型網羅）

**成果物**: `llvm_gen/type_mapping.ml`, 型マッピングテスト

### 3. DataLayoutとターゲット設定（14週目）
**担当領域**: ターゲット固有の設定

3.1. **DataLayout定義**
- x86_64 System V ABI用レイアウト文字列
  - `e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64`
- アラインメント要件の明示
- エンディアン設定（リトルエンディアン）

3.2. **TargetMachine設定**
- ターゲットトリプル: `x86_64-unknown-linux-gnu`
- CPU設定: `generic`（最大互換性）
- Feature文字列の設定（SSE2等）

3.3. **CLI統合**
- `--target` フラグの実装（デフォルト: x86_64-linux）
- ターゲット別DataLayoutの切り替え機構
- 検証用のターゲット情報出力

**成果物**: `llvm_gen/target_config.ml`, ターゲット設定

### 4. LLVM IRビルダー実装（14-15週目）
**担当領域**: Core IRからLLVM IRへの変換

4.1. **モジュール・関数生成**
- LLVMモジュールの初期化
- 関数宣言の生成（シグネチャ、リンケージ）
- グローバル変数の生成

4.2. **基本ブロック生成**
- Core IRのCFGをLLVMブロックへマッピング
- ラベルとブランチ命令の生成
- φノードの生成（SSA形式）

4.3. **式・命令生成**
- 算術・論理演算のLLVM命令生成
- 関数呼び出し命令（System V calling convention）
- メモリアクセス（load/store）命令

**成果物**: `llvm_gen/codegen.ml`, IR生成コア

### 5. ABI・呼び出し規約の実装（15週目）
**担当領域**: System V ABI準拠

5.1. **引数渡し規約**
- LLVM の `cc ccc` 呼び出し規約に委譲し、`llvm::TargetMachine` による ABI 処理を活用
- 構造体引数・戻り値は LLVM 属性（`sret`, `byval` 等）で表現
- System V / Windows で差異が出る箇所は `llvm::DataLayout` から取得し、手動でのレジスタ割り当てを避ける

5.2. **戻り値規約**
- LLVM の ABI 情報から戻り値ハンドリング（レジスタ/メモリ戻り）を取得
- 構造体戻り値は `sret` 属性を使い、タプル等の複数値は IR 上の構造体として返却

5.3. **ランタイム関数宣言**
- `mem_alloc`, `inc_ref`, `dec_ref`, `panic` のシグネチャ
- 外部リンケージ設定
- 属性付与（noreturn, nounwindなど）

**成果物**: `llvm_gen/abi.ml`, ABI準拠テスト

### 6. LLVM IR検証パイプライン（15-16週目）
**担当領域**: 生成IR品質保証

6.1. **検証スクリプト作成**
- `llvm-as`（アセンブル）の実行
- `opt -verify`（検証パス）の実行
- `llc`（コード生成）の実行

6.2. **エラー診断統合**
- LLVM検証エラーのパース
- Span情報へのマッピング（逆変換）
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) 形式での報告

6.3. **CI統合**
- GitHub Actions での検証パイプライン実行
- 失敗時のLLVM IRアーティファクト保存
- 検証結果の `0-3-audit-and-metrics.md` への記録

**成果物**: `scripts/verify_llvm_ir.sh`, CI検証ジョブ

### 7. `--emit-ir` 出力機能（16週目）
**担当領域**: IRダンプとデバッグ支援

7.1. **IR出力フォーマット**
- LLVM IR テキスト形式（`.ll`）出力
- ビットコード形式（`.bc`）出力（オプション）
- 最適化前後の両IR保存

7.2. **人間可読化**
- 変数名の保持（デバッグモード）
- コメント挿入（Core IRノードとの対応）
- インデント・フォーマット調整

7.3. **CLI統合**
- `--emit-ir` フラグ実装
- 出力先ディレクトリ指定（`--out-dir`）
- ファイル名規則の設定

**成果物**: `--emit-ir` CLI機能、IR出力例

### 8. テストとドキュメント（16週目）
**担当領域**: 品質保証と文書化

8.1. **ゴールデンテスト**
- 代表的なRemlコードのLLVM IR期待値
- 関数呼び出し、条件分岐、ループのIR検証
- スナップショット比較（`dune runtest`）

8.2. **統合テスト**
- Core IR → LLVM IR → 実行可能バイナリの一気通貫
- `examples/language-impl-comparison/` での検証
- 性能計測（コンパイル時間、生成IRサイズ）

8.3. **技術文書整備**
- LLVM IR生成アーキテクチャの解説
- 型マッピング・ABI仕様の文書化
- M3マイルストーン達成報告
- Windows/ARM64対応のTODO（Phase 2）

**成果物**: 完全なテストスイート、LLVM統合ドキュメント

## 成果物と検証
- LLVM IR のゴールデンテストを `tests/llvm-ir/` に追加し、代表例（関数呼び出し、条件分岐、ループ）で期待する出力を固定。
- CI で x86_64 Linux 用のクロスコンパイルを実行し、`llc` と `clang` を通して実行可能バイナリを生成。
- DataLayout とターゲットトリプルが `0-3-audit-and-metrics.md` に登録され、逸脱が報告されるようにする。

## リスクとフォローアップ
- LLVM バージョン差異により API が変化する可能性があるため、`0-3-audit-and-metrics.md` にバージョン固定と検証手順を追記。
- クロスコンパイルを macOS 上で実行する場合の依存解決が複雑なため、Docker/Podman のベースイメージ構成を `docs/notes/llvm-spec-status-survey.md` に共有。
- 生成 IR とランタイム ABI の不整合が検出された場合は、`0-4-risk-handling.md` に即時登録し、Phase 2 の Windows 対応タスクで流用できるようにする。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
