package main

import (
	"fmt"
	"strconv"
	"strings"
)

// TomlValue は TOML値を表す型
type TomlValue interface {
	tomlValue()
}

type TomlBool struct {
	Value bool
}

func (t TomlBool) tomlValue() {}

type TomlNumber struct {
	Value float64
}

func (t TomlNumber) tomlValue() {}

type TomlString struct {
	Value string
}

func (t TomlString) tomlValue() {}

type TomlArray struct {
	Items []TomlValue
}

func (t TomlArray) tomlValue() {}

type TomlTable struct {
	Pairs map[string]TomlValue
}

func (t TomlTable) tomlValue() {}

// TomlParser は TOMLパーサー
type TomlParser struct {
	lines        []string
	pos          int
	currentTable string
	tables       map[string]map[string]TomlValue
}

func NewTomlParser(input string) *TomlParser {
	lines := strings.Split(input, "\n")
	return &TomlParser{
		lines:        lines,
		pos:          0,
		currentTable: "",
		tables:       make(map[string]map[string]TomlValue),
	}
}

func (p *TomlParser) peek() string {
	if p.pos >= len(p.lines) {
		return ""
	}
	return p.lines[p.pos]
}

func (p *TomlParser) bump() string {
	line := p.peek()
	p.pos++
	return line
}

func (p *TomlParser) atEnd() bool {
	return p.pos >= len(p.lines)
}

// ParseToml は TOML をパース（簡易実装）
func ParseToml(input string) (TomlValue, error) {
	parser := NewTomlParser(input)

	// デフォルトテーブル
	parser.tables[""] = make(map[string]TomlValue)

	for !parser.atEnd() {
		line := parser.peek()
		trimmed := strings.TrimSpace(line)

		// 空行やコメントはスキップ
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			parser.bump()
			continue
		}

		// テーブルヘッダー
		if strings.HasPrefix(trimmed, "[") && strings.HasSuffix(trimmed, "]") {
			parser.bump()
			tableName := strings.Trim(trimmed, "[]")
			tableName = strings.TrimSpace(tableName)
			parser.currentTable = tableName
			if _, exists := parser.tables[tableName]; !exists {
				parser.tables[tableName] = make(map[string]TomlValue)
			}
			continue
		}

		// キー = 値
		if strings.Contains(trimmed, "=") {
			parser.bump()
			if err := parser.parseKeyValue(trimmed); err != nil {
				return nil, err
			}
			continue
		}

		// 未知の行はスキップ
		parser.bump()
	}

	// 結果を構築
	result := make(map[string]TomlValue)
	for tableName, pairs := range parser.tables {
		if tableName == "" {
			// ルートレベルのキー
			for k, v := range pairs {
				result[k] = v
			}
		} else {
			result[tableName] = TomlTable{Pairs: pairs}
		}
	}

	return TomlTable{Pairs: result}, nil
}

func (p *TomlParser) parseKeyValue(line string) error {
	parts := strings.SplitN(line, "=", 2)
	if len(parts) != 2 {
		return fmt.Errorf("無効なキー=値ペア: %s", line)
	}

	key := strings.TrimSpace(parts[0])
	valueStr := strings.TrimSpace(parts[1])

	value, err := p.parseValue(valueStr)
	if err != nil {
		return err
	}

	if p.tables[p.currentTable] == nil {
		p.tables[p.currentTable] = make(map[string]TomlValue)
	}
	p.tables[p.currentTable][key] = value

	return nil
}

func (p *TomlParser) parseValue(valueStr string) (TomlValue, error) {
	valueStr = strings.TrimSpace(valueStr)

	// 配列
	if strings.HasPrefix(valueStr, "[") && strings.HasSuffix(valueStr, "]") {
		return p.parseArray(valueStr)
	}

	// 文字列
	if (strings.HasPrefix(valueStr, "\"") && strings.HasSuffix(valueStr, "\"")) ||
		(strings.HasPrefix(valueStr, "'") && strings.HasSuffix(valueStr, "'")) {
		value := strings.Trim(valueStr, "\"'")
		return TomlString{Value: value}, nil
	}

	// boolean
	if valueStr == "true" {
		return TomlBool{Value: true}, nil
	}
	if valueStr == "false" {
		return TomlBool{Value: false}, nil
	}

	// 数値
	if num, err := strconv.ParseFloat(valueStr, 64); err == nil {
		return TomlNumber{Value: num}, nil
	}

	// デフォルトは文字列
	return TomlString{Value: valueStr}, nil
}

func (p *TomlParser) parseArray(arrayStr string) (TomlValue, error) {
	arrayStr = strings.Trim(arrayStr, "[]")
	arrayStr = strings.TrimSpace(arrayStr)

	if arrayStr == "" {
		return TomlArray{Items: []TomlValue{}}, nil
	}

	items := []TomlValue{}
	parts := strings.Split(arrayStr, ",")

	for _, part := range parts {
		part = strings.TrimSpace(part)
		value, err := p.parseValue(part)
		if err != nil {
			return nil, err
		}
		items = append(items, value)
	}

	return TomlArray{Items: items}, nil
}

// テスト例
func main() {
	toml := `
# TOML 設定ファイル例

title = "Reml Language"
version = 1.0

[owner]
name = "Reml Team"
active = true

[database]
host = "localhost"
port = 5432
connection_max = 5000
enabled = true

[servers]
alpha = "192.168.1.1"
beta = "192.168.1.2"

[features]
list = ["parser", "effects", "inference"]
`

	value, err := ParseToml(toml)
	if err != nil {
		fmt.Printf("パースエラー: %v\n", err)
		return
	}

	if table, ok := value.(TomlTable); ok {
		fmt.Printf("パース成功: %d キー\n", len(table.Pairs))
		for k, v := range table.Pairs {
			fmt.Printf("  %s: %T\n", k, v)
		}
	} else {
		fmt.Printf("パース成功: %T\n", value)
	}
}
