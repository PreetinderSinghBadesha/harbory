package handler

import (
	"context"
	"net/http"
	"runtime"
	"syscall"

	"github.com/PreetinderSinghBadesha/harbory/internal/utils/response"
	"github.com/docker/docker/client"
)

type SystemStats struct {
	CPU    CPUStats    `json:"cpu"`
	Memory MemoryStats `json:"memory"`
	Disk   DiskStats   `json:"disk"`
	Docker DockerStats `json:"docker"`
	System SystemInfo  `json:"system"`
}

type CPUStats struct {
	Cores      int     `json:"cores"`
	Goroutines int     `json:"goroutines"`
	Usage      float64 `json:"usage_percent,omitempty"`
}

type MemoryStats struct {
	Total       uint64  `json:"total_bytes"`
	Used        uint64  `json:"used_bytes"`
	Free        uint64  `json:"free_bytes"`
	UsagePercent float64 `json:"usage_percent"`
	TotalMB     uint64  `json:"total_mb"`
	UsedMB      uint64  `json:"used_mb"`
	FreeMB      uint64  `json:"free_mb"`
}

type DiskStats struct {
	Total       uint64  `json:"total_bytes"`
	Used        uint64  `json:"used_bytes"`
	Free        uint64  `json:"free_bytes"`
	UsagePercent float64 `json:"usage_percent"`
	TotalGB     uint64  `json:"total_gb"`
	UsedGB      uint64  `json:"used_gb"`
	FreeGB      uint64  `json:"free_gb"`
}

type DockerStats struct {
	Containers      int    `json:"containers_total"`
	ContainersRunning int  `json:"containers_running"`
	Images          int    `json:"images"`
	Version         string `json:"version"`
	ServerVersion   string `json:"server_version"`
}

type SystemInfo struct {
	OS           string `json:"os"`
	Architecture string `json:"architecture"`
	Hostname     string `json:"hostname,omitempty"`
}

func GetSystemStatsHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := context.Background()		
		cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())

		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		defer cli.Close()

		stats := SystemStats{}
		stats.CPU = getCPUStats()
		memStats, err := getMemoryStats()

		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		stats.Memory = memStats
		diskStats, err := getDiskStats()

		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}

		stats.Disk = diskStats
		dockerStats, err := getDockerStats(ctx, cli)

		if err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
		stats.Docker = dockerStats
		stats.System = getSystemInfo()

		if err := response.WriteJSONResponse(w, http.StatusOK, stats); err != nil {
			errorResp := response.GeneralErrorResponse(err)
			_ = response.WriteJSONResponse(w, http.StatusInternalServerError, errorResp)
			return
		}
	}
}

func getCPUStats() CPUStats {
	return CPUStats{
		Cores:      runtime.NumCPU(),
		Goroutines: runtime.NumGoroutine(),
	}
}

func getMemoryStats() (MemoryStats, error) {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)

	var si syscall.Sysinfo_t
	err := syscall.Sysinfo(&si)
	if err != nil {
		return MemoryStats{}, err
	}

	total := si.Totalram * uint64(si.Unit)
	free := si.Freeram * uint64(si.Unit)
	used := total - free
	usagePercent := float64(used) / float64(total) * 100

	return MemoryStats{
		Total:        total,
		Used:         used,
		Free:         free,
		UsagePercent: usagePercent,
		TotalMB:      total / 1024 / 1024,
		UsedMB:       used / 1024 / 1024,
		FreeMB:       free / 1024 / 1024,
	}, nil
}

func getDiskStats() (DiskStats, error) {
	var stat syscall.Statfs_t
	
	err := syscall.Statfs("/", &stat)
	if err != nil {
		return DiskStats{}, err
	}

	total := stat.Blocks * uint64(stat.Bsize)
	free := stat.Bavail * uint64(stat.Bsize)
	used := total - free
	usagePercent := float64(used) / float64(total) * 100

	return DiskStats{
		Total:        total,
		Used:         used,
		Free:         free,
		UsagePercent: usagePercent,
		TotalGB:      total / 1024 / 1024 / 1024,
		UsedGB:       used / 1024 / 1024 / 1024,
		FreeGB:       free / 1024 / 1024 / 1024,
	}, nil
}

func getDockerStats(ctx context.Context, cli *client.Client) (DockerStats, error) {
	info, err := cli.Info(ctx)
	if err != nil {
		return DockerStats{}, err
	}

	version, err := cli.ServerVersion(ctx)
	if err != nil {
		return DockerStats{}, err
	}

	return DockerStats{
		Containers:        info.Containers,
		ContainersRunning: info.ContainersRunning,
		Images:            info.Images,
		Version:           version.Version,
		ServerVersion:     version.APIVersion,
	}, nil
}

func getSystemInfo() SystemInfo {
	return SystemInfo{
		OS:           runtime.GOOS,
		Architecture: runtime.GOARCH,
	}
}
