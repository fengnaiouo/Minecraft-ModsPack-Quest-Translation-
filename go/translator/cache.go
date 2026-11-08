package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

type Cache struct {
	mu    sync.Mutex
	path  string
	data  map[string]string
	dirty int
}

func LoadCache(path string) (*Cache, error) {
	c := &Cache{path: path, data: map[string]string{}}
	raw, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return c, nil
		}
		return nil, err
	}
	if len(raw) == 0 {
		return c, nil
	}
	if err := json.Unmarshal(raw, &c.data); err != nil {
		return nil, fmt.Errorf("parse cache %s: %w", path, err)
	}
	if c.data == nil {
		c.data = map[string]string{}
	}
	return c, nil
}

func cacheKey(provider, model, target, text string) string {
	sum := sha256.Sum256([]byte(provider + "\x00" + model + "\x00" + target + "\x00" + text))
	return hex.EncodeToString(sum[:])
}

func (c *Cache) Get(key string) (string, bool) {
	c.mu.Lock()
	defer c.mu.Unlock()
	value, ok := c.data[key]
	return value, ok
}

func (c *Cache) Set(key, value string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if _, exists := c.data[key]; exists {
		return
	}
	c.data[key] = value
	c.dirty++
}

func (c *Cache) Len() int {
	c.mu.Lock()
	defer c.mu.Unlock()
	return len(c.data)
}

// Flush writes the cache atomically. Safe to call often.
func (c *Cache) Flush() error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.dirty == 0 {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(c.path), 0o755); err != nil {
		return err
	}
	raw, err := json.Marshal(c.data)
	if err != nil {
		return err
	}
	tmp := c.path + ".tmp"
	if err := os.WriteFile(tmp, raw, 0o644); err != nil {
		return err
	}
	if err := os.Rename(tmp, c.path); err != nil {
		return err
	}
	c.dirty = 0
	return nil
}
