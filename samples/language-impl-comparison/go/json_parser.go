package main

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
)

// JsonValue は JSON値を表す型
type JsonValue interface {
	jsonValue()
}

type JsonNull struct{}

func (j JsonNull) jsonValue() {}

type JsonBool struct {
	Value bool
}

func (j JsonBool) jsonValue() {}

type JsonNumber struct {
	Value float64
}

func (j JsonNumber) jsonValue() {}

type JsonString struct {
	Value string
}

func (j JsonString) jsonValue() {}

type JsonArray struct {
	Items []JsonValue
}

func (j JsonArray) jsonValue() {}

type JsonObject struct {
	Pairs map[string]JsonValue
}

func (j JsonObject) jsonValue() {}

// Parser はパーサー状態を保持
type Parser struct {
	input string
	pos   int
}

func NewParser(input string) *Parser {
	return &Parser{input: input, pos: 0}
}

func (p *Parser) peek() (rune, bool) {
	if p.pos >= len(p.input) {
		return 0, false
	}
	return rune(p.input[p.pos]), true
}

func (p *Parser) bump() (rune, bool) {
	ch, ok := p.peek()
	if ok {
		p.pos++
	}
	return ch, ok
}

func (p *Parser) skipWhitespace() {
	for {
		ch, ok := p.peek()
		if !ok || !unicode.IsSpace(ch) {
			break
		}
		p.bump()
	}
}

func (p *Parser) expect(target rune) error {
	ch, ok := p.bump()
	if !ok {
		return fmt.Errorf("予期しないEOF: '%c' を期待", target)
	}
	if ch != target {
		return fmt.Errorf("予期しない文字: '%c' を期待しましたが '%c' でした", target, ch)
	}
	return nil
}

func (p *Parser) expectLiteral(literal string) error {
	for _, expected := range literal {
		ch, ok := p.bump()
		if !ok || ch != expected {
			return fmt.Errorf("リテラル '%s' のパースに失敗", literal)
		}
	}
	return nil
}

// JSON値のパース
func (p *Parser) parseValue() (JsonValue, error) {
	p.skipWhitespace()
	ch, ok := p.peek()
	if !ok {
		return nil, fmt.Errorf("予期しないEOF")
	}

	switch ch {
	case 'n':
		return p.parseNull()
	case 't', 'f':
		return p.parseBool()
	case '"':
		return p.parseString()
	case '[':
		return p.parseArray()
	case '{':
		return p.parseObject()
	case '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9':
		return p.parseNumber()
	default:
		return nil, fmt.Errorf("予期しない文字: '%c'", ch)
	}
}

func (p *Parser) parseNull() (JsonValue, error) {
	if err := p.expectLiteral("null"); err != nil {
		return nil, err
	}
	return JsonNull{}, nil
}

func (p *Parser) parseBool() (JsonValue, error) {
	ch, _ := p.peek()
	if ch == 't' {
		if err := p.expectLiteral("true"); err != nil {
			return nil, err
		}
		return JsonBool{Value: true}, nil
	} else {
		if err := p.expectLiteral("false"); err != nil {
			return nil, err
		}
		return JsonBool{Value: false}, nil
	}
}

func (p *Parser) parseNumber() (JsonValue, error) {
	start := p.pos
	// 符号
	if ch, ok := p.peek(); ok && ch == '-' {
		p.bump()
	}
	// 整数部
	for {
		ch, ok := p.peek()
		if !ok || !unicode.IsDigit(ch) {
			break
		}
		p.bump()
	}
	// 小数部
	if ch, ok := p.peek(); ok && ch == '.' {
		p.bump()
		for {
			ch, ok := p.peek()
			if !ok || !unicode.IsDigit(ch) {
				break
			}
			p.bump()
		}
	}
	// 指数部
	if ch, ok := p.peek(); ok && (ch == 'e' || ch == 'E') {
		p.bump()
		if ch, ok := p.peek(); ok && (ch == '+' || ch == '-') {
			p.bump()
		}
		for {
			ch, ok := p.peek()
			if !ok || !unicode.IsDigit(ch) {
				break
			}
			p.bump()
		}
	}

	literal := p.input[start:p.pos]
	value, err := strconv.ParseFloat(literal, 64)
	if err != nil {
		return nil, fmt.Errorf("数値パースエラー: %s", literal)
	}
	return JsonNumber{Value: value}, nil
}

func (p *Parser) parseString() (JsonValue, error) {
	if err := p.expect('"'); err != nil {
		return nil, err
	}

	var buf strings.Builder
	for {
		ch, ok := p.bump()
		if !ok {
			return nil, fmt.Errorf("文字列が閉じられていません")
		}
		if ch == '"' {
			return JsonString{Value: buf.String()}, nil
		}
		if ch == '\\' {
			escaped, ok := p.bump()
			if !ok {
				return nil, fmt.Errorf("エスケープシーケンスが不完全です")
			}
			switch escaped {
			case '"':
				buf.WriteRune('"')
			case '\\':
				buf.WriteRune('\\')
			case '/':
				buf.WriteRune('/')
			case 'b':
				buf.WriteRune('\b')
			case 'f':
				buf.WriteRune('\f')
			case 'n':
				buf.WriteRune('\n')
			case 'r':
				buf.WriteRune('\r')
			case 't':
				buf.WriteRune('\t')
			case 'u':
				// Unicode エスケープ（簡易版）
				hex := ""
				for i := 0; i < 4; i++ {
					ch, ok := p.bump()
					if !ok {
						return nil, fmt.Errorf("Unicode エスケープが不完全です")
					}
					hex += string(ch)
				}
				code, err := strconv.ParseInt(hex, 16, 32)
				if err != nil {
					return nil, fmt.Errorf("無効な Unicode エスケープ: %s", hex)
				}
				buf.WriteRune(rune(code))
			default:
				return nil, fmt.Errorf("無効なエスケープシーケンス: \\%c", escaped)
			}
		} else {
			buf.WriteRune(ch)
		}
	}
}

func (p *Parser) parseArray() (JsonValue, error) {
	if err := p.expect('['); err != nil {
		return nil, err
	}

	items := []JsonValue{}
	p.skipWhitespace()

	// 空配列チェック
	if ch, ok := p.peek(); ok && ch == ']' {
		p.bump()
		return JsonArray{Items: items}, nil
	}

	for {
		value, err := p.parseValue()
		if err != nil {
			return nil, err
		}
		items = append(items, value)

		p.skipWhitespace()
		ch, ok := p.peek()
		if !ok {
			return nil, fmt.Errorf("配列が閉じられていません")
		}

		if ch == ',' {
			p.bump()
			continue
		} else if ch == ']' {
			p.bump()
			break
		} else {
			return nil, fmt.Errorf("予期しない文字: '%c'", ch)
		}
	}

	return JsonArray{Items: items}, nil
}

func (p *Parser) parseObject() (JsonValue, error) {
	if err := p.expect('{'); err != nil {
		return nil, err
	}

	pairs := make(map[string]JsonValue)
	p.skipWhitespace()

	// 空オブジェクトチェック
	if ch, ok := p.peek(); ok && ch == '}' {
		p.bump()
		return JsonObject{Pairs: pairs}, nil
	}

	for {
		p.skipWhitespace()
		keyVal, err := p.parseString()
		if err != nil {
			return nil, fmt.Errorf("オブジェクトのキーは文字列である必要があります: %v", err)
		}
		key := keyVal.(JsonString).Value

		p.skipWhitespace()
		if err := p.expect(':'); err != nil {
			return nil, err
		}

		value, err := p.parseValue()
		if err != nil {
			return nil, err
		}
		pairs[key] = value

		p.skipWhitespace()
		ch, ok := p.peek()
		if !ok {
			return nil, fmt.Errorf("オブジェクトが閉じられていません")
		}

		if ch == ',' {
			p.bump()
			continue
		} else if ch == '}' {
			p.bump()
			break
		} else {
			return nil, fmt.Errorf("予期しない文字: '%c'", ch)
		}
	}

	return JsonObject{Pairs: pairs}, nil
}

// Parse は JSON文字列をパースする
func Parse(source string) (JsonValue, error) {
	p := NewParser(source)
	value, err := p.parseValue()
	if err != nil {
		return nil, err
	}
	p.skipWhitespace()
	if p.pos < len(p.input) {
		return nil, fmt.Errorf("未消費入力があります")
	}
	return value, nil
}

// テスト例
func main() {
	testCases := []string{
		`{"number": 42, "ok": true}`,
		`[1, 2, 3, 4, 5]`,
		`{"name": "Reml", "nested": {"version": 1.0}}`,
		`null`,
		`true`,
		`false`,
		`123.456`,
		`"hello world"`,
	}

	for _, tc := range testCases {
		result, err := Parse(tc)
		if err != nil {
			fmt.Printf("エラー: %s => %v\n", tc, err)
		} else {
			fmt.Printf("成功: %s => %T\n", tc, result)
		}
	}
}
