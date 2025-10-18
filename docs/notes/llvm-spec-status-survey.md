# LLVM関連仕様の現状調査報告

> 作成日: 2025-10-04
> 目的: Reml言語仕様書内でLLVMに関わる事項がどの程度具体的に決定されているかを調査し、実装フェーズの判断材料を提供する。

## 0. 調査概要

### 0.1 調査範囲

リポジトリ内の全Markdownファイルを対象に「LLVM」キーワードで検索し、関連する仕様・計画・ノートを抽出した。

### 0.2 調査結果サマリ

- **言及ファイル数**: 16ファイル
- **決定済み仕様の成熟度**: x86_64向けMVP実装に必要な基礎仕様は高度に具体化
- **未決定領域**: マルチターゲット対応、JIT実行、高度な最適化は調査・計画段階

---

## 1. LLVMに言及している文書一覧

### 1.1 主要仕様文書（詳細度: 高）

| ファイル | 内容 | 具体化レベル |
|---------|------|------------|
| [guides/llvm-integration-notes.md](../guides/llvm-integration-notes.md) | **最も詳細なLLVM連携仕様**。コンパイルパイプライン、ABI、型マッピング、メモリモデル、実装段階の全体像を記述 | ★★★★★ |
| [2-6-execution-strategy.md](../spec/2-6-execution-strategy.md) | 実行戦略とRunConfig。ターゲット設定（`RunConfigTarget`）、成果物メタデータ（`RunArtifactMetadata`）の型定義 | ★★★★☆ |
| [guides/reml-ffi-handbook.md](../guides/reml-ffi-handbook.md) | FFIハンドブック。ABI、呼出規約、unsafeポインタ型マッピング、リンク手順 | ★★★★☆ |
| [1-2-types-Inference.md](../spec/1-2-types-Inference.md) | 型推論仕様。LLVM連携への参照を含む | ★★☆☆☆ |
| [1-3-effects-safety.md](../spec/1-3-effects-safety.md) | 効果と安全性。FFI/unsafeポインタのメモリ整列要件に言及 | ★★☆☆☆ |

### 1.2 計画・調査文書（詳細度: 中）

| ファイル | 内容 | ステータス |
|---------|------|-----------|
| [notes/a-jit.md](a-jit.md) | JIT/バックエンド拡張ノート。WASM/ARM64/GPU連携の検討事項 | 調査計画段階 |
| [notes/cross-compilation-spec-intro.md](cross-compilation-spec-intro.md) | クロスコンパイル導入の基礎調査。ターゲットプロファイル、ツールチェーン戦略 | 設計方針策定済 |
| [notes/cross-compilation-spec-update-plan.md](cross-compilation-spec-update-plan.md) | クロスコンパイル仕様の組み込み計画。フェーズ分けと更新対象文書の特定 | 実装前 |
| [5-2-registry-distribution.md](../spec/5-2-registry-distribution.md) | レジストリとLLVM tripleの扱い。ターゲットメタデータ配布の設計 | ドラフト |
| [guides/portability.md](../guides/portability.md) | ポータビリティガイド。`remlc --target`の使用方法とプラットフォーム適応 | ガイドライン策定済 |

### 1.3 参考文書（詳細度: 低）

- [README.md](../spec/README.md) - プロジェクト概要とLLVM連携ノートへのリンク
- [notes/reml-influence-study.md](reml-influence-study.md) - 言語設計の影響分析
- [notes/algebraic-effects-handlers-spec-proposal.md](algebraic-effects-handlers-spec-proposal.md) - 代数的効果のLLVM loweringの課題
- [notes/reml-design-goals-and-appendix.md](reml-design-goals-and-appendix.md) - 設計目標とLLVM連携
- [notes/reml-language-influences-analysis.md](reml-language-influences-analysis.md) - 言語影響分析
- [notes/guides-to-spec-integration-plan.md](guides-to-spec-integration-plan.md) - ガイド統合計画

---

## 2. 決定済みの具体的仕様

### 2.1 コンパイル戦略

#### 2.1.1 ブートストラップ戦略（guides/llvm-integration-notes.md §0-3）

| フェーズ | 目的 | 期間 | 成果物 |
|---------|------|------|--------|
| **Phase 0** | OCamlでRemlコンパイラ実装 | 2-3ヶ月（MVP）+ 4-6ヶ月（本格） | RemlソースからLLVM IRを生成するOCaml製コンパイラ |
| **Phase 1** | 言語仕様の検証・安定化 | - | 安定したReml仕様とCore.Parse実装 |
| **Phase 2** | RemlでRemlコンパイラを書き直し | 6-12ヶ月 | Reml自身で記述されたRemlコンパイラ |
| **Phase 3** | セルフホスト完了 | - | OCaml実装との出力一致検証と完全移行 |

**理由**: HM型推論・ADT実装に最適なOCamlを選択し、最短期間でRemlコンパイラを実現

#### 2.1.2 コンパイルパイプライン（guides/llvm-integration-notes.md §1）

```
Remlソースコード
  └─(1) 構文解析（Core.Parse）
      ├─ 字句: lexeme/symbol/identifier/number/string
      ├─ 構文: 式・宣言・モジュール
      ├─ 優先度: precedence宣言
      └─ 出力: 未型付けAST（位置情報Span含む）
    └─(2) 意味解析（Resolver/Typer）
        ├─ 名前解決（スコープ・モジュール）
        ├─ HM型推論（unify、多相化/インスタンス化）
        ├─ 型クラス制約解決
        └─ 出力: 型付きAST（TAST）
      └─(3) 降格・糖衣剥がし（Desugar→Core）
          ├─ パイプ展開
          ├─ パターンマッチ→決定木
          ├─ クロージャ→{env*, code_ptr}
          ├─ 型クラス辞書→追加引数
          └─ 出力: Core IR
        └─(4) 中間最適化（Core→MIR）
            ├─ β簡約/インライン
            ├─ モノモルフィゼーション
            ├─ クロージャ環境捕捉解析
            └─ 出力: MIR（基本ブロックCFG、SSA準備済）
          └─(5) LLVM IR生成
              ├─ 型レイアウト決定
              ├─ 呼出規約適用
              ├─ 所有権（RC）コード挿入
              └─ 出力: LLVM IR文字列
            └─(6) 実行
                └─ IR実行器へ投入（JIT/インタプリタ）
```

### 2.2 ターゲットABI/データレイアウト

#### 2.2.1 主要パラメータ（guides/llvm-integration-notes.md §5.0）

| 項目 | 既定値/方針 | 備考 |
|------|-----------|------|
| **主要ターゲット** | System V AMD64 (`x86_64-unknown-linux-gnu`)<br>Windows x64 (`x86_64-pc-windows-msvc`) | LLVMのデフォルトABIから逸脱しない |
| **将来追加予定** | ARM64、WASM/WASI | 調査中（notes/a-jit.md） |
| **Phase 2-3 計測計画** | Apple Silicon (arm64-apple-darwin) | `scripts/ci-local.sh --target macos --arch arm64` で DataLayout/ABI の実測を収集し、`reports/ffi-macos-summary.md` に記録する（2025-10 着手予定） |
| **エンディアン** | リトルエンディアン | LLVM DataLayout文字列で宣言 |
| **ポインタ幅** | 64bit (`p:64:64`) | Opaque Pointer前提（LLVM 15+） |
| **整数アラインメント** | `i8:8`, `i16:16`, `i32:32`, `i64:64` | System V互換 |
| **浮動小数点** | `f32:32`, `f64:64` | SSE2対応前提 |
| **構造体パッキング** | 自然境界（`#[repr(C)]`相当） | `@repr(packed)`は将来導入予定 |
| **ベクトル** | `v128:128` | SIMD拡張予定時の互換性確保 |

**DataLayout文字列例（System V AMD64）**:
```
e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64
```

#### 2.2.2 呼出規約（guides/llvm-integration-notes.md §5.2）

- **基本**: C互換（`cc ccc`）
- **FFI互換性**: 公開関数は既定でC呼出規約
- **Windows切替**: `RunConfig.extensions["runtime"].codegen.call_conv = "win64"`（将来提供予定）
- **関数名マングリング**: `k__mod__fn__i64_i64` 形式
- **Darwin計測タスク（Phase 2-3）**:
  - `llc -mtriple=arm64-apple-darwin` で生成した IR/オブジェクトを取得し、AAPCS64 呼出規約との整合性を比較する。
  - `scripts/verify_llvm_ir.sh --target arm64-apple-darwin` を拡張し、構造体戻り値・可変長引数・システムライブラリとの接続をサンプル化する。
  - `reports/ffi-macos-summary.md` に検証ログを保存し、差分が発生した場合は `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` と本ノート両方を更新する。

### 2.3 型対応表

#### 2.3.1 基本型マッピング（guides/llvm-integration-notes.md §5.1）

| Reml型 | LLVM IR型 | 備考 |
|--------|----------|------|
| `i1` / `Bool` | `i1` | 分岐/比較結果 |
| `i32` / `i64` | `i32` / `i64` | そのまま |
| `f64` | `double` | IEEE 754準拠 |
| ポインタ | `ptr` | LLVM 15+ opaque pointer |
| 文字列 | `{i8*, i64}` | データポインタ/長さ（RT管理） |
| 代数型（ADT） | `{i32 tag, [payload]}` | tag先頭、payloadは最大幅のunion表現 |

#### 2.3.2 構造体レイアウト（guides/llvm-integration-notes.md §5.0）

- **ADT**: `{i32 tag, [payload]}` を基本とし、payloadは最大幅のunionまたはvariantごとの構造体にLower
- **文字列/スライス**: `{ptr data, i64 len}`
- **FFI渡し**: 所有権ルール（RC `inc_ref`/`dec_ref`）が必須

### 2.4 メモリ管理と所有権モデル

#### 2.4.1 MVP実装（guides/llvm-integration-notes.md §5.3）

- **基本戦略**: 参照カウント（RC）ベースのランタイム
- **提供関数**: `inc_ref(ptr)`, `dec_ref(ptr)`
- **適用対象**: 文字列/ボックス化値/クロージャ環境
- **借用**: コンパイラ内の最適化に留め、言語表面には出さない

#### 2.4.2 FFI所有権契約（guides/reml-ffi-handbook.md §5）

| 方向 | 契約 | 実装 |
|------|------|------|
| **Reml → C** | 渡す前に`inc_ref`、C側で`reml_release_*`を呼ぶ | FFI呼び出し前後でRC操作を挿入 |
| **C → Reml** | `wrap_foreign_ptr`で`Ownership::Borrowed`設定、スコープ抜ける前に`release_foreign_ptr` | または`Ownership::Transferred`で解放関数登録 |
| **ゼロコピー文字列** | UTF-8前提。C側にはバイト列として渡す | `Span<u8>`を`ForeignBuffer`へ昇格 |

#### 2.4.3 将来拡張（guides/llvm-integration-notes.md §5.3）

- Arena/RCハイブリッド
- リージョン最適化
- ゼロコスト抽象化の追求

### 2.5 ランタイムAPI

#### 2.5.1 最小ランタイム（guides/llvm-integration-notes.md §5.4）

必須関数セット:
```c
void* mem_alloc(size_t size);
void  mem_free(void* ptr);
void  panic(const char* msg);
void  print_i64(int64_t value);
void  inc_ref(void* ptr);
void  dec_ref(void* ptr);
```

- これらは**別モジュールのIR**として用意し、リンクまたは実行器へ同時投入
- IR実行器に渡す際はランタイムIRと本体IRを連結

### 2.6 代表的なLowering

#### 2.6.1 制御構造（guides/llvm-integration-notes.md §5.5）

| Reml構文 | LLVM IR実装 |
|---------|-----------|
| **if式** | `br`で2ブロック + φノード |
| **パターンマッチ** | `switch tag` → caseごとにcast/extract |
| **クロージャ** | `{env*, code_ptr}` 形式、呼び出しは `code_ptr(env*, args...)` |
| **タプル/構造体** | `insertvalue`/`extractvalue` または `alloca` + `gep` |
| **配列/スライス** | `{ptr, len, cap}`（MVPは不変長のみ可） |

#### 2.6.2 LLVM IR生成例（guides/llvm-integration-notes.md §5.6, §10）

**Reml入力**:
```reml
fn add(a: i64, b: i64) -> i64 = a + b

pub fn main() -> i64 = add(2, 40)
```

**出力LLVM IR**:
```llvm
declare i64 @print_i64(i64)

define i64 @k__add(i64 %a, i64 %b) {
entry:
  %sum = add i64 %a, %b
  ret i64 %sum
}

define i64 @k__main() {
entry:
  %r = call i64 @k__add(i64 2, i64 40)
  ret i64 %r
}
```

**if式の例**:
```reml
fn abs(x: i64) -> i64 =
  if x < 0 then -x else x
```

```llvm
define i64 @k__abs(i64 %x) {
entry:
  %isneg = icmp slt i64 %x, 0
  br i1 %isneg, label %neg, label %pos

neg:
  %nx = sub i64 0, %x
  br label %join

pos:
  br label %join

join:
  %r = phi i64 [ %nx, %neg ], [ %x, %pos ]
  ret i64 %r
}
```

---

## 3. ターゲット設定とクロスコンパイル

### 3.1 RunConfigTarget構造体（2-6-execution-strategy.md §B-2-1）

コア仕様で定義されたターゲット情報の型:

```reml
type RunConfigTarget = {
  os: Str,                      // "linux", "windows", "macos", etc.
  family: Str,                  // "unix", "windows", "wasm"
  arch: Str,                    // "x86_64", "aarch64", "wasm32"
  abi: Option<Str>,            // "gnu", "msvc", "musl"
  vendor: Option<Str>,
  env: Option<Str>,
  profile_id: Option<Str>,     // "desktop-x86_64", "mobile-arm64"
  triple: Option<Str>,         // LLVM triple文字列
  features: Set<Str>,
  capabilities: Set<Str>,
  stdlib_version: Option<SemVer>,
  runtime_revision: Option<Str>,
  diagnostics: Bool,
  extra: Map<Str, Str>
}
```

**用途**:
- `@cfg`属性での条件付きコンパイル
- バックエンドへのターゲット指定
- クロスビルド時の検証

### 3.2 RunArtifactMetadata（コンパイラ出力メタデータ）

```reml
type RunArtifactMetadata = {
  target: RunConfigTarget,
  llvm_triple: Str,            // LLVM tripleの最終値
  data_layout: Str,            // LLVM DataLayout文字列
  runtime_revision: Str,
  stdlib_version: SemVer,
  emitted_capabilities: Set<Str>,
  timestamp: DateTime,
  hash: Str                    // 標準ライブラリキャッシュ検証用
}
```

**検証規則**（2-6-execution-strategy.md §B-2-1-a）:
- `runtime_revision`や`stdlib_version`がCLI/レジストリ提供値と不一致の場合、`target.abi.mismatch`を発行して停止
- `llvm_triple`と`RunConfigTarget.triple`は一致必須。不一致時は`target.config.unknown_value`
- ハッシュ計算は線形時間を維持（性能1.1準拠）

### 3.3 クロスコンパイル計画（notes/cross-compilation-spec-intro.md, notes/cross-compilation-spec-update-plan.md）

#### 3.3.1 現状

- **ステータス**: 調査・設計方針策定済み、仕様書への組み込みは未実施
- **参考**: Rust（target.json）、Zig（builtin.Target）、Go（GOOS/GOARCH）のパターンを分析済み

#### 3.3.2 予定される機能

| 機能 | 内容 | 実装フェーズ |
|------|------|------------|
| **ターゲットプロファイル管理** | `reml target list/show/validate` サブコマンド | Phase A（言語仕様） |
| **クロスビルド** | `reml build --target <profile>` | Phase C（エコシステム） |
| **標準ライブラリ配布** | ターゲット別の事前ビルド成果物を `artifact/std/<triple>/<hash>` に格納 | Phase B（標準API） |
| **ツールチェーン管理** | `reml toolchain install <profile>` で必要なライブラリ・ランタイムを取得 | Phase C |
| **レジストリ連携** | パッケージメタデータに`targets`配列を追加、互換性チェック | Phase C |

#### 3.3.3 実装計画（notes/cross-compilation-spec-update-plan.md）

| フェーズ | 対象文書 | 主な内容 |
|---------|---------|---------|
| **Phase A**（言語仕様） | 1-1-syntax.md, 1-2-types-Inference.md, 2-6-execution-strategy.md | `@cfg`キー拡張、ターゲットメタデータ生成フロー |
| **Phase B**（標準API） | 3-10-core-env.md, 3-8-core-runtime-capability.md, 3-6-core-diagnostics-audit.md | `TargetCapability`グループ、診断強化 |
| **Phase C**（エコシステム） | 5-1-package-manager-cli.md, 5-2-registry-distribution.md, 5-3-developer-toolchain.md | CLIサブコマンド、レジストリメタデータ |
| **Phase D**（ガイド） | guides/portability.md, guides/ci-strategy.md, guides/cross-compilation.md（新規） | クロスビルド手順、CIマトリクス |

### 3.4 ポータビリティ運用（guides/portability.md）

#### 3.4.1 推奨ワークフロー

1. **ターゲット初期化**: `infer_target_from_env()` → `RunConfig.extensions["target"]`
2. **条件付き宣言**: `@cfg`属性でモジュール・API切り替え
3. **ファイル・Env操作**: `Core.Path`と`Core.Env`経由で統一
4. **FFI/ABI適応**: `resolve_calling_convention` + `with_abi_adaptation`
5. **診断可視化**: `diagnostics=true`で`Diagnostic.extensions["cfg"]`に出力
6. **ツールチェーン検証**: `reml toolchain verify`を定期実行

#### 3.4.2 主要な`@cfg`キー（guides/portability.md §2.2）

| キー | 用途 | 典型値 |
|------|------|--------|
| `target_os` | OS判別 | `"windows"`, `"linux"`, `"macos"`, `"wasm"` |
| `target_family` | 共通分岐 | `"unix"`, `"windows"`, `"wasm"` |
| `target_arch` | ABI/命令差異 | `"x86_64"`, `"aarch64"`, `"wasm32"` |
| `target_abi` | ツールチェーン/ABI分岐 | `"gnu"`, `"msvc"`, `"musl"` |
| `target_profile` | プロファイル固有切替 | `"desktop-x86_64"`, `"mobile-arm64"` |
| `capability` | ターゲットCapability | `"unicode.nfc"`, `"fs.case_insensitive"` |

---

## 4. FFI/unsafe境界の仕様

### 4.1 ABI方針（guides/reml-ffi-handbook.md §2）

- **既定ターゲット**: System V AMD64 / Windows x64
- **将来追加**: ARM64 / WASM（調査中）
- **ヘッダ生成**: `remlc --emit-header`（将来実装予定）でC用シグネチャ生成
- **例外**: 境界を越えて伝播しない。C++例外はガードレイヤで捕捉→`FfiErrorKind::CallFailed`に変換

### 4.2 リンク手順（guides/reml-ffi-handbook.md §4）

```bash
# Linux
clang foo.c foo.ll libreml_runtime.a -o foo

# Windows
cl /Fe:foo.exe foo.c foo.ll libreml_runtime.lib
```

デバッグ情報を有効化する場合は `-g` 付きLLVM IRを生成し、`lldb`/`windbg`で解析。

### 4.3 unsafeポインタ型マッピング（guides/reml-ffi-handbook.md §9.1）

| Reml型 | C型 | Rust型 | Swift型 | Zig型 | 備考 |
|--------|-----|--------|---------|-------|------|
| `Ptr<T>` | `const T*` | `*const T` | `UnsafePointer<T>` | `[*]const T` | NULL許容、読み取り専用 |
| `MutPtr<T>` | `T*` | `*mut T` | `UnsafeMutablePointer<T>` | `[*]T` | 書き込み可能、データ競合注意 |
| `NonNullPtr<T>` | `T*` | `NonNull<T>` | `UnsafePointer<T>` | `*T` | 非NULL保証、`Span<T>`の基盤 |
| `Ptr<void>` | `void*` | `*mut c_void` | `OpaquePointer` | `*anyopaque` | 型情報なし、ダウンキャスト必須 |
| `FnPtr<A,R>` | `R (*)(A...)` | `extern "C" fn(A)->R` | `@convention(c) (A)->R` | `fn(A) callconv(.C) R` | クロージャなし |

### 4.4 安全ラッパ設計指針（guides/reml-ffi-handbook.md §9.2）

- 低レベルポインタは`Span<T>`/`Buffer`/`StructView`等の安全ラッパからのみ取得
- 公開APIは可能な限りラッパ型を返す
- `Span<T>`は長さ保持→境界チェック付き`read_exact`/`write_exact`を提供
- `StructView`は`byte_offset`でフィールドアクセス、ABI互換性はLLVM連携ノートに従う

---

## 5. JIT/バックエンド拡張

### 5.1 現状メモ（notes/a-jit.md §1）

- **LLVMネイティブコード生成**: x86_64（System V / Windows）を優先ターゲット
- **WASI/WASM**: AOT（`wasm32-wasi`バイナリ）を優先、JITの必要性は未評価
- **ARM64**: LLVM NEON/SVE最適化利用可能だが、JIT実行は検証必要

### 5.2 Phase 3 TODO（notes/a-jit.md §2）

- [ ] WASMターゲット向けJITの要否調査（WASI Preview 2でのJIT許可状況、AOTとの比較）
- [ ] ARM64 NEON/SVE用のTargetMachine設定テンプレートと自動検出ロジック整備
- [ ] GPUアクセラレータ連携時のJIT生成コード（PTX/Metal Shading Language）検討
- [ ] コンテナ/サーバーレス環境でのJIT実行セキュリティ評価（run-as-nonroot, seccomp, sandbox）

### 5.3 調査計画（notes/a-jit.md §4）

- [ ] LLVM ORC JITのWASM対応状況確認、必要なフラグと依存関係整理
- [ ] ARM64 NEON/SVE向け`TargetMachine`設定例作成、ベンチマーク候補選定（例: JSONパーサDSL）
- [ ] サーバーレス（AWS Lambda/Cloud Run）でのJIT実行制限調査、サンドボックス方針まとめ
- [ ] `docs/guides/ci-strategy.md`にJITベンチマークジョブ追加の前提条件列挙

### 5.4 参考情報（notes/a-jit.md §3）

- `docs/guides/runtime-bridges.md` のクラウド/サーバーレスセクション
- `docs/guides/portability.md` のターゲット戦略チェックリスト
- LLVM ORC JIT / MCJITのサポート状況

---

## 6. 実装段階とロードマップ

### 6.1 実装段階の定義（guides/llvm-integration-notes.md §8）

#### 6.1.1 MVP（最小実装）

**目的**: IR実行器で`main`が走る最小構成

| 要素 | 内容 |
|------|------|
| **型** | `i64`, `bool`、単相関数 |
| **機能** | let/if/fn/app、基本演算子のみのトレイト、標準I/O最小 |
| **メモリ** | プリミティブ中心（GC不要） |
| **トレイト** | 基本算術・比較演算子の組み込みトレイトのみ（i64, f64, Bool, String対応） |

#### 6.1.2 本格実装

**目的**: 実用的な機能セットの提供

| 要素 | 内容 |
|------|------|
| **型** | タプル/配列/文字列（RC管理）、クロージャ（env*） |
| **ジェネリクス** | モノモルフィゼーションでジェネリクス対応 |
| **トレイト** | ユーザ定義トレイト、where制約、制約解決 |

#### 6.1.3 完全実装

**目的**: 言語仕様の完全実現

| 要素 | 内容 |
|------|------|
| **ADT** | ADT/`match`/型クラス辞書パッシング、パターン網羅性チェック |
| **エラー処理** | `Result`を一級化、`?`演算子風デシュガ |
| **デバッグ** | デバッグ情報（DWARF）、最適化フラグ（`-O2`相当）連携 |
| **高度機能** | 高階型クラス、特殊化 |

#### 6.1.4 拡張実装

**目的**: 最適化と高度機能

| 要素 | 内容 |
|------|------|
| **最適化** | 左再帰・Packrat切替、インクリメンタル/LLDリンク |
| **FFI** | FFI完全対応 |
| **並列** | 並列/タスク処理 |

### 6.2 作業ブレークダウン（guides/llvm-integration-notes.md §9）

- [ ] 文法定義（Core.Parse）：式/宣言/モジュール、precedence表
- [ ] AST構造体 + Span
- [ ] 名前解決：スコープ/モジュール/インポート
- [ ] 型表現：単相→多相、ユニフィケーション、型エラー整形
- [ ] Desugar：パイプ/パターン/ラムダ/辞書
- [ ] Core/MIR：ブロックCFG、閉包表現、ADT表現
- [ ] IR選択：型マップ、呼出規約、ランタイムAPI
- [ ] IRエミッタ：関数/ブロック/命令/φ/スイッチ/構造体
- [ ] ランタイムIR：mem/rc/print/panic
- [ ] 統合：IR実行器連携ユーティリティ（モジュール結合、エントリ呼び）
- [ ] テスト：E2E、型推論、IR検証、メモリ衛生

### 6.3 テスト/検証の進め方（guides/llvm-integration-notes.md §7）

1. **E2Eスモーク**: 算術/if/let/関数呼び出し → IR → 実行器 → 期待値
2. **型推論の単体**: 多相関数の一般化/インスタンス化・エラー系
3. **Core等式性**: 糖衣前後で意味が等しいことのプロパティテスト
4. **IR整合性**: `opt -verify`相当の検証をCIで回す
5. **メモリ安全**: `inc_ref/dec_ref`のリーク/二重解放をサニタイザで監視
6. **パターンマッチ網羅**: 非網羅時に警告/エラー（将来的に必須）

### 6.4 Docker ベース x86_64 Linux 環境（1-5-runtime-integration.md §9）

- **ベースイメージ**: `ubuntu:22.04`、LLVM 18 / clang-18 / opam 2.1 を事前インストール
- **Dockerfile**: `tooling/ci/docker/bootstrap-runtime.Dockerfile` — 非特権ユーザ `reml`、`ocaml-base-compiler.5.2.1` スイッチ、`dune/menhir/llvm/odoc` を pre-install
- **ビルドスクリプト**: `scripts/docker/build-runtime-container.sh` — `docker buildx` / `podman build` 両対応、`--push`・`--build-arg` をサポート
- **実行スクリプト**: `scripts/docker/run-runtime-tests.sh` — `dune build`, `dune runtest`, `compiler/ocaml/scripts/verify_llvm_ir.sh`, `make -C runtime/native runtime` を一括実行
- **スモークテスト**: `scripts/docker/smoke-linux.sh` — `examples/language-impl-comparison/reml/basic_interpreter.reml` を `remlc --emit-ir --verify-ir` でビルド
- **メトリクス**: `tooling/ci/docker/metrics.json` にビルド所要時間/イメージサイズを記録し、`0-3-audit-and-metrics.md` へ転記
- **脆弱性監査**: `docker scout cves ghcr.io/reml/bootstrap-runtime:<tag>` または `trivy image` で月次チェックし、重大度 High 以上は `0-4-risk-handling.md` に登録

---

## 7. 未決定・今後の課題

### 7.1 明確に「未定」とされている項目

| 項目 | 現状 | 参照 |
|------|------|------|
| **WASM/WASI ABI** | DataLayoutとABIは未定。別途調査メモ参照予定 | guides/llvm-integration-notes.md §5.0 |
| **ARM64詳細仕様** | 正式対応は将来予定。調査結果はnotes/に記録予定 | notes/a-jit.md |
| **JIT実行詳細** | LLVM ORC JIT/MCJITの詳細仕様未確定 | notes/a-jit.md §4 |
| **クロスコンパイル** | 仕様書への組み込みは計画中だが未反映 | notes/cross-compilation-spec-update-plan.md |
| **高度メモリ管理** | Arena/RCハイブリッド、リージョン最適化は将来拡張 | guides/llvm-integration-notes.md §5.3 |
| **repr(packed)** | 構造体の`repr(packed)`対応は未定 | guides/reml-ffi-handbook.md §8 |
| **代数的効果lowering** | スタックフレーム安全性の検証が必要 | notes/algebraic-effects-handlers-spec-proposal.md |

### 7.2 検討中の項目

| 項目 | 現状 | 優先度 |
|------|------|--------|
| セルフホストコンパイラの実装スケジュール | Phase 2として6-12ヶ月を想定 | 中 |
| Packrat/左再帰の性能トレードオフ | `RunConfig.packrat`/`left_recursion`で切替可能だが、最適な既定値は未決定 | 低 |
| デバッグ情報（DWARF）生成 | 完全実装段階で対応予定 | 中 |
| 最適化フラグ連携 | `-O2`相当の連携仕様は未定 | 低 |
| GPU連携 | JIT生成コード（PTX/Metal）は調査段階 | 低 |

### 7.3 今後追加予定の機能（guides/reml-ffi-handbook.md §8）

- WASM/WASIのABI整備とホスト関数ブリッジ
- `async`ランタイムとの統合サンプル（io_uring/libuv）
- Rust向け安全ラッパ生成ツール（`reml-bindgen`仮称）
- C++ name manglingのガイド

---

## 8. 成熟度評価

### 8.1 領域別成熟度

| 領域 | 成熟度 | 評価 |
|------|--------|------|
| **基本コンパイルパイプライン** | ★★★★★ | 全フェーズの詳細設計完了 |
| **x86_64 ABI/データレイアウト** | ★★★★★ | System V/Windows両対応、具体的なパラメータ確定 |
| **型→LLVM IR基本マッピング** | ★★★★★ | プリミティブ・ADT・クロージャの対応明確 |
| **RCベースメモリ管理** | ★★★★☆ | 基本方針確定、最適化は将来課題 |
| **FFI呼出規約とポインタ** | ★★★★☆ | C/Rust/Swift/Zigとの対応表完備、実装詳細は一部保留 |
| **ブートストラップ戦略** | ★★★★☆ | フェーズ分けと期間見積もり完了 |
| **クロスコンパイル** | ★★★☆☆ | 設計方針確定、仕様書組み込みは未実施 |
| **ターゲット設定/プロファイル** | ★★★☆☆ | 型定義完了、CLI/ツールチェーン統合は計画段階 |
| **JIT実行** | ★★☆☆☆ | 調査計画策定済み、詳細仕様は未定 |
| **ARM64/WASM対応** | ★★☆☆☆ | 調査中、正式仕様は未策定 |
| **デバッグ情報生成** | ★☆☆☆☆ | 完全実装段階で対応予定、詳細未定 |
| **高度最適化** | ★☆☆☆☆ | Arena/リージョン等は将来拡張 |

### 8.2 実装準備状況

#### 8.2.1 即座に実装可能な領域

- ✅ MVP向けの基本型マッピング（i64/bool/f64）
- ✅ 単純な制御構造のLowering（if/関数呼び出し）
- ✅ RC管理ランタイムの最小実装
- ✅ x86_64向けのDataLayout設定
- ✅ OCamlブートストラップコンパイラの骨格

#### 8.2.2 設計完了・実装待ちの領域

- 🔶 ADT/パターンマッチのLowering（仕様明確、実装未着手）
- 🔶 クロージャ環境捕捉（仕様明確、実装未着手）
- 🔶 モノモルフィゼーション（仕様明確、実装未着手）
- 🔶 FFI基本（型マッピング確定、ツール生成は未実装）

#### 8.2.3 計画段階の領域

- 📋 クロスコンパイル（設計完了、仕様書統合待ち）
- 📋 JIT実行（調査計画あり、詳細仕様未定）
- 📋 ARM64/WASM対応（調査中）

#### 8.2.4 将来課題の領域

- ⏳ 高度メモリ最適化（Arena/リージョン）
- ⏳ デバッグ情報（DWARF）
- ⏳ 代数的効果のlowering

---

## 9. 推奨される次のステップ

### 9.1 短期（〜3ヶ月）

1. **OCamlブートストラップコンパイラのMVP実装開始**
   - 基本型（i64/bool）とif/関数のみのサブセット
   - LLVM IR文字列生成の基礎パイプライン構築
   - 参照: guides/llvm-integration-notes.md §8「MVP（最小実装）」

2. **RCランタイムの最小実装**
   - `inc_ref`/`dec_ref`のC実装
   - 単体テストとメモリリーク検証
   - 参照: guides/llvm-integration-notes.md §5.4

3. **E2Eスモークテストの整備**
   - 算術演算 → IR生成 → 実行 → 期待値検証
   - CI統合
   - 参照: guides/llvm-integration-notes.md §7

### 9.2 中期（3〜6ヶ月）

1. **本格実装への移行**
   - ADT/パターンマッチ/クロージャのLowering実装
   - モノモルフィゼーション
   - 参照: guides/llvm-integration-notes.md §8「本格実装」

2. **クロスコンパイル仕様の正式化**
   - notes/cross-compilation-spec-update-plan.mdのPhase A〜C実施
   - 仕様書への統合
   - `reml target`サブコマンドの設計

3. **FFIツールチェーンの整備**
   - `remlc --emit-header`の実装
   - C/Rustバインディングのサンプル作成
   - 参照: guides/reml-ffi-handbook.md §4

### 9.3 長期（6〜12ヶ月）

1. **セルフホスト移行の準備**
   - RemlでRemlコンパイラの書き直し開始
   - OCaml実装との出力比較基盤
   - 参照: guides/llvm-integration-notes.md §0「Phase 2」

2. **ARM64/WASM対応の調査完了と仕様化**
   - JIT実行の詳細仕様策定
   - ターゲット別テストマトリクスの構築
   - 参照: notes/a-jit.md

3. **完全実装への移行**
   - デバッグ情報（DWARF）
   - 最適化フラグ連携
   - 高階型クラス・特殊化
   - 参照: guides/llvm-integration-notes.md §8「完全実装」

---

## 10. まとめ

### 10.1 調査結論

Reml言語のLLVM関連仕様は、**x86_64向けMVP実装に必要な基礎仕様が非常に具体的に決定されている**状態にある。特に以下の領域は即座に実装開始可能:

- ✅ コンパイルパイプライン全体の設計
- ✅ x86_64（Linux/Windows）向けABI/データレイアウト
- ✅ 基本型のLLVM IRマッピング
- ✅ RCベースのメモリ管理モデル
- ✅ ブートストラップ戦略（OCaml → セルフホスト）

一方、以下の領域は**調査・計画段階**であり、MVP実装には必須ではないが中長期的に重要:

- 🔶 クロスコンパイル（設計完了、統合待ち）
- 🔶 JIT実行（調査計画策定済み）
- 🔶 ARM64/WASM対応（調査中）

### 10.2 実装フェーズとの対応

| 実装段階 | LLVM仕様の準備状況 | 推奨アクション |
|---------|------------------|--------------|
| **MVP** | ★★★★★ 完全準備完了 | 即座に実装開始可能 |
| **本格実装** | ★★★★☆ ほぼ準備完了 | 一部詳細（最適化パス等）は実装中に詳細化 |
| **完全実装** | ★★★☆☆ 概要設計済み | デバッグ情報等の詳細仕様策定が必要 |
| **拡張実装** | ★★☆☆☆ 調査段階 | クロスコンパイル等の仕様統合を優先 |

### 10.3 文書整備の優先度

1. **高**: guides/llvm-integration-notes.md の継続メンテナンス（実装知見の追記）
2. **高**: クロスコンパイル仕様の正式統合（notes/ → 本体仕様書）
3. **中**: JIT実行の詳細仕様策定（notes/a-jit.md の具体化）
4. **中**: FFIハンドブックのサンプル拡充
5. **低**: WASM/ARM64の調査完了と仕様化

---

## 参考文献

### 主要仕様文書

- [guides/llvm-integration-notes.md](../guides/llvm-integration-notes.md) - LLVM連携の中核文書
- [2-6-execution-strategy.md](../spec/2-6-execution-strategy.md) - 実行戦略とターゲット設定
- [guides/reml-ffi-handbook.md](../guides/reml-ffi-handbook.md) - FFI実務ガイド

### 計画・調査文書

- [notes/a-jit.md](a-jit.md) - JIT/バックエンド拡張
- [notes/cross-compilation-spec-intro.md](cross-compilation-spec-intro.md) - クロスコンパイル調査
- [notes/cross-compilation-spec-update-plan.md](cross-compilation-spec-update-plan.md) - 仕様統合計画

### 関連仕様

- [1-2-types-Inference.md](../spec/1-2-types-Inference.md) - 型推論
- [1-3-effects-safety.md](../spec/1-3-effects-safety.md) - 効果と安全性
- [5-2-registry-distribution.md](../spec/5-2-registry-distribution.md) - レジストリ
- [guides/portability.md](../guides/portability.md) - ポータビリティ

---

**調査実施者**: Claude Code
**最終更新**: 2025-10-04
**バージョン**: 1.0
