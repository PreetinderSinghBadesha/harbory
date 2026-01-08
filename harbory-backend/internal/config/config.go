package config

import (
	"os"
)

type Config struct {
	HTTPServer HTTPServerConfig
	Auth       AuthConfig
}

type HTTPServerConfig struct {
	Addr string
}

type AuthConfig struct {
	Password string
}

func MustLoad() *Config {
	addr := os.Getenv("HTTP_ADDR")
	if addr == "" {
		addr = "0.0.0.0:8080"
	}

	password := os.Getenv("HARBORY_PASSWORD")
	if password == "" {
		password = "admin"
	}

	return &Config{
		HTTPServer: HTTPServerConfig{
			Addr: addr,
		},
		Auth: AuthConfig{
			Password: password,
		},
	}
}
