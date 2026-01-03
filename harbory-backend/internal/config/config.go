package config

import (
	"os"
)

type Config struct {
	HTTPServer HTTPServerConfig
}

type HTTPServerConfig struct {
	Addr string
}

func MustLoad() *Config {
	addr := os.Getenv("HTTP_ADDR")
	if addr == "" {
		addr = "0.0.0.0:8080"
	}

	return &Config{
		HTTPServer: HTTPServerConfig{
			Addr: addr,
		},
	}
}
