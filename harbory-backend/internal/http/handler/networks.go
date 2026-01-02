package handler

import (
	"context"
	"net/http"

	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
	"github.com/docker/docker/api/types/network"
	"github.com/docker/docker/client"
)

func GetAllNetworksHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		networks, err := cli.NetworkList(context.Background(), network.ListOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		if err := response.WriteJSONResponse(w, http.StatusOK, networks); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}

func GetNetworkByParams() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		networkID := r.PathValue("id")
		networkResource, _, err := cli.NetworkInspectWithRaw(context.Background(), networkID, network.InspectOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		if err := response.WriteJSONResponse(w, http.StatusOK, networkResource); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}
