package main

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"
)

type DeepL struct {
	client  *http.Client
	apiKey  string
	baseURL string
}

func NewDeepL() *DeepL {
	key := os.Getenv("DEEPL_API_KEY")
	base := "https://api.deepl.com/v2/translate"
	if strings.HasSuffix(key, ":fx") {
		base = "https://api-free.deepl.com/v2/translate"
	}
	return &DeepL{client: &http.Client{Timeout: 120 * time.Second}, apiKey: key, baseURL: base}
}

func (d *DeepL) Name() string { return "deepl" }

func (d *DeepL) Translate(ctx context.Context, target string, texts []string) ([]string, error) {
	if d.apiKey == "" {
		return nil, errMissingKey
	}
	targetLang := "ZH-HANT"
	if strings.HasPrefix(target, "zh-CN") || strings.HasPrefix(target, "zh_cn") {
		targetLang = "ZH-HANS"
	}
	form := url.Values{}
	form.Set("auth_key", d.apiKey)
	form.Set("target_lang", targetLang)
	form.Set("source_lang", "EN")
	form.Set("preserve_formatting", "1")
	for _, text := range texts {
		form.Add("text", text)
	}

	var out []string
	err := retry(ctx, 3, func() (bool, error) {
		req, err := http.NewRequestWithContext(ctx, http.MethodPost, d.baseURL, strings.NewReader(form.Encode()))
		if err != nil {
			return false, err
		}
		req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
		resp, err := d.client.Do(req)
		if err != nil {
			return true, err
		}
		body := readBody(resp)
		if resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= 500 {
			return true, fmt.Errorf("deepl HTTP %d: %s", resp.StatusCode, truncate(body, 300))
		}
		if resp.StatusCode != http.StatusOK {
			return false, fmt.Errorf("deepl HTTP %d: %s", resp.StatusCode, truncate(body, 500))
		}
		var parsed struct {
			Translations []struct {
				Text string `json:"text"`
			} `json:"translations"`
		}
		if err := json.Unmarshal([]byte(body), &parsed); err != nil {
			return true, err
		}
		if len(parsed.Translations) != len(texts) {
			return false, fmt.Errorf("deepl: expected %d translations, got %d", len(texts), len(parsed.Translations))
		}
		out = make([]string, len(parsed.Translations))
		for i, translation := range parsed.Translations {
			out[i] = translation.Text
		}
		return false, nil
	})
	return out, err
}

// Offline leaves text untouched apart from glossary substitution, which is
// already applied by the Guard before the provider is called.
type Offline struct{}

func (o *Offline) Name() string { return "offline" }

func (o *Offline) Translate(_ context.Context, _ string, texts []string) ([]string, error) {
	out := make([]string, len(texts))
	copy(out, texts)
	return out, nil
}

// Mock is a deterministic provider for end-to-end tests without network access.
type Mock struct{}

func (m *Mock) Name() string { return "mock" }

func (m *Mock) Translate(_ context.Context, _ string, texts []string) ([]string, error) {
	out := make([]string, len(texts))
	for i, text := range texts {
		out[i] = "【譯】" + text
	}
	return out, nil
}
