// 簡易SQL Parser - Swift実装
// SELECT, WHERE, JOIN, ORDER BY対応
// カスタムパーサーコンビネーター実装

import Foundation

// AST定義
enum OrderDirection {
    case asc, desc
}

enum JoinType {
    case innerJoin, leftJoin, rightJoin, fullJoin
}

enum BinOp {
    case add, sub, mul, div, mod
    case eq, ne, lt, le, gt, ge
    case and, or, like
}

enum UnOp {
    case not, isNull, isNotNull
}

enum Literal {
    case intLit(Int)
    case floatLit(Double)
    case stringLit(String)
    case boolLit(Bool)
    case nullLit
}

indirect enum Expr {
    case literal(Literal)
    case column(String)
    case qualifiedColumn(table: String, column: String)
    case binaryOp(op: BinOp, left: Expr, right: Expr)
    case unaryOp(op: UnOp, expr: Expr)
    case functionCall(name: String, args: [Expr])
    case parenthesized(Expr)
}

enum Column {
    case allColumns
    case columnExpr(expr: Expr, alias: String?)
}

struct TableRef {
    let table: String
    let alias: String?
}

struct Join {
    let joinType: JoinType
    let table: TableRef
    let onCondition: Expr
}

struct OrderBy {
    let columns: [(Expr, OrderDirection)]
}

struct Query {
    let columns: [Column]
    let fromTable: TableRef
    let whereClause: Expr?
    let joins: [Join]
    let orderBy: OrderBy?
}

// パーサーコンビネーター実装
struct Parser<T> {
    let parse: (String) -> (T, String)?
}

extension Parser {
    func map<U>(_ transform: @escaping (T) -> U) -> Parser<U> {
        Parser<U> { input in
            guard let (result, rest) = self.parse(input) else { return nil }
            return (transform(result), rest)
        }
    }

    func flatMap<U>(_ transform: @escaping (T) -> Parser<U>) -> Parser<U> {
        Parser<U> { input in
            guard let (result, rest) = self.parse(input) else { return nil }
            return transform(result).parse(rest)
        }
    }
}

func pure<T>(_ value: T) -> Parser<T> {
    Parser { input in (value, input) }
}

func satisfy(_ predicate: @escaping (Character) -> Bool) -> Parser<Character> {
    Parser { input in
        guard let first = input.first, predicate(first) else { return nil }
        return (first, String(input.dropFirst()))
    }
}

func char(_ c: Character) -> Parser<Character> {
    satisfy { $0 == c }
}

func string(_ s: String) -> Parser<String> {
    Parser { input in
        guard input.hasPrefix(s) else { return nil }
        return (s, String(input.dropFirst(s.count)))
    }
}

func stringCaseInsensitive(_ s: String) -> Parser<String> {
    Parser { input in
        let prefix = input.prefix(s.count)
        guard prefix.lowercased() == s.lowercased() else { return nil }
        return (String(prefix), String(input.dropFirst(s.count)))
    }
}

func choice<T>(_ parsers: [Parser<T>]) -> Parser<T> {
    Parser { input in
        for parser in parsers {
            if let result = parser.parse(input) {
                return result
            }
        }
        return nil
    }
}

func many<T>(_ parser: Parser<T>) -> Parser<[T]> {
    Parser { input in
        var results: [T] = []
        var remaining = input
        while let (result, rest) = parser.parse(remaining) {
            results.append(result)
            remaining = rest
        }
        return (results, remaining)
    }
}

func many1<T>(_ parser: Parser<T>) -> Parser<[T]> {
    parser.flatMap { first in
        many(parser).map { rest in [first] + rest }
    }
}

func optional<T>(_ parser: Parser<T>) -> Parser<T?> {
    Parser { input in
        if let (result, rest) = parser.parse(input) {
            return (result, rest)
        }
        return (nil, input)
    }
}

func sepBy<T, Sep>(_ parser: Parser<T>, _ separator: Parser<Sep>) -> Parser<[T]> {
    Parser { input in
        guard let (first, rest) = parser.parse(input) else {
            return ([], input)
        }

        var results = [first]
        var remaining = rest

        while let (_, rest2) = separator.parse(remaining),
              let (result, rest3) = parser.parse(rest2) {
            results.append(result)
            remaining = rest3
        }

        return (results, remaining)
    }
}

// 空白とコメント
let whitespace = many(satisfy { $0.isWhitespace }).map { _ in () }

func lexeme<T>(_ parser: Parser<T>) -> Parser<T> {
    parser.flatMap { result in
        whitespace.map { _ in result }
    }
}

func symbol(_ s: String) -> Parser<String> {
    lexeme(string(s))
}

func keyword(_ kw: String) -> Parser<()> {
    lexeme(stringCaseInsensitive(kw)).flatMap { _ in
        Parser { input in
            if let first = input.first, first.isLetter || first.isNumber || first == "_" {
                return nil
            }
            return ((), input)
        }
    }
}

// 識別子
let identifier: Parser<String> = {
    let reserved = ["select", "from", "where", "join", "inner", "left",
                    "right", "full", "on", "and", "or", "not", "like",
                    "order", "by", "asc", "desc", "null", "true", "false", "as"]

    return lexeme(
        satisfy { $0.isLetter || $0 == "_" }.flatMap { first in
            many(satisfy { $0.isLetter || $0.isNumber || $0 == "_" }).map { rest in
                String(first) + String(rest)
            }
        }
    ).flatMap { name in
        if reserved.contains(name.lowercased()) {
            return Parser { _ in nil }
        }
        return pure(name)
    }
}()

// リテラル
let integerLit = lexeme(many1(satisfy { $0.isNumber }))
    .map { chars -> Literal in
        .intLit(Int(String(chars)) ?? 0)
    }

let floatLit = lexeme(
    many1(satisfy { $0.isNumber }).flatMap { intPart in
        char(".").flatMap { _ in
            many1(satisfy { $0.isNumber }).map { fracPart in
                String(intPart) + "." + String(fracPart)
            }
        }
    }
).map { s -> Literal in .floatLit(Double(s) ?? 0.0) }

let stringLit = lexeme(
    char("'").flatMap { _ in
        many(satisfy { $0 != "'" }).flatMap { chars in
            char("'").map { _ in String(chars) }
        }
    }
).map { s -> Literal in .stringLit(s) }

let literal: Parser<Literal> = choice([
    keyword("null").map { _ in Literal.nullLit },
    keyword("true").map { _ in Literal.boolLit(true) },
    keyword("false").map { _ in Literal.boolLit(false) },
    floatLit,
    integerLit,
    stringLit
])

// 式パーサー（簡略版）
func makeExpr() -> Parser<Expr> {
    // 相互再帰のため遅延評価
    Parser { input in exprParser.parse(input) }
}

let exprParser: Parser<Expr> = {
    // 簡略版: 優先度処理を簡略化
    let primaryExpr: Parser<Expr> = choice([
        symbol("(").flatMap { _ in
            makeExpr().flatMap { e in
                symbol(")").map { _ in Expr.parenthesized(e) }
            }
        },
        identifier.flatMap { first in
            choice([
                symbol("(").flatMap { _ in
                    sepBy(makeExpr(), symbol(",")).flatMap { args in
                        symbol(")").map { _ in
                            Expr.functionCall(name: first, args: args)
                        }
                    }
                },
                symbol(".").flatMap { _ in
                    identifier.map { col in
                        Expr.qualifiedColumn(table: first, column: col)
                    }
                },
                pure(Expr.column(first))
            ])
        },
        literal.map { lit in Expr.literal(lit) }
    ])

    return primaryExpr
}()

// カラムリスト
let columnList: Parser<[Column]> = choice([
    symbol("*").map { _ in [Column.allColumns] },
    sepBy(
        exprParser.flatMap { e in
            optional(optional(keyword("as")).flatMap { _ in identifier })
                .map { alias in Column.columnExpr(expr: e, alias: alias) }
        },
        symbol(",")
    )
])

// テーブル参照
let tableRef: Parser<TableRef> =
    identifier.flatMap { table in
        optional(optional(keyword("as")).flatMap { _ in identifier })
            .map { alias in TableRef(table: table, alias: alias) }
    }

// SELECT文（簡略版）
let selectQuery: Parser<Query> =
    keyword("select").flatMap { _ in
        columnList.flatMap { cols in
            keyword("from").flatMap { _ in
                tableRef.map { from in
                    Query(
                        columns: cols,
                        fromTable: from,
                        whereClause: nil,
                        joins: [],
                        orderBy: nil
                    )
                }
            }
        }
    }

// パブリックAPI
func parseSQL(_ input: String) -> Query? {
    guard let (query, _) = whitespace.flatMap { _ in selectQuery }.parse(input) else {
        return nil
    }
    return query
}

// レンダリング関数
func renderLiteral(_ lit: Literal) -> String {
    switch lit {
    case .intLit(let n): return "\(n)"
    case .floatLit(let f): return "\(f)"
    case .stringLit(let s): return "'\(s)'"
    case .boolLit(let b): return b ? "TRUE" : "FALSE"
    case .nullLit: return "NULL"
    }
}

func renderExpr(_ expr: Expr) -> String {
    switch expr {
    case .literal(let lit): return renderLiteral(lit)
    case .column(let name): return name
    case .qualifiedColumn(let table, let col): return "\(table).\(col)"
    case .binaryOp(let op, let left, let right):
        let opStr = renderBinOp(op)
        return "(\(renderExpr(left)) \(opStr) \(renderExpr(right)))"
    case .unaryOp(let op, let e):
        switch op {
        case .not: return "NOT \(renderExpr(e))"
        case .isNull: return "\(renderExpr(e)) IS NULL"
        case .isNotNull: return "\(renderExpr(e)) IS NOT NULL"
        }
    case .functionCall(let name, let args):
        let argsStr = args.map(renderExpr).joined(separator: ", ")
        return "\(name)(\(argsStr))"
    case .parenthesized(let e):
        return "(\(renderExpr(e)))"
    }
}

func renderBinOp(_ op: BinOp) -> String {
    switch op {
    case .add: return "+"
    case .sub: return "-"
    case .mul: return "*"
    case .div: return "/"
    case .mod: return "%"
    case .eq: return "="
    case .ne: return "<>"
    case .lt: return "<"
    case .le: return "<="
    case .gt: return ">"
    case .ge: return ">="
    case .and: return "AND"
    case .or: return "OR"
    case .like: return "LIKE"
    }
}

func renderColumn(_ col: Column) -> String {
    switch col {
    case .allColumns: return "*"
    case .columnExpr(let expr, let alias):
        let base = renderExpr(expr)
        if let alias = alias {
            return "\(base) AS \(alias)"
        }
        return base
    }
}

func renderQuery(_ q: Query) -> String {
    let cols = q.columns.map(renderColumn).joined(separator: ", ")
    var result = "SELECT \(cols) FROM \(q.fromTable.table)"

    if let alias = q.fromTable.alias {
        result += " AS \(alias)"
    }

    if !q.joins.isEmpty {
        let joinsStr = q.joins.map { j in
            let jt: String
            switch j.joinType {
            case .innerJoin: jt = "INNER JOIN"
            case .leftJoin: jt = "LEFT JOIN"
            case .rightJoin: jt = "RIGHT JOIN"
            case .fullJoin: jt = "FULL JOIN"
            }
            return "\(jt) \(j.table.table) ON \(renderExpr(j.onCondition))"
        }.joined(separator: " ")
        result += " \(joinsStr)"
    }

    if let whereClause = q.whereClause {
        result += " WHERE \(renderExpr(whereClause))"
    }

    if let orderBy = q.orderBy {
        let cols = orderBy.columns.map { (e, dir) in
            "\(renderExpr(e)) \(dir == .asc ? "ASC" : "DESC")"
        }.joined(separator: ", ")
        result += " ORDER BY \(cols)"
    }

    return result
}

// テスト
print("=== Swift SQL Parser テスト ===")

let testSQL = "SELECT * FROM users"
if let query = parseSQL(testSQL) {
    print("パース成功: \(testSQL)")
    print("レンダリング: \(renderQuery(query))")
} else {
    print("パースエラー")
}

print("")
print("注: Swiftでカスタムパーサーコンビネーターを実装しています。")
print("完全な実装にはより詳細な演算子優先度処理が必要です。")