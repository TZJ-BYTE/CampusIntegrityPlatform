package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"
)

func main() {
	port := getenv("PORT", "8787")
	repoDir := getenv("CONTENT_REPO_DIR", filepath.Join(".", "content-repo"))
	repoDir, _ = filepath.Abs(repoDir)

	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			writeJSON(w, http.StatusNotFound, map[string]any{
				"ok": false,
				"error": map[string]any{
					"code":    "NOT_FOUND",
					"message": "not found",
				},
			})
			return
		}
		if r.URL.Path == "/" || r.URL.Path == "/health" {
			writeJSON(w, http.StatusOK, map[string]any{"ok": true, "repoDir": repoDir})
			return
		}
		if r.URL.Path == "/versions.json" {
			serveFileSafe(w, repoDir, "versions.json", "application/json; charset=utf-8")
			return
		}
		if r.URL.Path == "/content-pack.zip" {
			serveFileSafe(w, repoDir, "content-pack.zip", "application/zip")
			return
		}
		writeJSON(w, http.StatusNotFound, map[string]any{
			"ok": false,
			"error": map[string]any{
				"code":    "NOT_FOUND",
				"message": "not found",
			},
		})
	})

	addr := "127.0.0.1:" + port
	fmt.Printf("content server (go) listening on http://%s\n", addr)
	fmt.Printf("repoDir: %s\n", repoDir)
	if err := http.ListenAndServe(addr, mux); err != nil {
		panic(err)
	}
}

func serveFileSafe(w http.ResponseWriter, base, reqPath, contentType string) {
	target := filepath.Join(base, reqPath)
	target = filepath.Clean(target)
	baseClean := filepath.Clean(base)
	if !strings.HasPrefix(target, baseClean) {
		writeJSON(w, http.StatusNotFound, map[string]any{
			"ok": false,
			"error": map[string]any{
				"code":    "NOT_FOUND",
				"message": "not found",
			},
		})
		return
	}
	b, err := os.ReadFile(target)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]any{
			"ok": false,
			"error": map[string]any{
				"code":    "NOT_FOUND",
				"message": "not found",
			},
		})
		return
	}
	w.Header().Set("Content-Type", contentType)
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(b)
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	b, _ := json.Marshal(v)
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_, _ = w.Write(b)
}

func getenv(k, def string) string {
	v := strings.TrimSpace(os.Getenv(k))
	if v == "" {
		return def
	}
	return v
}
