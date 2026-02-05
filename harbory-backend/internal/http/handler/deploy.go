package handler

import (
	"encoding/json"
	"net/http"

	"github.com/PreetinderSinghBadesha/harbory/internal/deploy"
)

type DeployRequest struct {
	RepoUrl        string `json:"repo_url"`
	HasDockerfile  bool   `json:"has_dockerfile"`
	DockerfilePath string `json:"dockerfile_path"`
	Framework      string `json:"framework"`
	GithubToken    string `json:"github_token,omitempty"`
}

func DeployGithubHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req DeployRequest

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		payload := deploy.DeployPayload{
			RepoUrl:        req.RepoUrl,
			HasDockerfile:  req.HasDockerfile,
			DockerfilePath: req.DockerfilePath,
			Framework:      req.Framework,
			GithubToken:    req.GithubToken,
		}

		err := deploy.DeployFromPayload(payload)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"success": true}`))
	}
}
