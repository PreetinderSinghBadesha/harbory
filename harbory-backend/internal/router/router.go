package router

import (
	"net/http"
	"time"

	"github.com/PreetinderSinghBadesha/harbory/internal/http/handler"
	"github.com/PreetinderSinghBadesha/harbory/internal/middleware"
)

func Router(startTime time.Time) *http.ServeMux {
	mux := http.NewServeMux()

	// Public routes (no authentication required)
	mux.HandleFunc("/api/health", handler.HealthHandler(startTime))
	mux.HandleFunc("/api/auth/login", handler.LoginHandler())
	mux.HandleFunc("POST /api/auth/login", handler.LoginHandler())
	mux.HandleFunc("/api/auth/verify", handler.VerifyHandler())
	mux.HandleFunc("GET /api/auth/verify", handler.VerifyHandler())

	// Protected routes (authentication required)
	mux.HandleFunc("/api/auth/logout", middleware.AuthMiddleware(handler.LogoutHandler()))
	mux.HandleFunc("POST /api/auth/logout", middleware.AuthMiddleware(handler.LogoutHandler()))

	mux.HandleFunc("/api/auth/change-password", middleware.AuthMiddleware(handler.ChangePasswordHandler()))
	mux.HandleFunc("POST /api/auth/change-password", middleware.AuthMiddleware(handler.ChangePasswordHandler()))

	// router for containers
	mux.HandleFunc("GET /api/containers", middleware.AuthMiddleware(handler.GetAllContainersHandler()))
	mux.HandleFunc("GET /api/containers/{id}", middleware.AuthMiddleware(handler.GetContainerByParams()))

	// router for images
	mux.HandleFunc("GET /api/images", middleware.AuthMiddleware(handler.GetAllImagesHandler()))
	mux.HandleFunc("GET /api/images/{id}", middleware.AuthMiddleware(handler.GetImageByParams()))

	//router for volumes
	mux.HandleFunc("GET /api/volumes", middleware.AuthMiddleware(handler.GetAllVolumesHandler()))
	mux.HandleFunc("GET /api/volumes/{id}", middleware.AuthMiddleware(handler.GetVolumeByParams()))

	//router for networks
	mux.HandleFunc("GET /api/networks", middleware.AuthMiddleware(handler.GetAllNetworksHandler()))
	mux.HandleFunc("GET /api/networks/{id}", middleware.AuthMiddleware(handler.GetNetworkByParams()))

	//router for nodes
	mux.HandleFunc("GET /api/nodes", middleware.AuthMiddleware(handler.GetAllNodesHandler()))
	mux.HandleFunc("GET /api/nodes/{id}", middleware.AuthMiddleware(handler.GetNodeByParams()))

	//router for deployment
	mux.HandleFunc("POST /api/deploy", middleware.AuthMiddleware(handler.DeployGithubHandler()))
	mux.Handle("/api/deploy/ws", middleware.AuthMiddlewareHandler(handler.DeployWebSocketHandler()))

	//router for GitHub
	mux.HandleFunc("POST /api/github/search", handler.GithubSearchHandler())
	mux.HandleFunc("POST /api/github/user/repos", handler.GithubUserReposHandler())

	//router for system stats
	mux.HandleFunc("GET /api/system/stats", middleware.AuthMiddleware(handler.GetSystemStatsHandler()))

	return mux
}
