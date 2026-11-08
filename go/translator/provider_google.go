package main

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"
)

// GoogleFree uses the public Google Translate web endpoints. No key needed,
// good for a zero-config first run. Several strings are packed into one GET
// (the endpoint accepts repeated `q` parameters), which makes a full modpack
// feasible without hitting rate limits.
type GoogleFree struct {
	client  *http.Client
	workers int
}

func NewGoogleFree(workers int) *GoogleFree {
	if workers < 1 {
		workers = 4
	}
	return &GoogleFree{
		client:  &http.Client{Timeout: 60 * time.Second},
		workers: workers,
	}
}

func (g *GoogleFree) Name() string { return "google_free" }

const (
	maxBatchItems = 8
	maxBatchChars = 1400
	longTextLimit = 1200
)

type googleBatch struct {
	indices []int
	texts   []string
	chars   int
}

func (g *GoogleFree) Translate(ctx context.Context, target string, texts []string) ([]string, error) {
	out := make([]string, len(texts))
	var batches []googleBatch
	var singles []int
	current := googleBatch{}

	for i, text := range texts {
		if len(text) > longTextLimit {
			singles = append(singles, i)
			continue
		}
		if len(current.indices) >= maxBatchItems || (current.chars+len(text) > maxBatchChars && len(current.indices) > 0) {
			batches = append(batches, current)
			current = googleBatch{}
		}
		current.indices = append(current.indices, i)
		current.texts = append(current.texts, text)
		current.chars += len(text)
	}
	if len(current.indices) > 0 {
		batches = append(batches, current)
	}

	var firstErr error
	var mu sync.Mutex
	noteError := func(err error) {
		mu.Lock()
		if firstErr == nil {
			firstErr = err
		}
		mu.Unlock()
	}

	// Batched requests run in parallel.
	jobs := make(chan int)
	var wg sync.WaitGroup
	for w := 0; w < g.workers; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for index := range jobs {
				batch := batches[index]
				translated, err := g.batch(ctx, target, batch.texts)
				if err != nil {
					// salvage the batch one string at a time
					for j, text := range batch.texts {
						one, oneErr := g.one(ctx, target, text)
						if oneErr != nil {
							noteError(oneErr)
							continue
						}
						out[batch.indices[j]] = one
					}
					continue
				}
				for j, value := range translated {
					out[batch.indices[j]] = value
				}
			}
		}()
	}
	for i := range batches {
		jobs <- i
	}
	close(jobs)
	wg.Wait()

	for _, index := range singles {
		translated, err := g.one(ctx, target, texts[index])
		if err != nil {
			noteError(err)
			continue
		}
		out[index] = translated
	}

	if firstErr != nil {
		return nil, firstErr
	}
	return out, nil
}

func (g *GoogleFree) one(ctx context.Context, target, text string) (string, error) {
	chunks := splitLongText(text, 1600)
	parts := make([]string, 0, len(chunks))
	for _, chunk := range chunks {
		values := url.Values{}
		values.Set("client", "dict-chrome-ex")
		values.Set("sl", "en")
		values.Set("tl", target)
		values.Add("q", chunk)
		endpoint := "https://clients5.google.com/translate_a/t?" + values.Encode()
		translated, err := g.request(ctx, endpoint, 1)
		if err != nil {
			return "", err
		}
		parts = append(parts, translated[0])
	}
	return strings.Join(parts, " "), nil
}

func (g *GoogleFree) batch(ctx context.Context, target string, texts []string) ([]string, error) {
	values := url.Values{}
	values.Set("client", "dict-chrome-ex")
	values.Set("sl", "en")
	values.Set("tl", target)
	for _, text := range texts {
		values.Add("q", text)
	}
	endpoint := "https://clients5.google.com/translate_a/t?" + values.Encode()
	return g.request(ctx, endpoint, len(texts))
}

func (g *GoogleFree) request(ctx context.Context, endpoint string, expected int) ([]string, error) {
	var result []string
	err := retry(ctx, 3, func() (bool, error) {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
		if err != nil {
			return false, err
		}
		req.Header.Set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) qtrans")
		resp, err := g.client.Do(req)
		if err != nil {
			return true, err
		}
		body := readBody(resp)
		if resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= 500 {
			return true, fmt.Errorf("google_free HTTP %d: %s", resp.StatusCode, truncate(body, 160))
		}
		if resp.StatusCode != http.StatusOK {
			return false, fmt.Errorf("google_free HTTP %d: %s", resp.StatusCode, truncate(body, 160))
		}
		parsed, err := parseGoogleResponse(body, expected)
		if err != nil {
			return true, err
		}
		if len(parsed) != expected {
			return true, fmt.Errorf("google_free: expected %d translations, got %d", expected, len(parsed))
		}
		result = parsed
		return false, nil
	})
	return result, err
}

func parseGoogleResponse(body string, expected int) ([]string, error) {
	var parsed any
	if err := json.Unmarshal([]byte(body), &parsed); err != nil {
		return nil, fmt.Errorf("google_free: invalid JSON: %s", truncate(body, 120))
	}
	list, ok := parsed.([]any)
	if !ok {
		return nil, fmt.Errorf("google_free: unexpected payload: %s", truncate(body, 120))
	}

	// clients5 returns ["翻譯", "翻譯2", ...] for repeated q parameters.
	if len(list) == expected {
		out := make([]string, 0, expected)
		valid := true
		for _, entry := range list {
			value, ok := entry.(string)
			if !ok {
				valid = false
				break
			}
			out = append(out, value)
		}
		if valid {
			return out, nil
		}
	}

	// translate_a/single returns [["翻譯","來源",...], ...]
	var builder strings.Builder
	for _, entry := range list {
		if segments, ok := entry.([]any); ok {
			for _, segment := range segments {
				parts, ok := segment.([]any)
				if !ok || len(parts) == 0 {
					continue
				}
				if piece, ok := parts[0].(string); ok {
					builder.WriteString(piece)
				}
			}
		}
	}
	if builder.Len() > 0 {
		return []string{builder.String()}, nil
	}
	return nil, fmt.Errorf("google_free: could not parse response: %s", truncate(body, 160))
}

func splitLongText(text string, limit int) []string {
	if len(text) <= limit {
		return []string{text}
	}
	words := strings.Fields(text)
	chunks := make([]string, 0, 2)
	var current strings.Builder
	for _, word := range words {
		if current.Len() > 0 && current.Len()+1+len(word) > limit {
			chunks = append(chunks, current.String())
			current.Reset()
		}
		if current.Len() > 0 {
			current.WriteByte(' ')
		}
		current.WriteString(word)
	}
	if current.Len() > 0 {
		chunks = append(chunks, current.String())
	}
	return chunks
}

func truncate(text string, max int) string {
	if len(text) <= max {
		return text
	}
	return text[:max] + "..."
}
