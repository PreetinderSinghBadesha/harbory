package router

import (
    "net/http"
    "time"

    "github.com/PreetinderSinghBadesha/harbory/internal/handlers/http"
)

func Router(startTime time.Time) *http.ServeMux {
    mux := http.NewServeMux()

    mux.HandleFunc("/api/health", handler.HealthHandler(startTime))

    return mux
}
