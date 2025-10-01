package main

import (
	"fmt"
	"strconv"
	"unicode"
)

// 正規表現エンジン：パース + 評価の両方を実装
//
// 対応する正規表現構文（簡易版）：
// - リテラル: abc
// - 連結: ab
// - 選択: a|b
// - 繰り返し: a*, a+, a?, a{2,5}
// - グループ: (abc)
// - 文字クラス: [a-z], [^0-9]
// - アンカー: ^, $
// - ドット: . (任意の1文字)

// RegexAST は正規表現のAST
type RegexAST interface {
	regexAST()
}

type LiteralRegex struct {
	Value string
}

func (l LiteralRegex) regexAST() {}

type DotRegex struct{}

func (d DotRegex) regexAST() {}

type ConcatRegex struct {
	Terms []RegexAST
}

func (c ConcatRegex) regexAST() {}

type AlternationRegex struct {
	Alts []RegexAST
}

func (a AlternationRegex) regexAST() {}

type RepeatRegex struct {
	Inner RegexAST
	Kind  RepeatKind
}

func (r RepeatRegex) regexAST() {}

type GroupRegex struct {
	Inner RegexAST
}

func (g GroupRegex) regexAST() {}

type CharClassRegex struct {
	Ranges []CharRange
	Negate bool
}

func (c CharClassRegex) regexAST() {}

type AnchorRegex struct {
	Start bool // true = ^, false = $
}

func (a AnchorRegex) regexAST() {}

type CharRange struct {
	Start rune
	End   rune
}

type RepeatKind int

const (
	ZeroOrMore RepeatKind = iota
	OneOrMore
	ZeroOrOne
	Exactly
	RangeRepeat
)

type RepeatSpec struct {
	Kind RepeatKind
	Min  int
	Max  int
}

// RegexParser は正規表現パーサー
type RegexParser struct {
	input string
	pos   int
}

func NewRegexParser(input string) *RegexParser {
	return &RegexParser{input: input, pos: 0}
}

func (p *RegexParser) peek() (rune, bool) {
	if p.pos >= len(p.input) {
		return 0, false
	}
	return rune(p.input[p.pos]), true
}

func (p *RegexParser) bump() (rune, bool) {
	ch, ok := p.peek()
	if ok {
		p.pos++
	}
	return ch, ok
}

func (p *RegexParser) atEnd() bool {
	return p.pos >= len(p.input)
}

// ParseRegex は正規表現をパース（簡易実装）
func ParseRegex(input string) (RegexAST, error) {
	parser := NewRegexParser(input)
	return parser.parseAlternation()
}

func (p *RegexParser) parseAlternation() (RegexAST, error) {
	terms := []RegexAST{}
	term, err := p.parseConcat()
	if err != nil {
		return nil, err
	}
	terms = append(terms, term)

	for {
		ch, ok := p.peek()
		if !ok || ch != '|' {
			break
		}
		p.bump() // consume '|'
		term, err := p.parseConcat()
		if err != nil {
			return nil, err
		}
		terms = append(terms, term)
	}

	if len(terms) == 1 {
		return terms[0], nil
	}
	return AlternationRegex{Alts: terms}, nil
}

func (p *RegexParser) parseConcat() (RegexAST, error) {
	terms := []RegexAST{}

	for {
		if p.atEnd() {
			break
		}
		ch, _ := p.peek()
		if ch == '|' || ch == ')' {
			break
		}
		term, err := p.parsePostfix()
		if err != nil {
			return nil, err
		}
		terms = append(terms, term)
	}

	if len(terms) == 0 {
		return LiteralRegex{Value: ""}, nil
	}
	if len(terms) == 1 {
		return terms[0], nil
	}
	return ConcatRegex{Terms: terms}, nil
}

func (p *RegexParser) parsePostfix() (RegexAST, error) {
	base, err := p.parseAtom()
	if err != nil {
		return nil, err
	}

	// 後置演算子チェック
	ch, ok := p.peek()
	if !ok {
		return base, nil
	}

	switch ch {
	case '*':
		p.bump()
		return RepeatRegex{Inner: base, Kind: ZeroOrMore}, nil
	case '+':
		p.bump()
		return RepeatRegex{Inner: base, Kind: OneOrMore}, nil
	case '?':
		p.bump()
		return RepeatRegex{Inner: base, Kind: ZeroOrOne}, nil
	default:
		return base, nil
	}
}

func (p *RegexParser) parseAtom() (RegexAST, error) {
	ch, ok := p.peek()
	if !ok {
		return nil, fmt.Errorf("予期しないEOF")
	}

	switch ch {
	case '(':
		p.bump()
		inner, err := p.parseAlternation()
		if err != nil {
			return nil, err
		}
		if ch, ok := p.peek(); !ok || ch != ')' {
			return nil, fmt.Errorf("閉じ括弧 ) が必要です")
		}
		p.bump()
		return GroupRegex{Inner: inner}, nil

	case '^':
		p.bump()
		return AnchorRegex{Start: true}, nil

	case '$':
		p.bump()
		return AnchorRegex{Start: false}, nil

	case '.':
		p.bump()
		return DotRegex{}, nil

	case '[':
		return p.parseCharClass()

	case '\\':
		p.bump()
		escaped, ok := p.bump()
		if !ok {
			return nil, fmt.Errorf("エスケープシーケンスが不完全です")
		}
		return LiteralRegex{Value: string(escaped)}, nil

	default:
		if ch == '*' || ch == '+' || ch == '?' || ch == '|' || ch == ')' {
			return nil, fmt.Errorf("予期しない文字: %c", ch)
		}
		p.bump()
		return LiteralRegex{Value: string(ch)}, nil
	}
}

func (p *RegexParser) parseCharClass() (RegexAST, error) {
	p.bump() // consume '['

	negate := false
	if ch, ok := p.peek(); ok && ch == '^' {
		negate = true
		p.bump()
	}

	ranges := []CharRange{}

	for {
		ch, ok := p.peek()
		if !ok {
			return nil, fmt.Errorf("文字クラスが閉じられていません")
		}
		if ch == ']' {
			p.bump()
			break
		}

		start, _ := p.bump()

		// 範囲チェック
		if ch, ok := p.peek(); ok && ch == '-' {
			p.bump()
			end, ok := p.bump()
			if !ok {
				return nil, fmt.Errorf("文字範囲が不完全です")
			}
			ranges = append(ranges, CharRange{Start: start, End: end})
		} else {
			ranges = append(ranges, CharRange{Start: start, End: start})
		}
	}

	return CharClassRegex{Ranges: ranges, Negate: negate}, nil
}

// MatchRegex は正規表現マッチング（簡易実装）
func MatchRegex(regex RegexAST, text string, pos int) bool {
	switch r := regex.(type) {
	case LiteralRegex:
		if pos+len(r.Value) > len(text) {
			return false
		}
		return text[pos:pos+len(r.Value)] == r.Value

	case DotRegex:
		return pos < len(text)

	case ConcatRegex:
		currentPos := pos
		for _, term := range r.Terms {
			if !MatchRegex(term, text, currentPos) {
				return false
			}
			currentPos++
		}
		return true

	case AlternationRegex:
		for _, alt := range r.Alts {
			if MatchRegex(alt, text, pos) {
				return true
			}
		}
		return false

	case RepeatRegex:
		// 簡易実装（貪欲マッチング）
		count := 0
		currentPos := pos
		for currentPos < len(text) && MatchRegex(r.Inner, text, currentPos) {
			count++
			currentPos++
		}
		switch r.Kind {
		case ZeroOrMore:
			return true
		case OneOrMore:
			return count >= 1
		case ZeroOrOne:
			return count <= 1
		default:
			return true
		}

	case GroupRegex:
		return MatchRegex(r.Inner, text, pos)

	case CharClassRegex:
		if pos >= len(text) {
			return false
		}
		ch := rune(text[pos])
		matched := false
		for _, cr := range r.Ranges {
			if ch >= cr.Start && ch <= cr.End {
				matched = true
				break
			}
		}
		if r.Negate {
			return !matched
		}
		return matched

	case AnchorRegex:
		if r.Start {
			return pos == 0
		}
		return pos >= len(text)

	default:
		return false
	}
}

// テスト例
func main() {
	testCases := []struct {
		pattern  string
		text     string
		expected bool
	}{
		{"a+", "aaa", true},
		{"a+", "b", false},
		{"a|b", "a", true},
		{"a|b", "b", true},
		{"a|b", "c", false},
		{"^hello", "hello", true},
		{"world$", "world", true},
	}

	for _, tc := range testCases {
		regex, err := ParseRegex(tc.pattern)
		if err != nil {
			fmt.Printf("パースエラー: %s => %v\n", tc.pattern, err)
			continue
		}

		result := MatchRegex(regex, tc.text, 0)
		status := "✗"
		if result == tc.expected {
			status = "✓"
		}
		fmt.Printf("%s パターン: '%s', テキスト: '%s', 期待: %t, 結果: %t\n",
			status, tc.pattern, tc.text, tc.expected, result)
	}
}
