package main

import (
	"context"
	"errors"
	"flag"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

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
		Handler:           withLogging(mux),
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
