package main

import (
	"context"
	"errors"
	"io"
	"net/http"
	"time"
)

type Provider interface {
	Name() string
	Translate(ctx context.Context, target string, texts []string) ([]string, error)
}

type ContextualProvider interface {
	TranslateWithContext(ctx context.Context, target string, texts, contexts []string) ([]string, error)
}

var errMissingKey = errors.New("provider API key is not configured")

func newHTTPClient() *http.Client {
	return &http.Client{Timeout: 90 * time.Second}
}

func readBody(resp *http.Response) string {
	defer resp.Body.Close()
	raw, _ := io.ReadAll(io.LimitReader(resp.Body, 4<<20))
	return string(raw)
}

// retry runs fn up to attempts times with exponential backoff for transient
// failures (429 / 5xx / network errors).
func retry(ctx context.Context, attempts int, fn func() (bool, error)) error {
	var last error
	for attempt := 0; attempt < attempts; attempt++ {
		retryable, err := fn()
		if err == nil {
			return nil
		}
		last = err
		if !retryable {
			return err
		}
		if ctx.Err() != nil {
			return ctx.Err()
		}
		wait := time.Duration(1<<attempt) * 700 * time.Millisecond
		if wait > 8*time.Second {
			wait = 8 * time.Second
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(wait):
		}
	}
	return last
}
