package middleware

import (
	"crypto/rand"
	"encoding/base64"
	"net/http"
	"sync"
	"time"

	"github.com/PreetinderSinghBadesha/harbory/internal/config"
)

type SessionStore struct {
	sessions map[string]time.Time
	mu       sync.RWMutex
	password string
}

var store *SessionStore

func InitSessionStore(cfg *config.Config) {
	store = &SessionStore{
		sessions: make(map[string]time.Time),
		password: cfg.Auth.Password,
	}
	
	go store.cleanupExpiredSessions()
}

func (s *SessionStore) GenerateToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	token := base64.URLEncoding.EncodeToString(b)
	
	s.mu.Lock()
	s.sessions[token] = time.Now().Add(24 * time.Hour)
	s.mu.Unlock()
	
	return token, nil
}

func (s *SessionStore) ValidateToken(token string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	
	expiry, exists := s.sessions[token]
	if !exists {
		return false
	}
	
	return time.Now().Before(expiry)
}

func (s *SessionStore) RevokeToken(token string) {
	s.mu.Lock()
	delete(s.sessions, token)
	s.mu.Unlock()
}

func (s *SessionStore) ValidatePassword(password string) bool {
	return password == s.password
}

func (s *SessionStore) ChangePassword(oldPassword, newPassword string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	
	if oldPassword != s.password {
		return false
	}
	
	s.password = newPassword
	s.sessions = make(map[string]time.Time)
	return true
}

func (s *SessionStore) cleanupExpiredSessions() {
	ticker := time.NewTicker(30 * time.Minute)
	for range ticker.C {
		s.mu.Lock()
		now := time.Now()
		for token, expiry := range s.sessions {
			if now.After(expiry) {
				delete(s.sessions, token)
			}
		}
		s.mu.Unlock()
	}
}

func AuthMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		token := r.Header.Get("Authorization")
		if token == "" {
			cookie, err := r.Cookie("harbory_token")
			if err == nil {
				token = cookie.Value
			}
		} else {
			if len(token) > 7 && token[:7] == "Bearer " {
				token = token[7:]
			}
		}

		if token == "" || !store.ValidateToken(token) {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusUnauthorized)
			w.Write([]byte(`{"error": "Unauthorized"}`))
			return
		}

		next.ServeHTTP(w, r)
	}
}

func AuthMiddlewareHandler(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		token := r.Header.Get("Authorization")
		if token == "" {
			cookie, err := r.Cookie("harbory_token")
			if err == nil {
				token = cookie.Value
			}
		} else {
			if len(token) > 7 && token[:7] == "Bearer " {
				token = token[7:]
			}
		}

		if token == "" || !store.ValidateToken(token) {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusUnauthorized)
			w.Write([]byte(`{"error": "Unauthorized"}`))
			return
		}

		next.ServeHTTP(w, r)
	})
}

func GetSessionStore() *SessionStore {
	return store
}
