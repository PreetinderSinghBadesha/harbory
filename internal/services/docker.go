package services

import (
	"context"
	"fmt"
	"io"
	"os"

	"github.com/docker/docker/api/types"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/api/types/image"
	"github.com/docker/docker/client"
	"github.com/docker/go-connections/nat"
)

type DockerService struct {
	cli *client.Client
}

func NewDockerService() (*DockerService, error) {
	cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		return nil, err
	}
	return &DockerService{cli: cli}, nil
}

func (s *DockerService) ListContainers() ([]types.Container, error) {
	return s.cli.ContainerList(context.Background(), container.ListOptions{All: true})
}

func (s *DockerService) CreateContainer(imageName, containerName, hostPort, containerPort string) error {
	ctx := context.Background()

	// Pull the image first
	reader, err := s.cli.ImagePull(ctx, imageName, image.PullOptions{})
	if err != nil {
		return fmt.Errorf("failed to pull image: %w", err)
	}
	io.Copy(os.Stdout, reader) // Optional: pipe output to stdout

	// Configure port binding
	portBinding := nat.PortBinding{
		HostIP:   "0.0.0.0",
		HostPort: hostPort,
	}
	containerPortNat, err := nat.NewPort("tcp", containerPort)
	if err != nil {
		return fmt.Errorf("invalid container port: %w", err)
	}

	portMap := nat.PortMap{
		containerPortNat: []nat.PortBinding{portBinding},
	}

	resp, err := s.cli.ContainerCreate(ctx, &container.Config{
		Image: imageName,
		Tty:   true,
	}, &container.HostConfig{
		PortBindings: portMap,
	}, nil, nil, containerName)

	if err != nil {
		return fmt.Errorf("failed to create container: %w", err)
	}

	if err := s.cli.ContainerStart(ctx, resp.ID, container.StartOptions{}); err != nil {
		return fmt.Errorf("failed to start container: %w", err)
	}

	return nil
}
