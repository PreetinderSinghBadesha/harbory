package deploy

import (
	"errors"
	"os"
)

func GenerateDockerfile(framework string) error {
	var content string

	switch framework {
	case "node":
		content = NodeDockerfile
	case "react":
		content = ReactDockerfile
	case "go":
		content = GoDockerfile
	case "flutter":
		content = FlutterDockerfile
	default:
		return errors.New("unsupported framework")
	}

	return os.WriteFile("Dockerfile", []byte(content), 0644)
}
