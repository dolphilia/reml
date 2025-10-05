package main

import (
	"fmt"
	"strconv"
	"strings"
)

// YamlValue は YAML値を表す型
type YamlValue interface {
	yamlValue()
}

type YamlNull struct{}

func (y YamlNull) yamlValue() {}

type YamlBool struct {
	Value bool
}

func (y YamlBool) yamlValue() {}

type YamlNumber struct {
	Value float64
}

func (y YamlNumber) yamlValue() {}

type YamlString struct {
	Value string
}

func (y YamlString) yamlValue() {}

type YamlArray struct {
	Items []YamlValue
}

func (y YamlArray) yamlValue() {}

type YamlObject struct {
	Pairs map[string]YamlValue
}

func (y YamlObject) yamlValue() {}

// YamlParser は YAMLパーサー
type YamlParser struct {
	lines []string
	pos   int
}

func NewYamlParser(input string) *YamlParser {
	lines := strings.Split(input, "\n")
	return &YamlParser{lines: lines, pos: 0}
}

func (p *YamlParser) peek() string {
	if p.pos >= len(p.lines) {
		return ""
	}
	return p.lines[p.pos]
}

func (p *YamlParser) bump() string {
	line := p.peek()
	p.pos++
	return line
}

func (p *YamlParser) atEnd() bool {
	return p.pos >= len(p.lines)
}

// ParseYaml は YAML をパース（簡易実装）
func ParseYaml(input string) (YamlValue, error) {
	parser := NewYamlParser(input)
	return parser.parseValue(0)
}

func (p *YamlParser) parseValue(indent int) (YamlValue, error) {
	if p.atEnd() {
		return YamlNull{}, nil
	}

	line := p.peek()
	trimmed := strings.TrimSpace(line)

	// 空行やコメントはスキップ
	if trimmed == "" || strings.HasPrefix(trimmed, "#") {
		p.bump()
		return p.parseValue(indent)
	}

	// インデントレベルを計算
	currentIndent := len(line) - len(strings.TrimLeft(line, " "))

	// インデントが浅い場合は終了
	if currentIndent < indent {
		return YamlNull{}, nil
	}

	// 配列アイテム
	if strings.HasPrefix(trimmed, "- ") {
		return p.parseArray(currentIndent)
	}

	// キー: 値
	if strings.Contains(trimmed, ":") {
		return p.parseObject(currentIndent)
	}

	// スカラー値
	p.bump()
	return p.parseScalar(trimmed)
}

func (p *YamlParser) parseArray(indent int) (YamlValue, error) {
	items := []YamlValue{}

	for !p.atEnd() {
		line := p.peek()
		trimmed := strings.TrimSpace(line)

		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			p.bump()
			continue
		}

		currentIndent := len(line) - len(strings.TrimLeft(line, " "))

		if currentIndent < indent {
			break
		}

		if !strings.HasPrefix(trimmed, "- ") {
			break
		}

		p.bump()
		itemStr := strings.TrimPrefix(trimmed, "- ")
		itemStr = strings.TrimSpace(itemStr)

		if itemStr == "" {
			// 次の行に値がある場合
			value, err := p.parseValue(currentIndent + 2)
			if err != nil {
				return nil, err
			}
			items = append(items, value)
		} else {
			value, err := p.parseScalar(itemStr)
			if err != nil {
				return nil, err
			}
			items = append(items, value)
		}
	}

	return YamlArray{Items: items}, nil
}

func (p *YamlParser) parseObject(indent int) (YamlValue, error) {
	pairs := make(map[string]YamlValue)

	for !p.atEnd() {
		line := p.peek()
		trimmed := strings.TrimSpace(line)

		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			p.bump()
			continue
		}

		currentIndent := len(line) - len(strings.TrimLeft(line, " "))

		if currentIndent < indent {
			break
		}

		if !strings.Contains(trimmed, ":") {
			break
		}

		p.bump()

		parts := strings.SplitN(trimmed, ":", 2)
		if len(parts) != 2 {
			return nil, fmt.Errorf("無効なキー:値ペア: %s", trimmed)
		}

		key := strings.TrimSpace(parts[0])
		valueStr := strings.TrimSpace(parts[1])

		var value YamlValue
		var err error

		if valueStr == "" {
			// 次の行に値がある場合
			value, err = p.parseValue(currentIndent + 2)
			if err != nil {
				return nil, err
			}
		} else {
			value, err = p.parseScalar(valueStr)
			if err != nil {
				return nil, err
			}
		}

		pairs[key] = value
	}

	return YamlObject{Pairs: pairs}, nil
}

func (p *YamlParser) parseScalar(value string) (YamlValue, error) {
	// null
	if value == "null" || value == "~" {
		return YamlNull{}, nil
	}

	// boolean
	if value == "true" {
		return YamlBool{Value: true}, nil
	}
	if value == "false" {
		return YamlBool{Value: false}, nil
	}

	// 数値
	if num, err := strconv.ParseFloat(value, 64); err == nil {
		return YamlNumber{Value: num}, nil
	}

	// 文字列（引用符を除去）
	if strings.HasPrefix(value, "\"") && strings.HasSuffix(value, "\"") {
		value = strings.Trim(value, "\"")
	} else if strings.HasPrefix(value, "'") && strings.HasSuffix(value, "'") {
		value = strings.Trim(value, "'")
	}

	return YamlString{Value: value}, nil
}

// テスト例
func main() {
	yaml := `
name: Reml
version: 1.0
features:
  - パーサーコンビネーター
  - 代数的効果
  - 型推論
config:
  debug: true
  timeout: 30
`

	value, err := ParseYaml(yaml)
	if err != nil {
		fmt.Printf("パースエラー: %v\n", err)
		return
	}

	if obj, ok := value.(YamlObject); ok {
		fmt.Printf("パース成功: %d キー\n", len(obj.Pairs))
		for k, v := range obj.Pairs {
			fmt.Printf("  %s: %T\n", k, v)
		}
	} else {
		fmt.Printf("パース成功: %T\n", value)
	}
}
