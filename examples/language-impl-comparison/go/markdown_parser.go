package main

import (
	"fmt"
	"regexp"
	"strings"
)

// MarkdownNode は Markdown の AST ノード
type MarkdownNode interface {
	mdNode()
}

type Heading struct {
	Level   int
	Content string
}

func (h Heading) mdNode() {}

type Paragraph struct {
	Content string
}

func (p Paragraph) mdNode() {}

type CodeBlock struct {
	Language string
	Content  string
}

func (c CodeBlock) mdNode() {}

type UnorderedList struct {
	Items []string
}

func (u UnorderedList) mdNode() {}

type OrderedList struct {
	Items []string
}

func (o OrderedList) mdNode() {}

type Blockquote struct {
	Content string
}

func (b Blockquote) mdNode() {}

type HorizontalRule struct{}

func (h HorizontalRule) mdNode() {}

// Parser は Markdown パーサー
type MarkdownParser struct {
	lines []string
	pos   int
}

func NewMarkdownParser(input string) *MarkdownParser {
	lines := strings.Split(input, "\n")
	return &MarkdownParser{lines: lines, pos: 0}
}

func (p *MarkdownParser) peek() string {
	if p.pos >= len(p.lines) {
		return ""
	}
	return p.lines[p.pos]
}

func (p *MarkdownParser) bump() string {
	line := p.peek()
	p.pos++
	return line
}

func (p *MarkdownParser) atEnd() bool {
	return p.pos >= len(p.lines)
}

// ParseMarkdown は Markdown をパース
func ParseMarkdown(input string) []MarkdownNode {
	parser := NewMarkdownParser(input)
	nodes := []MarkdownNode{}

	for !parser.atEnd() {
		line := parser.peek()

		// 空行はスキップ
		if strings.TrimSpace(line) == "" {
			parser.bump()
			continue
		}

		// 見出し
		if node := parser.tryHeading(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// コードブロック
		if node := parser.tryCodeBlock(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// 水平線
		if node := parser.tryHorizontalRule(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// 順序なしリスト
		if node := parser.tryUnorderedList(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// 順序付きリスト
		if node := parser.tryOrderedList(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// 引用
		if node := parser.tryBlockquote(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// 段落（デフォルト）
		if node := parser.tryParagraph(); node != nil {
			nodes = append(nodes, node)
			continue
		}

		// どれにもマッチしない場合はスキップ
		parser.bump()
	}

	return nodes
}

func (p *MarkdownParser) tryHeading() MarkdownNode {
	line := p.peek()
	re := regexp.MustCompile(`^(#{1,6})\s+(.+)$`)
	matches := re.FindStringSubmatch(line)
	if matches != nil {
		p.bump()
		level := len(matches[1])
		content := matches[2]
		return Heading{Level: level, Content: content}
	}
	return nil
}

func (p *MarkdownParser) tryCodeBlock() MarkdownNode {
	line := p.peek()
	if strings.HasPrefix(line, "```") {
		p.bump()
		language := strings.TrimPrefix(line, "```")
		language = strings.TrimSpace(language)

		var content strings.Builder
		for !p.atEnd() {
			line := p.bump()
			if strings.HasPrefix(line, "```") {
				return CodeBlock{Language: language, Content: content.String()}
			}
			if content.Len() > 0 {
				content.WriteString("\n")
			}
			content.WriteString(line)
		}
	}
	return nil
}

func (p *MarkdownParser) tryHorizontalRule() MarkdownNode {
	line := p.peek()
	if regexp.MustCompile(`^(-{3,}|\*{3,}|_{3,})$`).MatchString(strings.TrimSpace(line)) {
		p.bump()
		return HorizontalRule{}
	}
	return nil
}

func (p *MarkdownParser) tryUnorderedList() MarkdownNode {
	line := p.peek()
	re := regexp.MustCompile(`^[\*\-\+]\s+(.+)$`)
	if re.MatchString(line) {
		items := []string{}
		for !p.atEnd() {
			line := p.peek()
			matches := re.FindStringSubmatch(line)
			if matches == nil {
				break
			}
			items = append(items, matches[1])
			p.bump()
		}
		if len(items) > 0 {
			return UnorderedList{Items: items}
		}
	}
	return nil
}

func (p *MarkdownParser) tryOrderedList() MarkdownNode {
	line := p.peek()
	re := regexp.MustCompile(`^\d+\.\s+(.+)$`)
	if re.MatchString(line) {
		items := []string{}
		for !p.atEnd() {
			line := p.peek()
			matches := re.FindStringSubmatch(line)
			if matches == nil {
				break
			}
			items = append(items, matches[1])
			p.bump()
		}
		if len(items) > 0 {
			return OrderedList{Items: items}
		}
	}
	return nil
}

func (p *MarkdownParser) tryBlockquote() MarkdownNode {
	line := p.peek()
	if strings.HasPrefix(line, ">") {
		var content strings.Builder
		for !p.atEnd() {
			line := p.peek()
			if !strings.HasPrefix(line, ">") {
				break
			}
			if content.Len() > 0 {
				content.WriteString("\n")
			}
			content.WriteString(strings.TrimPrefix(line, ">"))
			p.bump()
		}
		return Blockquote{Content: strings.TrimSpace(content.String())}
	}
	return nil
}

func (p *MarkdownParser) tryParagraph() MarkdownNode {
	var content strings.Builder
	for !p.atEnd() {
		line := p.peek()
		if strings.TrimSpace(line) == "" {
			break
		}
		// 特殊な構文の開始をチェック
		if strings.HasPrefix(line, "#") || strings.HasPrefix(line, "```") ||
			strings.HasPrefix(line, ">") || regexp.MustCompile(`^[\*\-\+]\s`).MatchString(line) ||
			regexp.MustCompile(`^\d+\.\s`).MatchString(line) ||
			regexp.MustCompile(`^(-{3,}|\*{3,}|_{3,})$`).MatchString(strings.TrimSpace(line)) {
			break
		}
		if content.Len() > 0 {
			content.WriteString(" ")
		}
		content.WriteString(strings.TrimSpace(line))
		p.bump()
	}
	if content.Len() > 0 {
		return Paragraph{Content: content.String()}
	}
	return nil
}

// テスト例
func main() {
	markdown := `# Title

This is a paragraph.

## Subtitle

- Item 1
- Item 2
- Item 3

` + "```go\nfunc main() {}\n```" + `

> This is a quote

---

1. First
2. Second
3. Third
`

	nodes := ParseMarkdown(markdown)
	for i, node := range nodes {
		fmt.Printf("[%d] %T: ", i, node)
		switch n := node.(type) {
		case Heading:
			fmt.Printf("Level %d: %s\n", n.Level, n.Content)
		case Paragraph:
			fmt.Printf("%s\n", n.Content)
		case CodeBlock:
			fmt.Printf("Language: %s\n", n.Language)
		case UnorderedList:
			fmt.Printf("%d items\n", len(n.Items))
		case OrderedList:
			fmt.Printf("%d items\n", len(n.Items))
		case Blockquote:
			fmt.Printf("%s\n", n.Content)
		case HorizontalRule:
			fmt.Println("---")
		}
	}
}
