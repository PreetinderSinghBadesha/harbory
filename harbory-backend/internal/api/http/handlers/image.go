package handlers

import (
	"context"
	"net/http"

	"github.com/docker/docker/api/types/filters"
	"github.com/docker/docker/api/types/image"
	"github.com/docker/docker/client"
	"github.com/gin-gonic/gin"
	"github.com/preetindersinghbadesha/harbory/internal/utils/response"
)

func GetAllImagesHandler() gin.HandlerFunc {
	return func(c *gin.Context) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		images, err := cli.ImageList(context.Background(), image.ListOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

		// Send images as JSON response
		if err := response.WriteJSONResponse(c.Writer, http.StatusOK, images); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

	}
}

func GetImageByParams() gin.HandlerFunc {
	return func(c *gin.Context) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		imageID := c.Param("id")
		image, _, err := cli.ImageInspectWithRaw(c.Request.Context(), imageID)
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

		// Send image as JSON response
		if err := response.WriteJSONResponse(c.Writer, http.StatusOK, image); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

	}
}

func DeleteImageHandler() gin.HandlerFunc {
	return func(c *gin.Context) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		imageID := c.Param("id")
		_, err = cli.ImageRemove(c.Request.Context(), imageID, image.RemoveOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

		// Send success response
		if err := response.WriteJSONResponse(c.Writer, http.StatusNoContent, nil); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

	}
}

func PruneUnusedImagesHandler() gin.HandlerFunc {
	return func(c *gin.Context) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		pruneFilters := filters.NewArgs()
		pruneFilters.Add("dangling", "false")

		rep, err := cli.ImagesPrune(c.Request.Context(), pruneFilters)
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}

		var prunedImages []string
		deletedlayercnt := 0

		for _, img := range rep.ImagesDeleted {
			if img.Untagged != "" {
				prunedImages = append(prunedImages, img.Untagged)
			}
			if img.Deleted != "" {
				deletedlayercnt++
			}
		}

		spaceReclaimedMB := rep.SpaceReclaimed / (1024 * 1024)

		resp := map[string]interface{}{
			"status":               "success",
			"pruned_images":        prunedImages,
			"deleted_layers_count": deletedlayercnt,
			"space_reclaimed_mb":   spaceReclaimedMB,
		}

		if err := response.WriteJSONResponse(c.Writer, http.StatusOK, resp); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(c.Writer, http.StatusInternalServerError, errorResp)
			return
		}
	}
}
