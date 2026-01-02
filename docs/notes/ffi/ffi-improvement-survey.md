# Reml FFI Improvement Survey

## 1. 目的
Reml の FFI (Foreign Function Interface) 機能をより実用的かつ強力にするための調査と提案をまとめる。
現状の Reml FFI は `extern "C"` ブロックによる低レベルな定義を基本としているが、大規模なライブラリ利用や安全性確保においては、より高レベルな支援機能が必要である。
本稿では Rust, OCaml, Zig などの先行事例を分析し、Reml に取り入れるべき設計を示す。

## 2. 現状の Reml FFI (Analysis)
`docs/spec/3-9-core-async-ffi-unsafe.md` および `examples/ffi` に基づく現状の分析。

- **基本構文**: Rust に似た `extern "C"` ブロックで関数シグネチャを宣言。
- **データ表現**: `Ptr<T>`, `Span<T>` などの低レベルプリミティブを提供。
- **安全性**: FFI 呼び出しは `unsafe` 効果を要求する。
- **課題**:
    - C のヘッダファイルを別途参照しながら手動で `extern` 定義を書く必要があり、維持コストが高い。
    - メモリ管理（所有権の移動、借用）の責任がプログラマに委ねられており、誤用しやすい。
    - ビルドシステムとの統合（ライブラリパスの解決など）が手動設定頼みである。

## 3. 既存言語の FFI 事例調査

### 3.1 Rust: `bindgen` による自動化と `unsafe` の隔離
Rust は Reml に最も近いメモリモデルを持つ。
- **`bindgen`**: `libclang` を利用して C/C++ ヘッダから Rust の `extern` 定義を**自動生成**する。これが Rust の C エコシステム活用の決め手となっている。
    - **Pros**: 大規模ライブラリ（OpenSSL, LLVM 等）のバインディングが一瞬で生成できる。
    - **Cons**: 生成されるコードは低レベルで `unsafe` だらけになるため、人間が使うには "Safe Wrapper" を手書きする必要がある。
- **`unsafe` Block**: FFI 呼び出しを `unsafe` ブロックで囲むことを強制し、監査可能性を担保する。Reml の `effect {unsafe}` と非常に親和性が高い。
- **`cxx`**: C++ との相互運用に特化し、安全なブリッジコードを自動生成する。

### 3.2 OCaml: `ctypes` による純粋言語レベルの記述
OCaml の `ctypes` ライブラリは、C のスタブコードを書かずに OCaml コードだけで C の型と関数を記述するアプローチをとる。
- **Combinator API**: C の `int`, `ptr`, `struct` などを OCaml の値として組み立てる。
    ```ocaml
    let puts = foreign "puts" (string @-> returning int)
    ```
- **Dynamic & Static**: `libffi` を用いた動的呼び出しと、C スタブコード生成（Cstubs）の両方をサポートする。
- **Pros**: C 言語を書く必要がない。Reml の "DSL-first" な哲学に非常にフィットする（言語内で宣言的に記述できる）。
- **Cons**: 実行時オーバーヘッド（動的呼び出しの場合）や、ビルドプロセスの複雑化（スタブ生成の場合）。

### 3.3 Zig: Build System 統合と Lazy Import
Zig は C とのシームレスな統合を売りにしている。
- **`@cImport`**: ソースコード中で C ヘッダを直接インポートできる。コンパイラが内部で C パーサを持ち、型を自動変換する。
    ```zig
    const c = @cImport({ @cInclude("stdio.h"); });
    c.printf("Hello\n");
    ```
- **Build System**: `zig build` が C コンパイラとしても振る舞い、クロスコンパイル設定を共有する。
- **Pros**: バインディング生成ステップが不要で、体験が圧倒的に良い。
- **Cons**: コンパイラ自体が C パーサを内包する必要があり、実装コストが高い。

### 3.4 WASM Component Model (WIT)
WebAssembly の新しい相互運用規格。
- **Interface Types**: 数値やポインタではなく、`string`, `record`, `variant` などの高レベル型でインターフェースを定義する（`.wit` ファイル）。
- **Canonical ABI**: 言語間のメモリレイアウトの違いをランタイムが吸収する。
- **Pros**: 完全に安全な相互運用が可能。Shared Nothing アーキテクチャによりメモリ破損が伝播しない。

## 4. Reml への提案 (Gap Analysis & Proposal)

Reml は「型安全」と「DSL」を重視する言語であるため、Rust のような低レベルな自動生成と、OCaml のような高レベルな記述性の**ハイブリッド**を目指すべきである。

### 4.1 短期計画: 自動生成ツールの整備 (`reml-bindgen`)
Rust の `bindgen` 相当のツールは必須である。手書き `extern` はスケールしない。
- Clang AST を解析し、対応する Reml の `extern "C"` 定義を出力する CLI ツールを開発する。
- 生成されたコードは `examples/ffi` のような低レベル API となる。

### 4.2 中期計画: コンビネータベースの安全なラッパー (`Core.Ffi.Dsl`)
OCaml `ctypes` にインスパイアされた、FFI 定義 DSL を標準ライブラリに導入する。
これにより、ユーザは `extern` ブロックを直接書くのではなく、Reml の式として FFI を定義できるようになる。

```reml
// 構想例
let lib = ffi.bind_library("m")
let cos = lib.bind_fn("cos", ffi.double -> ffi.double)
// cos は自動的に effect {ffi} を持つ関数として型付けされる
```

### 4.3 長期計画: ビルドシステム統合 (`reml build`)
Zig のように、`reml.json` (マニフェスト) で C ライブラリの依存関係を記述し、`reml build` 時に自動的にヘッダ解析・リンク解決を行う。

---

この調査に基づき、まずは **「コンビネータベースの FFI 定義の設計」** と **「バインディング自動生成の仕様策定」** を今後のタスクとして提案する。
