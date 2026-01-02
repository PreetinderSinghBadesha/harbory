package handler

import(
	"net/http"
	"context"
	
	"github.com/docker/docker/client"
	"github.com/docker/docker/api/types/volume"
	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
)

func GetAllVolumesHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request){
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()
		
		volumes, err := cli.VolumeList(context.Background(), volume.ListOptions{})
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		
		if err := response.WriteJSONResponse(w, http.StatusOK, volumes); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}

func GetVolumeByParams() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request){
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()
		
		volumeID := r.PathValue("id")
		volume, _, err := cli.VolumeInspectWithRaw(context.Background(), volumeID)
		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		
		if err := response.WriteJSONResponse(w, http.StatusOK, volume); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}