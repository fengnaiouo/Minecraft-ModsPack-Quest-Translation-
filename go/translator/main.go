package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"
)

const version = "0.1.0"

type Server struct {
	cache     *Cache
	providers map[string]Provider
}

func main() {
	addr := flag.String("addr", envOr("QTRANS_ADDR", "127.0.0.1:8787"), "listen address")
	cachePath := flag.String("cache", envOr("QTRANS_CACHE", "work/cache/translations.json"), "translation cache file")
	workers := flag.Int("workers", 4, "parallel requests for google_free")
	flag.Parse()

	cache, err := LoadCache(*cachePath)
	if err != nil {
		log.Fatalf("cache: %v", err)
	}

	server := &Server{
		cache: cache,
		providers: map[string]Provider{
			"google_free": NewGoogleFree(*workers),
			"openai":      NewOpenAI(),
			"deepl":       NewDeepL(),
			"offline":     &Offline{},
			"mock":        &Mock{},
		},
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/health", server.handleHealth)
	mux.HandleFunc("/v1/translate", server.handleTranslate)
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "not found"})
	})

	httpServer := &http.Server{
		Addr:              *addr,
		Handler:           withCORS(withLogging(mux)),
		ReadHeaderTimeout: 10 * time.Second,
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	go func() {
		ticker := time.NewTicker(15 * time.Second)
		defer ticker.Stop()
		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				if err := cache.Flush(); err != nil {
					log.Printf("cache flush: %v", err)
				}
			}
		}
	}()

	go func() {
		log.Printf("qtrans translator %s listening on http://%s (cache: %s, %d entries)", version, *addr, *cachePath, cache.Len())
		if err := httpServer.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("listen: %v", err)
		}
	}()

	<-ctx.Done()
	log.Printf("shutting down...")
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_ = httpServer.Shutdown(shutdownCtx)
	if err := cache.Flush(); err != nil {
		log.Printf("cache flush: %v", err)
	}
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
	if request.Provider == "openai" && request.Model != "" {
		provider = NewOpenAIWithModel(request.Model)
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
		key := cacheKey(request.Provider, request.Model, request.Target, state.Text)
		keys[i] = key
		if cached, hit := s.cache.Get(key); hit {
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
		translated, err := provider.Translate(ctx, request.Target, missText)
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
					one, err := provider.Translate(ctx, request.Target, []string{protected[index].Text})
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

func withCORS(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		w.Header().Set("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
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
