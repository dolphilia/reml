# 2.3 FFI 契約拡張計画

## 目的
- Phase 2 で `3-9-core-async-ffi-unsafe.md` に定義された ABI/所有権契約を OCaml 実装へ反映し、x86_64 Linux (System V) と Windows x64 (MSVC) の両方でブリッジコードを検証する。
- `AuditEnvelope` に FFI 呼び出しのメタデータを記録し、診断と監査の一貫性を確保する。

## スコープ
- **含む**: FFI 宣言構文の Parser 拡張、Typer による ABI/所有権チェック、ブリッジコード生成、プラットフォーム別ビルド、監査ログ拡張。
- **含まない**: 非同期ランタイム実装の刷新、プラグイン経由の FFI 自動生成。これらは Phase 3 以降。
- **前提**: Phase 1 のランタイム連携が完成し、Phase 2 の効果システム統合と衝突しない設計であること。

## 作業ブレークダウン
1. **ABI モデル整備**: `3-9-core-async-ffi-unsafe.md` のテーブルを OCaml でデータ構造化し、ターゲットごとに設定可能にする。
2. **Parser/Typer 拡張**: FFI 宣言に ABI 属性・所有権注釈を付加し、Typer が整合性チェックを実施。
3. **ブリッジコード生成**: それぞれのターゲット向けに stub を生成し、`cbindgen` 等を利用して C ヘッダを導出する検討を行う。
4. **監査ログ出力**: `AuditEnvelope.metadata` に `bridge.stage.*` および ABI 情報を記録。
5. **テストと CI**: x86_64 Linux と Windows x64 の両方でサンプル FFI 呼び出しをビルドし、実行テストを行う。
6. **ドキュメント更新**: 仕様差分を `3-9-core-async-ffi-unsafe.md` および `guides/runtime-bridges.md` に反映し、必要な TODO を明示。

## 成果物と検証
- 双方のターゲットで FFI サンプルが成功し、所有権違反時に診断が出力される。
- `AuditEnvelope` に FFI 呼び出しのトレースが追加され、`0-3-audit-and-metrics.md` で確認できる。
- 仕様ドキュメントの更新がレビュー済みで、記録が残る。

## リスクとフォローアップ
- Windows (MSVC) の呼出規約差異によりバグが潜む恐れがあるため、`2-6-windows-support.md` と連携してテストケースを共有。
- 所有権注釈の表現力が不足している場合、Phase 3 で DSL 拡張を検討する。
- FFI ブリッジ生成に外部ツールを使う場合はライセンス・再現性を `0-3-audit-and-metrics.md` に記録。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [3-9-core-async-ffi-unsafe.md](../../3-9-core-async-ffi-unsafe.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)

