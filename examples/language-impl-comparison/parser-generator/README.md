# パーサージェネレーター比較サンプル

`pl0-antlr` は ANTLR4 を用いて PL/0 風の言語構文を扱う最小例です。`PL0.g4` が構文定義、`Pl0Driver.java` が実行例となります。

## 利用メモ

1. `antlr4 PL0.g4` を実行して Java コードを生成します。
2. `javac PL0*.java Pl0Driver.java` でコンパイルします。
3. `java Pl0Driver "const a = 1; var x; begin x := a; write x end ."` のように実行すると、構文木が標準出力に表示されます。

> **補足**: テキストは日本語コメントを優先しつつ、ANTLR のキーワードは英語表記を維持しています。
