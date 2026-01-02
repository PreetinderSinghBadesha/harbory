package handler

import (
	"context"
	"net/http"

	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
)

func GetAllContainersHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		containers, err := cli.ContainerList(context.Background(), container.ListOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		// Send all containers as JSON response
		if err := response.WriteJSONResponse(w, http.StatusOK, containers); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}

func GetContainerByParams() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		// Retrieve "id" from path values (Go 1.22+) or query parameters as a fallback/alternative depending on router setup.
		// Since the original was c.Param("id"), it expects a path parameter.
		// Standard net/http in Go 1.22+ uses r.PathValue("id").
		containerID := r.PathValue("id")

		container, _, err := cli.ContainerInspectWithRaw(context.Background(), containerID, true)
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		// Send container details as JSON response
		if err := response.WriteJSONResponse(w, http.StatusOK, container); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}
