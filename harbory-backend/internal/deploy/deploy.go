package deploy

import (
	"bufio"
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
	return DeployFromPayloadWithProgress(p, nil)
}

func DeployFromPayloadWithProgress(p DeployPayload, logChan chan<- string) error {
	sendLog := func(msg string) {
		if logChan != nil {
			logChan <- msg
		}
	}

	repo := strings.Split(p.RepoUrl, "/")
	name := strings.TrimSuffix(repo[len(repo)-1], ".git")

	deployDir := fmt.Sprintf("/tmp/harbory-deploy-%s", name)
	sendLog(fmt.Sprintf("Creating deployment directory: %s", deployDir))

	os.RemoveAll(deployDir)
	if err := os.MkdirAll(deployDir, 0755); err != nil {
		return fmt.Errorf("failed to create deployment directory: %w", err)
	}

	originalDir, _ := os.Getwd()
	defer os.Chdir(originalDir)

	if err := os.Chdir(deployDir); err != nil {
		return fmt.Errorf("failed to change to deployment directory: %w", err)
	}

	sendLog(fmt.Sprintf("Cloning repository: %s", p.RepoUrl))
	if err := runWithProgress("git", logChan, "clone", p.RepoUrl); err != nil {
		return fmt.Errorf("failed to clone repository: %w", err)
	}

	sendLog(fmt.Sprintf("Changing directory to: %s", name))
	repoPath := fmt.Sprintf("%s/%s", deployDir, name)
	if err := os.Chdir(repoPath); err != nil {
		return fmt.Errorf("failed to change directory: %w", err)
	}

	if p.HasDockerfile {
		path := p.DockerfilePath
		if path == "" {
			path = "Dockerfile"
		}
		sendLog(fmt.Sprintf("Using existing Dockerfile: %s", path))
		return buildAndRunWithProgress(name, path, logChan)
	}

	if p.Framework == "" {
		return errors.New("framework required when Dockerfile not provided")
	}

	sendLog(fmt.Sprintf("Generating Dockerfile for framework: %s", p.Framework))
	err := GenerateDockerfile(p.Framework)
	if err != nil {
		return err
	}

	return buildAndRunWithProgress(name, "Dockerfile", logChan)
}

func run(name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func runWithProgress(name string, logChan chan<- string, args ...string) error {
	cmd := exec.Command(name, args...)
	
	if logChan != nil {
		stdout, _ := cmd.StdoutPipe()
		stderr, _ := cmd.StderrPipe()
		
		if err := cmd.Start(); err != nil {
			return err
		}

		go func() {
			scanner := bufio.NewScanner(stdout)
			for scanner.Scan() {
				logChan <- scanner.Text()
			}
		}()

		go func() {
			scanner := bufio.NewScanner(stderr)
			for scanner.Scan() {
				logChan <- scanner.Text()
			}
		}()
		
		return cmd.Wait()
	}
	
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

func buildAndRunWithProgress(name, dockerfilePath string, logChan chan<- string) error {
	sendLog := func(msg string) {
		if logChan != nil {
			logChan <- msg
		}
	}

	sendLog(fmt.Sprintf("Checking for existing container: %s", name))
	exec.Command("docker", "rm", "-f", name).Run()


	sendLog(fmt.Sprintf("Cleaning up old images..."))
	exec.Command("docker", "rmi", "-f", name).Run()

	sendLog(fmt.Sprintf("Building Docker image: %s", name))
	err := runWithProgress("docker", logChan, "build", "-f", dockerfilePath, "-t", name, ".")
	if err != nil {
		return fmt.Errorf("failed to build image: %w", err)
	}

	sendLog(fmt.Sprintf("Detecting exposed ports..."))
	ports := detectExposedPorts(name, logChan)
	
	runArgs := []string{"run", "-d", "--name", name}
	
	for _, port := range ports {
		runArgs = append(runArgs, "-p", fmt.Sprintf("%s:%s", port, port))
		sendLog(fmt.Sprintf("Mapping port: %s -> %s", port, port))
	}
	
	runArgs = append(runArgs, name)

	sendLog(fmt.Sprintf("Starting container: %s", name))
	err = runWithProgress("docker", logChan, runArgs...)
	if err != nil {
		return fmt.Errorf("failed to run container: %w", err)
	}

	sendLog(fmt.Sprintf("Container %s is now running!", name))
	if len(ports) > 0 {
		sendLog(fmt.Sprintf("Access your application at: http://localhost:%s", ports[0]))
	}
	return nil
}

func detectExposedPorts(imageName string, logChan chan<- string) []string {
	cmd := exec.Command("docker", "inspect", "--format", "{{json .Config.ExposedPorts}}", imageName)
	output, err := cmd.Output()
	if err != nil {
		return []string{}
	}

	outputStr := string(output)
	ports := []string{}

	for i := 0; i < len(outputStr); i++ {
		if outputStr[i] >= '0' && outputStr[i] <= '9' {
			port := ""
			for i < len(outputStr) && outputStr[i] >= '0' && outputStr[i] <= '9' {
				port += string(outputStr[i])
				i++
			}
			if port != "" {
				ports = append(ports, port)
			}
		}
	}
	
	return ports
}

