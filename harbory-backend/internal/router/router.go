package router

import (
    "net/http"
    "time"
    "github.com/PreetinderSinghBadesha/harbory/internal/http/handler"
)

func Router(startTime time.Time) *http.ServeMux {
    mux := http.NewServeMux()

		// router for health 
    mux.HandleFunc("/api/health", handler.HealthHandler(startTime))
		
		// router for containers
		mux.HandleFunc("GET /api/containers", handler.GetAllContainersHandler())
		mux.HandleFunc("GET /api/containers/{id}", handler.GetContainerByParams())


    return mux
}
