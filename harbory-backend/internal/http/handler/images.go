package handler

import (
	"context"
	"net/http"

	"github.com/docker/docker/client"
	"github.com/docker/docker/api/types/image"
	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
)

func GetAllImagesHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request){
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()
		
		images, err := cli.ImageList(context.Background(), image.ListOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		if err := response.WriteJSONResponse(w, http.StatusOK, images); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

	}
}

func GetImageByParams() http.HandlerFunc{
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		imageID := r.PathValue("id")
		image, _, err := cli.ImageInspectWithRaw(context.Background(), imageID)
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		if err := response.WriteJSONResponse(w, http.StatusOK, image); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

	}
}