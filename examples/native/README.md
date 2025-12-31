# native サンプル

`Core.Native` の intrinsic と埋め込み API を検証するサンプルを収録します。

## サブディレクトリ
- `asm/`: Inline ASM の最小サンプル（`@unstable("inline_asm")` の検証用）
- `intrinsics/`: `sqrt`/`ctpop` など最小の intrinsic 呼び出し
- `embedding/`: `reml_*` 埋め込み API の最小フローと `abi_mismatch`/`unsupported_target` の検証
- `llvm_ir/`: LLVM IR 直書きの最小サンプル（`@unstable("llvm_ir")` の検証用）
- `unstable/`: Inline ASM/LLVM IR の研究プロトタイプ（ビルド不能な PoC を隔離）
