package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"unicode/utf8"
)

func TestParseGoogleResponse(t *testing.T) {
	got, err := parseGoogleResponse(`["你好","世界"]`, 2)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Join(got, ",") != "你好,世界" {
		t.Fatalf("translations = %#v", got)
	}

	got, err = parseGoogleResponse(`[[["你好",null,null,null],["世界",null,null,null]]]`, 1)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 || got[0] != "你好世界" {
		t.Fatalf("single translation = %#v", got)
	}
}

func TestGoogleRequestUsesResponseShape(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`["譯文"]`))
	}))
	defer server.Close()

	provider := NewGoogleFree(1)
	got, err := provider.request(context.Background(), server.URL, 1)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 || got[0] != "譯文" {
		t.Fatalf("translations = %#v", got)
	}
}

func TestOpenAITranslateUsesConfiguredModelAndReturnsItems(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if got := r.Header.Get("Authorization"); got != "Bearer test-key" {
			t.Errorf("authorization = %q", got)
		}
		var request struct {
			Model    string `json:"model"`
			Messages []struct {
				Role    string `json:"role"`
				Content string `json:"content"`
			} `json:"messages"`
		}
		if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
			t.Errorf("decode request: %v", err)
		}
		if request.Model != "test-model" {
			t.Errorf("model = %q, want test-model", request.Model)
		}
		if len(request.Messages) == 0 || !strings.Contains(request.Messages[0].Content, "Tokens such as {p0}, {g0}") {
			t.Errorf("system prompt does not describe the actual single-brace placeholders")
		}
		if len(request.Messages) < 2 || !strings.Contains(request.Messages[1].Content, "FTB quest chapter title") {
			t.Errorf("user payload does not include translation context")
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"choices": []any{map[string]any{"message": map[string]string{
				"content": `{"0":"繁體中文"}`,
			}}},
		})
	}))
	defer server.Close()

	t.Setenv("OPENAI_BASE_URL", server.URL)
	t.Setenv("OPENAI_API_KEY", "test-key")
	provider := NewOpenAIWithModel("test-model")
	got, err := provider.TranslateWithContext(
		context.Background(),
		"zh-TW",
		[]string{"Hello"},
		[]string{"FTB quest chapter title"},
	)
	if err != nil {
		t.Fatal(err)
	}

	if len(got) != 1 || got[0] != "繁體中文" {
		t.Fatalf("translations = %#v", got)
	}
}

func TestSplitLongTextPreservesWhitespaceAndUnicode(t *testing.T) {
	input := strings.Repeat("a", 9) + " \r\n" + strings.Repeat("界", 5)
	chunks, separators := splitLongText(input, 7)
	var rebuilt strings.Builder
	for index, chunk := range chunks {
		if !utf8.ValidString(chunk) {
			t.Fatalf("chunk %d is invalid UTF-8: %q", index, chunk)
		}
		if len(chunk) > 7 {
			t.Fatalf("chunk %d has %d bytes, limit is 7", index, len(chunk))
		}
		rebuilt.WriteString(chunk)
		if index < len(separators) {
			rebuilt.WriteString(separators[index])
		}
	}
	if got := rebuilt.String(); got != input {
		t.Fatalf("reconstructed text = %q, want %q", got, input)
	}
}

func TestDeepLTranslateUsesTraditionalChineseTarget(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseForm(); err != nil {
			t.Errorf("parse form: %v", err)
		}
		if got := r.Form.Get("target_lang"); got != "ZH-HANT" {
			t.Errorf("target_lang = %q, want ZH-HANT", got)
		}
		if got := r.Form.Get("text"); got != "Hello & goodbye" {
			t.Errorf("text = %q, want Hello & goodbye", got)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"translations":[{"text":"繁體中文"}]}`))
	}))
	defer server.Close()

	provider := &DeepL{client: server.Client(), apiKey: "test-key", baseURL: server.URL}
	got, err := provider.Translate(context.Background(), "zh-TW", []string{"Hello & goodbye"})
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 || got[0] != "繁體中文" {
		t.Fatalf("translations = %#v", got)
	}
}

func TestParseOpenAIJSONAcceptsObjectAndArray(t *testing.T) {
	for _, content := range []string{
		`{"0":"A","1":"B"}`,
		`[{"id":"0","text":"A"},{"id":"1","text":"B"}]`,
		"```json\n{\"0\":\"A\",\"1\":\"B\"}\n```",
	} {
		got, err := parseOpenAIJSON(content)
		if err != nil {
			t.Fatalf("parse %q: %v", content, err)
		}
		if got["0"] != "A" || got["1"] != "B" {
			t.Fatalf("parse %q = %#v", content, got)
		}
	}
}
