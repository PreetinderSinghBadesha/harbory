package deploy

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"strings"
)

type DeployPayload struct {
	RepoUrl        string
	HasDockerfile  bool
	DockerfilePath string
	Framework      string
}

func DeployFromPayload(p DeployPayload) error {

	repo := strings.Split(p.RepoUrl, "/")
	name := strings.TrimSuffix(repo[len(repo)-1], ".git")

	run("git", "clone", p.RepoUrl)
	os.Chdir(name)

	if p.HasDockerfile {
		path := p.DockerfilePath
		if path == "" {
			path = "Dockerfile"
		}
		return buildAndRun(name, path)
	}

	if p.Framework == "" {
		return errors.New("framework required when Dockerfile not provided")
	}

	err := GenerateDockerfile(p.Framework)
	if err != nil {
		return err
	}

	return buildAndRun(name, "Dockerfile")
}

func run(name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func buildAndRun(name, dockerfilePath string) error {
	err := run("docker", "build", "-f", dockerfilePath, "-t", name, ".")
	if err != nil {
		return fmt.Errorf("failed to build image: %w", err)
	}

	err = run("docker", "run", "-d", "--name", name, name)
	if err != nil {
		return fmt.Errorf("failed to run container: %w", err)
	}

	return nil
}

