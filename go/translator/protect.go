package main

import (
	"fmt"
	"regexp"
	"sort"
	"strings"
)

// Guard builds per-text protection state. Glossary tokens ("{{gN}}") are
// global, syntactic tokens ("{{pN}}") are local to one text so that the same
// token never means two different things in one batch.
type Guard struct {
	glossary  *regexp.Regexp
	termToken map[string]string // lower-case term -> token
	glossaryV map[string]string // token -> Chinese term
	patterns  *regexp.Regexp
}

type Protected struct {
	Text     string
	literals map[string]string
}

var (
	// Single-brace tokens survive Google much more reliably than {{double}}
	// ones: the translator treats "{{p0}}" as markup and often deletes it,
	// while "{p0}" passes through untouched.
	tokenRe = regexp.MustCompile(`\{\s*([pg]\d+)\s*\}`)

	// Minecraft color codes (&a, §b) are intentionally NOT protected: Google
	// keeps them intact on its own, and every extra placeholder raises the
	// chance that the translator drops one. Everything that would otherwise be
	// mangled (keys, printf slots, escapes, urls, ids) is protected.
	patternRe = regexp.MustCompile(`%(\d+\$)?[sd]|\$\{[^}]*\}|\{[^{}]*\}|\\[ntr]|https?://[^\s]+|@[A-Za-z0-9_.]+|#[a-z0-9_.-]+:[a-z0-9_./-]+|\b[a-z0-9_.-]+:[a-z0-9_./-]+\b`)
)

func NewGuard(glossary []Glossary) *Guard {
	g := &Guard{
		termToken: map[string]string{},
		glossaryV: map[string]string{},
	}

	terms := make([]Glossary, 0, len(glossary))
	for _, entry := range glossary {
		if strings.TrimSpace(entry.Term) == "" || strings.TrimSpace(entry.Translation) == "" {
			continue
		}
		terms = append(terms, entry)
	}
	sort.SliceStable(terms, func(i, j int) bool {
		return len(terms[i].Term) > len(terms[j].Term)
	})

	alternatives := make([]string, 0, len(terms))
	for i, entry := range terms {
		token := fmt.Sprintf("{g%d}", i)
		g.termToken[entry.Term] = token
		g.glossaryV[token] = entry.Translation
		alternatives = append(alternatives, regexp.QuoteMeta(entry.Term))
	}
	if len(alternatives) > 0 {
		// Case-sensitive on purpose: "Create" is a mod name but "create" is an
		// everyday verb, and replacing every verb with 機械動力 ruins far more
		// text than it fixes.
		g.glossary = regexp.MustCompile(`\b(?:` + strings.Join(alternatives, "|") + `)\b`)
	}
	g.patterns = patternRe
	return g
}

// Protect returns a state object; always restore with the same object.
func (g *Guard) Protect(text string) *Protected {
	state := &Protected{literals: map[string]string{}}
	counter := 0
	seen := map[string]string{}

	protected := g.patterns.ReplaceAllStringFunc(text, func(match string) string {
		if token, ok := seen[match]; ok {
			return token
		}
		token := fmt.Sprintf("{p%d}", counter)
		counter++
		seen[match] = token
		state.literals[token] = match
		return token
	})

	if g.glossary != nil {
		protected = g.glossary.ReplaceAllStringFunc(protected, func(match string) string {
			if token, ok := g.termToken[match]; ok {
				return token
			}
			return match
		})
	}
	state.Text = protected
	return state
}

func (p *Protected) Restore(text string) string {
	return tokenRe.ReplaceAllStringFunc(text, func(match string) string {
		key := "{" + tokenRe.FindStringSubmatch(match)[1] + "}"
		if value, ok := p.literals[key]; ok {
			return value
		}
		return match
	})
}

func (g *Guard) restoreGlossary(text string) string {
	return tokenRe.ReplaceAllStringFunc(text, func(match string) string {
		key := "{" + tokenRe.FindStringSubmatch(match)[1] + "}"
		if value, ok := g.glossaryV[key]; ok {
			return value
		}
		return match
	})
}

// RestoreAll restores syntactic tokens first, then glossary tokens.
func (g *Guard) RestoreAll(state *Protected, text string) string {
	return g.restoreGlossary(state.Restore(text))
}

// MissingTokens reports tokens that the translator dropped or rewrote.
func (g *Guard) MissingTokens(state *Protected, output string) []string {
	want := tokenRe.FindAllString(state.Text, -1)
	have := map[string]bool{}
	for _, token := range tokenRe.FindAllString(output, -1) {
		have["{"+tokenRe.FindStringSubmatch(token)[1]+"}"] = true
	}
	missing := make([]string, 0)
	for _, token := range want {
		key := "{" + tokenRe.FindStringSubmatch(token)[1] + "}"
		if !have[key] {
			missing = append(missing, key)
		}
	}
	return missing
}

// residueRe matches leftovers of the token namespace. Google sometimes strips
// part of a placeholder ("{{p3", "met}}"), which the exact token check cannot
// see; without this guard those fragments would end up in the game text.
var residueRe = regexp.MustCompile(`\{\{|\}\}|\{[pg]\d`)

// HasResidue reports whether a restored string still contains broken tokens.
func HasResidue(text string) bool {
	// Minecraft JSON text components keep their braces by design.
	if strings.Contains(text, `"text"`) {
		return false
	}
	return residueRe.MatchString(text)
}

var (
	spaceBeforePunct = regexp.MustCompile(`[ \t]+([，。！？；：、）」』】》]|\\n)`)
	spaceAfterOpen   = regexp.MustCompile(`([（「『【《])[ \t]+`)
	spaceBetweenHan  = regexp.MustCompile(`([\p{Han}，。！？；：、）」』】》])[ \t]+([\p{Han}，。！？；：、（）「『【《])`)
)

// Tidy removes spaces that machine translation inserts around Chinese
// punctuation while keeping English word spacing intact.
func Tidy(text string) string {
	text = spaceBeforePunct.ReplaceAllString(text, "$1")
	text = spaceAfterOpen.ReplaceAllString(text, "$1")
	// run twice so overlapping matches like "字 字 字" are all handled
	text = spaceBetweenHan.ReplaceAllString(text, "$1$2")
	text = spaceBetweenHan.ReplaceAllString(text, "$1$2")
	return text
}
