package router

import (
	"harbory/internal/handlers"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
)

func NewRouter(h *handlers.Handler) http.Handler {
	r := chi.NewRouter()

	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)

	r.Get("/", h.Dashboard)
	r.Get("/create", h.CreateContainerPage)
	r.Post("/create", h.CreateContainer)

	return r
}
