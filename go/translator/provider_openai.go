package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"
	"time"
)

type OpenAI struct {
	client  *http.Client
	baseURL string
	apiKey  string
	model   string
}

func NewOpenAI() *OpenAI {
	return NewOpenAIWithModel(os.Getenv("OPENAI_MODEL"))
}

func NewOpenAIWithModel(model string) *OpenAI {
	base := strings.TrimRight(os.Getenv("OPENAI_BASE_URL"), "/")
	if base == "" {
		base = "https://api.openai.com/v1"
	}
	if strings.TrimSpace(model) == "" {
		model = "gpt-4o-mini"
	}
	return &OpenAI{
		client:  &http.Client{Timeout: 180 * time.Second},
		baseURL: base,
		apiKey:  os.Getenv("OPENAI_API_KEY"),
		model:   model,
	}
}

func (o *OpenAI) Name() string { return "openai" }

func (o *OpenAI) Translate(ctx context.Context, target string, texts []string) ([]string, error) {
	return o.TranslateWithContext(ctx, target, texts, nil)
}

func (o *OpenAI) TranslateWithContext(ctx context.Context, target string, texts, contexts []string) ([]string, error) {
	if o.apiKey == "" {
		return nil, errMissingKey
	}
	if len(contexts) != 0 && len(contexts) != len(texts) {
		return nil, fmt.Errorf("openai: expected %d contexts, got %d", len(texts), len(contexts))
	}
	langName := target
	switch target {
	case "zh-TW", "zh_tw":
		langName = "Traditional Chinese (Taiwan, zh-TW)"
	case "zh-CN", "zh_cn":
		langName = "Simplified Chinese (zh-CN)"
	}

	type entry struct {
		ID      string `json:"id"`
		Text    string `json:"text"`
		Context string `json:"context,omitempty"`
	}
	payload := make([]entry, len(texts))
	for i, text := range texts {
		payload[i] = entry{ID: fmt.Sprintf("%d", i), Text: text}
		if len(contexts) > 0 {
			payload[i].Context = contexts[i]
		}
	}
	userJSON, _ := json.Marshal(payload)

	system := "You are a professional game localization translator for Minecraft modpacks. " +
		"Translate every input text from English into " + langName + ".\n" +
		"Rules:\n" +
		"1. Tokens such as {p0}, {g0} must be copied exactly (letters, digits, braces). Never translate or reorder them.\n" +
		"2. {gN} tokens stand for fixed glossary terms; keep the token itself.\n" +
		"3. Keep Minecraft color codes (&a, §b), printf placeholders (%s, %1$d), \\n escapes, URLs and item ids unchanged.\n" +
		"4. Use the supplied context only to disambiguate each string; do not include context in the translation.\n" +
		"5. Use natural, concise Taiwanese Mandarin. Keep widely known mod names in English if there is no common translation.\n" +
		"6. Do not add explanations, notes or quotation marks around the result.\n" +
		"Respond ONLY with a JSON object whose keys are the input ids and whose values are the translated strings."

	requestBody := map[string]any{
		"model": o.model,
		"messages": []map[string]string{
			{"role": "system", "content": system},
			{"role": "user", "content": string(userJSON)},
		},
		"temperature":     0.2,
		"response_format": map[string]string{"type": "json_object"},
	}
	content, err := o.chat(ctx, requestBody)
	if err != nil && strings.Contains(err.Error(), "response_format") {
		delete(requestBody, "response_format")
		content, err = o.chat(ctx, requestBody)
	}
	if err != nil {
		return nil, err
	}

	translated, err := parseOpenAIJSON(content)
	if err != nil {
		return nil, err
	}
	out := make([]string, len(texts))
	for i := range texts {
		value, ok := translated[fmt.Sprintf("%d", i)]
		if !ok || strings.TrimSpace(value) == "" {
			return nil, fmt.Errorf("openai: missing translation for item %d", i)
		}
		out[i] = value
	}
	return out, nil
}

type chatResponse struct {
	Choices []struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	} `json:"choices"`
	Error *struct {
		Message string `json:"message"`
	} `json:"error"`
}

func (o *OpenAI) chat(ctx context.Context, body map[string]any) (string, error) {
	raw, _ := json.Marshal(body)
	endpoint := o.baseURL + "/chat/completions"
	var content string
	err := retry(ctx, 3, func() (bool, error) {
		req, err := http.NewRequestWithContext(ctx, http.MethodPost, endpoint, bytes.NewReader(raw))
		if err != nil {
			return false, err
		}
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("Authorization", "Bearer "+o.apiKey)
		resp, err := o.client.Do(req)
		if err != nil {
			return true, err
		}
		text := readBody(resp)
		if resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= 500 {
			return true, fmt.Errorf("openai HTTP %d: %s", resp.StatusCode, truncate(text, 400))
		}
		if resp.StatusCode != http.StatusOK {
			return false, fmt.Errorf("openai HTTP %d: %s", resp.StatusCode, truncate(text, 600))
		}
		var parsed chatResponse
		if err := json.Unmarshal([]byte(text), &parsed); err != nil {
			return true, fmt.Errorf("openai: bad JSON: %w", err)
		}
		if parsed.Error != nil {
			return false, fmt.Errorf("openai: %s", parsed.Error.Message)
		}
		if len(parsed.Choices) == 0 {
			return true, fmt.Errorf("openai: empty choices")
		}
		content = parsed.Choices[0].Message.Content
		return false, nil
	})
	return content, err
}

func parseOpenAIJSON(content string) (map[string]string, error) {
	content = strings.TrimSpace(content)
	content = strings.TrimPrefix(content, "```json")
	content = strings.TrimPrefix(content, "```")
	content = strings.TrimSuffix(content, "```")
	content = strings.TrimSpace(content)

	var mapping map[string]string
	if err := json.Unmarshal([]byte(content), &mapping); err == nil {
		return mapping, nil
	}
	var array []struct {
		ID   string `json:"id"`
		Text string `json:"text"`
	}
	if err := json.Unmarshal([]byte(content), &array); err == nil {
		mapping = map[string]string{}
		for _, item := range array {
			mapping[item.ID] = item.Text
		}
		return mapping, nil
	}
	// Last resort: find the outermost JSON object.
	start := strings.IndexByte(content, '{')
	end := strings.LastIndexByte(content, '}')
	if start >= 0 && end > start {
		if err := json.Unmarshal([]byte(content[start:end+1]), &mapping); err == nil {
			return mapping, nil
		}
	}
	return nil, fmt.Errorf("openai: could not parse translations from: %s", truncate(content, 300))
}
