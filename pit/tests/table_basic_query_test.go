package tests

import (
	"context"
	"encoding/csv"
	"errors"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"testing"
	"time"

	"github.com/smogork/ISBD-MIMUW/pit"
	apiclient "github.com/smogork/ISBD-MIMUW/pit/client"
	"github.com/stretchr/testify/require"
)

func verifyPeopleCsv(t *testing.T, csv_path string, results apiclient.QueryResultInner, hasHeader bool, csv_col_num_to_schema_col_num []int, rowLimit *int32) {
	f, err := os.Open(csv_path)
	require.NoError(t, err, "failed to read csv file")
	defer f.Close()

	colLen := 0

	if rowLimit != nil {
		for _, column := range results.Columns {
			if column.ArrayOfInt64 != nil {
				require.Equal(t, len(*column.ArrayOfInt64), int(*rowLimit), "int column len does not equal row limit")
			} else {
				require.Equal(t, len(*column.ArrayOfString), int(*rowLimit), "str column len does not equal row limit")
			}
		}

		colLen = int(*rowLimit)
	} else {
		if results.Columns[0].ArrayOfInt64 != nil {
			colLen = len(*results.Columns[0].ArrayOfInt64)
		} else {
			colLen = len(*results.Columns[0].ArrayOfString)
		}

		for _, column := range results.Columns[1:] {
			if column.ArrayOfInt64 != nil {
				require.Equal(t, len(*column.ArrayOfInt64), colLen, "column length differs between columns")
			} else {
				require.Equal(t, len(*column.ArrayOfString), colLen, "column length differs between columns")
			}
		}
	}

	reader := *csv.NewReader(f)
	reader.Comma = ';'
	// skip header
	if hasHeader {
		_, err = reader.Read()
		require.NoError(t, err, "faied to read csv header")
	}

	i := 0
	for {
		record, err := reader.Read()
		if err == io.EOF {
			break
		} else if err != nil {
			require.NoError(t, err, "failed to read csv row")
		}

		for col_num, col_csv := range record {
			if col_num >= len(results.Columns) {
				break
			}
			column := results.Columns[csv_col_num_to_schema_col_num[col_num]]
			if column.ArrayOfInt64 != nil {
				csv_field, err := strconv.ParseInt(col_csv, 10, 64)
				require.NoError(t, err, "failed to parse csv field as int")
				require.Equal(t, (*column.ArrayOfInt64)[i], csv_field, "data differs from source csv col %d row %d", col_num, i)
			} else {
				require.Equal(t, (*column.ArrayOfString)[i], col_csv, "data differs from source csv col %d row %d", col_num, i)
			}
		}

		i += 1
		if rowLimit != nil && i >= int(*rowLimit) {
			break
		}
	}

	require.Equal(t, i, colLen, "csv len != min(rowLimit, column length) %d %d", i, colLen)
}

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

func selectData(t *testing.T, apiClient *apiclient.APIClient, ctx context.Context, tableId string, mayFail bool) (string, *http.Response, error) {
	selectQuery := apiclient.NewSelectQuery()
	selectQuery.SetTableName(tableId)

	queryId, resp, err := apiClient.ExecutionAPI.SubmitQuery(ctx).
		ExecuteQueryRequest(
			*apiclient.NewExecuteQueryRequest(
				apiclient.SelectQueryAsQueryQueryDefinition(selectQuery),
			),
		).
		Execute()

	if !mayFail {
		require.NoError(t, err)
		require.Equal(t, http.StatusOK, resp.StatusCode)
	}

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

		queryId, _, _ = selectData(t, dbClient, ctx, tableId, false)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(5))

		// NOTE: tests sort columns in schema before creating table
		csv_col_num_to_schema_col_num := []int{1, 2, 3, 0}
		verifyPeopleCsv(t, filePathWithoutHeader, results[0], false, csv_col_num_to_schema_col_num, nil)
	})

	t.Run("TableCopyWithHeaderAndSelect", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathWithHeader, true, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId, false)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(5))

		// NOTE: tests sort columns in schema before creating table
		csv_col_num_to_schema_col_num := []int{1, 2, 3, 0}
		verifyPeopleCsv(t, filePathWithHeader, results[0], true, csv_col_num_to_schema_col_num, nil)
	})

	// TODO what should happen with file with headers and without order?

	t.Run("TableCopyAndSelectWithLimit", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId, false)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		var rowLimit int32 = 1000
		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, &rowLimit, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(rowLimit))

		// NOTE: tests sort columns in schema before creating table
		csv_col_num_to_schema_col_num := []int{1, 2, 3, 0}
		verifyPeopleCsv(t, filePathLong, results[0], false, csv_col_num_to_schema_col_num, &rowLimit)
	})

	t.Run("TableCopyTooManyColumns", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathTooManyColumns, true, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		queryId, _, _ = selectData(t, dbClient, ctx, tableId, false)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		// TODO what's the point of results array?
		require.Equal(t, *results[0].RowCount, int32(5))
		csv_col_num_to_schema_col_num := []int{1, 2, 3, 0}
		verifyPeopleCsv(t, filePathTooManyColumns, results[0], true, csv_col_num_to_schema_col_num, nil)
	})

	t.Run("TableCopyTooManyColumnsWithoutOrder", func(t *testing.T) {
		tableId := createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathTooManyColumns, true, nil)
		_, _, _ = waitForQueryToFail(t, dbClient, ctx, queryId)
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
		tableId, _, _ := createTable(t, dbClient, ctx, peopleSchema, false)

		queryId, _, _ := copyData(t, dbClient, ctx, tableId, filePathLong, false, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		// select data and check if row count matches
		queryId, _, _ = selectData(t, dbClient, ctx, tableId, false)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ := getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		require.Equal(t, *results[0].RowCount, int32(1000000))
		csv_col_num_to_schema_col_num := []int{1, 2, 3, 0}
		verifyPeopleCsv(t, filePathLong, results[0], false, csv_col_num_to_schema_col_num, nil)

		resp, _ := deleteTable(t, dbClient, ctx, tableId, true)
		require.Equal(t, http.StatusOK, resp.StatusCode, true)
		t.Log(pit.FormatResponse(resp))

		// create table again
		tableId = createTableWithCleanup(t, dbClient, ctx, peopleSchema)

		queryId, _, _ = copyData(t, dbClient, ctx, tableId, filePathLong, false, csvColumnOrder)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		// select data and check if row count matches on the new table
		queryId, _, _ = selectData(t, dbClient, ctx, tableId, false)
		_, _, _ = waitForQueryToComplete(t, dbClient, ctx, queryId)

		results, _, _ = getQueryResults(t, dbClient, ctx, queryId, nil, nil)
		require.Equal(t, *results[0].RowCount, int32(1000000))
		verifyPeopleCsv(t, filePathLong, results[0], false, csv_col_num_to_schema_col_num, nil)
	})
}
