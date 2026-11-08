package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"testing"
)

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
}
