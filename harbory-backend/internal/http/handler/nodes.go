package handler

import (
	"context"
	"net/http"

	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
	"github.com/docker/docker/api/types"
	"github.com/docker/docker/client"
)

func GetAllNodesHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		nodes, err := cli.NodeList(context.Background(), types.NodeListOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		if err := response.WriteJSONResponse(w, http.StatusOK, nodes); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}

func GetNodeByParams() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		nodeID := r.PathValue("id")
		nodeResource, _, err := cli.NodeInspectWithRaw(context.Background(), nodeID)
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		if err := response.WriteJSONResponse(w, http.StatusOK, nodeResource); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}
