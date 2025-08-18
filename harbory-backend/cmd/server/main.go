package main

import (
	"context"
	"fmt"
	"log"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/preetindersinghbadesha/harbory/internal/config"
	"github.com/preetindersinghbadesha/harbory/internal/router"
)

func main() {
	// load config
	cfg := config.MustLoad()

	// setup router
	router := router.SetupRouter()
	
	// CORS middleware
	corsHandler := func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Access-Control-Allow-Origin", "*")
			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

			if r.Method == "OPTIONS" {
				w.WriteHeader(http.StatusOK)
				return
			}

			next.ServeHTTP(w, r)
		})
	}

	// setup server
	server := http.Server{
		Addr:    cfg.HTTPServer.Addr,
		Handler: corsHandler(router),
	}

	slog.Info("Server started", slog.String("address", cfg.HTTPServer.Addr))
	fmt.Printf("server starting on http://%s\n", cfg.HTTPServer.Addr)

	// handle graceful shutdown
	done := make(chan os.Signal, 1)
	signal.Notify(done, os.Interrupt, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Failed to start server: %s", err.Error())
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
