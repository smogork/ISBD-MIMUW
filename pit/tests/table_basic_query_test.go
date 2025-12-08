package tests

import (
	"context"
	"errors"
	"net/http"
	"path/filepath"
	"testing"
	"time"

	"github.com/smogork/ISBD-MIMUW/pit"
	apiclient "github.com/smogork/ISBD-MIMUW/pit/client"
	"github.com/stretchr/testify/require"
)

func getQueryErrors(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, queryId string) (*apiclient.MultipleProblemsError, *http.Response, error) {
	results, resp, err := apiClient.ExecutionAPI.
		GetQueryError(ctx, queryId).
		Execute()

	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)

	return results, resp, err
}

func waitForQueryToFail(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, queryId string) (*apiclient.Query, *http.Response, error) {
	for range 100 {
		query, resp, err := apiClient.ExecutionAPI.GetQueryById(ctx, queryId).Execute()

		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)

		if query.Status == apiclient.FAILED {
			t.Log(pit.FormatResponse(resp))
			_, resp, _ := getQueryErrors(t, apiClient, ctx, queryId)
			t.Log(pit.FormatResponse(resp))
			return query, resp, err
		} else if query.Status == apiclient.COMPLETED {
			t.Log(pit.FormatResponse(resp))
			require.Fail(t, "query completed, but was supposed to fail")
			return query, resp, err
		}

		time.Sleep(100 * time.Millisecond)
	}

	require.Fail(t, "query timeout")
	return nil, nil, errors.New("query timeout")
}

func waitForQueryToComplete(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, queryId string) (*apiclient.Query, *http.Response, error) {
	for range 100 {
		query, resp, err := apiClient.ExecutionAPI.GetQueryById(ctx, queryId).Execute()

		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)

		if query.Status == apiclient.FAILED {
			t.Log(pit.FormatResponse(resp))
			require.Fail(t, "query failed")
			return query, resp, err
		} else if query.Status == apiclient.COMPLETED {
			t.Log(pit.FormatResponse(resp))
			return query, resp, err
		}

		time.Sleep(100 * time.Millisecond)
	}

	require.Fail(t, "query timeout")
	return nil, nil, errors.New("query timeout")
}

func getQueryResults(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, queryId string, rowLimit *int32, flushResult *bool) ([]apiclient.QueryResultInner, *http.Response, error) {
	queryResultRequest := apiclient.NewGetQueryResultRequest()

	if rowLimit != nil {
		queryResultRequest.SetRowLimit(*rowLimit)
	}

	if flushResult != nil {
		queryResultRequest.SetFlushResult(*flushResult)
	}

	results, resp, err := apiClient.ExecutionAPI.
		GetQueryResult(ctx, queryId).
		GetQueryResultRequest(*queryResultRequest).
		Execute()

	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)

	return results, resp, err
}

func copyData(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableId string, filename string, hasHeader bool, columnOrder []string) (string, *http.Response, error) {
	copyQuery := apiclient.NewCopyQuery(filename, tableId)
	copyQuery.SetDoesCsvContainHeader(hasHeader)
	copyQuery.SetDestinationColumns(columnOrder)

	queryId, resp, err := apiClient.ExecutionAPI.SubmitQuery(ctx).
		ExecuteQueryRequest(
			*apiclient.NewExecuteQueryRequest(
				apiclient.CopyQueryAsQueryQueryDefinition(copyQuery),
			),
		).
		Execute()

	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)

	return queryId, resp, err
}

func selectData(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableId string) (string, *http.Response, error) {
	selectQuery := apiclient.NewSelectQuery()
	selectQuery.SetTableName(tableId)

	queryId, resp, err := apiClient.ExecutionAPI.SubmitQuery(ctx).
		ExecuteQueryRequest(
			*apiclient.NewExecuteQueryRequest(
				apiclient.SelectQueryAsQueryQueryDefinition(selectQuery),
			),
		).
		Execute()

	require.NoError(t, err)
	require.Equal(t, http.StatusOK, resp.StatusCode)

	return queryId, resp, err
}

func TestBasicQueries(t *testing.T) {
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()

	// Read the people table schema from file
	peopleSchema, err := readPeopleSchema()
	require.NoError(t, err)

	csvColumnOrder := []string{"id", "name", "surname", "age"}

	filePathWithoutHeader, err := filepath.Abs(filepath.Join("..", "tables", "people", "data_without_headers.csv"))
	require.NoError(t, err)

	filePathWithHeader, err := filepath.Abs(filepath.Join("..", "tables", "people", "data.csv"))
	require.NoError(t, err)

	filePathLong, err := filepath.Abs(filepath.Join("..", "tables", "people", "data_long.csv"))
	require.NoError(t, err)

	filePathTooManyColumns, err := filepath.Abs(filepath.Join("..", "tables", "people", "data_too_many_columns.csv"))
	require.NoError(t, err)

	t.Run("TableCopyWithoutHeaderAndSelect", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathWithoutHeader, false, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(5))
		// TODO verify rest of the result
	})

	t.Run("TableCopyWithHeaderAndSelect", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathWithHeader, true, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(5))
		// TODO verify rest of the result
	})

	t.Run("TableCopyAndSelectWithLimit", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		var rowLimit int32 = 1000
		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, &rowLimit, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(1000))
		// TODO verify rest of the result
	})

	t.Run("TableCopyTooManyColumns", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathTooManyColumns, true, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(5))
		// TODO verify rest of the result
	})

	t.Run("TableCopyWithNonexistentFile", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, "/does_not_exit.csv", false, csvColumnOrder)
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyWithUnknownOrderColumns", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false,
			[]string{"id", "name", "surname", "unknown"})
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyWithTooLongOrder", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false,
			[]string{"id", "name", "surname", "age", "unknown"})
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyWithTooShortOrder", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false,
			[]string{"id", "name", "surname"})
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyWithDuplicatedOrder", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false,
			[]string{"id", "name", "surname", "surname"})
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyWithNonexistentTable", func(t *testing.T) {
		queryId, _, _ := copyData(t, dbClient, ctx, "unknown", filePathLong, false,
			[]string{"id", "name", "surname", "surname"})
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyWithWrongTypesInOrder", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false,
			[]string{"name", "id", "surname", "surname"})
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
	})

	t.Run("TableCopyDeleteRecreate", func(t *testing.T) {
	})
}
