package main

import (
	"crypto/rand"
	"crypto/sha256"
	"database/sql"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
	"unicode/utf8"

	"archive/zip"

	_ "github.com/mattn/go-sqlite3"
	"golang.org/x/text/encoding/simplifiedchinese"
	"golang.org/x/text/transform"

	"campus-integrity-platform/server-go/internal/cms"
)

type Store struct {
	NextSeq int64   `json:"nextSeq"`
	Events  []Event `json:"events"`
}

type Event struct {
	Seq        int64           `json:"seq"`
	EventID    string          `json:"eventId"`
	DeviceID   string          `json:"deviceId"`
	EventType  string          `json:"eventType"`
	EntityType string          `json:"entityType"`
	EntityID   string          `json:"entityId"`
	Payload    json.RawMessage `json:"payload"`
	OccurredAt int64           `json:"occurredAt"`
}

type UserRecord struct {
	UserID       string `json:"userId"`
	Username     string `json:"username"`
	Salt         string `json:"salt"`
	PasswordHash string `json:"passwordHash"`
}

type UsersFile struct {
	Users []UserRecord `json:"users"`
}

type TokenRecord struct {
	Token     string `json:"token"`
	UserID    string `json:"userId"`
	ExpiresAt int64  `json:"expiresAt"`
}

type TokensFile struct {
	Tokens []TokenRecord `json:"tokens"`
}

type Server struct {
	mu sync.Mutex

	syncDir    string
	usersPath  string
	tokensPath string
	users      map[string]UserRecord
	tokens     map[string]TokenRecord

	contentDir string
	cmsDB      *sql.DB
	cmsAuth    *cms.Auth
}

func main() {
	port := getenv("PORT", "8788")

	syncDir := getenv("SYNC_REPO_DIR", filepath.Join(".", "sync-repo"))
	if err := os.MkdirAll(syncDir, 0o755); err != nil {
		panic(err)
	}
	usersPath := filepath.Join(syncDir, "users.json")
	tokensPath := filepath.Join(syncDir, "tokens.json")

	contentDir := getenv("CONTENT_REPO_DIR", filepath.Join(".", "content-repo"))
	if err := os.MkdirAll(contentDir, 0o755); err != nil {
		panic(err)
	}

	s := &Server{
		syncDir:    syncDir,
		usersPath:  usersPath,
		tokensPath: tokensPath,
		users:      map[string]UserRecord{},
		tokens:     map[string]TokenRecord{},
		contentDir: contentDir,
	}
	_ = s.loadAll()

	auth, err := cms.LoadAuthFromEnv()
	if err != nil {
		panic(err)
	}
	s.cmsAuth = auth

	// Initialize CMS DB
	cmsDBPath := filepath.Join(syncDir, "cms.db")
	db, err := sql.Open("sqlite3", cmsDBPath)
	if err != nil {
		panic(err)
	}
	s.cmsDB = db
	s.initCMSDB()

	// Sync data from client content.db to cms.db if cms.db is empty
	s.syncFromClientContentDB()

	mux := http.NewServeMux()
	mux.HandleFunc("/health", s.handleHealth)
	mux.HandleFunc("/versions.json", s.handleVersions)
	mux.HandleFunc("/content-pack.zip", s.handleContentPack)
	mux.HandleFunc("/v1/auth/login", s.handleLogin)
	mux.HandleFunc("/v1/sync/push", s.handlePush)
	mux.HandleFunc("/v1/sync/pull", s.handlePull)

	// CMS Routes
	mux.HandleFunc("/cms/api/login", s.handleCMSLogin)
	mux.HandleFunc("/cms/api/logout", s.handleCMSLogout)
	mux.HandleFunc("/cms/api/me", s.handleCMSMe)
	mux.HandleFunc("/cms/api/regulations", s.handleCMSRegulations)
	mux.HandleFunc("/cms/api/regulations/", s.handleCMSRegulationDetail)
	mux.HandleFunc("/cms/api/venues", s.handleCMSVenues)
	mux.HandleFunc("/cms/api/venues/", s.handleCMSVenueDetail)
	mux.HandleFunc("/cms/api/cases", s.handleCMSCases)
	mux.HandleFunc("/cms/api/cases/", s.handleCMSCaseDetail)
	mux.HandleFunc("/cms/api/stories", s.handleCMSStories)
	mux.HandleFunc("/cms/api/stories/", s.handleCMSStoryDetail)
	mux.HandleFunc("/cms/api/media", s.handleCMSMedia)
	mux.HandleFunc("/cms/api/media/upload", s.handleCMSMediaUpload)
	mux.HandleFunc("/cms/api/media/", s.handleCMSMediaItem)
	mux.HandleFunc("/cms/api/publish", s.handleCMSPublish)

	// Backward compatible CMS Routes
	mux.HandleFunc("/api/cms/login", s.handleCMSLogin)
	mux.HandleFunc("/api/cms/logout", s.handleCMSLogout)
	mux.HandleFunc("/api/cms/me", s.handleCMSMe)
	mux.HandleFunc("/api/cms/regulations", s.handleCMSRegulations)
	mux.HandleFunc("/api/cms/regulations/", s.handleCMSRegulationDetail)
	mux.HandleFunc("/api/cms/venues", s.handleCMSVenues)
	mux.HandleFunc("/api/cms/venues/", s.handleCMSVenueDetail)
	mux.HandleFunc("/api/cms/cases", s.handleCMSCases)
	mux.HandleFunc("/api/cms/cases/", s.handleCMSCaseDetail)
	mux.HandleFunc("/api/cms/stories", s.handleCMSStories)
	mux.HandleFunc("/api/cms/stories/", s.handleCMSStoryDetail)
	mux.HandleFunc("/api/cms/media", s.handleCMSMedia)
	mux.HandleFunc("/api/cms/media/upload", s.handleCMSMediaUpload)
	mux.HandleFunc("/api/cms/media/", s.handleCMSMediaItem)
	mux.HandleFunc("/api/cms/publish", s.handleCMSPublish)

	mediaDir := filepath.Join(contentDir, "media")
	if err := os.MkdirAll(mediaDir, 0o755); err != nil {
		panic(err)
	}
	mux.Handle("/media/", http.StripPrefix("/media/", http.FileServer(http.Dir(mediaDir))))

	// Serve CMS Frontend
	cmsDir := filepath.Join(".", "cms-ui")
	mux.HandleFunc("/cms/", func(w http.ResponseWriter, r *http.Request) {
		// If the path is exactly /cms/ or doesn't have an extension, serve index.html
		if r.URL.Path == "/cms/" || !strings.Contains(r.URL.Path, ".") {
			http.ServeFile(w, r, filepath.Join(cmsDir, "index.html"))
			return
		}
		// Otherwise serve static files
		fs := http.FileServer(http.Dir(cmsDir))
		http.StripPrefix("/cms/", fs).ServeHTTP(w, r)
	})

	addr := "127.0.0.1:" + port
	fmt.Printf("server (go) listening on http://%s\n", addr)
	fmt.Printf("syncRepoDir: %s\n", absOr(s.syncDir, s.syncDir))
	fmt.Printf("contentRepoDir: %s\n", absOr(s.contentDir, s.contentDir))
	if err := http.ListenAndServe(addr, mux); err != nil {
		panic(err)
	}
}

func (s *Server) handleHealth(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"ok":             true,
		"syncRepoDir":    absOr(s.syncDir, s.syncDir),
		"contentRepoDir": absOr(s.contentDir, s.contentDir),
	})
}

// CMS Auth Middleware
func (s *Server) requireCMSAuth(w http.ResponseWriter, r *http.Request) bool {
	if s.cmsAuth == nil || !s.cmsAuth.IsAuthed(r) {
		writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
		return false
	}
	return true
}

func (s *Server) initCMSDB() {
	if s.cmsDB == nil {
		return
	}
	_, err := s.cmsDB.Exec(`
		CREATE TABLE IF NOT EXISTS regulations (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			level TEXT NOT NULL,
			cover_url TEXT,
			source TEXT,
			published_at TEXT,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS regulation_sections (
			id TEXT PRIMARY KEY,
			regulation_id TEXT NOT NULL,
			chapter TEXT,
			article_no TEXT,
			title TEXT,
			body TEXT NOT NULL,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS venues (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			type TEXT NOT NULL,
			cover_url TEXT,
			location TEXT,
			description TEXT,
			contact TEXT,
			open_hours TEXT,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS cases (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			scene TEXT NOT NULL,
			summary TEXT NOT NULL,
			cover_url TEXT,
			body TEXT NOT NULL,
			violation TEXT,
			correct_action TEXT,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS stories (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			cover_url TEXT,
			body TEXT NOT NULL,
			source TEXT,
			day_of_year INTEGER,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS cms_media (
			id TEXT PRIMARY KEY,
			filename TEXT NOT NULL,
			mime TEXT NOT NULL,
			size INTEGER NOT NULL,
			sha256 TEXT NOT NULL,
			created_at INTEGER NOT NULL
		);
	`)
	if err != nil {
		fmt.Printf("Failed to init CMS DB: %v\n", err)
	}

	// Best-effort schema upgrades for existing cms.db
	_, _ = s.cmsDB.Exec("ALTER TABLE regulations ADD COLUMN cover_url TEXT")
	_, _ = s.cmsDB.Exec("ALTER TABLE venues ADD COLUMN cover_url TEXT")
	_, _ = s.cmsDB.Exec("ALTER TABLE cases ADD COLUMN cover_url TEXT")
	_, _ = s.cmsDB.Exec("ALTER TABLE stories ADD COLUMN cover_url TEXT")
}

func (s *Server) syncFromClientContentDB() {
	// Try multiple possible paths for the client content.db
	possiblePaths := []string{
		filepath.Join("..", "src-tauri", "content.db"),
		filepath.Join("..", "..", "src-tauri", "content.db"),
		filepath.Join(".", "..", "src-tauri", "content.db"),
	}

	var clientDBPath string
	for _, p := range possiblePaths {
		absPath, _ := filepath.Abs(p)
		if _, err := os.Stat(absPath); err == nil {
			clientDBPath = absPath
			break
		}
	}

	if clientDBPath == "" {
		fmt.Println("[CMS Sync] Client content.db not found in any expected location, skipping sync.")
		return
	}

	fmt.Printf("[CMS Sync] Found client DB at: %s\n", clientDBPath)

	clientDB, err := sql.Open("sqlite3", clientDBPath)
	if err != nil {
		fmt.Printf("[CMS Sync] Failed to open client DB: %v\n", err)
		return
	}
	defer clientDB.Close()

	// Check if CMS DB already has data
	var count int
	s.cmsDB.QueryRow("SELECT COUNT(*) FROM regulations").Scan(&count)
	if count > 0 {
		fmt.Printf("[CMS Sync] CMS DB already has %d records, skipping sync.\n", count)
		return
	}

	fmt.Println("[CMS Sync] CMS DB is empty. Syncing regulations from client content.db...")
	rows, err := clientDB.Query("SELECT id, title, level, source, published_at, updated_at FROM regulations")
	if err != nil {
		fmt.Printf("[CMS Sync] Query client regulations failed: %v\n", err)
		return
	}
	defer rows.Close()

	tx, _ := s.cmsDB.Begin()
	stmt, _ := tx.Prepare("INSERT OR REPLACE INTO regulations(id, title, level, cover_url, source, published_at, updated_at) VALUES(?,?,?,?,?,?,?)")
	defer stmt.Close()

	importedCount := 0
	for rows.Next() {
		var id, title, level string
		var source, pubAt sql.NullString
		var updatedAt int64
		rows.Scan(&id, &title, &level, &source, &pubAt, &updatedAt)
		stmt.Exec(id, title, level, "", source.String, pubAt.String, updatedAt)
		importedCount++

		// Sync sections
		secRows, _ := clientDB.Query("SELECT id, chapter, article_no, title, body, updated_at FROM regulation_sections WHERE regulation_id=?", id)
		secStmt, _ := tx.Prepare("INSERT OR REPLACE INTO regulation_sections(id, regulation_id, chapter, article_no, title, body, updated_at) VALUES(?,?,?,?,?,?,?)")
		secCount := 0
		for secRows.Next() {
			var sid, chap, artNo, stitle, sbody string
			var sUpdatedAt int64
			secRows.Scan(&sid, &chap, &artNo, &stitle, &sbody, &sUpdatedAt)
			secStmt.Exec(sid, id, chap, artNo, stitle, sbody, sUpdatedAt)
			secCount++
		}
		secRows.Close()
		secStmt.Close()
		fmt.Printf("[CMS Sync] Imported regulation '%s' with %d sections\n", title, secCount)
	}
	tx.Commit()
	fmt.Printf("[CMS Sync] Sync completed. Total imported: %d regulations\n", importedCount)
}

func (s *Server) handleCMSLogin(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	body, _ := readBodyLimit(r.Body, 1024)
	var payload struct {
		Password string `json:"password"`
		Remember bool   `json:"remember"`
	}
	_ = json.Unmarshal(body, &payload)
	if strings.TrimSpace(payload.Password) == "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "password required"})
		return
	}
	if s.cmsAuth == nil || !s.cmsAuth.CheckPassword(payload.Password) {
		writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "invalid password"})
		return
	}
	s.cmsAuth.IssueCookie(w, payload.Remember)
	writeJSON(w, http.StatusOK, map[string]any{"ok": true})
}

func (s *Server) handleCMSLogout(w http.ResponseWriter, r *http.Request) {
	if s.cmsAuth != nil {
		s.cmsAuth.ClearCookie(w)
	}
	writeJSON(w, http.StatusOK, map[string]any{"ok": true})
}

func (s *Server) handleCMSPublish(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	if r.Method != http.MethodPost {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}

	body, _ := readBodyLimit(r.Body, 1024*1024)
	var pubPayload map[string]any
	_ = json.Unmarshal(body, &pubPayload)

	// 1. Create temp content.db
	tmpDir, _ := os.MkdirTemp("", "cms-publish-*")
	defer os.RemoveAll(tmpDir)
	contentDBPath := filepath.Join(tmpDir, "content.db")
	conn, err := sql.Open("sqlite3", contentDBPath)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}
	defer conn.Close()

	// Migrate schema (simplified version of db.rs logic)
	_, _ = conn.Exec(`
		CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
		CREATE TABLE venues (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			type TEXT NOT NULL,
			cover_url TEXT,
			location TEXT,
			description TEXT,
			contact TEXT,
			open_hours TEXT,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE cases (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			scene TEXT NOT NULL,
			summary TEXT NOT NULL,
			cover_url TEXT,
			body TEXT NOT NULL,
			violation TEXT,
			correct_action TEXT,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE regulations (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			level TEXT NOT NULL,
			cover_url TEXT,
			source TEXT,
			published_at TEXT,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE regulation_sections (
			id TEXT PRIMARY KEY,
			regulation_id TEXT NOT NULL,
			chapter TEXT,
			article_no TEXT,
			title TEXT,
			body TEXT NOT NULL,
			updated_at INTEGER NOT NULL
		);
		CREATE TABLE stories (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			cover_url TEXT,
			body TEXT NOT NULL,
			source TEXT,
			day_of_year INTEGER,
			updated_at INTEGER NOT NULL
		);
	`)

	// Copy data from cms.db to content.db
	venueRows, _ := s.cmsDB.Query("SELECT id, name, type, cover_url, location, description, contact, open_hours, updated_at FROM venues")
	for venueRows != nil && venueRows.Next() {
		var id, name, t string
		var cover, location, desc, contact, openHours sql.NullString
		var updatedAt int64
		venueRows.Scan(&id, &name, &t, &cover, &location, &desc, &contact, &openHours, &updatedAt)
		conn.Exec("INSERT OR REPLACE INTO venues VALUES(?,?,?,?,?,?,?,?,?)", id, name, t, cover.String, location.String, desc.String, contact.String, openHours.String, updatedAt)
	}
	if venueRows != nil {
		venueRows.Close()
	}

	caseRows, _ := s.cmsDB.Query("SELECT id, title, scene, summary, cover_url, body, violation, correct_action, updated_at FROM cases")
	for caseRows != nil && caseRows.Next() {
		var id, title, scene, summary, body string
		var cover, violation, correctAction sql.NullString
		var updatedAt int64
		caseRows.Scan(&id, &title, &scene, &summary, &cover, &body, &violation, &correctAction, &updatedAt)
		conn.Exec("INSERT OR REPLACE INTO cases VALUES(?,?,?,?,?,?,?,?,?)", id, title, scene, summary, cover.String, body, violation.String, correctAction.String, updatedAt)
	}
	if caseRows != nil {
		caseRows.Close()
	}

	rows, _ := s.cmsDB.Query("SELECT id, title, level, cover_url, source, published_at, updated_at FROM regulations")
	for rows.Next() {
		var id, title, level string
		var cover, source, pubAt sql.NullString
		var updatedAt int64
		rows.Scan(&id, &title, &level, &cover, &source, &pubAt, &updatedAt)
		conn.Exec("INSERT OR REPLACE INTO regulations VALUES(?,?,?,?,?,?,?)", id, title, level, cover.String, source.String, pubAt.String, updatedAt)

		secRows, _ := s.cmsDB.Query("SELECT id, regulation_id, chapter, article_no, title, body, updated_at FROM regulation_sections WHERE regulation_id=?", id)
		for secRows.Next() {
			var sid, rid, chap, artNo, stitle, sbody string
			var sUpdatedAt int64
			secRows.Scan(&sid, &rid, &chap, &artNo, &stitle, &sbody, &sUpdatedAt)
			conn.Exec("INSERT OR REPLACE INTO regulation_sections VALUES(?,?,?,?,?,?,?)", sid, rid, chap, artNo, stitle, sbody, sUpdatedAt)
		}
		secRows.Close()
	}
	rows.Close()

	storyRows, _ := s.cmsDB.Query("SELECT id, title, cover_url, body, source, day_of_year, updated_at FROM stories")
	for storyRows != nil && storyRows.Next() {
		var id, title, body string
		var cover, source sql.NullString
		var dayOfYear sql.NullInt64
		var updatedAt int64
		storyRows.Scan(&id, &title, &cover, &body, &source, &dayOfYear, &updatedAt)
		conn.Exec("INSERT OR REPLACE INTO stories VALUES(?,?,?,?,?,?,?)", id, title, cover.String, body, source.String, dayOfYear.Int64, updatedAt)
	}
	if storyRows != nil {
		storyRows.Close()
	}

	// Update version
	newVersion := strings.TrimSpace(getStringOne(pubPayload, "version"))
	if newVersion == "" {
		newVersion = fmt.Sprintf("%d", time.Now().UnixMilli())
	}
	conn.Exec("INSERT OR REPLACE INTO meta(key, value) VALUES('content_version', ?)", newVersion)

	// 2. Generate manifest.json
	manifest := map[string]any{
		"contentVersion": newVersion,
		"minAppVersion":  nil,
	}
	manifestBytes, _ := json.MarshalIndent(manifest, "", "  ")
	os.WriteFile(filepath.Join(tmpDir, "manifest.json"), manifestBytes, 0644)

	// 3. Build zip
	zipPath := filepath.Join(s.contentDir, "content-pack.zip")
	if err := buildZip(zipPath, tmpDir); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}

	// 4. Update versions.json
	notes := strings.TrimSpace(getStringOne(pubPayload, "notes"))
	if notes == "" {
		notes = "Published via CMS at " + time.Now().Format(time.RFC3339)
	}
	versions := map[string]any{
		"latest": map[string]any{
			"version": newVersion,
			"url":     "/content-pack.zip",
			"notes":   notes,
		},
	}
	vBytes, _ := json.MarshalIndent(versions, "", "  ")
	os.WriteFile(filepath.Join(s.contentDir, "versions.json"), vBytes, 0644)

	writeJSON(w, http.StatusOK, map[string]any{"version": newVersion})
}

func buildZip(zipPath, srcDir string) error {
	file, err := os.Create(zipPath)
	if err != nil {
		return err
	}
	defer file.Close()

	w := zip.NewWriter(file)
	defer w.Close()

	files := []string{"manifest.json", "content.db"}
	for _, fname := range files {
		f, err := os.Open(filepath.Join(srcDir, fname))
		if err != nil {
			return err
		}
		defer f.Close()

		info, err := f.Stat()
		if err != nil {
			return err
		}

		header, err := zip.FileInfoHeader(info)
		if err != nil {
			return err
		}
		header.Method = zip.Deflate

		writer, err := w.CreateHeader(header)
		if err != nil {
			return err
		}
		_, err = io.Copy(writer, f)
		if err != nil {
			return err
		}
	}
	return nil
}

func (s *Server) handleCMSMe(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"authenticated": true})
}

func (s *Server) handleCMSRegulations(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}

	switch r.Method {
	case http.MethodGet:
		rows, err := s.cmsDB.Query("SELECT id, title, level, cover_url, source, published_at, updated_at FROM regulations ORDER BY updated_at DESC")
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		defer rows.Close()
		var regs []map[string]any
		for rows.Next() {
			var id, title, level string
			var cover, source, pubAt sql.NullString
			var updatedAt int64
			rows.Scan(&id, &title, &level, &cover, &source, &pubAt, &updatedAt)
			regs = append(regs, map[string]any{
				"id":           id,
				"title":        convertGBKToUTF8(title),
				"level":        convertGBKToUTF8(level),
				"cover_url":    cover.String,
				"source":       convertGBKToUTF8(source.String),
				"published_at": pubAt.String,
				"updated_at":   updatedAt,
				"status":       "published",
			})
		}
		writeJSON(w, http.StatusOK, regs)

	case http.MethodPost:
		body, _ := readBodyLimit(r.Body, 1024*1024)
		var payload map[string]any
		json.Unmarshal(body, &payload)

		id := getStringOne(payload, "id")
		if id == "" {
			id = fmt.Sprintf("reg_%d", time.Now().UnixMilli())
		}
		now := time.Now().UnixMilli()
		tx, err := s.cmsDB.Begin()
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		_, err = tx.Exec(
			"INSERT OR REPLACE INTO regulations(id, title, level, cover_url, source, published_at, updated_at) VALUES(?,?,?,?,?,?,?)",
			id,
			getStringOne(payload, "title"),
			getStringOne(payload, "level"),
			getStringOne(payload, "cover_url"),
			getStringOne(payload, "source"),
			getStringOne(payload, "published_at"),
			now,
		)
		if err != nil {
			_ = tx.Rollback()
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		_, _ = tx.Exec("DELETE FROM regulation_sections WHERE regulation_id=?", id)
		if arr, ok := payload["sections"].([]any); ok {
			for i, v := range arr {
				m, ok := v.(map[string]any)
				if !ok {
					continue
				}
				sid := strings.TrimSpace(getStringOne(m, "id"))
				if sid == "" {
					sid = fmt.Sprintf("sec_%d_%d", now, i)
				}
				_, err := tx.Exec(
					"INSERT OR REPLACE INTO regulation_sections(id, regulation_id, chapter, article_no, title, body, updated_at) VALUES(?,?,?,?,?,?,?)",
					sid,
					id,
					getStringOne(m, "chapter"),
					getStringOne(m, "article_no"),
					getStringOne(m, "title"),
					getStringOne(m, "body"),
					now,
				)
				if err != nil {
					_ = tx.Rollback()
					writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
					return
				}
			}
		}
		if err := tx.Commit(); err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"id": id})

	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSRegulationDetail(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}

	id := strings.TrimPrefix(r.URL.Path, "/cms/api/regulations/")
	if id == r.URL.Path {
		id = strings.TrimPrefix(r.URL.Path, "/api/cms/regulations/")
	}
	id = strings.TrimSpace(id)
	if id == "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "missing id"})
		return
	}

	switch r.Method {
	case http.MethodGet:
		var title, level string
		var cover, source, pubAt sql.NullString
		var updatedAt int64
		err := s.cmsDB.QueryRow("SELECT id, title, level, cover_url, source, published_at, updated_at FROM regulations WHERE id=?", id).Scan(
			&id, &title, &level, &cover, &source, &pubAt, &updatedAt,
		)
		if err != nil {
			fmt.Printf("[CMS Error] Query failed for ID %s: %v\n", id, err)
			writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
			return
		}
		reg := map[string]any{
			"id":           id,
			"title":        convertGBKToUTF8(title),
			"level":        convertGBKToUTF8(level),
			"cover_url":    cover.String,
			"source":       convertGBKToUTF8(source.String),
			"published_at": pubAt.String,
			"updated_at":   updatedAt,
			"status":       "draft",
		}

		secRows, _ := s.cmsDB.Query("SELECT id, chapter, article_no, title, body FROM regulation_sections WHERE regulation_id=?", id)
		var sections []map[string]any
		if secRows != nil {
			for secRows.Next() {
				var sid, chap, artNo, stitle, sbody string
				secRows.Scan(&sid, &chap, &artNo, &stitle, &sbody)
				sections = append(sections, map[string]any{
					"id":         sid,
					"chapter":    convertGBKToUTF8(chap),
					"article_no": convertGBKToUTF8(artNo),
					"title":      convertGBKToUTF8(stitle),
					"body":       convertGBKToUTF8(sbody),
				})
			}
			secRows.Close()
		}
		if sections == nil {
			sections = []map[string]any{}
		}
		reg["sections"] = sections
		writeJSON(w, http.StatusOK, reg)

	case http.MethodPost, http.MethodPut:
		body, _ := readBodyLimit(r.Body, 1024*1024)
		var payload map[string]any
		json.Unmarshal(body, &payload)
		payload["id"] = id
		now := time.Now().UnixMilli()
		tx, err := s.cmsDB.Begin()
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		_, err = tx.Exec(
			"INSERT OR REPLACE INTO regulations(id, title, level, cover_url, source, published_at, updated_at) VALUES(?,?,?,?,?,?,?)",
			id,
			getStringOne(payload, "title"),
			getStringOne(payload, "level"),
			getStringOne(payload, "cover_url"),
			getStringOne(payload, "source"),
			getStringOne(payload, "published_at"),
			now,
		)
		if err != nil {
			_ = tx.Rollback()
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		_, _ = tx.Exec("DELETE FROM regulation_sections WHERE regulation_id=?", id)
		if arr, ok := payload["sections"].([]any); ok {
			for i, v := range arr {
				m, ok := v.(map[string]any)
				if !ok {
					continue
				}
				sid := strings.TrimSpace(getStringOne(m, "id"))
				if sid == "" {
					sid = fmt.Sprintf("sec_%d_%d", now, i)
				}
				_, err := tx.Exec(
					"INSERT OR REPLACE INTO regulation_sections(id, regulation_id, chapter, article_no, title, body, updated_at) VALUES(?,?,?,?,?,?,?)",
					sid,
					id,
					getStringOne(m, "chapter"),
					getStringOne(m, "article_no"),
					getStringOne(m, "title"),
					getStringOne(m, "body"),
					now,
				)
				if err != nil {
					_ = tx.Rollback()
					writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
					return
				}
			}
		}
		if err := tx.Commit(); err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"ok": true, "id": id})

	case http.MethodDelete:
		s.cmsDB.Exec("DELETE FROM regulation_sections WHERE regulation_id=?", id)
		s.cmsDB.Exec("DELETE FROM regulations WHERE id=?", id)
		writeJSON(w, http.StatusOK, map[string]any{"ok": true})

	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) cmsMediaDir() string {
	return filepath.Join(s.contentDir, "media")
}

func (s *Server) handleCMSVenues(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	switch r.Method {
	case http.MethodGet:
		rows, err := s.cmsDB.Query("SELECT id, name, type, cover_url, location, updated_at FROM venues ORDER BY updated_at DESC")
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		defer rows.Close()
		var items []map[string]any
		for rows.Next() {
			var id, name, t string
			var cover, location sql.NullString
			var updatedAt int64
			rows.Scan(&id, &name, &t, &cover, &location, &updatedAt)
			items = append(items, map[string]any{
				"id":         id,
				"name":       convertGBKToUTF8(name),
				"type":       convertGBKToUTF8(t),
				"cover_url":  cover.String,
				"location":   convertGBKToUTF8(location.String),
				"updated_at": updatedAt,
			})
		}
		writeJSON(w, http.StatusOK, items)
	case http.MethodPost:
		body, _ := readBodyLimit(r.Body, 1024*1024)
		var payload map[string]any
		_ = json.Unmarshal(body, &payload)
		id := strings.TrimSpace(getStringOne(payload, "id"))
		if id == "" {
			id = fmt.Sprintf("ven_%d", time.Now().UnixMilli())
		}
		now := time.Now().UnixMilli()
		_, err := s.cmsDB.Exec(
			"INSERT OR REPLACE INTO venues(id, name, type, cover_url, location, description, contact, open_hours, updated_at) VALUES(?,?,?,?,?,?,?,?,?)",
			id,
			getStringOne(payload, "name"),
			getStringOne(payload, "type"),
			getStringOne(payload, "cover_url"),
			getStringOne(payload, "location"),
			getStringOne(payload, "description"),
			getStringOne(payload, "contact"),
			getStringOne(payload, "open_hours"),
			now,
		)
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"id": id})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSVenueDetail(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	id := strings.TrimPrefix(r.URL.Path, "/cms/api/venues/")
	if id == r.URL.Path {
		id = strings.TrimPrefix(r.URL.Path, "/api/cms/venues/")
	}
	id, _ = url.PathUnescape(strings.TrimSpace(id))
	if id == "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "missing id"})
		return
	}
	switch r.Method {
	case http.MethodGet:
		var name, t string
		var cover, location, desc, contact, openHours sql.NullString
		var updatedAt int64
		err := s.cmsDB.QueryRow(
			"SELECT name, type, cover_url, location, description, contact, open_hours, updated_at FROM venues WHERE id=?",
			id,
		).Scan(&name, &t, &cover, &location, &desc, &contact, &openHours, &updatedAt)
		if err != nil {
			writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{
			"id":          id,
			"name":        convertGBKToUTF8(name),
			"type":        convertGBKToUTF8(t),
			"cover_url":   cover.String,
			"location":    convertGBKToUTF8(location.String),
			"description": convertGBKToUTF8(desc.String),
			"contact":     convertGBKToUTF8(contact.String),
			"open_hours":  convertGBKToUTF8(openHours.String),
			"updated_at":  updatedAt,
		})
	case http.MethodDelete:
		_, _ = s.cmsDB.Exec("DELETE FROM venues WHERE id=?", id)
		writeJSON(w, http.StatusOK, map[string]any{"ok": true})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSCases(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	switch r.Method {
	case http.MethodGet:
		rows, err := s.cmsDB.Query("SELECT id, title, scene, summary, cover_url, updated_at FROM cases ORDER BY updated_at DESC")
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		defer rows.Close()
		var items []map[string]any
		for rows.Next() {
			var id, title, scene, summary string
			var cover sql.NullString
			var updatedAt int64
			rows.Scan(&id, &title, &scene, &summary, &cover, &updatedAt)
			items = append(items, map[string]any{
				"id":         id,
				"title":      convertGBKToUTF8(title),
				"scene":      convertGBKToUTF8(scene),
				"summary":    convertGBKToUTF8(summary),
				"cover_url":  cover.String,
				"updated_at": updatedAt,
			})
		}
		writeJSON(w, http.StatusOK, items)
	case http.MethodPost:
		body, _ := readBodyLimit(r.Body, 1024*1024)
		var payload map[string]any
		_ = json.Unmarshal(body, &payload)
		id := strings.TrimSpace(getStringOne(payload, "id"))
		if id == "" {
			id = fmt.Sprintf("case_%d", time.Now().UnixMilli())
		}
		now := time.Now().UnixMilli()
		_, err := s.cmsDB.Exec(
			"INSERT OR REPLACE INTO cases(id, title, scene, summary, cover_url, body, violation, correct_action, updated_at) VALUES(?,?,?,?,?,?,?,?,?)",
			id,
			getStringOne(payload, "title"),
			getStringOne(payload, "scene"),
			getStringOne(payload, "summary"),
			getStringOne(payload, "cover_url"),
			getStringOne(payload, "body"),
			getStringOne(payload, "violation"),
			getStringOne(payload, "correct_action"),
			now,
		)
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"id": id})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSCaseDetail(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	id := strings.TrimPrefix(r.URL.Path, "/cms/api/cases/")
	if id == r.URL.Path {
		id = strings.TrimPrefix(r.URL.Path, "/api/cms/cases/")
	}
	id, _ = url.PathUnescape(strings.TrimSpace(id))
	if id == "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "missing id"})
		return
	}
	switch r.Method {
	case http.MethodGet:
		var title, scene, summary, body string
		var cover, violation, correctAction sql.NullString
		var updatedAt int64
		err := s.cmsDB.QueryRow(
			"SELECT title, scene, summary, cover_url, body, violation, correct_action, updated_at FROM cases WHERE id=?",
			id,
		).Scan(&title, &scene, &summary, &cover, &body, &violation, &correctAction, &updatedAt)
		if err != nil {
			writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{
			"id":             id,
			"title":          convertGBKToUTF8(title),
			"scene":          convertGBKToUTF8(scene),
			"summary":        convertGBKToUTF8(summary),
			"cover_url":      cover.String,
			"body":           convertGBKToUTF8(body),
			"violation":      convertGBKToUTF8(violation.String),
			"correct_action": convertGBKToUTF8(correctAction.String),
			"updated_at":     updatedAt,
		})
	case http.MethodDelete:
		_, _ = s.cmsDB.Exec("DELETE FROM cases WHERE id=?", id)
		writeJSON(w, http.StatusOK, map[string]any{"ok": true})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSStories(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	switch r.Method {
	case http.MethodGet:
		rows, err := s.cmsDB.Query("SELECT id, title, source, day_of_year, cover_url, updated_at FROM stories ORDER BY updated_at DESC")
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		defer rows.Close()
		var items []map[string]any
		for rows.Next() {
			var id, title string
			var source sql.NullString
			var day sql.NullInt64
			var cover sql.NullString
			var updatedAt int64
			rows.Scan(&id, &title, &source, &day, &cover, &updatedAt)
			items = append(items, map[string]any{
				"id":          id,
				"title":       convertGBKToUTF8(title),
				"source":      convertGBKToUTF8(source.String),
				"day_of_year": day.Int64,
				"cover_url":   cover.String,
				"updated_at":  updatedAt,
			})
		}
		writeJSON(w, http.StatusOK, items)
	case http.MethodPost:
		body, _ := readBodyLimit(r.Body, 1024*1024)
		var payload map[string]any
		_ = json.Unmarshal(body, &payload)
		id := strings.TrimSpace(getStringOne(payload, "id"))
		if id == "" {
			id = fmt.Sprintf("story_%d", time.Now().UnixMilli())
		}
		now := time.Now().UnixMilli()
		day := int64(0)
		if v, ok := payload["day_of_year"]; ok {
			switch x := v.(type) {
			case float64:
				day = int64(x)
			case int64:
				day = x
			}
		}
		_, err := s.cmsDB.Exec(
			"INSERT OR REPLACE INTO stories(id, title, cover_url, body, source, day_of_year, updated_at) VALUES(?,?,?,?,?,?,?)",
			id,
			getStringOne(payload, "title"),
			getStringOne(payload, "cover_url"),
			getStringOne(payload, "body"),
			getStringOne(payload, "source"),
			day,
			now,
		)
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"id": id})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSStoryDetail(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	id := strings.TrimPrefix(r.URL.Path, "/cms/api/stories/")
	if id == r.URL.Path {
		id = strings.TrimPrefix(r.URL.Path, "/api/cms/stories/")
	}
	id, _ = url.PathUnescape(strings.TrimSpace(id))
	if id == "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "missing id"})
		return
	}
	switch r.Method {
	case http.MethodGet:
		var title, body string
		var source sql.NullString
		var day sql.NullInt64
		var cover sql.NullString
		var updatedAt int64
		err := s.cmsDB.QueryRow(
			"SELECT title, cover_url, body, source, day_of_year, updated_at FROM stories WHERE id=?",
			id,
		).Scan(&title, &cover, &body, &source, &day, &updatedAt)
		if err != nil {
			writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{
			"id":          id,
			"title":       convertGBKToUTF8(title),
			"cover_url":   cover.String,
			"body":        convertGBKToUTF8(body),
			"source":      convertGBKToUTF8(source.String),
			"day_of_year": day.Int64,
			"updated_at":  updatedAt,
		})
	case http.MethodDelete:
		_, _ = s.cmsDB.Exec("DELETE FROM stories WHERE id=?", id)
		writeJSON(w, http.StatusOK, map[string]any{"ok": true})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleCMSMedia(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	if r.Method != http.MethodGet {
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
		return
	}
	rows, err := s.cmsDB.Query("SELECT id, filename, mime, size, sha256, created_at FROM cms_media ORDER BY created_at DESC LIMIT 200")
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}
	defer rows.Close()
	var items []map[string]any
	for rows.Next() {
		var id, filename, mime, sha string
		var size int64
		var createdAt int64
		rows.Scan(&id, &filename, &mime, &size, &sha, &createdAt)
		items = append(items, map[string]any{
			"id":         id,
			"filename":   filename,
			"mime":       mime,
			"size":       size,
			"sha256":     sha,
			"created_at": createdAt,
			"url":        "/media/" + id,
		})
	}
	writeJSON(w, http.StatusOK, items)
}

func (s *Server) handleCMSMediaUpload(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	if r.Method != http.MethodPost {
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
		return
	}
	if err := r.ParseMultipartForm(20 << 20); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "bad multipart"})
		return
	}
	f, hdr, err := r.FormFile("file")
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "file required"})
		return
	}
	defer f.Close()
	buf := make([]byte, 512)
	n, _ := f.Read(buf)
	_, _ = f.Seek(0, io.SeekStart)
	mimeType := http.DetectContentType(buf[:n])
	id, _ := randomToken()
	ext := strings.ToLower(filepath.Ext(hdr.Filename))
	if ext == "" {
		switch mimeType {
		case "image/png":
			ext = ".png"
		case "image/jpeg":
			ext = ".jpg"
		case "image/webp":
			ext = ".webp"
		case "video/mp4":
			ext = ".mp4"
		default:
			ext = ".bin"
		}
	}
	fileID := id + ext
	dstPath := filepath.Join(s.cmsMediaDir(), fileID)
	out, err := os.Create(dstPath)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}
	defer out.Close()
	h := sha256.New()
	size, err := io.Copy(io.MultiWriter(out, h), f)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}
	sum := fmt.Sprintf("%x", h.Sum(nil))
	now := time.Now().UnixMilli()
	_, err = s.cmsDB.Exec(
		"INSERT OR REPLACE INTO cms_media(id, filename, mime, size, sha256, created_at) VALUES(?,?,?,?,?,?)",
		fileID,
		hdr.Filename,
		mimeType,
		size,
		sum,
		now,
	)
	if err != nil {
		_ = os.Remove(dstPath)
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"id":       fileID,
		"url":      "/media/" + fileID,
		"mime":     mimeType,
		"size":     size,
		"sha256":   sum,
		"filename": hdr.Filename,
	})
}

func (s *Server) handleCMSMediaItem(w http.ResponseWriter, r *http.Request) {
	if !s.requireCMSAuth(w, r) {
		return
	}
	id := strings.TrimPrefix(r.URL.Path, "/cms/api/media/")
	if id == r.URL.Path {
		id = strings.TrimPrefix(r.URL.Path, "/api/cms/media/")
	}
	id, _ = url.PathUnescape(strings.TrimSpace(id))
	if id == "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "missing id"})
		return
	}
	switch r.Method {
	case http.MethodGet:
		var filename, mime, sha string
		var size int64
		var createdAt int64
		err := s.cmsDB.QueryRow("SELECT filename, mime, size, sha256, created_at FROM cms_media WHERE id=?", id).Scan(&filename, &mime, &size, &sha, &createdAt)
		if err != nil {
			writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{
			"id":         id,
			"filename":   filename,
			"mime":       mime,
			"size":       size,
			"sha256":     sha,
			"created_at": createdAt,
			"url":        "/media/" + id,
		})
	case http.MethodDelete:
		_, _ = s.cmsDB.Exec("DELETE FROM cms_media WHERE id=?", id)
		_ = os.Remove(filepath.Join(s.cmsMediaDir(), id))
		writeJSON(w, http.StatusOK, map[string]any{"ok": true})
	default:
		writeJSON(w, http.StatusMethodNotAllowed, map[string]any{"error": "method not allowed"})
	}
}

func (s *Server) handleVersions(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	s.serveFileSafe(w, s.contentDir, "versions.json", "application/json; charset=utf-8")
}

func (s *Server) handleContentPack(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	s.serveFileSafe(w, s.contentDir, "content-pack.zip", "application/zip")
}

func (s *Server) serveFileSafe(w http.ResponseWriter, baseDir, fileName, contentType string) {
	baseAbs, err := filepath.Abs(baseDir)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	targetAbs := filepath.Join(baseAbs, fileName)
	rel, err := filepath.Rel(baseAbs, targetAbs)
	if err != nil || strings.HasPrefix(rel, "..") || rel == "." {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	b, err := os.ReadFile(targetAbs)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	w.Header().Set("Content-Type", contentType)
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(b)
}

func (s *Server) handleLogin(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	body, err := readBodyLimit(r.Body, 2*1024*1024)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": err.Error()})
		return
	}
	var payload map[string]any
	if len(body) > 0 {
		if err := json.Unmarshal(body, &payload); err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]any{"error": "bad json"})
			return
		}
	}

	username := strings.TrimSpace(getStringOne(payload, "username"))
	password := getStringOne(payload, "password")
	if username == "" {
		username = "user"
	}

	var user UserRecord
	s.mu.Lock()
	existing, ok := s.users[username]
	if !ok {
		salt, err := randomToken()
		if err != nil {
			s.mu.Unlock()
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "salt"})
			return
		}
		userID, err := randomToken()
		if err != nil {
			s.mu.Unlock()
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "user"})
			return
		}
		user = UserRecord{
			UserID:       userID,
			Username:     username,
			Salt:         salt,
			PasswordHash: hashPassword(salt, password),
		}
		s.users[username] = user
		if err := s.saveUsersLocked(); err != nil {
			s.mu.Unlock()
			writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "save"})
			return
		}
	} else {
		user = existing
		if user.PasswordHash != hashPassword(user.Salt, password) {
			s.mu.Unlock()
			writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
			return
		}
	}
	s.mu.Unlock()

	token, err := randomToken()
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "token"})
		return
	}

	s.mu.Lock()
	rec := TokenRecord{
		Token:     token,
		UserID:    user.UserID,
		ExpiresAt: time.Now().Add(30 * 24 * time.Hour).UnixMilli(),
	}
	s.tokens[token] = rec
	if err := s.saveTokensLocked(); err != nil {
		s.mu.Unlock()
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "save"})
		return
	}
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, map[string]any{"accessToken": token})
}

func (s *Server) handlePush(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	userID, ok := s.requireAuth(w, r)
	if !ok {
		return
	}

	body, err := readBodyLimit(r.Body, 4*1024*1024)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": err.Error()})
		return
	}
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "bad json"})
		return
	}
	rawEvents, _ := payload["events"].([]any)

	acked := make([]string, 0, len(rawEvents))
	now := time.Now().UnixMilli()

	s.mu.Lock()
	store, err := s.loadUserStoreLocked(userID)
	if err != nil {
		s.mu.Unlock()
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "store"})
		return
	}
	eventIDSet := map[string]struct{}{}
	for _, e := range store.Events {
		eventIDSet[e.EventID] = struct{}{}
	}

	for _, it := range rawEvents {
		m, ok := it.(map[string]any)
		if !ok {
			continue
		}
		eventID := getStringMany(m, "eventId", "event_id")
		if strings.TrimSpace(eventID) == "" {
			continue
		}
		acked = append(acked, eventID)
		if _, exists := eventIDSet[eventID]; exists {
			continue
		}
		eventIDSet[eventID] = struct{}{}

		payloadBytes, _ := json.Marshal(m["payload"])
		if len(payloadBytes) == 0 {
			payloadBytes = []byte("{}")
		}

		ev := Event{
			Seq:        store.NextSeq,
			EventID:    eventID,
			DeviceID:   getStringMany(m, "deviceId", "device_id"),
			EventType:  getStringMany(m, "eventType", "event_type"),
			EntityType: getStringMany(m, "entityType", "entity_type"),
			EntityID:   getStringMany(m, "entityId", "entity_id"),
			Payload:    payloadBytes,
			OccurredAt: getInt64Many(m, now, "occurredAt", "occurred_at"),
		}
		store.NextSeq++
		store.Events = append(store.Events, ev)
	}

	if err := s.saveUserStoreLocked(userID, store); err != nil {
		s.mu.Unlock()
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "save"})
		return
	}
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, map[string]any{"acked": acked})
}

func (s *Server) handlePull(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	userID, ok := s.requireAuth(w, r)
	if !ok {
		return
	}

	cursorStr := r.URL.Query().Get("cursor")
	cursor := int64(0)
	if cursorStr != "" {
		if v, err := strconv.ParseInt(cursorStr, 10, 64); err == nil && v >= 0 {
			cursor = v
		}
	}

	s.mu.Lock()
	store, err := s.loadUserStoreLocked(userID)
	s.mu.Unlock()
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "store"})
		return
	}

	page := make([]Event, 0, 500)
	for _, e := range store.Events {
		if e.Seq > cursor {
			page = append(page, e)
		}
	}
	sort.Slice(page, func(i, j int) bool {
		if page[i].Seq != page[j].Seq {
			return page[i].Seq < page[j].Seq
		}
		return page[i].EventID < page[j].EventID
	})
	if len(page) > 500 {
		page = page[:500]
	}

	nextCursor := cursor
	if len(page) > 0 {
		nextCursor = page[len(page)-1].Seq
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"events":     page,
		"nextCursor": fmt.Sprintf("%d", nextCursor),
	})
}

func (s *Server) requireAuth(w http.ResponseWriter, r *http.Request) (string, bool) {
	h := r.Header.Get("Authorization")
	parts := strings.SplitN(h, " ", 2)
	if len(parts) != 2 || !strings.EqualFold(parts[0], "Bearer") {
		writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
		return "", false
	}
	token := strings.TrimSpace(parts[1])
	if token == "" {
		writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
		return "", false
	}
	now := time.Now().UnixMilli()

	s.mu.Lock()
	rec, ok := s.tokens[token]
	if ok && rec.ExpiresAt <= now {
		delete(s.tokens, token)
		_ = s.saveTokensLocked()
		ok = false
	}
	if ok {
		if rec.ExpiresAt-now < (7 * 24 * 60 * 60 * 1000) {
			rec.ExpiresAt = time.Now().Add(30 * 24 * time.Hour).UnixMilli()
			s.tokens[token] = rec
			_ = s.saveTokensLocked()
		}
	}
	s.mu.Unlock()
	if !ok {
		writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
		return "", false
	}
	return rec.UserID, true
}

func (s *Server) loadAll() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	_ = s.loadUsersLocked()
	_ = s.loadTokensLocked()
	return nil
}

func (s *Server) loadUsersLocked() error {
	b, err := os.ReadFile(s.usersPath)
	if err != nil {
		return nil
	}
	var f UsersFile
	if err := json.Unmarshal(b, &f); err != nil {
		return err
	}
	s.users = map[string]UserRecord{}
	for _, u := range f.Users {
		if strings.TrimSpace(u.Username) == "" || strings.TrimSpace(u.UserID) == "" {
			continue
		}
		s.users[u.Username] = u
	}
	return nil
}

func (s *Server) saveUsersLocked() error {
	list := make([]UserRecord, 0, len(s.users))
	for _, u := range s.users {
		list = append(list, u)
	}
	sort.Slice(list, func(i, j int) bool { return list[i].Username < list[j].Username })
	f := UsersFile{Users: list}
	b, err := json.MarshalIndent(f, "", "  ")
	if err != nil {
		return err
	}
	tmp := s.usersPath + ".tmp"
	if err := os.WriteFile(tmp, b, 0o644); err != nil {
		return err
	}
	return os.Rename(tmp, s.usersPath)
}

func (s *Server) loadTokensLocked() error {
	b, err := os.ReadFile(s.tokensPath)
	if err != nil {
		return nil
	}
	var f TokensFile
	if err := json.Unmarshal(b, &f); err != nil {
		return err
	}
	now := time.Now().UnixMilli()
	s.tokens = map[string]TokenRecord{}
	for _, t := range f.Tokens {
		if strings.TrimSpace(t.Token) == "" || strings.TrimSpace(t.UserID) == "" {
			continue
		}
		if t.ExpiresAt <= now {
			continue
		}
		s.tokens[t.Token] = t
	}
	return nil
}

func (s *Server) saveTokensLocked() error {
	list := make([]TokenRecord, 0, len(s.tokens))
	for _, t := range s.tokens {
		list = append(list, t)
	}
	sort.Slice(list, func(i, j int) bool { return list[i].Token < list[j].Token })
	f := TokensFile{Tokens: list}
	b, err := json.MarshalIndent(f, "", "  ")
	if err != nil {
		return err
	}
	tmp := s.tokensPath + ".tmp"
	if err := os.WriteFile(tmp, b, 0o644); err != nil {
		return err
	}
	return os.Rename(tmp, s.tokensPath)
}

func (s *Server) userStorePath(userID string) (string, string) {
	dir := filepath.Join(s.syncDir, "events", userID)
	return dir, filepath.Join(dir, "events.json")
}

func (s *Server) loadUserStoreLocked(userID string) (Store, error) {
	dir, file := s.userStorePath(userID)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return Store{}, err
	}
	b, err := os.ReadFile(file)
	if err != nil {
		return Store{NextSeq: 1, Events: []Event{}}, nil
	}
	var st Store
	if err := json.Unmarshal(b, &st); err != nil {
		return Store{}, err
	}
	if st.NextSeq <= 0 {
		st.NextSeq = 1
	}
	if st.Events == nil {
		st.Events = []Event{}
	}
	return st, nil
}

func (s *Server) saveUserStoreLocked(userID string, st Store) error {
	dir, file := s.userStorePath(userID)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(st, "", "  ")
	if err != nil {
		return err
	}
	tmp := file + ".tmp"
	if err := os.WriteFile(tmp, b, 0o644); err != nil {
		return err
	}
	return os.Rename(tmp, file)
}

func getenv(k, def string) string {
	v := strings.TrimSpace(os.Getenv(k))
	if v == "" {
		return def
	}
	return v
}

func absOr(p, fallback string) string {
	a, err := filepath.Abs(p)
	if err != nil {
		return fallback
	}
	return a
}

func randomToken() (string, error) {
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(buf), nil
}

func hashPassword(salt, password string) string {
	sum := sha256.Sum256([]byte(salt + ":" + password))
	return base64.RawURLEncoding.EncodeToString(sum[:])
}

func readBodyLimit(r io.Reader, limit int64) ([]byte, error) {
	if limit <= 0 {
		return nil, errors.New("bad limit")
	}
	lr := io.LimitReader(r, limit+1)
	b, err := io.ReadAll(lr)
	if err != nil {
		return nil, err
	}
	if int64(len(b)) > limit {
		return nil, errors.New("body too large")
	}
	return b, nil
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	b, _ := json.Marshal(v)
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.Header().Set("Content-Length", strconv.Itoa(len(b)))
	w.WriteHeader(status)
	_, _ = w.Write(b)
}

func convertGBKToUTF8(s string) string {
	if s == "" {
		return s
	}
	// Check if it's already valid UTF-8
	if utf8.ValidString(s) {
		return s
	}
	// Try to decode as GBK
	decoder := simplifiedchinese.GBK.NewDecoder()
	result, _, err := transform.String(decoder, s)
	if err != nil {
		fmt.Printf("[Encoding] Failed to convert GBK to UTF-8: %v, original: %s\n", err, s)
		return s
	}
	return result
}

func getStringOne(m map[string]any, key string) string {
	if m == nil {
		return ""
	}
	v, ok := m[key]
	if !ok {
		return ""
	}
	switch t := v.(type) {
	case string:
		return t
	default:
		return fmt.Sprintf("%v", t)
	}
}

func getStringMany(m map[string]any, keys ...string) string {
	if m == nil {
		return ""
	}
	for _, k := range keys {
		if v, ok := m[k]; ok {
			switch t := v.(type) {
			case string:
				return t
			default:
				return fmt.Sprintf("%v", t)
			}
		}
	}
	return ""
}

func getInt64Many(m map[string]any, def int64, keys ...string) int64 {
	if m == nil {
		return def
	}
	for _, k := range keys {
		if v, ok := m[k]; ok {
			switch t := v.(type) {
			case float64:
				return int64(t)
			case int64:
				return t
			case string:
				if n, err := strconv.ParseInt(t, 10, 64); err == nil {
					return n
				}
			}
		}
	}
	return def
}
