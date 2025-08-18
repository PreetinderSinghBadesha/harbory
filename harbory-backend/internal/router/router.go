package router

import (
	"github.com/gin-gonic/gin"
	"github.com/preetindersinghbadesha/harbory/internal/api"
	"github.com/preetindersinghbadesha/harbory/internal/api/http/handlers"
)

func SetupRouter() *gin.Engine{
	router := gin.Default()

	// Health endpoints
	router.GET("/api/health", api.HealthHandler())

	// Container endpoints
	router.GET("/api/container/all", handlers.GetAllContainersHandler())
	router.GET("/api/containers/:id", handlers.GetContainerByParams())

	// Image endpoints
	router.GET("/api/images/all", handlers.GetAllImagesHandler())
	router.GET("/api/images/:id", handlers.GetImageByParams())
	router.POST("api/images/prune", handlers.PruneUnusedImagesHandler())

	return router
}