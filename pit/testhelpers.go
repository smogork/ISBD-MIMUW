package pit

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"time"

	"github.com/docker/docker/api/types/container"
	"github.com/docker/go-connections/nat"
	openapi1 "github.com/smogork/ISBD-MIMUW/pit/client/openapi1"
	openapi2 "github.com/smogork/ISBD-MIMUW/pit/client/openapi2"
	tc "github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/wait"
)

// DbClient1 creates an API client for interface version 1 (Project 3)
func DbClient1(url string) *openapi1.APIClient {
	cfg := openapi1.NewConfiguration()
	cfg.Servers[0].URL = url
	cfg.HTTPClient = &http.Client{}
	return openapi1.NewAPIClient(cfg)
}

// DbClient2 creates an API client for interface version 2 (Project 4)
func DbClient2(url string) *openapi2.APIClient {
	cfg := openapi2.NewConfiguration()
	cfg.Servers[0].URL = url
	cfg.HTTPClient = &http.Client{}
	return openapi2.NewAPIClient(cfg)
}

// DbClient is an alias for DbClient2 for backward compatibility
func DbClient(url string) *openapi2.APIClient {
	return DbClient2(url)
}

func waitForHTTP(url string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	client := &http.Client{}
	for time.Now().Before(deadline) {
		req, _ := http.NewRequestWithContext(context.Background(), http.MethodGet, url, nil)
		resp, err := client.Do(req)
		if err == nil {
			io.Copy(io.Discard, resp.Body)
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return nil
			}
		}
		time.Sleep(500 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for %s", url)
}

// StartTestContainer starts the test container (unless DB_RUN_DOCKER is set), waits for
// it to become healthy and returns the base URL and a teardown function.
//
// Configuration via environment variables:
// - DB_RUN_DOCKER: if set to anything other than "false", skip docker and connect to existing database
// - DB_IMAGE: docker image name (default: "isbd-mimuw-db:latest")
// - DB_HOSTNAME: hostname of running database (default: "localhost")
// - DB_PORT: port on which database listens (default: "8080")
func StartTestContainer(ctx context.Context, dbMemoryBytes int64) (string, func(), error) {
	dbRunDocker := os.Getenv("DB_RUN_DOCKER")
	if dbRunDocker == "" {
		dbRunDocker = "false" // By default use already running DB
	}

	if dbRunDocker == "false" {
		// Connect to existing database without starting container
		hostname := os.Getenv("DB_HOSTNAME")
		if hostname == "" {
			hostname = "localhost"
		}
		port := os.Getenv("DB_PORT")
		if port == "" {
			port = "8080"
		}

		base := fmt.Sprintf("http://%s:%s", hostname, port)
		if err := waitForHTTP(base+"/system/info", 30*time.Second); err != nil {
			return "", nil, err
		}
		return base, func() {}, nil
	}

	// Start docker container
	image := os.Getenv("DB_IMAGE")
	if image == "" {
		image = "isbd-mimuw-db:latest"
	}

	port := os.Getenv("DB_PORT")
	if port == "" {
		port = "8080"
	}

	// Get absolute path to tables directory for bind mount
	// The tables directory is at pit/tables, relative to where go test runs (pit/tests)
	tablesDir, err := filepath.Abs(filepath.Join("..", "tables"))
	if err != nil {
		return "", nil, fmt.Errorf("failed to get tables directory path: %w", err)
	}

	portSpec := port + "/tcp"
	req := tc.ContainerRequest{
		Image:        image,
		ExposedPorts: []string{portSpec},
		WaitingFor:   wait.ForHTTP("/system/info").WithPort(nat.Port(portSpec)).WithStartupTimeout(30 * time.Second),
		Mounts: tc.Mounts(
			tc.BindMount(tablesDir, "/data/tables"),
		),
		// Use cgroups to limit accessible memory
		HostConfigModifier: func(hc *container.HostConfig) {
			if dbMemoryBytes > 0 {
				// Set memory limit and disallow swap (set swap == memory) so OOM kills the process
				hc.Resources.Memory = dbMemoryBytes
				hc.Resources.MemorySwap = dbMemoryBytes
			}
		},
	}

	cont, err := tc.GenericContainer(ctx, tc.GenericContainerRequest{ContainerRequest: req, Started: true})
	if err != nil {
		return "", nil, err
	}

	host, err := cont.Host(ctx)
	if err != nil {
		_ = cont.Terminate(ctx)
		return "", nil, err
	}
	mp, err := cont.MappedPort(ctx, nat.Port(portSpec))
	if err != nil {
		_ = cont.Terminate(ctx)
		return "", nil, err
	}
	base := fmt.Sprintf("http://%s:%s", host, mp.Port())

	// Wait for system info to be available (uses waitForHTTP defined in this package)
	if err := waitForHTTP(base+"/system/info", 30*time.Second); err != nil {
		_ = cont.Terminate(ctx)
		return "", nil, err
	}

	teardown := func() { _ = cont.Terminate(ctx) }
	return base, teardown, nil
}
