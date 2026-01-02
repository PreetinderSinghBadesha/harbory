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

    // router for images
    mux.HandleFunc("GET /api/images", handler.GetAllImagesHandler())
    mux.HandleFunc("GET /api/images/{id}", handler.GetImageByParams())

    //router for volumes
    mux.HandleFunc("GET /api/volumes", handler.GetAllVolumesHandler())
    mux.HandleFunc("GET /api/volumes/{id}", handler.GetVolumeByParams())

    //router for networks
    mux.HandleFunc("GET /api/networks", handler.GetAllNetworksHandler())
    mux.HandleFunc("GET /api/networks/{id}", handler.GetNetworkByParams())

    //router for nodes
    mux.HandleFunc("GET /api/nodes", handler.GetAllNodesHandler())
    mux.HandleFunc("GET /api/nodes/{id}", handler.GetNodeByParams())

    return mux
}
