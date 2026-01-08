package main

import (
        "context"
        "fmt"
        "log/slog"
        "net/http"
        "os"
        "os/signal"
        "syscall"
        "time"

        "github.com/PreetinderSinghBadesha/harbory/internal/config"
        "github.com/PreetinderSinghBadesha/harbory/internal/middleware"
        "github.com/PreetinderSinghBadesha/harbory/internal/router"
)

func main() {
    cfg := config.MustLoad()
    startTime := time.Now().UTC()
    middleware.InitSessionStore(cfg)

    mux := router.Router(startTime)

    corsHandler := func(next http.Handler) http.Handler {
        return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
            w.Header().Set("Access-Control-Allow-Origin", "*")
            w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
            w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

            if r.Method == http.MethodOptions {
                w.WriteHeader(http.StatusOK)
                return
            }

            next.ServeHTTP(w, r)
        })
    }

    server := http.Server{
        Addr:         cfg.HTTPServer.Addr,
        Handler:      corsHandler(mux),
        ReadTimeout:  10 * time.Second,
        WriteTimeout: 10 * time.Second,
        IdleTimeout:  60 * time.Second,
    }

    slog.Info("Server started", slog.String("address", cfg.HTTPServer.Addr))
    fmt.Printf("server starting on http://%s\n", cfg.HTTPServer.Addr)

    done := make(chan os.Signal, 1)
    signal.Notify(done, os.Interrupt, syscall.SIGINT, syscall.SIGTERM)

                go func() {
        if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
        slog.Error("Server error", "error", err)
        done <- syscall.SIGTERM
         }
                }()

    <-done

    slog.Info("Shutting down the server")

    ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
    defer cancel()

    if err := server.Shutdown(ctx); err != nil {
        slog.Error("Failed to gracefully shutdown the server", "error", err)
    }

    slog.Info("Server gracefully stopped")
}
