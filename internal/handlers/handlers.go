package handlers

import (
	"harbory/internal/services"
	"html/template"
	"net/http"
	"path/filepath"
)

type Handler struct {
	dockerService *services.DockerService
}

func NewHandler(dockerService *services.DockerService) *Handler {
	return &Handler{dockerService: dockerService}
}

func (h *Handler) Dashboard(w http.ResponseWriter, r *http.Request) {
	containers, err := h.dockerService.ListContainers()
	if err != nil {
		http.Error(w, "Failed to list containers", http.StatusInternalServerError)
		return
	}

	tmpl, err := template.ParseFiles(
		filepath.Join("internal", "templates", "layout.html"),
		filepath.Join("internal", "templates", "dashboard.html"),
	)
	if err != nil {
		http.Error(w, "Failed to parse templates: "+err.Error(), http.StatusInternalServerError)
		return
	}

	if err := tmpl.ExecuteTemplate(w, "layout", containers); err != nil {
		http.Error(w, "Failed to execute template", http.StatusInternalServerError)
	}
}

func (h *Handler) CreateContainerPage(w http.ResponseWriter, r *http.Request) {
	tmpl, err := template.ParseFiles(
		filepath.Join("internal", "templates", "layout.html"),
		filepath.Join("internal", "templates", "create.html"),
	)
	if err != nil {
		http.Error(w, "Failed to parse templates", http.StatusInternalServerError)
		return
	}

	if err := tmpl.ExecuteTemplate(w, "layout", nil); err != nil {
		http.Error(w, "Failed to execute template", http.StatusInternalServerError)
	}
}

func (h *Handler) CreateContainer(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseForm(); err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	imageName := r.FormValue("image")
	containerName := r.FormValue("name")
	hostPort := r.FormValue("hostPort")
	containerPort := r.FormValue("containerPort")

	if err := h.dockerService.CreateContainer(imageName, containerName, hostPort, containerPort); err != nil {
		http.Error(w, "Failed to create container: "+err.Error(), http.StatusInternalServerError)
		return
	}

	http.Redirect(w, r, "/", http.StatusSeeOther)
}
