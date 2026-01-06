# 調査メモ: 第11章 LLVMへのラウアリング

## 対象モジュール

- `compiler/backend/llvm/src/codegen.rs`
- `compiler/backend/llvm/src/type_mapping.rs`
- `compiler/backend/llvm/src/ffi_lowering.rs`
- `compiler/backend/llvm/src/bridge_metadata.rs`
- `compiler/backend/llvm/src/verify.rs`
- `compiler/backend/llvm/src/target_machine.rs`
- `compiler/backend/llvm/src/integration.rs`

## 入口と全体像

- エントリは `integration.rs` の `generate_snapshot`。`CodegenContext::new` でターゲット・型マップ・FFI ロワリングを組み立て、`emit_function` で MIR 関数を順にラウアリングし、`Verifier` で監査・診断をまとめる。`BackendDiffSnapshot` には LLVM 風 IR・診断・監査ログ・Bridge メタデータが詰められる。
  - `compiler/backend/llvm/src/integration.rs:1289-1333`

## データ構造

- **LLVM 内部名の正規化**: `sanitize_llvm_ident`/`sanitize_llvm_symbol` が非 ASCII を `_uXXXX` へ変換し、先頭数字を `_` で補正する。構文仕様の「バックエンド内部名」ルールと整合させる必要がある。
  - `compiler/backend/llvm/src/codegen.rs:46-82`
  - `docs/spec/1-1-syntax.md:25-34`
- **MIR 表現**: `MirFunction` が引数型・戻り型・呼出規約・属性・FFI 呼び出し・式ツリーを保持。`GeneratedFunction` が LLVM 風 IR 結果を保持する。
  - `compiler/backend/llvm/src/codegen.rs:566-670`
- **TypeLayout**: `TypeMappingContext::layout_of` が Reml 型を LLVM 風型記述へ丸める。`RowTuple`・`Adt` のサイズ計算や配列のオーバーフロー検知 TODO がある。
  - `compiler/backend/llvm/src/type_mapping.rs:3-154`
- **FFI ロワリング**: `FfiCallSignature`/`LoweredFfiCall`/`FfiStubPlan` が呼出規約・ABI・所有権・ターゲット情報を保持し、監査タグを生成する。Apple Darwin では Register Save Area を付加。
  - `compiler/backend/llvm/src/ffi_lowering.rs:4-193`
- **Bridge メタデータ**: `BridgeMetadataContext` が `reml.bridge.version=1` と `reml.bridge.stubs[...]` を構築し、監査ログにも転写できる。
  - `compiler/backend/llvm/src/bridge_metadata.rs:3-172`
  - `docs/spec/3-9-core-async-ffi-unsafe.md:1388`
- **TargetMachine**: `Triple` と `DataLayoutSpec` が LLVM 互換情報の核。`TargetMachineBuilder` が ABI と DataLayout を確定する。
  - `compiler/backend/llvm/src/target_machine.rs:5-250`

## コアロジック

- **CodegenContext の組み立て**: `TargetMachine` から `TypeMappingContext` と `FfiLowering` を派生し、`BridgeMetadataContext` を初期化する。
  - `compiler/backend/llvm/src/codegen.rs:1131-1175`
- **関数ラウアリング**:
  - 返り値レイアウト算出 → FFI 呼び出しをロワリングし Bridge メタデータを記録。
  - `@intrinsic`/`@unstable` 属性を解析して監査用の使用記録を集計。
  - Inline ASM/LLVM IR 直書きを収集し、テンプレートハッシュやプレースホルダ検証に備える。
  - `match` 由来の分岐計画とブロック生成（`BasicBlock`/`LlvmBlock`）を組み立て、`LlvmIrBuilder` で LLVM 風 IR をレンダリング。
  - `compiler/backend/llvm/src/codegen.rs:1197-1305`
- **Match ロワリング**:
  - `lower_match_to_blocks` が各 arm のパターンチェック → guard → alias → body を `LlvmBlock` に展開。
  - `panic`/`propagate` の早期終了は専用分岐を作り `phi` で合流する。
  - `compiler/backend/llvm/src/codegen.rs:1472-1672`
- **入口式ロワリング**:
  - `lower_entry_expr_to_blocks` が `panic`/`propagate`/`unsafe`/`effect` を検出し、必要に応じて特化ブロックへ分岐する。
  - `compiler/backend/llvm/src/codegen.rs:1705-1759`
- **Inline ASM / LLVM IR**:
  - `build_inline_asm_constraint_list`/`parse_inline_asm_options` で制約文字列を生成。
  - `collect_llvm_ir_uses` が SSA 位置の解析と placeholder チェックを行い、監査ログ用の入力型を記録。
  - `compiler/backend/llvm/src/codegen.rs:139-195`, `compiler/backend/llvm/src/codegen.rs:1329-1390`

## 検証・診断

- `Verifier::verify_module` がモジュール空/レイアウト欠落/型サイズ不正を検出し、監査ログへ `native.*` 系キーを記録する。Inline ASM の制約文字列と LLVM IR placeholder の不整合を診断化。
  - `compiler/backend/llvm/src/verify.rs:94-260`
- 仕様側の `native` 効果/`@unstable` 要件と LLVM IR 直書き条件は Chapter 11 の「安全性・監査」節で必ず参照すべき。
  - `docs/spec/1-3-effects-safety.md:198-205`

## 仕様との対応メモ

- **内部名正規化**は `docs/spec/1-1-syntax.md` のバックエンド内部名規則に沿う。`sanitize_llvm_ident` が `_uXXXX` 形式と先頭数値の補正を実装。
- **Bridge メタデータ**は `docs/spec/3-9-core-async-ffi-unsafe.md` で必須化された `reml.bridge.version` と `reml.bridge.stubs` の出力と一致。
- Inline ASM / LLVM IR 直書きの監査キーは `docs/spec/1-3-effects-safety.md` と `docs/spec/3-6-core-diagnostics-audit.md` の整合性確認が必要（本章で追記予定）。

## TODO / 不明点

- `TypeMappingContext::layout_of` の `Unit` が `ptr` に寄せられている意図（LLVM の `void` との使い分け）を確認したい。
- `Triple::AppleDarwin` が `macos-arm64` を返しつつ data layout は x86-64 を参照しているように見える（`TargetSpec` の `cpu` と `data_layout` の整合性確認）。
- `LlvmIrBuilder` は LLVM 風 IR のレンダリングであり実 LLVM IR 生成の前段。実 LLVM 連携との境界を章内で明確化する必要あり。
