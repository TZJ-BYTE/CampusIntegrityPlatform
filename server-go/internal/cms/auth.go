package cms

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"
)

type Auth struct {
	PasswordHashHex string
	SessionSecret   []byte
	CookieName      string
	SessionTTL      time.Duration
	RememberTTL     time.Duration
	CookieSecure    bool
}

func LoadAuthFromEnv() (*Auth, error) {
	pwPlain := strings.TrimSpace(os.Getenv("CMS_PASSWORD"))
	pwHash := strings.TrimSpace(os.Getenv("CMS_PASSWORD_HASH"))
	if pwPlain == "" && pwHash == "" {
		pwPlain = "admin"
	}
	if pwPlain != "" && pwHash == "" {
		sum := sha256.Sum256([]byte(pwPlain))
		pwHash = hex.EncodeToString(sum[:])
	}
	_, err := hex.DecodeString(pwHash)
	if err != nil {
		return nil, errors.New("CMS_PASSWORD_HASH must be sha256 hex")
	}
	secret := os.Getenv("CMS_SESSION_SECRET")
	if secret == "" {
		b := make([]byte, 32)
		_, _ = rand.Read(b)
		secret = base64.StdEncoding.EncodeToString(b)
	}
	sec, err := base64.StdEncoding.DecodeString(secret)
	if err != nil {
		sec = []byte(secret)
	}
	secure := strings.TrimSpace(os.Getenv("CMS_COOKIE_SECURE")) == "1"
	return &Auth{
		PasswordHashHex: strings.ToLower(strings.TrimSpace(pwHash)),
		SessionSecret:   sec,
		CookieName:      "cms_session",
		SessionTTL:      12 * time.Hour,
		RememberTTL:     14 * 24 * time.Hour,
		CookieSecure:    secure,
	}, nil
}

func (a *Auth) CheckPassword(password string) bool {
	sum := sha256.Sum256([]byte(password))
	got := hex.EncodeToString(sum[:])
	return subtleEq(got, strings.ToLower(strings.TrimSpace(a.PasswordHashHex)))
}

func (a *Auth) IssueCookie(w http.ResponseWriter, remember bool) {
	ttl := a.SessionTTL
	if remember {
		ttl = a.RememberTTL
	}
	exp := time.Now().Add(ttl).Unix()
	nonce := make([]byte, 18)
	_, _ = rand.Read(nonce)
	nonceB64 := base64.RawURLEncoding.EncodeToString(nonce)
	payload := strconv.FormatInt(exp, 10) + "." + nonceB64
	sig := a.sign(payload)
	val := payload + "." + sig
	http.SetCookie(w, &http.Cookie{
		Name:     a.CookieName,
		Value:    val,
		Path:     "/",
		HttpOnly: true,
		SameSite: http.SameSiteLaxMode,
		Secure:   a.CookieSecure,
		Expires:  time.Unix(exp, 0),
	})
}

func (a *Auth) ClearCookie(w http.ResponseWriter) {
	http.SetCookie(w, &http.Cookie{
		Name:     a.CookieName,
		Value:    "",
		Path:     "/",
		HttpOnly: true,
		SameSite: http.SameSiteLaxMode,
		Secure:   a.CookieSecure,
		Expires:  time.Unix(0, 0),
		MaxAge:   -1,
	})
}

func (a *Auth) IsAuthed(r *http.Request) bool {
	c, err := r.Cookie(a.CookieName)
	if err != nil || c.Value == "" {
		return false
	}
	parts := strings.Split(c.Value, ".")
	if len(parts) != 3 {
		return false
	}
	expStr := parts[0]
	nonce := parts[1]
	sig := parts[2]
	_ = nonce
	exp, err := strconv.ParseInt(expStr, 10, 64)
	if err != nil {
		return false
	}
	if time.Now().Unix() > exp {
		return false
	}
	payload := parts[0] + "." + parts[1]
	if !subtleEq(sig, a.sign(payload)) {
		return false
	}
	return true
}

func (a *Auth) sign(payload string) string {
	m := hmac.New(sha256.New, a.SessionSecret)
	_, _ = m.Write([]byte(payload))
	return base64.RawURLEncoding.EncodeToString(m.Sum(nil))
}

func subtleEq(a, b string) bool {
	if len(a) != len(b) {
		return false
	}
	var out byte
	for i := 0; i < len(a); i++ {
		out |= a[i] ^ b[i]
	}
	return out == 0
}
