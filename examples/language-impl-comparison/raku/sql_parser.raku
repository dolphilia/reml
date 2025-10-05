#!/usr/bin/env raku

# 簡易SQL Parser - Raku実装
# SELECT, WHERE, JOIN, ORDER BY対応
# Rakuの組み込み文法機能（Grammar）を使用

# AST定義
enum OrderDirection <Asc Desc>;
enum JoinType <InnerJoin LeftJoin RightJoin FullJoin>;
enum BinOp <Add Sub Mul Div Mod Eq Ne Lt Le Gt Ge And Or Like>;
enum UnOp <Not IsNull IsNotNull>;

class Literal {
  has $.value;
  has $.type; # 'int', 'float', 'string', 'bool', 'null'
}

class Expr {
  has $.kind;
  has $.data;
}

class Column {
  has $.kind; # 'all' or 'expr'
  has Expr $.expr;
  has Str $.alias;
}

class TableRef {
  has Str $.table;
  has Str $.alias;
}

class Join {
  has JoinType $.join-type;
  has TableRef $.table;
  has Expr $.on-condition;
}

class OrderBy {
  has @.columns; # Array of (Expr, OrderDirection)
}

class Query {
  has @.columns;
  has TableRef $.from-table;
  has Expr $.where-clause;
  has @.joins;
  has OrderBy $.order-by;
}

# SQL文法定義
grammar SQLGrammar {
  token TOP { <ws> <select-query> <ws> ';'? <ws> }

  token ws { [ \s+ | <line-comment> | <block-comment> ]* }
  token line-comment { '--' \N* }
  token block-comment { '/*' .*? '*/' }

  token select-query {
    <kw-select> <ws> <column-list> <ws>
    <kw-from> <ws> <table-ref> <ws>
    <join-clause>*
    [ <ws> <where-clause> ]?
    [ <ws> <order-by-clause> ]?
  }

  token kw-select { :i 'select' <!before \w> }
  token kw-from { :i 'from' <!before \w> }
  token kw-where { :i 'where' <!before \w> }
  token kw-join { :i 'join' <!before \w> }
  token kw-inner { :i 'inner' <!before \w> }
  token kw-left { :i 'left' <!before \w> }
  token kw-right { :i 'right' <!before \w> }
  token kw-full { :i 'full' <!before \w> }
  token kw-on { :i 'on' <!before \w> }
  token kw-and { :i 'and' <!before \w> }
  token kw-or { :i 'or' <!before \w> }
  token kw-not { :i 'not' <!before \w> }
  token kw-is { :i 'is' <!before \w> }
  token kw-null { :i 'null' <!before \w> }
  token kw-true { :i 'true' <!before \w> }
  token kw-false { :i 'false' <!before \w> }
  token kw-like { :i 'like' <!before \w> }
  token kw-order { :i 'order' <!before \w> }
  token kw-by { :i 'by' <!before \w> }
  token kw-asc { :i 'asc' <!before \w> }
  token kw-desc { :i 'desc' <!before \w> }
  token kw-as { :i 'as' <!before \w> }

  token identifier {
    <[a..zA..Z_]> <[a..zA..Z0..9_]>*
  }

  token column-list {
    '*' | <column-expr>+ % [ <ws> ',' <ws> ]
  }

  token column-expr {
    <expr> [ <ws> <kw-as>? <ws> <identifier> ]?
  }

  token table-ref {
    <identifier> [ <ws> <kw-as>? <ws> <identifier> ]?
  }

  token join-clause {
    <join-type> <ws> <table-ref> <ws> <kw-on> <ws> <expr>
  }

  token join-type {
    [ <kw-inner> <ws> <kw-join> ] |
    [ <kw-left> <ws> <kw-join> ] |
    [ <kw-right> <ws> <kw-join> ] |
    [ <kw-full> <ws> <kw-join> ] |
    <kw-join>
  }

  token where-clause {
    <kw-where> <ws> <expr>
  }

  token order-by-clause {
    <kw-order> <ws> <kw-by> <ws> <order-expr>+ % [ <ws> ',' <ws> ]
  }

  token order-expr {
    <expr> [ <ws> [ <kw-asc> | <kw-desc> ] ]?
  }

  # 式（優先度を考慮）
  token expr { <or-expr> }
  token or-expr { <and-expr>+ % [ <ws> <kw-or> <ws> ] }
  token and-expr { <cmp-expr>+ % [ <ws> <kw-and> <ws> ] }
  token cmp-expr {
    <add-expr> [ <ws> <cmp-op> <ws> <add-expr> ]?
  }
  token cmp-op {
    '=' | '<>' | '!=' | '<=' | '>=' | '<' | '>' | <kw-like>
  }
  token add-expr { <mul-expr>+ % [ <ws> <add-op> <ws> ] }
  token add-op { '+' | '-' }
  token mul-expr { <unary-expr>+ % [ <ws> <mul-op> <ws> ] }
  token mul-op { '*' | '/' | '%' }
  token unary-expr {
    [ <kw-not> <ws> <unary-expr> ] | <postfix-expr>
  }
  token postfix-expr {
    <primary-expr> [ <ws> <kw-is> <ws> <kw-not>? <ws> <kw-null> ]?
  }
  token primary-expr {
    <paren-expr> | <func-call> | <column-ref> | <literal>
  }
  token paren-expr { '(' <ws> <expr> <ws> ')' }
  token func-call {
    <identifier> <ws> '(' <ws> [ <expr>+ % [ <ws> ',' <ws> ] ]? <ws> ')'
  }
  token column-ref {
    <identifier> [ <ws> '.' <ws> <identifier> ]?
  }

  token literal {
    <kw-null> | <kw-true> | <kw-false> | <float-lit> | <integer> | <string-lit>
  }
  token integer { \d+ }
  token float-lit { \d+ '.' \d+ }
  token string-lit { "'" <-[']>* "'" }
}

# アクションクラス（ASTビルド）
class SQLActions {
  method TOP($/) {
    make $<select-query>.made;
  }

  method select-query($/) {
    my @columns = $<column-list>.made;
    my $from = $<table-ref>.made;
    my @joins = $<join-clause>».made;
    my $where = $<where-clause> ?? $<where-clause>.made !! Nil;
    my $order = $<order-by-clause> ?? $<order-by-clause>.made !! Nil;

    make Query.new(
      columns => @columns,
      from-table => $from,
      where-clause => $where,
      joins => @joins,
      order-by => $order
    );
  }

  method column-list($/) {
    if ~$/ eq '*' {
      make [Column.new(kind => 'all')];
    } else {
      make $<column-expr>».made;
    }
  }

  method column-expr($/) {
    my $expr = $<expr>.made;
    my $alias = $<identifier> ?? ~$<identifier> !! Nil;
    make Column.new(kind => 'expr', expr => $expr, alias => $alias);
  }

  method table-ref($/) {
    my @ids = $<identifier>».Str;
    my ($table, $alias) = @ids.elems == 2 ?? (@ids[0], @ids[1]) !! (@ids[0], Nil);
    make TableRef.new(table => $table, alias => $alias);
  }

  method expr($/) {
    make $<or-expr>.made;
  }

  method literal($/) {
    if $<kw-null> {
      make Expr.new(kind => 'literal', data => Literal.new(value => Nil, type => 'null'));
    } elsif $<kw-true> {
      make Expr.new(kind => 'literal', data => Literal.new(value => True, type => 'bool'));
    } elsif $<kw-false> {
      make Expr.new(kind => 'literal', data => Literal.new(value => False, type => 'bool'));
    } elsif $<float-lit> {
      make Expr.new(kind => 'literal', data => Literal.new(value => +~$<float-lit>, type => 'float'));
    } elsif $<integer> {
      make Expr.new(kind => 'literal', data => Literal.new(value => +~$<integer>, type => 'int'));
    } elsif $<string-lit> {
      my $str = ~$<string-lit>;
      $str = $str.substr(1, *-1); # Remove quotes
      make Expr.new(kind => 'literal', data => Literal.new(value => $str, type => 'string'));
    }
  }

  # 他のメソッドは簡略化のため省略
}

# パブリックAPI
sub parse-sql(Str $input) is export {
  my $match = SQLGrammar.parse($input, actions => SQLActions.new);
  return $match ?? $match.made !! Nil;
}

# レンダリング関数
sub render-literal(Literal $lit) {
  given $lit.type {
    when 'int' { ~$lit.value }
    when 'float' { ~$lit.value }
    when 'string' { "'" ~ $lit.value ~ "'" }
    when 'bool' { $lit.value ?? 'TRUE' !! 'FALSE' }
    when 'null' { 'NULL' }
  }
}

sub render-expr(Expr $expr) {
  given $expr.kind {
    when 'literal' { render-literal($expr.data) }
    when 'column' { $expr.data }
    when 'qualified' { $expr.data[0] ~ '.' ~ $expr.data[1] }
    default { '(expr)' }
  }
}

sub render-column(Column $col) {
  if $col.kind eq 'all' {
    '*'
  } else {
    my $str = render-expr($col.expr);
    $str ~= ' AS ' ~ $col.alias if $col.alias;
    $str
  }
}

sub render-query(Query $q) is export {
  my $cols = $q.columns.map(&render-column).join(', ');
  my $from = 'FROM ' ~ $q.from-table.table;
  $from ~= ' AS ' ~ $q.from-table.alias if $q.from-table.alias;

  my $result = "SELECT $cols $from";

  if $q.where-clause {
    $result ~= ' WHERE ' ~ render-expr($q.where-clause);
  }

  return $result;
}

# テスト
sub MAIN() {
  say "=== Raku SQL Parser テスト ===";

  my $test-sql = "SELECT * FROM users WHERE id = 1";
  my $query = parse-sql($test-sql);

  if $query {
    say "パース成功: $test-sql";
    say "レンダリング: ", render-query($query);
  } else {
    say "パースエラー";
  }

  say "";
  say "注: Rakuの文法機能を使った実装です。";
  say "完全な実装にはすべてのアクションメソッドが必要です。";
}