# native/unstable サンプル

このディレクトリは Phase 4 の研究プロトタイプ専用です。`feature = "native-unstable"` と `@cfg(target)` を満たさない限り実行できません。

## 方針
- Inline ASM は構文解析のみを行い、実行コード生成は無効化します。
- LLVM IR 直書きはビルドガードで常時無効化します。
- 実行不能であることを前提に、監査ログの確認のみを目的とします。
- バックエンド側では暫定的に `unstable:inline_asm` / `unstable:llvm_ir` 属性の検出を行います。

## サンプル
- `inline-asm-prototype.reml`: Inline ASM 構文の素振り用（実行不可）
