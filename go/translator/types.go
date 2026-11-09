package main

type Glossary struct {
	Term        string `json:"term"`
	Translation string `json:"translation"`
}

type Item struct {
	ID      string `json:"id"`
	Text    string `json:"text"`
	Context string `json:"context,omitempty"`
}

type TranslateRequest struct {
	Provider string     `json:"provider"`
	Model    string     `json:"model"`
	Target   string     `json:"target"`
	Force    bool       `json:"force,omitempty"`
	Glossary []Glossary `json:"glossary"`
	Items    []Item     `json:"items"`
}

type ResultItem struct {
	ID     string `json:"id"`
	Text   string `json:"text"`
	Status string `json:"status,omitempty"`
}

type Stats struct {
	Provider   string `json:"provider"`
	Cached     int    `json:"cached"`
	Translated int    `json:"translated"`
	Failed     int    `json:"failed"`
	Chars      int    `json:"chars"`
}

type TranslateResponse struct {
	Items []ResultItem `json:"items"`
	Stats Stats        `json:"stats"`
}

type HealthResponse struct {
	OK        bool     `json:"ok"`
	Version   string   `json:"version"`
	Providers []string `json:"providers"`
	CacheSize int      `json:"cache_size"`
}
