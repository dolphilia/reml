package main

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
)

// JsonExtValue は拡張JSON値を表す型
type JsonExtValue interface {
	jsonExtValue()
}

type JsonExtNull struct{}

func (j JsonExtNull) jsonExtValue() {}

type JsonExtBool struct {
	Value bool
}

func (j JsonExtBool) jsonExtValue() {}

type JsonExtNumber struct {
	Value float64
}

func (j JsonExtNumber) jsonExtValue() {}

type JsonExtString struct {
	Value string
}

func (j JsonExtString) jsonExtValue() {}

type JsonExtArray struct {
	Items []JsonExtValue
}

func (j JsonExtArray) jsonExtValue() {}

type JsonExtObject struct {
	Pairs map[string]JsonExtValue
}

func (j JsonExtObject) jsonExtValue() {}

// JsonExtParser は拡張JSONパーサー
// 拡張機能:
// - コメント: // と /* */
// - 末尾カンマ許容
// - 16進数リテラル: 0x1F
// - NaN と Infinity
type JsonExtParser struct {
	input string
	pos   int
}

func NewJsonExtParser(input string) *JsonExtParser {
	return &JsonExtParser{input: input, pos: 0}
}

func (p *JsonExtParser) peek() (rune, bool) {
	if p.pos >= len(p.input) {
		return 0, false
	}
	return rune(p.input[p.pos]), true
}

func (p *JsonExtParser) bump() (rune, bool) {
	ch, ok := p.peek()
	if ok {
		p.pos++
	}
	return ch, ok
}

func (p *JsonExtParser) skipWhitespaceAndComments() {
	for {
		ch, ok := p.peek()
		if !ok {
			break
		}

		// 空白をスキップ
		if unicode.IsSpace(ch) {
			p.bump()
			continue
		}

		// コメントをスキップ
		if ch == '/' && p.pos+1 < len(p.input) {
			next := rune(p.input[p.pos+1])
			if next == '/' {
				// 単一行コメント
				p.bump()
				p.bump()
				for {
					ch, ok := p.peek()
					if !ok || ch == '\n' {
						break
					}
					p.bump()
				}
				continue
			} else if next == '*' {
				// 複数行コメント
				p.bump()
				p.bump()
				for {
					ch, ok := p.peek()
					if !ok {
						break
					}
					if ch == '*' && p.pos+1 < len(p.input) && rune(p.input[p.pos+1]) == '/' {
						p.bump()
						p.bump()
						break
					}
					p.bump()
				}
				continue
			}
		}

		break
	}
}

func (p *JsonExtParser) expect(target rune) error {
	ch, ok := p.bump()
	if !ok {
		return fmt.Errorf("予期しないEOF: '%c' を期待", target)
	}
	if ch != target {
		return fmt.Errorf("予期しない文字: '%c' を期待しましたが '%c' でした", target, ch)
	}
	return nil
}

func (p *JsonExtParser) expectLiteral(literal string) error {
	for _, expected := range literal {
		ch, ok := p.bump()
		if !ok || ch != expected {
			return fmt.Errorf("リテラル '%s' のパースに失敗", literal)
		}
	}
	return nil
}

// ParseJsonExt は拡張JSONをパース
func ParseJsonExt(input string) (JsonExtValue, error) {
	p := NewJsonExtParser(input)
	value, err := p.parseValue()
	if err != nil {
		return nil, err
	}
	p.skipWhitespaceAndComments()
	if p.pos < len(p.input) {
		return nil, fmt.Errorf("未消費入力があります")
	}
	return value, nil
}

func (p *JsonExtParser) parseValue() (JsonExtValue, error) {
	p.skipWhitespaceAndComments()
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
	case 'N': // NaN
		return p.parseNaN()
	case 'I': // Infinity
		return p.parseInfinity()
	default:
		return nil, fmt.Errorf("予期しない文字: '%c'", ch)
	}
}

func (p *JsonExtParser) parseNull() (JsonExtValue, error) {
	if err := p.expectLiteral("null"); err != nil {
		return nil, err
	}
	return JsonExtNull{}, nil
}

func (p *JsonExtParser) parseBool() (JsonExtValue, error) {
	ch, _ := p.peek()
	if ch == 't' {
		if err := p.expectLiteral("true"); err != nil {
			return nil, err
		}
		return JsonExtBool{Value: true}, nil
	} else {
		if err := p.expectLiteral("false"); err != nil {
			return nil, err
		}
		return JsonExtBool{Value: false}, nil
	}
}

func (p *JsonExtParser) parseNaN() (JsonExtValue, error) {
	if err := p.expectLiteral("NaN"); err != nil {
		return nil, err
	}
	return JsonExtNumber{Value: 0.0}, nil // 簡易実装
}

func (p *JsonExtParser) parseInfinity() (JsonExtValue, error) {
	if err := p.expectLiteral("Infinity"); err != nil {
		return nil, err
	}
	return JsonExtNumber{Value: 1e308}, nil // 簡易実装
}

func (p *JsonExtParser) parseNumber() (JsonExtValue, error) {
	start := p.pos

	// 16進数チェック
	if ch, _ := p.peek(); ch == '0' && p.pos+1 < len(p.input) && rune(p.input[p.pos+1]) == 'x' {
		p.bump() // 0
		p.bump() // x
		hexStr := ""
		for {
			ch, ok := p.peek()
			if !ok || !isHexDigit(ch) {
				break
			}
			hexStr += string(ch)
			p.bump()
		}
		value, err := strconv.ParseInt(hexStr, 16, 64)
		if err != nil {
			return nil, fmt.Errorf("16進数パースエラー: %s", hexStr)
		}
		return JsonExtNumber{Value: float64(value)}, nil
	}

	// 通常の数値（10進数）
	if ch, ok := p.peek(); ok && ch == '-' {
		p.bump()
	}
	for {
		ch, ok := p.peek()
		if !ok || !unicode.IsDigit(ch) {
			break
		}
		p.bump()
	}
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
	return JsonExtNumber{Value: value}, nil
}

func isHexDigit(ch rune) bool {
	return (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}

func (p *JsonExtParser) parseString() (JsonExtValue, error) {
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
			return JsonExtString{Value: buf.String()}, nil
		}
		if ch == '\\' {
			escaped, ok := p.bump()
			if !ok {
				return nil, fmt.Errorf("エスケープシーケンスが不完全です")
			}
			switch escaped {
			case '"', '\\', '/', 'b', 'f', 'n', 'r', 't':
				buf.WriteRune(escaped)
			default:
				return nil, fmt.Errorf("無効なエスケープシーケンス: \\%c", escaped)
			}
		} else {
			buf.WriteRune(ch)
		}
	}
}

func (p *JsonExtParser) parseArray() (JsonExtValue, error) {
	if err := p.expect('['); err != nil {
		return nil, err
	}

	items := []JsonExtValue{}
	p.skipWhitespaceAndComments()

	if ch, ok := p.peek(); ok && ch == ']' {
		p.bump()
		return JsonExtArray{Items: items}, nil
	}

	for {
		value, err := p.parseValue()
		if err != nil {
			return nil, err
		}
		items = append(items, value)

		p.skipWhitespaceAndComments()
		ch, ok := p.peek()
		if !ok {
			return nil, fmt.Errorf("配列が閉じられていません")
		}

		if ch == ',' {
			p.bump()
			p.skipWhitespaceAndComments()
			// 末尾カンマチェック
			if ch, ok := p.peek(); ok && ch == ']' {
				p.bump()
				break
			}
			continue
		} else if ch == ']' {
			p.bump()
			break
		} else {
			return nil, fmt.Errorf("予期しない文字: '%c'", ch)
		}
	}

	return JsonExtArray{Items: items}, nil
}

func (p *JsonExtParser) parseObject() (JsonExtValue, error) {
	if err := p.expect('{'); err != nil {
		return nil, err
	}

	pairs := make(map[string]JsonExtValue)
	p.skipWhitespaceAndComments()

	if ch, ok := p.peek(); ok && ch == '}' {
		p.bump()
		return JsonExtObject{Pairs: pairs}, nil
	}

	for {
		p.skipWhitespaceAndComments()
		keyVal, err := p.parseString()
		if err != nil {
			return nil, fmt.Errorf("オブジェクトのキーは文字列である必要があります: %v", err)
		}
		key := keyVal.(JsonExtString).Value

		p.skipWhitespaceAndComments()
		if err := p.expect(':'); err != nil {
			return nil, err
		}

		value, err := p.parseValue()
		if err != nil {
			return nil, err
		}
		pairs[key] = value

		p.skipWhitespaceAndComments()
		ch, ok := p.peek()
		if !ok {
			return nil, fmt.Errorf("オブジェクトが閉じられていません")
		}

		if ch == ',' {
			p.bump()
			p.skipWhitespaceAndComments()
			// 末尾カンマチェック
			if ch, ok := p.peek(); ok && ch == '}' {
				p.bump()
				break
			}
			continue
		} else if ch == '}' {
			p.bump()
			break
		} else {
			return nil, fmt.Errorf("予期しない文字: '%c'", ch)
		}
	}

	return JsonExtObject{Pairs: pairs}, nil
}

// テスト例
func main() {
	testCases := []string{
		`{"number": 42, "ok": true, /* コメント */ }`,
		`[1, 2, 3, ]`, // 末尾カンマ
		`{"hex": 0x1F, "name": "Reml"}`,
		`// コメント
		{"value": 123}`,
		`{"nan": NaN, "inf": Infinity}`,
	}

	for _, tc := range testCases {
		result, err := ParseJsonExt(tc)
		if err != nil {
			fmt.Printf("エラー: %s => %v\n", tc, err)
		} else {
			fmt.Printf("成功: %s => %T\n", tc, result)
		}
	}
}
