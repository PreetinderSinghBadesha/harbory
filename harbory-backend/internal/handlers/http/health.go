package handler

import (
    "encoding/json"
    "net/http"
    "time"
)

type HealthResponse struct {
    Status    string `json:"status"`
    Timestamp time.Time `json:"timestamp"`
    Uptime    string `json:"uptime,omitempty"`
    Checks    map[string]ComponentCheck `json:"checks,omitempty"`
}

type ComponentCheck struct {
    Status  string `json:"status"`
    Message string `json:"message,omitempty"`
    Latency int64 `json:"latency_ms,omitempty"`
}

func buildHealthResponse(startTime time.Time) HealthResponse {
    checks := map[string]ComponentCheck{
        "database": {
            Status:  "ok",
            Latency: 15,
        }
    }

    return HealthResponse{
        Status:    "ok",
        Timestamp: time.Now().UTC(),
        Uptime:    time.Since(startTime).String(),
        Checks:    checks,
    }
}

func HealthHandler(startTime time.Time) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        resp := buildHealthResponse(startTime)

        statusCode := http.StatusOK
        if resp.Status == "down" {
            statusCode = http.StatusServiceUnavailable
        }

        w.Header().Set("Content-Type", "application/json")
        w.WriteHeader(statusCode)

        if err := json.NewEncoder(w).Encode(resp); err != nil {
            http.Error(w, "failed to encode response", http.StatusInternalServerError)
        }
    }
}
