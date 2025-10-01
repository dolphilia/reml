package main

import (
	"fmt"
	"strings"
)

// TemplateNode はテンプレートのAST
type TemplateNode interface {
	templateNode()
}

type TextNode struct {
	Content string
}

func (t TextNode) templateNode() {}

type VarNode struct {
	Name string
}

func (v VarNode) templateNode() {}

type IfNode struct {
	Condition string
	Then      []TemplateNode
	Else      []TemplateNode
}

func (i IfNode) templateNode() {}

type ForNode struct {
	Variable   string
	Collection string
	Body       []TemplateNode
}

func (f ForNode) templateNode() {}

// TemplateParser はテンプレートパーサー
type TemplateParser struct {
	input string
	pos   int
}

func NewTemplateParser(input string) *TemplateParser {
	return &TemplateParser{input: input, pos: 0}
}

func (p *TemplateParser) peek() (rune, bool) {
	if p.pos >= len(p.input) {
		return 0, false
	}
	return rune(p.input[p.pos]), true
}

func (p *TemplateParser) bump() (rune, bool) {
	ch, ok := p.peek()
	if ok {
		p.pos++
	}
	return ch, ok
}

func (p *TemplateParser) atEnd() bool {
	return p.pos >= len(p.input)
}

// ParseTemplate はテンプレートをパース
func ParseTemplate(input string) ([]TemplateNode, error) {
	parser := NewTemplateParser(input)
	return parser.parseNodes()
}

func (p *TemplateParser) parseNodes() ([]TemplateNode, error) {
	nodes := []TemplateNode{}

	for !p.atEnd() {
		ch, _ := p.peek()

		if ch == '{' {
			// {{ または {% をチェック
			if p.pos+1 < len(p.input) {
				next := rune(p.input[p.pos+1])
				if next == '{' {
					// {{ variable }}
					node, err := p.parseVariable()
					if err != nil {
						return nil, err
					}
					nodes = append(nodes, node)
					continue
				} else if next == '%' {
					// {% if/for/end %}
					node, err := p.parseControl()
					if err != nil {
						return nil, err
					}
					if node != nil {
						nodes = append(nodes, node)
					}
					continue
				}
			}
		}

		// 通常のテキスト
		node := p.parseText()
		if node != nil {
			nodes = append(nodes, node)
		}
	}

	return nodes, nil
}

func (p *TemplateParser) parseText() TemplateNode {
	var buf strings.Builder

	for !p.atEnd() {
		ch, _ := p.peek()
		if ch == '{' && p.pos+1 < len(p.input) {
			next := rune(p.input[p.pos+1])
			if next == '{' || next == '%' {
				break
			}
		}
		buf.WriteRune(ch)
		p.bump()
	}

	if buf.Len() > 0 {
		return TextNode{Content: buf.String()}
	}
	return nil
}

func (p *TemplateParser) parseVariable() (TemplateNode, error) {
	// {{ を消費
	p.bump()
	p.bump()

	var name strings.Builder
	for !p.atEnd() {
		ch, _ := p.peek()
		if ch == '}' && p.pos+1 < len(p.input) && rune(p.input[p.pos+1]) == '}' {
			break
		}
		name.WriteRune(ch)
		p.bump()
	}

	// }} を消費
	if p.atEnd() {
		return nil, fmt.Errorf("変数が閉じられていません")
	}
	p.bump() // }
	p.bump() // }

	return VarNode{Name: strings.TrimSpace(name.String())}, nil
}

func (p *TemplateParser) parseControl() (TemplateNode, error) {
	// {% を消費
	p.bump()
	p.bump()

	// コントロール文を取得
	var control strings.Builder
	for !p.atEnd() {
		ch, _ := p.peek()
		if ch == '%' && p.pos+1 < len(p.input) && rune(p.input[p.pos+1]) == '}' {
			break
		}
		control.WriteRune(ch)
		p.bump()
	}

	// %} を消費
	if p.atEnd() {
		return nil, fmt.Errorf("コントロール文が閉じられていません")
	}
	p.bump() // %
	p.bump() // }

	controlStr := strings.TrimSpace(control.String())
	parts := strings.Fields(controlStr)

	if len(parts) == 0 {
		return nil, fmt.Errorf("空のコントロール文")
	}

	switch parts[0] {
	case "if":
		return p.parseIf(parts[1:])
	case "for":
		return p.parseFor(parts[1:])
	case "endif", "endfor":
		// これらは親の parseIf/parseFor で処理される
		return nil, nil
	default:
		return nil, fmt.Errorf("未知のコントロール文: %s", parts[0])
	}
}

func (p *TemplateParser) parseIf(conditionParts []string) (TemplateNode, error) {
	condition := strings.Join(conditionParts, " ")

	// then ブロックをパース
	thenNodes, err := p.parseUntil([]string{"else", "endif"})
	if err != nil {
		return nil, err
	}

	// else ブロックがあるかチェック（簡易実装）
	elseNodes := []TemplateNode{}

	return IfNode{
		Condition: condition,
		Then:      thenNodes,
		Else:      elseNodes,
	}, nil
}

func (p *TemplateParser) parseFor(parts []string) (TemplateNode, error) {
	// {% for item in items %}
	if len(parts) < 3 || parts[1] != "in" {
		return nil, fmt.Errorf("for 文の構文エラー")
	}

	variable := parts[0]
	collection := parts[2]

	// body をパース
	bodyNodes, err := p.parseUntil([]string{"endfor"})
	if err != nil {
		return nil, err
	}

	return ForNode{
		Variable:   variable,
		Collection: collection,
		Body:       bodyNodes,
	}, nil
}

func (p *TemplateParser) parseUntil(endTags []string) ([]TemplateNode, error) {
	nodes := []TemplateNode{}

	for !p.atEnd() {
		// 終了タグをチェック
		savedPos := p.pos
		if ch, _ := p.peek(); ch == '{' && p.pos+1 < len(p.input) && rune(p.input[p.pos+1]) == '%' {
			// {% を消費
			p.bump()
			p.bump()

			var tag strings.Builder
			for !p.atEnd() {
				ch, _ := p.peek()
				if ch == '%' && p.pos+1 < len(p.input) && rune(p.input[p.pos+1]) == '}' {
					break
				}
				tag.WriteRune(ch)
				p.bump()
			}

			// %} を消費
			if !p.atEnd() {
				p.bump()
				p.bump()
			}

			tagStr := strings.TrimSpace(tag.String())
			for _, endTag := range endTags {
				if tagStr == endTag {
					return nodes, nil
				}
			}

			// 終了タグでなければ位置を戻す
			p.pos = savedPos
		}

		// 通常のノードをパース
		if ch, _ := p.peek(); ch == '{' {
			if p.pos+1 < len(p.input) {
				next := rune(p.input[p.pos+1])
				if next == '{' {
					node, err := p.parseVariable()
					if err != nil {
						return nil, err
					}
					nodes = append(nodes, node)
					continue
				}
			}
		}

		node := p.parseText()
		if node != nil {
			nodes = append(nodes, node)
		}
	}

	return nodes, nil
}

// RenderTemplate はテンプレートをレンダリング
func RenderTemplate(nodes []TemplateNode, context map[string]interface{}) (string, error) {
	var result strings.Builder

	for _, node := range nodes {
		switch n := node.(type) {
		case TextNode:
			result.WriteString(n.Content)

		case VarNode:
			if val, ok := context[n.Name]; ok {
				result.WriteString(fmt.Sprintf("%v", val))
			}

		case IfNode:
			// 簡易実装: 変数が存在するかチェック
			if _, ok := context[n.Condition]; ok {
				rendered, err := RenderTemplate(n.Then, context)
				if err != nil {
					return "", err
				}
				result.WriteString(rendered)
			} else {
				rendered, err := RenderTemplate(n.Else, context)
				if err != nil {
					return "", err
				}
				result.WriteString(rendered)
			}

		case ForNode:
			// 簡易実装: スライスのみ対応
			if val, ok := context[n.Collection]; ok {
				if items, ok := val.([]interface{}); ok {
					for _, item := range items {
						newContext := make(map[string]interface{})
						for k, v := range context {
							newContext[k] = v
						}
						newContext[n.Variable] = item
						rendered, err := RenderTemplate(n.Body, newContext)
						if err != nil {
							return "", err
						}
						result.WriteString(rendered)
					}
				}
			}
		}
	}

	return result.String(), nil
}

// テスト例
func main() {
	template := `Hello, {{ name }}!

{% for item in items %}
  - {{ item }}
{% endfor %}

{% if admin %}
Admin mode enabled.
{% endif %}`

	nodes, err := ParseTemplate(template)
	if err != nil {
		fmt.Printf("パースエラー: %v\n", err)
		return
	}

	context := map[string]interface{}{
		"name":  "Reml",
		"items": []interface{}{"Parser", "Effects", "Inference"},
		"admin": true,
	}

	result, err := RenderTemplate(nodes, context)
	if err != nil {
		fmt.Printf("レンダリングエラー: %v\n", err)
		return
	}

	fmt.Println(result)
}
