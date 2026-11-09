package main

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strings"
	"testing"
)

type repeatedByteReader struct {
	remaining int64
	value     byte
}

func (r *repeatedByteReader) Read(buffer []byte) (int, error) {
	if r.remaining == 0 {
		return 0, io.EOF
	}
	size := min(int64(len(buffer)), r.remaining)
	for index := range buffer[:size] {
		buffer[index] = r.value
	}
	r.remaining -= size
	return int(size), nil
}

func TestTranslateHandlerUsesProviderAndCache(t *testing.T) {
	cache, err := LoadCache(filepath.Join(t.TempDir(), "cache.json"))
	if err != nil {
		t.Fatal(err)
	}
	server := &Server{
		cache: cache,
		providers: map[string]Provider{
			"mock": &Mock{},
		},
	}

	request := TranslateRequest{
		Provider: "mock",
		Target:   "zh-TW",
		Items:    []Item{{ID: "one", Text: "Welcome"}},
	}
	for call, wantCached := range []int{0, 1} {
		body, err := json.Marshal(request)
		if err != nil {
			t.Fatal(err)
		}
		recorder := httptest.NewRecorder()
		server.handleTranslate(recorder, httptest.NewRequest(http.MethodPost, "/v1/translate", bytes.NewReader(body)))
		if recorder.Code != http.StatusOK {
			t.Fatalf("call %d: status = %d, want %d", call, recorder.Code, http.StatusOK)
		}
		var response TranslateResponse
		if err := json.Unmarshal(recorder.Body.Bytes(), &response); err != nil {
			t.Fatal(err)
		}
		if got, want := response.Items[0].Text, "【譯】Welcome"; got != want {
			t.Fatalf("call %d: text = %q, want %q", call, got, want)
		}
		if got := response.Stats.Cached; got != wantCached {
			t.Fatalf("call %d: cached = %d, want %d", call, got, wantCached)
		}
	}
}

func TestTranslateHandlerForceRetranslatesAndReplacesCache(t *testing.T) {
	cache, err := LoadCache(filepath.Join(t.TempDir(), "cache.json"))
	if err != nil {
		t.Fatal(err)
	}
	key := cacheKey("mock", "", "zh-TW", "Hello", "")
	cache.Set(key, "舊翻譯")
	server := &Server{
		cache:     cache,
		providers: map[string]Provider{"mock": &Mock{}},
	}
	body := []byte(`{"provider":"mock","target":"zh-TW","force":true,"items":[{"id":"one","text":"Hello"}]}`)
	recorder := httptest.NewRecorder()
	server.handleTranslate(
		recorder,
		httptest.NewRequest(http.MethodPost, "/v1/translate", bytes.NewReader(body)),
	)
	var response TranslateResponse
	if err := json.Unmarshal(recorder.Body.Bytes(), &response); err != nil {
		t.Fatal(err)
	}
	if response.Stats.Cached != 0 || response.Stats.Translated != 1 {
		t.Fatalf("stats = %#v, want a fresh translation", response.Stats)
	}
	if got, ok := cache.Get(key); !ok || got != "【譯】Hello" {
		t.Fatalf("cache value = %q, %t; want newly translated text", got, ok)
	}
}

func TestTranslateHandlerRejectsInvalidRequests(t *testing.T) {
	cache, err := LoadCache(filepath.Join(t.TempDir(), "cache.json"))
	if err != nil {
		t.Fatal(err)
	}
	server := &Server{cache: cache, providers: map[string]Provider{"mock": &Mock{}}}

	recorder := httptest.NewRecorder()
	server.handleTranslate(recorder, httptest.NewRequest(http.MethodGet, "/v1/translate", nil))
	if recorder.Code != http.StatusMethodNotAllowed {
		t.Fatalf("GET status = %d, want %d", recorder.Code, http.StatusMethodNotAllowed)
	}

	body := []byte(`{"provider":"unknown","items":[{"id":"one","text":"Hello"}]}`)
	recorder = httptest.NewRecorder()
	server.handleTranslate(recorder, httptest.NewRequest(http.MethodPost, "/v1/translate", bytes.NewReader(body)))
	if recorder.Code != http.StatusBadRequest {
		t.Fatalf("unknown provider status = %d, want %d", recorder.Code, http.StatusBadRequest)
	}

	body = []byte(`{"provider":"mock","items":[]} {"provider":"mock","items":[]}`)
	recorder = httptest.NewRecorder()
	server.handleTranslate(recorder, httptest.NewRequest(http.MethodPost, "/v1/translate", bytes.NewReader(body)))
	if recorder.Code != http.StatusBadRequest {
		t.Fatalf("multiple JSON values status = %d, want %d", recorder.Code, http.StatusBadRequest)
	}
}

func TestTranslateHandlerRejectsOversizedRequestBody(t *testing.T) {
	cache, err := LoadCache(filepath.Join(t.TempDir(), "cache.json"))
	if err != nil {
		t.Fatal(err)
	}
	server := &Server{cache: cache}
	body := &repeatedByteReader{remaining: maxRequestBytes + 1, value: ' '}
	recorder := httptest.NewRecorder()
	server.handleTranslate(
		recorder,
		httptest.NewRequest(http.MethodPost, "/v1/translate", body),
	)
	if recorder.Code != http.StatusRequestEntityTooLarge {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusRequestEntityTooLarge)
	}
}

func TestValidateTranslateRequestEnforcesInputLimitsAndUniqueIDs(t *testing.T) {
	tests := []struct {
		name    string
		request TranslateRequest
	}{
		{
			name: "duplicate IDs",
			request: TranslateRequest{
				Target: "zh-TW",
				Items: []Item{
					{ID: "one", Text: "First"},
					{ID: "one", Text: "Second"},
				},
			},
		},
		{
			name: "too many items",
			request: TranslateRequest{
				Target: "zh-TW",
				Items:  make([]Item, maxTranslateItems+1),
			},
		},
		{
			name: "oversized text",
			request: TranslateRequest{
				Target: "zh-TW",
				Items:  []Item{{ID: "one", Text: strings.Repeat("x", maxItemTextBytes+1)}},
			},
		},
		{
			name: "invalid target path",
			request: TranslateRequest{
				Target: "../outside",
			},
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if err := validateTranslateRequest(test.request); err == nil {
				t.Fatal("expected invalid request to be rejected")
			}
		})
	}
}

type shortProvider struct{}

func (shortProvider) Name() string { return "short" }

func (shortProvider) Translate(context.Context, string, []string) ([]string, error) {
	return nil, nil
}

type contextRecordingProvider struct {
	contexts []string
}

func (p *contextRecordingProvider) Name() string { return "context-recording" }

func (p *contextRecordingProvider) Translate(_ context.Context, _ string, texts []string) ([]string, error) {
	return texts, nil
}

func (p *contextRecordingProvider) TranslateWithContext(
	_ context.Context,
	_ string,
	texts, contexts []string,
) ([]string, error) {
	p.contexts = append([]string(nil), contexts...)
	return texts, nil
}

func TestTranslateHandlerPassesItemContextToContextualProvider(t *testing.T) {
	cache, err := LoadCache(filepath.Join(t.TempDir(), "cache.json"))
	if err != nil {
		t.Fatal(err)
	}
	provider := &contextRecordingProvider{}
	server := &Server{
		cache:     cache,
		providers: map[string]Provider{"contextual": provider},
	}
	body := []byte(`{"provider":"contextual","items":[{"id":"one","text":"Open","context":"quest title"}]}`)
	recorder := httptest.NewRecorder()
	server.handleTranslate(
		recorder,
		httptest.NewRequest(http.MethodPost, "/v1/translate", bytes.NewReader(body)),
	)
	if len(provider.contexts) != 1 || provider.contexts[0] != "quest title" {
		t.Fatalf("provider contexts = %#v, want quest title", provider.contexts)
	}
}

func TestTranslateHandlerRejectsProviderResultCountMismatch(t *testing.T) {
	cache, err := LoadCache(filepath.Join(t.TempDir(), "cache.json"))
	if err != nil {
		t.Fatal(err)
	}
	server := &Server{
		cache:     cache,
		providers: map[string]Provider{"short": shortProvider{}},
	}
	body := []byte(`{"provider":"short","items":[{"id":"one","text":"Hello"}]}`)
	recorder := httptest.NewRecorder()
	server.handleTranslate(
		recorder,
		httptest.NewRequest(http.MethodPost, "/v1/translate", bytes.NewReader(body)),
	)
	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusOK)
	}
	var response TranslateResponse
	if err := json.Unmarshal(recorder.Body.Bytes(), &response); err != nil {
		t.Fatal(err)
	}
	if len(response.Items) != 1 || response.Items[0].Status != "failed" {
		t.Fatalf("response items = %#v, want one failed item", response.Items)
	}
}

func TestCacheFlushCanReplaceExistingCache(t *testing.T) {
	path := filepath.Join(t.TempDir(), "cache.json")
	cache, err := LoadCache(path)
	if err != nil {
		t.Fatal(err)
	}

	cache.Set("one", "first")
	if err := cache.Flush(); err != nil {
		t.Fatal(err)
	}
	cache.Set("two", "second")
	if err := cache.Flush(); err != nil {
		t.Fatal(err)
	}
	reloaded, err := LoadCache(path)
	if err != nil {
		t.Fatal(err)
	}
	if got := reloaded.Len(); got != 2 {
		t.Fatalf("cache entries = %d, want 2", got)
	}
}

func TestCacheKeyIncludesTranslationContext(t *testing.T) {
	first := cacheKey("openai", "model", "zh-TW", "Open", "quest title")
	second := cacheKey("openai", "model", "zh-TW", "Open", "button label")
	if first == second {
		t.Fatal("cache key did not account for translation context")
	}
}
