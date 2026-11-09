package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"
	"time"
)

var version = "dev"

type Server struct {
	cache     *Cache
	providers map[string]Provider
}

func (s *Server) handleHealth(w http.ResponseWriter, _ *http.Request) {
	names := make([]string, 0, len(s.providers))
	for name := range s.providers {
		names = append(names, name)
	}
	writeJSON(w, http.StatusOK, HealthResponse{OK: true, Version: version, Providers: names, CacheSize: s.cache.Len()})
}

func (s *Server) handleTranslate(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeJSON(w, http.StatusMethodNotAllowed, map[string]string{"error": "POST required"})
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 64<<20)
	var request TranslateRequest
	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid JSON: " + err.Error()})
		return
	}
	if len(request.Items) == 0 {
		writeJSON(w, http.StatusOK, TranslateResponse{Items: []ResultItem{}, Stats: Stats{Provider: request.Provider}})
		return
	}
	if request.Target == "" {
		request.Target = "zh-TW"
	}
	if request.Provider == "" {
		request.Provider = "google_free"
	}

	provider, ok := s.providers[request.Provider]
	if !ok {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "unknown provider: " + request.Provider})
		return
	}
	cacheModel := request.Model
	if request.Provider == "openai" {
		if request.Model != "" {
			provider = NewOpenAIWithModel(request.Model)
		}
		if openAI, ok := provider.(*OpenAI); ok {
			cacheModel = openAI.model
		}
	}

	guard := NewGuard(request.Glossary)
	protected := make([]*Protected, len(request.Items))
	keys := make([]string, len(request.Items))
	results := make([]ResultItem, len(request.Items))
	stats := Stats{Provider: request.Provider}

	var missIdx []int
	var missText []string

	for i, item := range request.Items {
		stats.Chars += len(item.Text)
		state := guard.Protect(item.Text)
		protected[i] = state
		key := cacheKey(request.Provider, cacheModel, request.Target, state.Text, item.Context)
		keys[i] = key
		if cached, hit := s.cache.Get(key); hit && !request.Force {
			results[i] = ResultItem{ID: item.ID, Text: Tidy(guard.RestoreAll(state, cached)), Status: "ok"}
			stats.Cached++
			continue
		}
		missIdx = append(missIdx, i)
		missText = append(missText, state.Text)
	}

	if len(missText) > 0 {
		ctx, cancel := context.WithTimeout(r.Context(), 10*time.Minute)
		defer cancel()
		var translated []string
		var err error
		missContexts := make([]string, len(missIdx))
		for i, index := range missIdx {
			missContexts[i] = request.Items[index].Context
		}
		if contextual, ok := provider.(ContextualProvider); ok {
			translated, err = contextual.TranslateWithContext(ctx, request.Target, missText, missContexts)
		} else {
			translated, err = provider.Translate(ctx, request.Target, missText)
		}
		if err == nil && len(translated) != len(missIdx) {
			err = fmt.Errorf("provider %s returned %d results for %d items", request.Provider, len(translated), len(missIdx))
		}
		if err != nil {
			log.Printf("provider %s failed for %d items: %v", request.Provider, len(missText), err)
			for _, index := range missIdx {
				item := request.Items[index]
				results[index] = ResultItem{
					ID:     item.ID,
					Text:   guard.RestoreAll(protected[index], protected[index].Text),
					Status: "failed",
				}
				stats.Failed++
			}
		} else {
			accept := func(index int, text string) bool {
				final := Tidy(guard.RestoreAll(protected[index], text))
				if HasResidue(final) {
					log.Printf("provider %s left token residue for item %s", request.Provider, request.Items[index].ID)
					return false
				}
				s.cache.Set(keys[index], text)
				results[index] = ResultItem{
					ID:     request.Items[index].ID,
					Text:   final,
					Status: "ok",
				}
				stats.Translated++
				return true
			}

			var retry []int
			for j, index := range missIdx {
				item := request.Items[index]
				if translated[j] == "" {
					retry = append(retry, index)
					continue
				}
				if missing := guard.MissingTokens(protected[index], translated[j]); len(missing) > 0 {
					log.Printf("provider %s dropped tokens %v for item %s", request.Provider, missing, item.ID)
					retry = append(retry, index)
					continue
				}
				if !accept(index, translated[j]) {
					retry = append(retry, index)
				}
			}

			// Retry dropped-token items one at a time; shorter requests are far
			// more likely to keep every placeholder intact.
			for _, index := range retry {
				item := request.Items[index]
				ok := false
				for attempt := 0; attempt < 2 && !ok; attempt++ {
					var one []string
					if contextual, supportsContext := provider.(ContextualProvider); supportsContext {
						one, err = contextual.TranslateWithContext(
							ctx,
							request.Target,
							[]string{protected[index].Text},
							[]string{item.Context},
						)
					} else {
						one, err = provider.Translate(ctx, request.Target, []string{protected[index].Text})
					}
					if err != nil || len(one) == 0 || one[0] == "" {
						continue
					}
					if len(guard.MissingTokens(protected[index], one[0])) > 0 {
						continue
					}
					ok = accept(index, one[0])
				}
				if !ok {
					log.Printf("provider %s gave up on item %s: %s", request.Provider, item.ID, truncate(protected[index].Text, 120))
					results[index] = ResultItem{ID: item.ID, Text: item.Text, Status: "failed"}
					stats.Failed++
				}
			}
		}
	}

	if err := s.cache.Flush(); err != nil {
		log.Printf("cache flush: %v", err)
	}
	log.Printf(
		"provider=%s items=%d cached=%d translated=%d failed=%d chars=%d",
		request.Provider, len(request.Items), stats.Cached, stats.Translated, stats.Failed, stats.Chars,
	)
	writeJSON(w, http.StatusOK, TranslateResponse{Items: results, Stats: stats})
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	encoder := json.NewEncoder(w)
	encoder.SetEscapeHTML(false)
	_ = encoder.Encode(value)
}

func withLogging(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		if r.URL.Path != "/health" {
			log.Printf("%s %s %s", r.Method, r.URL.Path, time.Since(start).Round(time.Millisecond))
		}
	})
}

func envOr(key, fallback string) string {
	if value := strings.TrimSpace(os.Getenv(key)); value != "" {
		return value
	}
	return fallback
}
