package main

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
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
	mu         sync.Mutex
	dataDir    string
	usersPath  string
	tokensPath string

	users  map[string]UserRecord
	tokens map[string]TokenRecord
}

func main() {
	port := getenv("PORT", "8788")
	dataDir := getenv("SYNC_REPO_DIR", filepath.Join(".", "sync-repo"))
	if err := os.MkdirAll(dataDir, 0o755); err != nil {
		panic(err)
	}

	s := &Server{
		dataDir: dataDir,
		usersPath: filepath.Join(dataDir, "users.json"),
		tokensPath: filepath.Join(dataDir, "tokens.json"),
		users:   map[string]UserRecord{},
		tokens:  map[string]TokenRecord{},
	}
	_ = s.loadAll()

	mux := http.NewServeMux()
	mux.HandleFunc("/v1/auth/login", s.handleLogin)
	mux.HandleFunc("/v1/sync/push", s.handlePush)
	mux.HandleFunc("/v1/sync/pull", s.handlePull)
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		_ = writeJSON(w, http.StatusOK, map[string]any{"ok": true})
	})

	addr := "127.0.0.1:" + port
	fmt.Printf("sync server (go) listening on http://%s\n", addr)
	fmt.Printf("repoDir: %s\n", dataDir)
	if err := http.ListenAndServe(addr, mux); err != nil {
		panic(err)
	}
}

func (s *Server) handleLogin(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		_ = writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	body, err := readBodyLimit(r.Body, 2*1024*1024)
	if err != nil {
		_ = writeJSON(w, http.StatusBadRequest, map[string]any{"error": err.Error()})
		return
	}
	var payload map[string]any
	if len(body) > 0 {
		if err := json.Unmarshal(body, &payload); err != nil {
			_ = writeJSON(w, http.StatusBadRequest, map[string]any{"error": "bad json"})
			return
		}
	}

	username := strings.TrimSpace(getString(payload, "username"))
	password := getString(payload, "password")
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
			_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "salt"})
			return
		}
		userID, err := randomToken()
		if err != nil {
			s.mu.Unlock()
			_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "user"})
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
			_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "save"})
			return
		}
	} else {
		user = existing
		if user.PasswordHash != hashPassword(user.Salt, password) {
			s.mu.Unlock()
			_ = writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
			return
		}
	}
	s.mu.Unlock()

	token, err := randomToken()
	if err != nil {
		_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "token"})
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
		_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "save"})
		return
	}
	s.mu.Unlock()

	_ = writeJSON(w, http.StatusOK, map[string]any{"accessToken": token})
}

func (s *Server) handlePush(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		_ = writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
		return
	}
	userID, ok := s.requireAuth(w, r)
	if !ok {
		return
	}

	body, err := readBodyLimit(r.Body, 4*1024*1024)
	if err != nil {
		_ = writeJSON(w, http.StatusBadRequest, map[string]any{"error": err.Error()})
		return
	}
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		_ = writeJSON(w, http.StatusBadRequest, map[string]any{"error": "bad json"})
		return
	}
	rawEvents, _ := payload["events"].([]any)

	acked := make([]string, 0, len(rawEvents))
	now := time.Now().UnixMilli()

	s.mu.Lock()
	store, err := s.loadUserStoreLocked(userID)
	if err != nil {
		s.mu.Unlock()
		_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "store"})
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
		eventID := getString(m, "eventId", "event_id")
		if strings.TrimSpace(eventID) == "" {
			continue
		}
		acked = append(acked, eventID)
		if _, exists := eventIDSet[eventID]; exists {
			continue
		}
		eventIDSet[eventID] = struct{}{}

		payloadBytes, _ := json.Marshal(getAny(m, "payload"))
		if len(payloadBytes) == 0 {
			payloadBytes = []byte("{}")
		}

		ev := Event{
			Seq:        store.NextSeq,
			EventID:    eventID,
			DeviceID:   getString(m, "deviceId", "device_id"),
			EventType:  getString(m, "eventType", "event_type"),
			EntityType: getString(m, "entityType", "entity_type"),
			EntityID:   getString(m, "entityId", "entity_id"),
			Payload:    payloadBytes,
			OccurredAt: getInt64(m, now, "occurredAt", "occurred_at"),
		}
		store.NextSeq++
		store.Events = append(store.Events, ev)
	}

	if err := s.saveUserStoreLocked(userID, store); err != nil {
		s.mu.Unlock()
		_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}
	s.mu.Unlock()
	_ = writeJSON(w, http.StatusOK, map[string]any{"acked": acked})
}

func (s *Server) handlePull(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		_ = writeJSON(w, http.StatusNotFound, map[string]any{"error": "not found"})
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
	if err != nil {
		s.mu.Unlock()
		_ = writeJSON(w, http.StatusInternalServerError, map[string]any{"error": "store"})
		return
	}
	s.mu.Unlock()

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
	resp := map[string]any{
		"events":     page,
		"nextCursor": fmt.Sprintf("%d", nextCursor),
	}
	_ = writeJSON(w, http.StatusOK, resp)
}

func (s *Server) requireAuth(w http.ResponseWriter, r *http.Request) (string, bool) {
	h := r.Header.Get("Authorization")
	parts := strings.SplitN(h, " ", 2)
	if len(parts) != 2 || !strings.EqualFold(parts[0], "Bearer") {
		_ = writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
		return "", false
	}
	token := strings.TrimSpace(parts[1])
	if token == "" {
		_ = writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
		return "", false
	}
	now := time.Now()
	s.mu.Lock()
	rec, ok := s.tokens[token]
	if ok && time.UnixMilli(rec.ExpiresAt).Before(now) {
		delete(s.tokens, token)
		_ = s.saveTokensLocked()
		ok = false
	}
	s.mu.Unlock()
	if !ok {
		_ = writeJSON(w, http.StatusUnauthorized, map[string]any{"error": "unauthorized"})
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
	dir := filepath.Join(s.dataDir, "events", userID)
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

func writeJSON(w http.ResponseWriter, status int, v any) error {
	b, err := json.Marshal(v)
	if err != nil {
		return err
	}
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.Header().Set("Content-Length", strconv.Itoa(len(b)))
	w.WriteHeader(status)
	_, err = w.Write(b)
	return err
}

func getAny(m map[string]any, key string) any {
	if v, ok := m[key]; ok {
		return v
	}
	return nil
}

func getString(m map[string]any, keys ...string) string {
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

func getInt64(m map[string]any, def int64, keys ...string) int64 {
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
