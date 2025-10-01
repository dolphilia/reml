package main

import (
	"fmt"
	"strings"
	"unicode"
)

// SQLNode は SQL の AST ノード
type SQLNode interface {
	sqlNode()
}

type SelectStmt struct {
	Columns []string
	From    string
	Where   Expr
	Joins   []JoinClause
}

func (s SelectStmt) sqlNode() {}

type JoinClause struct {
	Table string
	On    Expr
}

type Expr interface {
	expr()
}

type ColumnExpr struct {
	Name string
}

func (c ColumnExpr) expr() {}

type LiteralExpr struct {
	Value string
}

func (l LiteralExpr) expr() {}

type BinaryOp struct {
	Op  string
	Lhs Expr
	Rhs Expr
}

func (b BinaryOp) expr() {}

// SQLParser は SQL パーサー
type SQLParser struct {
	tokens []string
	pos    int
}

func NewSQLParser(input string) *SQLParser {
	tokens := tokenizeSQL(input)
	return &SQLParser{tokens: tokens, pos: 0}
}

func tokenizeSQL(input string) []string {
	tokens := []string{}
	var current strings.Builder

	for _, ch := range input {
		if unicode.IsSpace(ch) {
			if current.Len() > 0 {
				tokens = append(tokens, current.String())
				current.Reset()
			}
		} else if ch == ',' || ch == '(' || ch == ')' || ch == '=' || ch == '<' || ch == '>' {
			if current.Len() > 0 {
				tokens = append(tokens, current.String())
				current.Reset()
			}
			tokens = append(tokens, string(ch))
		} else {
			current.WriteRune(ch)
		}
	}

	if current.Len() > 0 {
		tokens = append(tokens, current.String())
	}

	return tokens
}

func (p *SQLParser) peek() string {
	if p.pos >= len(p.tokens) {
		return ""
	}
	return p.tokens[p.pos]
}

func (p *SQLParser) bump() string {
	token := p.peek()
	p.pos++
	return token
}

func (p *SQLParser) atEnd() bool {
	return p.pos >= len(p.tokens)
}

func (p *SQLParser) expect(token string) error {
	current := p.bump()
	if strings.ToUpper(current) != strings.ToUpper(token) {
		return fmt.Errorf("期待: %s, 実際: %s", token, current)
	}
	return nil
}

// ParseSQL は SQL をパース
func ParseSQL(input string) (SQLNode, error) {
	parser := NewSQLParser(input)

	token := strings.ToUpper(parser.peek())
	switch token {
	case "SELECT":
		return parser.parseSelect()
	default:
		return nil, fmt.Errorf("未対応のSQL: %s", token)
	}
}

func (p *SQLParser) parseSelect() (SQLNode, error) {
	if err := p.expect("SELECT"); err != nil {
		return nil, err
	}

	// カラムリスト
	columns := []string{}
	for {
		if p.atEnd() {
			return nil, fmt.Errorf("予期しないEOF")
		}

		token := p.peek()
		if strings.ToUpper(token) == "FROM" {
			break
		}

		if token == "," {
			p.bump()
			continue
		}

		columns = append(columns, p.bump())
	}

	// FROM句
	if err := p.expect("FROM"); err != nil {
		return nil, err
	}

	tableName := p.bump()
	if tableName == "" {
		return nil, fmt.Errorf("テーブル名が必要です")
	}

	stmt := SelectStmt{
		Columns: columns,
		From:    tableName,
		Joins:   []JoinClause{},
	}

	// JOINとWHERE句の解析
	for !p.atEnd() {
		token := strings.ToUpper(p.peek())

		switch token {
		case "JOIN", "INNER", "LEFT", "RIGHT":
			join, err := p.parseJoin()
			if err != nil {
				return nil, err
			}
			stmt.Joins = append(stmt.Joins, join)

		case "WHERE":
			p.bump()
			where, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			stmt.Where = where
			return stmt, nil

		default:
			// 未知のキーワードは無視
			p.bump()
		}
	}

	return stmt, nil
}

func (p *SQLParser) parseJoin() (JoinClause, error) {
	token := strings.ToUpper(p.peek())

	// JOIN キーワードのスキップ
	if token == "LEFT" || token == "RIGHT" || token == "INNER" {
		p.bump()
	}

	if err := p.expect("JOIN"); err != nil {
		return JoinClause{}, err
	}

	tableName := p.bump()
	if tableName == "" {
		return JoinClause{}, fmt.Errorf("JOIN テーブル名が必要です")
	}

	if err := p.expect("ON"); err != nil {
		return JoinClause{}, err
	}

	onExpr, err := p.parseExpr()
	if err != nil {
		return JoinClause{}, err
	}

	return JoinClause{Table: tableName, On: onExpr}, nil
}

func (p *SQLParser) parseExpr() (Expr, error) {
	// 簡易版: 二項演算のみ対応
	left, err := p.parsePrimary()
	if err != nil {
		return nil, err
	}

	// 演算子チェック
	if !p.atEnd() {
		token := p.peek()
		if token == "=" || token == "<" || token == ">" || strings.ToUpper(token) == "AND" || strings.ToUpper(token) == "OR" {
			op := p.bump()
			right, err := p.parsePrimary()
			if err != nil {
				return nil, err
			}
			return BinaryOp{Op: op, Lhs: left, Rhs: right}, nil
		}
	}

	return left, nil
}

func (p *SQLParser) parsePrimary() (Expr, error) {
	if p.atEnd() {
		return nil, fmt.Errorf("予期しないEOF")
	}

	token := p.bump()

	// リテラル（文字列または数値）
	if strings.HasPrefix(token, "'") || unicode.IsDigit(rune(token[0])) {
		return LiteralExpr{Value: token}, nil
	}

	// カラム名
	return ColumnExpr{Name: token}, nil
}

// テスト例
func main() {
	testCases := []string{
		"SELECT id, name FROM users",
		"SELECT * FROM products WHERE price > 100",
		"SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id",
		"SELECT name FROM customers WHERE country = 'Japan'",
	}

	for _, sql := range testCases {
		fmt.Printf("SQL: %s\n", sql)
		node, err := ParseSQL(sql)
		if err != nil {
			fmt.Printf("  エラー: %v\n", err)
		} else {
			if stmt, ok := node.(SelectStmt); ok {
				fmt.Printf("  カラム: %v\n", stmt.Columns)
				fmt.Printf("  FROM: %s\n", stmt.From)
				if stmt.Where != nil {
					fmt.Printf("  WHERE: %T\n", stmt.Where)
				}
				if len(stmt.Joins) > 0 {
					fmt.Printf("  JOINs: %d\n", len(stmt.Joins))
				}
			}
		}
		fmt.Println()
	}
}
