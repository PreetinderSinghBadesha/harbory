package handler

import (
	"encoding/json"
	"log"
	"net/http"

	"github.com/PreetinderSinghBadesha/harbory/internal/deploy"
	"golang.org/x/net/websocket"
)

type DeployMessage struct {
	Type    string `json:"type"` // "status", "log", "error", "success"
	Message string `json:"message"`
	Step    string `json:"step,omitempty"` // "cloning", "building", "running"
}

func DeployWebSocketHandler() http.Handler {
	return websocket.Handler(func(ws *websocket.Conn) {
		defer ws.Close()

		var req DeployRequest
		if err := websocket.JSON.Receive(ws, &req); err != nil {
			sendWSMessage(ws, DeployMessage{
				Type:    "error",
				Message: "Invalid request: " + err.Error(),
			})
			return
		}

		sendWSMessage(ws, DeployMessage{
			Type:    "status",
			Message: "Starting deployment...",
			Step:    "initializing",
		})

		payload := deploy.DeployPayload{
			RepoUrl:        req.RepoUrl,
			HasDockerfile:  req.HasDockerfile,
			DockerfilePath: req.DockerfilePath,
			Framework:      req.Framework,
			GithubToken:    req.GithubToken,
		}

		logChan := make(chan string, 100)
		errChan := make(chan error, 1)

		go func() {
			if err := deploy.DeployFromPayloadWithProgress(payload, logChan); err != nil {
				errChan <- err
			} else {
				close(logChan)
			}
		}()

		for {
			select {
			case logMsg, ok := <-logChan:
				if !ok {
					sendWSMessage(ws, DeployMessage{
						Type:    "success",
						Message: "Deployment completed successfully!",
					})
					return
				}
				sendWSMessage(ws, DeployMessage{
					Type:    "log",
					Message: logMsg,
				})
			case err := <-errChan:
				sendWSMessage(ws, DeployMessage{
					Type:    "error",
					Message: "Deployment failed: " + err.Error(),
				})
				return
			}
		}
	})
}

func sendWSMessage(ws *websocket.Conn, msg DeployMessage) {
	data, err := json.Marshal(msg)
	if err != nil {
		log.Printf("Error marshaling message: %v", err)
		return
	}
	if err := websocket.Message.Send(ws, string(data)); err != nil {
		log.Printf("Error sending message: %v", err)
	}
}
