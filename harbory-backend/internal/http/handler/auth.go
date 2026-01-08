package handler

import (
	"encoding/json"
	"net/http"

	"github.com/PreetinderSinghBadesha/harbory/internal/middleware"
	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
)

type LoginRequest struct {
	Password string `json:"password"`
}

type LoginResponse struct {
	Token   string `json:"token"`
	Message string `json:"message"`
}

type LogoutResponse struct {
	Message string `json:"message"`
}

func LoginHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			response.SendError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}

		var req LoginRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			response.SendError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		store := middleware.GetSessionStore()
		if !store.ValidatePassword(req.Password) {
			response.SendError(w, http.StatusUnauthorized, "Invalid password")
			return
		}

		token, err := store.GenerateToken()
		if err != nil {
			response.SendError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		http.SetCookie(w, &http.Cookie{
			Name:     "harbory_token",
			Value:    token,
			Path:     "/",
			HttpOnly: true,
			Secure:   false
			SameSite: http.SameSiteLaxMode,
			MaxAge:   86400,
		})

		response.SendJSON(w, http.StatusOK, LoginResponse{
			Token:   token,
			Message: "Login successful",
		})
	}
}

func LogoutHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			response.SendError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}

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

		if token != "" {
			store := middleware.GetSessionStore()
			store.RevokeToken(token)
		}

		http.SetCookie(w, &http.Cookie{
			Name:     "harbory_token",
			Value:    "",
			Path:     "/",
			HttpOnly: true,
			MaxAge:   -1,
		})

		response.SendJSON(w, http.StatusOK, LogoutResponse{
			Message: "Logout successful",
		})
	}
}

func VerifyHandler() http.HandlerFunc {
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

		store := middleware.GetSessionStore()
		if token == "" || !store.ValidateToken(token) {
			response.SendError(w, http.StatusUnauthorized, "Invalid or expired session")
			return
		}

		response.SendJSON(w, http.StatusOK, map[string]string{
			"message": "Session valid",
		})
	}
}

type ChangePasswordRequest struct {
	OldPassword string `json:"old_password"`
	NewPassword string `json:"new_password"`
}

type ChangePasswordResponse struct {
	Message string `json:"message"`
}

func ChangePasswordHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			response.SendError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}

		var req ChangePasswordRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			response.SendError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		if req.OldPassword == "" || req.NewPassword == "" {
			response.SendError(w, http.StatusBadRequest, "Old password and new password are required")
			return
		}

		if len(req.NewPassword) < 4 {
			response.SendError(w, http.StatusBadRequest, "New password must be at least 4 characters long")
			return
		}

		store := middleware.GetSessionStore()
		if !store.ChangePassword(req.OldPassword, req.NewPassword) {
			response.SendError(w, http.StatusUnauthorized, "Invalid old password")
			return
		}

		response.SendJSON(w, http.StatusOK, ChangePasswordResponse{
			Message: "Password changed successfully. Please login again.",
		})
	}
}
