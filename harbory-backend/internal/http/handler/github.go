package handler

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

type GithubSearchRequest struct {
	Query string `json:"query"`
	Page  int    `json:"page"`
	Token string `json:"token,omitempty"`
}

type GithubUserReposRequest struct {
	Token string `json:"token"`
	Page  int    `json:"page"`
}

type GithubRepository struct {
	ID          int64  `json:"id"`
	Name        string `json:"name"`
	FullName    string `json:"full_name"`
	Description string `json:"description"`
	CloneURL    string `json:"clone_url"`
	HTMLURL     string `json:"html_url"`
	Private     bool   `json:"private"`
	Language    string `json:"language"`
	Stars       int    `json:"stargazers_count"`
	Forks       int    `json:"forks_count"`
	Owner       struct {
		Login     string `json:"login"`
		AvatarURL string `json:"avatar_url"`
	} `json:"owner"`
}

type GithubSearchResponse struct {
	TotalCount int                `json:"total_count"`
	Items      []GithubRepository `json:"items"`
}

func GithubSearchHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req GithubSearchRequest
		
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		if req.Query == "" {
			http.Error(w, "query is required", http.StatusBadRequest)
			return
		}

		if req.Page < 1 {
			req.Page = 1
		}

		url := fmt.Sprintf("https://api.github.com/search/repositories?q=%s&page=%d&per_page=30", req.Query, req.Page)
		
		githubReq, err := http.NewRequest("GET", url, nil)
		if err != nil {
			http.Error(w, "failed to create request", http.StatusInternalServerError)
			return
		}

		githubReq.Header.Set("Accept", "application/vnd.github.v3+json")
		if req.Token != "" {
			githubReq.Header.Set("Authorization", "token "+req.Token)
		}

		client := &http.Client{}
		resp, err := client.Do(githubReq)
		if err != nil {
			http.Error(w, "failed to search repositories", http.StatusInternalServerError)
			return
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			body, _ := io.ReadAll(resp.Body)
			http.Error(w, fmt.Sprintf("GitHub API error: %s", string(body)), resp.StatusCode)
			return
		}

		var searchResp GithubSearchResponse
		if err := json.NewDecoder(resp.Body).Decode(&searchResp); err != nil {
			http.Error(w, "failed to decode response", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(searchResp)
	}
}

func GithubUserReposHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req GithubUserReposRequest
		
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		if req.Token == "" {
			http.Error(w, "token is required", http.StatusBadRequest)
			return
		}

		if req.Page < 1 {
			req.Page = 1
		}

		url := fmt.Sprintf("https://api.github.com/user/repos?page=%d&per_page=30&sort=updated&affiliation=owner,collaborator", req.Page)

		githubReq, err := http.NewRequest("GET", url, nil)
		if err != nil {
			http.Error(w, "failed to create request", http.StatusInternalServerError)
			return
		}

		githubReq.Header.Set("Accept", "application/vnd.github.v3+json")
		githubReq.Header.Set("Authorization", "token "+req.Token)

		client := &http.Client{}
		resp, err := client.Do(githubReq)
		if err != nil {
			http.Error(w, "failed to fetch repositories", http.StatusInternalServerError)
			return
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			body, _ := io.ReadAll(resp.Body)
			http.Error(w, fmt.Sprintf("GitHub API error: %s", string(body)), resp.StatusCode)
			return
		}

		var repos []GithubRepository
		if err := json.NewDecoder(resp.Body).Decode(&repos); err != nil {
			http.Error(w, "failed to decode response", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(repos)
	}
}
