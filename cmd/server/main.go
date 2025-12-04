package main

import (
	"harbory/internal/handlers"
	"harbory/internal/router"
	"harbory/internal/services"
	"log"
	"net/http"
)

func main() {
	dockerService, err := services.NewDockerService()
	if err != nil {
		log.Fatalf("Failed to create Docker service: %v", err)
	}

	h := handlers.NewHandler(dockerService)
	r := router.NewRouter(h)

	log.Println("Server running on :8080")
	if err := http.ListenAndServe(":8080", r); err != nil {
		log.Fatalf("Server failed: %v", err)
	}
}
