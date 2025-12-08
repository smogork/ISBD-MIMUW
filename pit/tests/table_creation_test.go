package tests

import (
	"bufio"
	"context"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	"github.com/smogork/ISBD-MIMUW/pit"
	apiclient "github.com/smogork/ISBD-MIMUW/pit/client"
	"github.com/stretchr/testify/require"
)

func readPeopleSchema() (*apiclient.TableSchema, error) {
	// Get the path to the schema file
	schemaPath := filepath.Join("..", "tables", "people", "schema.txt")
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

	return apiclient.NewTableSchema("people", columns), nil
}

func createTable(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableSchema *apiclient.TableSchema, mayFail bool) (string, *http.Response, error) {
	tableId, resp, err := apiClient.SchemaAPI.CreateTable(ctx).TableSchema(*tableSchema).Execute()
	t.Log(pit.FormatResponse(resp))
	if !mayFail {
		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)
	}
	return tableId[1 : len(tableId)-1], resp, err
}

func deleteTable(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableId string, mayFail bool) (*http.Response, error) {
	resp, err := apiClient.SchemaAPI.DeleteTable(ctx, tableId).Execute()
	t.Log(pit.FormatResponse(resp))
	if !mayFail {
		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)
	}
	return resp, err
}

func createTableWithCleanup(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, schema *apiclient.TableSchema) string {
	// Create table
	tableId, resp, err := apiClient.SchemaAPI.CreateTable(ctx).TableSchema(*schema).Execute()

	t.Log(pit.FormatResponse(resp))

	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)
	require.NotEmpty(t, tableId)

	t.Cleanup(func() {
		resp, err := apiClient.SchemaAPI.DeleteTable(ctx, tableId).Execute()

		t.Log(pit.FormatResponse(resp))

		// Log errof instead of panic
		if err != nil || resp.StatusCode != http.StatusOK {
			t.Errorf("Cleanup failed: Could not delete table %s: %v, status: %d", tableId, err, resp.StatusCode)
		}
	})

	return tableId[1 : len(tableId)-1]
}

func TestTableCreation(t *testing.T) {
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()

	// Read the people table schema from file
	peopleSchema, err := readPeopleSchema()
	require.NoError(t, err)

	t.Run("TableEmptyList", func(t *testing.T) {
		tables, resp, err := dbClient.SchemaAPI.GetTables(ctx).Execute()
		t.Log(pit.FormatResponse(resp))
		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)
		require.Len(t, tables, 0)
	})

	t.Run("TableCreationAndList", func(t *testing.T) {
		// Create the people table
		_ = createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		// List tables and verify the table exists
		tables, resp, err := dbClient.SchemaAPI.GetTables(ctx).Execute()
		t.Log(pit.FormatResponse(resp))
		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)
		require.Len(t, tables, 1)
		require.Equal(t, "people", tables[0].Name)
	})

	t.Run("TableCreationAndDetails", func(t *testing.T) {
		// Create the people table
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		// Get table details
		details, resp, err := dbClient.SchemaAPI.GetTableById(ctx, tableId).Execute()
		t.Log(pit.FormatResponse(resp))
		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)
		require.Equal(t, "people", details.Name)
		require.Len(t, details.Columns, 4)

		// Sort columns by name to make sure this will relaly be equal
		sort.Slice(details.Columns, func(i, j int) bool {
			return details.Columns[i].Name < details.Columns[j].Name
		})
		require.Equal(t, *peopleSchema, *details)
	})

	t.Run("TableDoubleCreation", func(t *testing.T) {
		// Create the people table
		_ = createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		peopleSchema2, err := readPeopleSchema()
		require.NoError(t, err)
		peopleSchema2.Name = "people2"

		// Create another table with the same schema but different name - should succeed
		_ = createTableWithCleanup(t, dbClient, ctx, peopleSchema2)
	})

	t.Run("TableDoubleNameCreation", func(t *testing.T) {
		// Create the people table
		_ = createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		// Try to create a table with the same name - should fail
		_, resp, _ := createTable(t, dbClient, ctx, peopleSchema, true)
		t.Log(pit.FormatResponse(resp))
		require.Equal(t, http.StatusBadRequest, resp.StatusCode)
	})

	t.Run("TableDoubleRemove", func(t *testing.T) {
		// Create the people table
		tableId, _, _ := createTable(t, dbClient, ctx, peopleSchema, false)

		// Delete the table - should succeed
		_, _ = deleteTable(t, dbClient, ctx, tableId, false)

		// Try to delete the table again - should fail
		resp, _ := dbClient.SchemaAPI.DeleteTable(ctx, tableId).Execute()
		t.Log(pit.FormatResponse(resp))
		require.Equal(t, http.StatusNotFound, resp.StatusCode)
	})

	t.Run("TableNoDetailsAfterRemoval", func(t *testing.T) {
		// Create the people table
		tableId, _, _ := createTable(t, dbClient, ctx, peopleSchema, false)

		// Delete the table - should succeed
		_, _ = deleteTable(t, dbClient, ctx, tableId, false)

		// Try to get details of deleted table - should fail
		_, resp, _ := dbClient.SchemaAPI.GetTableById(ctx, tableId).Execute()
		t.Log(pit.FormatResponse(resp))
		require.Equal(t, http.StatusNotFound, resp.StatusCode)
	})

	t.Run("TableCreateDuplicateColumns", func(t *testing.T) {
		var schema = apiclient.NewTableSchema("duplicated", []apiclient.Column{
			apiclient.Column{Name: "test", Type: apiclient.INT64},
			apiclient.Column{Name: "test2", Type: apiclient.INT64},
			apiclient.Column{Name: "test2", Type: apiclient.INT64},
		})

		_, resp, _ := createTable(t, dbClient, ctx, schema, true)
		require.Equal(t, http.StatusBadRequest, resp.StatusCode)
		t.Log(pit.FormatResponse(resp))
	})

	t.Run("TableCreateNoColumns", func(t *testing.T) {
		var schema = apiclient.NewTableSchema("empty", []apiclient.Column{})

		_, resp, _ := createTable(t, dbClient, ctx, schema, true)
		require.Equal(t, http.StatusBadRequest, resp.StatusCode)
		t.Log(pit.FormatResponse(resp))
	})

	t.Run("TableDeleteNonExistent", func(t *testing.T) {
		resp, _ := deleteTable(t, dbClient, ctx, "test", true)
		require.Equal(t, http.StatusNotFound, resp.StatusCode, true)
		t.Log(pit.FormatResponse(resp))
	})
}
