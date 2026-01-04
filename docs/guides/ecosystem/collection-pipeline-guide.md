# コレクション収集戦略ガイド

Reml の `Iter`/`Collector` と `List`/`Map`/`Table` を組み合わせた収集戦略を整理する実装ガイド。Chapter 3.1/3.2 の仕様と `examples/language-impl-samples/` の実例を紐づけ、DSL 作者が安全かつ高速にデータパイプラインを構築できるようにする。

## 1. 前提と設計原則

- **性能の維持**: 収集処理は 0-1 章の性能指標（線形時間・低メモリ）を満たすよう、構造共有と短絡 (`try_collect`) を活用する。【F:0-1-project-purpose.md†L11-L37】
- **安全性の確保**: `CollectError` を `Result` で伝播し、未処理例外を排除する。`MapCollector` など順序保証を持つコンテナを用いれば監査ログと列情報の一貫性を維持できる。【F:3-6-core-diagnostics-audit.md†L42-L88】
- **学習コストの抑制**: `List.fold` と `Iter.fold` の評価順が一致することを明示し、DSL 作者が概念を再学習せずに済む導線を用意する。【F:3-1-core-prelude-iteration.md†L188-L197】【F:3-2-core-collections.md†L83-L89】

## 2. 典型的な収集パターン

| シナリオ | 推奨 API | 利点 | 注意点 |
| --- | --- | --- | --- |
| 逐次処理の保持 | `Iter.collect_list`, `Iter.collect_vec` | 入力順をそのまま維持。テンプレートの `List.fold` と親和性。 | `Vec` を選ぶ場合は `effect {mut}` が伝搬する点を明記。 |
| ソート済みマップ生成 | `Iter.try_collect(MapCollector::new)` | キー昇順・重複検知を自動化。 | `CollectError::DuplicateKey` をハンドリングし、診断へ変換。 |
| 挿入順マップ | `Iter.try_collect(TableCollector::new)` | DSL のエラー提示で元データ順を保持。 | `table_to_map` でソートされる点をコメントに残す。 |
| 差分マージ | `Map.merge`, `Set.diff` | 大規模設定差分を線形に処理。 | `Iter.from_list` → `try_collect` のような2段構成では中間の順序変化を明示。 |

## 3. 実装レシピ

### 3.1 YAML パーサの `Map` 収集

`Map.from_list` は簡潔だが、重複キーを後勝ちで黙殺する危険がある。`MapCollector` を経由すると `CollectError` を DSL 側で補足できる。

```reml
fn parse_map(indent: Int) -> Parser<YamlValue> =
  rule(format("map.{indent}"),
    Parse.many1(parse_map_entry(indent).skipL(Parse.opt(newline)))
      .and_then(|entries|
        entries
          |> Iter.from_list
          |> Iter.try_collect(MapCollector::new(|existing, next| {
               match existing {
                 Some(_) -> Err(CollectError::DuplicateKey(next.0))
                 None -> Ok(Some(next.1))
               }
             }))
      )
      .map(|result|
        match result {
          Ok(map) -> Ok(Map(map))
          Err(err) -> Err(ParseError::from_collect(err))
        }
      )
  )
```

- `Iter.from_list` で訪問順を保持しつつ、`MapCollector` がキー昇順へ再配置する点を `Diagnostic` の期待行に反映する。【F:3-1-core-prelude-iteration.md†L192-L197】
- 実際のサンプルでは簡潔さを重視して `Map.from_list` を使っているが、設定ファイルなどで重複検出が重要な場合は本レシピを採用する。【F:examples/language-impl-samples/reml/yaml_parser.reml†L155-L163】

### 3.2 テンプレート DSL のリスト操作

テンプレートエンジンでは `List.fold` を利用したフィルター適用が頻出する。`Iter` を介すと `?` による早期終了を挿入しやすい。

```reml
fn render_variable(name: Ident, filters: List<Filter>, ctx: Context)
  -> Result<String, Diagnostic> =
  let value = get_value(ctx, name)?;

  filters
    |> Iter.from_list
    |> Iter.try_fold(value, |acc, filter|
         apply_filter(filter, acc)
       )
    |> Result.map(|filtered|
         value_to_string(filtered)
       )
```

- `apply_filter` が `Result` を返すよう拡張すると、カスタムフィルターで発生したエラーを `Diagnostic` へ橋渡しできる。
- `List.fold` と同等の挙動を保ちつつ、途中で `Err` を返した時点で残りのフィルター評価を打ち切れる。【F:examples/language-impl-samples/reml/template_engine.reml†L343-L355】

### 3.3 Table と Map の切り替え

挿入順が重要な DSL（例: SQL 風 AST、ログ収集 DSL）では `Table` を終端とし、比較表示時だけ `Map` へ昇格させる。

- `TableCollector` を使う場合は `effect {mut}` が必要となるため、外部公開 API では `Result<Table<_,_>, Diagnostic>` として副作用境界を明示する。
- 監査出力時にソート済み構造が必要であれば `table_to_map` を呼び出し、`Map` 化が順序を昇順へ再配置することをコメントに残す。【F:3-2-core-collections.md†L83-L89】

## 4. エラーハンドリングと診断統合

1. `CollectError` を `IntoDiagnostic` 実装で `Diagnostic::conflict_key` などへ変換する。
2. 監査ログへ渡す場合は `Collections.audit_bridge`（提案中）を経由し、`change_set` に衝突キーや順序情報を残す。【F:3-6-core-diagnostics-audit.md†L42-L88】
3. DSL 側では `Result` をそのまま `ParseError`/`TemplateError` に包み、発生箇所の `span` と期待キー集合を同期させる。

## 5. チェックリスト

- [ ] `Iter` から `List`/`Map`/`Table` への収集で訪問順変化の有無をコメント化した。
- [ ] `CollectError` を `Result` で捕捉し、未処理の `panic` が存在しない。
- [ ] 挿入順が必要な箇所で `Table`/`Vec` を採用し、`effect` タグを明記した。
- [ ] サンプル DSL から本ガイドへのリンクを README 等で案内した。

## 6. 参考資料

- [3.1 Core Prelude & Iteration](../../spec/3-1-core-prelude-iteration.md)
- [3.2 Core Collections](../../spec/3-2-core-collections.md)
- [examples/language-impl-samples/README.md](../../spec/README.md)
- [Reml プロジェクトの目的と指針](../../spec/0-1-project-purpose.md)
