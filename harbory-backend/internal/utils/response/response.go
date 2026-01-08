package response

import (
	"encoding/json"
	"net/http"
)

type Response struct {
	Status string `json:"status"`
	Error  string `json:"error,omitempty"`
}

const (
	StatusOK    = "OK"
	StatusError = "Error"
)

func WriteJSONResponse(w http.ResponseWriter, status int, data interface{}) error {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)

	if data != nil {
		return json.NewEncoder(w).Encode(data)
	}

	return nil
}

func GeneralErrorResponse(err error) Response {
	return Response{
		Status: StatusError,
		Error:  err.Error(),
	}
}

func SendJSON(w http.ResponseWriter, statusCode int, data interface{}) {
w.Header().Set("Content-Type", "application/json")
w.WriteHeader(statusCode)
json.NewEncoder(w).Encode(data)
}

func SendError(w http.ResponseWriter, statusCode int, message string) {
w.Header().Set("Content-Type", "application/json")
w.WriteHeader(statusCode)
json.NewEncoder(w).Encode(Response{
Status: StatusError,
Error:  message,
})
}
