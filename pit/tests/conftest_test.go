package tests

import (
	"bufio"
	"context"
	"flag"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/smogork/ISBD-MIMUW/pit"
	apiclient "github.com/smogork/ISBD-MIMUW/pit/client/openapi2"
	"github.com/smogork/ISBD-MIMUW/pit/parser"
	"github.com/stretchr/testify/require"
)

// ============================================================================
// Global Configuration
// ============================================================================

var BaseURL string

// AsyncMode controls whether tests run in async mode (submit all, then wait)
var AsyncMode bool

// DbMemoryBytes is the available memory in the database for stress tests (in bytes)
// When set > 0, stress tests will try to sort 2x this amount of data
var DbMemoryBytes int64

// InterfaceVersion controls which interface version tests to run
// Version 1: Basic interface (Project 3) - COPY, basic SELECT, table operations
// Version 2: Extended interface (Project 4) - operators, functions, expressions, sorting
var InterfaceVersion int

// Command-line flags for test configuration
var (
	dbImage          = flag.String("db-image", "", "Docker image name (env: DB_IMAGE, default: isbd-mimuw-db:latest)")
	dbHostname       = flag.String("db-hostname", "", "Hostname of running database (env: DB_HOSTNAME, default: localhost)")
	dbPort           = flag.String("db-port", "", "Port on which database listens (env: DB_PORT, default: 8080)")
	dbRunDocker      = flag.String("db-run-docker", "", "Skip docker container and use existing database (env: DB_RUN_DOCKER, default: false)")
	asyncFlag        = flag.Bool("async", false, "Run tests in async mode: submit all queries first, then wait for results")
	dbMemoryFlag     = flag.Int64("db-memory", 0, "Database available memory in bytes for stress tests (0 = skip stress tests)")
	interfaceVerFlag = flag.Int("interface-version", 0, "Interface version to test: 1=Project3 (basic), 2=Project4 (extended), 0=all (env: INTERFACE_VERSION)")
)

// ============================================================================
// Test Result Tracking
// ============================================================================

// TestResult holds information about a test execution
type TestResult struct {
	Name       string
	Passed     bool
	FailureMsg string
}

// testResultTracker tracks all test results for summary
type testResultTracker struct {
	mu          sync.Mutex
	results     []TestResult
	failureMsgs map[string]string // testName -> failure message
}

var globalTestTracker = &testResultTracker{
	failureMsgs: make(map[string]string),
}

// RecordFailureMessage records a failure message for a test
func (tr *testResultTracker) RecordFailureMessage(testName, msg string) {
	tr.mu.Lock()
	defer tr.mu.Unlock()
	// Keep only the first failure message (usually most relevant)
	if _, exists := tr.failureMsgs[testName]; !exists {
		tr.failureMsgs[testName] = msg
	}
}

// RecordResult records a test result
func (tr *testResultTracker) RecordResult(name string, passed bool) {
	tr.mu.Lock()
	defer tr.mu.Unlock()
	result := TestResult{Name: name, Passed: passed}
	if !passed {
		if msg, exists := tr.failureMsgs[name]; exists {
			result.FailureMsg = msg
		}
	}
	tr.results = append(tr.results, result)
}

// GetFailedTests returns only the failed tests
func (tr *testResultTracker) GetFailedTests() []TestResult {
	tr.mu.Lock()
	defer tr.mu.Unlock()
	var failed []TestResult
	for _, r := range tr.results {
		if !r.Passed {
			failed = append(failed, r)
		}
	}
	return failed
}

// GetStats returns total, passed, and failed counts
func (tr *testResultTracker) GetStats() (total, passed, failed int) {
	tr.mu.Lock()
	defer tr.mu.Unlock()
	total = len(tr.results)
	for _, r := range tr.results {
		if r.Passed {
			passed++
		} else {
			failed++
		}
	}
	return total, passed, failed
}

// TrackTest registers a test for result tracking. Call at the start of each test.
// The test result will be recorded when the test completes.
func TrackTest(t *testing.T) {
	testName := t.Name()
	t.Cleanup(func() {
		globalTestTracker.RecordResult(testName, !t.Failed())
	})
}

// RunTracked is a wrapper around t.Run that automatically tracks test results.
// Use this instead of t.Run() for automatic result tracking.
func RunTracked(t *testing.T, name string, f func(t *testing.T)) bool {
	return t.Run(name, func(t *testing.T) {
		TrackTest(t)
		f(t)
	})
}

// FailWithMessage fails the test and records the message for summary.
// Use this instead of t.Fatalf/t.Errorf when you want the message in summary.
func FailWithMessage(t *testing.T, format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	globalTestTracker.RecordFailureMessage(t.Name(), msg)
	t.Fatal(msg)
}

// ErrorWithMessage marks test as failed and records the message for summary.
// Unlike FailWithMessage, this doesn't stop test execution immediately.
func ErrorWithMessage(t *testing.T, format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	globalTestTracker.RecordFailureMessage(t.Name(), msg)
	t.Error(msg)
}

// PrintTestSummary prints a summary of test results to stderr
func PrintTestSummary() {
	total, passed, failed := globalTestTracker.GetStats()

	fmt.Fprintln(os.Stderr)
	fmt.Fprintln(os.Stderr, "========================================")
	fmt.Fprintln(os.Stderr, "           TEST SUMMARY")
	fmt.Fprintln(os.Stderr, "========================================")
	fmt.Fprintf(os.Stderr, "Total: %d  |  Passed: %d  |  Failed: %d\n", total, passed, failed)
	fmt.Fprintln(os.Stderr, "----------------------------------------")

	if failed == 0 {
		fmt.Fprintln(os.Stderr, "✅ ALL TESTS PASSED")
	} else {
		fmt.Fprintln(os.Stderr, "❌ FAILED TESTS:")
		fmt.Fprintln(os.Stderr)

		failedTests := globalTestTracker.GetFailedTests()
		for i, test := range failedTests {
			fmt.Fprintf(os.Stderr, "  %d. %s\n", i+1, test.Name)
			if test.FailureMsg != "" {
				// Indent the failure message and handle multi-line
				lines := strings.Split(test.FailureMsg, "\n")
				for _, line := range lines {
					if line != "" {
						fmt.Fprintf(os.Stderr, "     → %s\n", line)
					}
				}
			}
		}
	}

	fmt.Fprintln(os.Stderr, "========================================")
	fmt.Fprintln(os.Stderr)
}

func applyFlagToEnv() {
	if *dbImage != "" {
		os.Setenv("DB_IMAGE", *dbImage)
	}
	if *dbHostname != "" {
		os.Setenv("DB_HOSTNAME", *dbHostname)
	}
	if *dbPort != "" {
		os.Setenv("DB_PORT", *dbPort)
	}
	if *dbRunDocker != "" {
		os.Setenv("DB_RUN_DOCKER", *dbRunDocker)
	}
}

func TestMain(m *testing.M) {
	flag.Parse()
	applyFlagToEnv()

	// Set AsyncMode from flag or environment variable
	AsyncMode = *asyncFlag
	if !AsyncMode {
		if envAsync := os.Getenv("TEST_ASYNC"); envAsync == "true" || envAsync == "1" {
			AsyncMode = true
		}
	}

	// Set DbMemoryBytes from flag or environment variable
	DbMemoryBytes = *dbMemoryFlag
	if DbMemoryBytes == 0 {
		if envMemory := os.Getenv("DB_MEMORY"); envMemory != "" {
			if parsed, err := strconv.ParseInt(envMemory, 10, 64); err == nil {
				DbMemoryBytes = parsed
			}
		}
	}

	// Set InterfaceVersion from flag or environment variable
	// 0 = run all tests, 1 = Project3 (basic), 2 = Project4 (extended)
	InterfaceVersion = *interfaceVerFlag
	if InterfaceVersion == 0 {
		if envVer := os.Getenv("INTERFACE_VERSION"); envVer != "" {
			if parsed, err := strconv.Atoi(envVer); err == nil {
				InterfaceVersion = parsed
			}
		}
	}

	ctx := context.Background()
	base, teardown, err := pit.StartTestContainer(ctx, DbMemoryBytes)
	if err != nil {
		fmt.Fprintln(os.Stderr, "failed to start test container:", err)
		os.Exit(1)
	}
	BaseURL = base

	code := m.Run()

	// Print test summary before teardown
	PrintTestSummary()

	if teardown != nil {
		teardown()
	}

	os.Exit(code)
}

// ============================================================================
// Interface Version Helpers
// ============================================================================

// RequireInterfaceVersion skips the test if the configured interface version
// doesn't match the required version. If InterfaceVersion is 0, all tests run.
func RequireInterfaceVersion(t *testing.T, requiredVersion int) {
	if InterfaceVersion != 0 && InterfaceVersion != requiredVersion {
		t.Skipf("Skipping test: requires interface version %d, but running version %d", requiredVersion, InterfaceVersion)
	}
}

// ============================================================================
// Table Schema Helpers
// ============================================================================

// ReadTableSchema reads a table schema from the tables directory
func ReadTableSchema(tableName string) (*apiclient.TableSchema, error) {
	schemaPath := filepath.Join("..", "tables", tableName, "schema.txt")
	file, err := os.Open(schemaPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open schema file: %w", err)
	}
	defer file.Close()

	var columns []apiclient.Column
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}

		parts := strings.Fields(line)
		if len(parts) != 2 {
			return nil, fmt.Errorf("invalid schema line: %s", line)
		}

		colName := parts[0]
		colType := apiclient.LogicalColumnType(parts[1])
		columns = append(columns, apiclient.Column{Name: colName, Type: colType})
	}

	sort.Slice(columns, func(i, j int) bool {
		return columns[i].Name < columns[j].Name
	})

	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("error reading schema file: %w", err)
	}

	return apiclient.NewTableSchema(tableName, columns), nil
}

// SetupTestTable creates a table and registers cleanup
func SetupTestTable(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableName string) string {
	schema, err := ReadTableSchema(tableName)
	require.NoError(t, err, "Failed to read schema for table %s", tableName)

	t.Log(pit.FormatRequest("PUT", "/table", schema))
	tableId, resp, err := apiClient.SchemaAPI.CreateTable(ctx).TableSchema(*schema).Execute()
	t.Log(pit.FormatResponse(resp))
	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)

	t.Cleanup(func() {
		t.Logf("Sending request:\nDELETE /table/%s", tableId)
		resp, err := apiClient.SchemaAPI.DeleteTable(ctx, tableId).Execute()
		t.Log(pit.FormatResponse(resp))
		// 404 is OK - table may have been deleted as part of the test
		if err != nil || (resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusNotFound) {
			t.Errorf("Cleanup failed: Could not delete table %s: %v", tableId, err)
		}
	})

	return tableId
}

// LoadTestData loads CSV data into a table using COPY query
func LoadTestData(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableName string) {
	// Use the container path where tables are mounted
	dataPath := fmt.Sprintf("/data/tables/%s/data.csv", tableName)

	copyQuery := &apiclient.CopyQuery{
		SourceFilepath:       dataPath,
		DestinationTableName: tableName,
	}
	doesContainHeader := true
	copyQuery.DoesCsvContainHeader = &doesContainHeader

	queryDef := apiclient.CopyQueryAsQueryQueryDefinition(copyQuery)
	req := apiclient.ExecuteQueryRequest{QueryDefinition: queryDef}

	t.Log(pit.FormatRequest("POST", "/query", copyQuery))
	queryId, resp, err := apiClient.ExecutionAPI.SubmitQuery(ctx).ExecuteQueryRequest(req).Execute()
	t.Log(pit.FormatResponse(resp))
	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)

	// Wait for COPY to complete
	_, err = WaitForQueryCompletion(apiClient, ctx, queryId, 30*time.Second)
	require.NoError(t, err, "COPY query should complete")
}

// ============================================================================
// Query Execution Helpers
// ============================================================================

// SubmitSelectQuery parses SQL and submits it, returning query ID
func SubmitSelectQuery(apiClient *apiclient.APIClient, ctx context.Context, sql string) (string, *http.Response, error) {
	selectQuery, err := parser.ParseSQL(sql)
	if err != nil {
		return "", nil, fmt.Errorf("failed to parse SQL: %w", err)
	}

	queryDef := apiclient.SelectQueryAsQueryQueryDefinition(selectQuery)
	req := apiclient.ExecuteQueryRequest{QueryDefinition: queryDef}

	fmt.Println(pit.FormatRequest("POST", "/query", selectQuery))
	return apiClient.ExecutionAPI.SubmitQuery(ctx).ExecuteQueryRequest(req).Execute()
}

// WaitForQueryCompletion waits for a query to reach COMPLETED or FAILED status
func WaitForQueryCompletion(apiClient *apiclient.APIClient, ctx context.Context, queryId string, timeout time.Duration) (*apiclient.Query, error) {
	deadline := time.Now().Add(timeout)

	for time.Now().Before(deadline) {
		query, _, err := apiClient.ExecutionAPI.GetQueryById(ctx, queryId).Execute()
		if err != nil {
			return nil, fmt.Errorf("failed to get query status: %w", err)
		}

		status := query.GetStatus()
		if status == apiclient.COMPLETED || status == apiclient.FAILED {
			// Log error details if query failed
			if status == apiclient.FAILED {
				if problems, err := GetQueryError(apiClient, ctx, queryId); err == nil && problems != nil {
					fmt.Printf("Query %s FAILED with errors:\n", queryId)
					for _, p := range problems.Problems {
						fmt.Printf("  - %s\n", p.Error)
					}
				}
			}
			return query, nil
		}

		time.Sleep(50 * time.Millisecond)
	}

	return nil, fmt.Errorf("timeout waiting for query %s to complete", queryId)
}

// WaitForQueryCompletion waits for a query to reach COMPLETED or FAILED status
// After completion, it flushes the query result to release database resources
func WaitForQueryCompletionWithFlush(apiClient *apiclient.APIClient, ctx context.Context, queryId string, timeout time.Duration) (*apiclient.Query, error) {
	deadline := time.Now().Add(timeout)

	for time.Now().Before(deadline) {
		query, _, err := apiClient.ExecutionAPI.GetQueryById(ctx, queryId).Execute()
		if err != nil {
			return nil, fmt.Errorf("failed to get query status: %w", err)
		}

		status := query.GetStatus()
		if status == apiclient.COMPLETED || status == apiclient.FAILED {
			// Log error details if query failed
			if status == apiclient.FAILED {
				if problems, err := GetQueryError(apiClient, ctx, queryId); err == nil && problems != nil {
					fmt.Printf("Query %s FAILED with errors:\n", queryId)
					for _, p := range problems.Problems {
						fmt.Printf("  - %s\n", p.Error)
					}
				}
			}

			// Flush result to release DB resources (ignore errors)
			flushReq := apiclient.GetQueryResultRequest{}
			flushReq.SetFlushResult(true)
			_, _, _ = apiClient.ExecutionAPI.GetQueryResult(ctx, queryId).GetQueryResultRequest(flushReq).Execute()

			return query, nil
		}

		time.Sleep(50 * time.Millisecond)
	}

	return nil, fmt.Errorf("timeout waiting for query %s to complete", queryId)
}

// GetQueryResult fetches the result of a completed query and flushes it
func GetQueryResult(apiClient *apiclient.APIClient, ctx context.Context, queryId string) ([]apiclient.QueryResultInner, error) {
	req := apiclient.GetQueryResultRequest{}
	req.SetFlushResult(true)
	result, _, err := apiClient.ExecutionAPI.GetQueryResult(ctx, queryId).GetQueryResultRequest(req).Execute()
	return result, err
}

// GetQueryError retrieves error details for a failed query
func GetQueryError(apiClient *apiclient.APIClient, ctx context.Context, queryId string) (*apiclient.MultipleProblemsError, error) {
	problems, _, err := apiClient.ExecutionAPI.GetQueryError(ctx, queryId).Execute()
	return problems, err
}
