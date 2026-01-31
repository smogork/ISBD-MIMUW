package tests

import (
	"context"
	"testing"

	"github.com/smogork/ISBD-MIMUW/pit"
)

// ============================================================================
// TESTS: Arithmetic Operators
// ============================================================================

func TestFunctional_ArithmeticOperators(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// Addition
	runner.AddCase("INT64_Plus_Literal",
		"SELECT int_col + 10 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(11)}, {int64(12)}, {int64(13)}})

	runner.AddCase("INT64_Plus_INT64",
		"SELECT int_col + int_col FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(2)}, {int64(4)}, {int64(6)}})

	// Subtraction
	runner.AddCase("INT64_Minus_Literal",
		"SELECT int_col - 1 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(0)}, {int64(1)}, {int64(2)}})

	// Multiplication
	runner.AddCase("INT64_Multiply_Literal",
		"SELECT int_col * 3 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(3)}, {int64(6)}, {int64(9)}})

	// Division
	runner.AddCase("INT64_Divide_Literal",
		"SELECT int_col / 1 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(1)}, {int64(2)}, {int64(3)}})

	// Complex expression
	runner.AddCase("ComplexArithmetic",
		"SELECT (int_col + 1) * 2 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(4)}, {int64(6)}, {int64(8)}})

	runner.Run()
}

// ============================================================================
// TESTS: Unary Minus
// ============================================================================

func TestFunctional_UnaryMinus(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	runner.AddCase("UnaryMinus_Column",
		"SELECT -int_col FROM types_test ORDER BY 0 DESC",
		[][]interface{}{{int64(-1)}, {int64(-2)}, {int64(-3)}})

	runner.AddCase("UnaryMinus_Expression",
		"SELECT -(int_col + 10) FROM types_test ORDER BY 0 DESC",
		[][]interface{}{{int64(-11)}, {int64(-12)}, {int64(-13)}})

	runner.Run()
}

// ============================================================================
// TESTS: Comparison Operators
// ============================================================================

func TestFunctional_ComparisonOperators(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// Equal
	runner.AddCase("INT64_Equal_True",
		"SELECT int_col = 2 FROM types_test WHERE int_col = 2",
		[][]interface{}{{true}})

	runner.AddCase("VARCHAR_Equal",
		"SELECT varchar_col = 'hello' FROM types_test WHERE int_col = 1",
		[][]interface{}{{true}})

	// Not Equal
	runner.AddCase("INT64_NotEqual",
		"SELECT int_col, int_col <> 2 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{1, true}, {2, false}, {3, true}})

	// Less Than
	runner.AddCase("INT64_LessThan",
		"SELECT int_col, int_col < 2 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{1, true}, {2, false}, {3, false}})

	// Less Than or Equal
	runner.AddCase("INT64_LessEqual",
		"SELECT int_col, int_col <= 2 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{1, true}, {2, true}, {3, false}})

	// Greater Than
	runner.AddCase("INT64_GreaterThan",
		"SELECT int_col, int_col > 2 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{1, false}, {2, false}, {3, true}})

	// Greater Than or Equal
	runner.AddCase("INT64_GreaterEqual",
		"SELECT int_col, int_col >= 2 FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{1, false}, {2, true}, {3, true}})

	runner.Run()
}

// ============================================================================
// TESTS: Logical Operators
// ============================================================================

func TestFunctional_LogicalOperators(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// AND
	runner.AddCase("BOOL_AND_TrueTrue",
		"SELECT (int_col > 0) AND (int_col < 10) FROM types_test WHERE int_col = 1",
		[][]interface{}{{true}})

	runner.AddCase("BOOL_AND_TrueFalse",
		"SELECT (int_col > 0) AND (int_col > 10) FROM types_test WHERE int_col = 1",
		[][]interface{}{{false}})

	// OR
	runner.AddCase("BOOL_OR_TrueFalse",
		"SELECT (int_col > 0) OR (int_col > 10) FROM types_test WHERE int_col = 1",
		[][]interface{}{{true}})

	runner.AddCase("BOOL_OR_FalseFalse",
		"SELECT (int_col < 0) OR (int_col > 10) FROM types_test WHERE int_col = 1",
		[][]interface{}{{false}})

	// NOT
	runner.AddCase("NOT_True",
		"SELECT NOT (int_col > 10) FROM types_test WHERE int_col = 1",
		[][]interface{}{{true}})

	runner.AddCase("NOT_False",
		"SELECT NOT (int_col > 0) FROM types_test WHERE int_col = 1",
		[][]interface{}{{false}})

	runner.Run()
}

// ============================================================================
// TESTS: String Functions
// ============================================================================

func TestFunctional_StringFunctions(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// STRLEN
	runner.AddCase("STRLEN",
		"SELECT STRLEN(varchar_col) FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(4)}, {int64(5)}, {int64(5)}})

	// UPPER
	runner.AddCase("UPPER",
		"SELECT UPPER(varchar_col) FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{"HELLO"}, {"TEST"}, {"WORLD"}})

	// LOWER
	runner.AddCase("LOWER",
		"SELECT LOWER(varchar_col) FROM types_test WHERE int_col = 1",
		[][]interface{}{{"hello"}})

	// CONCAT
	runner.AddCase("CONCAT",
		"SELECT CONCAT(varchar_col, '!') FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{"hello!"}, {"test!"}, {"world!"}})

	// REPLACE
	runner.AddCase("REPLACE",
		"SELECT REPLACE(varchar_col, 'l', 'L') FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{"heLLo"}, {"test"}, {"worLd"}})

	// Nested functions
	runner.AddCase("NestedFunctions",
		"SELECT STRLEN(UPPER(varchar_col)) FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(4)}, {int64(5)}, {int64(5)}})

	runner.Run()
}

func TestFunctional_StringFunctionsLiterals(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// SELECT 1
	runner.AddCase("SELECT_1",
		"SELECT 1",
		[][]interface{}{{int64(1)}})

	runner.AddCase("STRLEN",
		"SELECT STRLEN('test')",
		[][]interface{}{{int64(4)}})

	runner.AddCase("UPPER",
		"SELECT UPPER('hello')",
		[][]interface{}{{"HELLO"}})

	runner.AddCase("LOWER",
		"SELECT LOWER('HeLLo')",
		[][]interface{}{{"hello"}})

	runner.AddCase("CONCAT",
		"SELECT CONCAT('hello', '!')",
		[][]interface{}{{"hello!"}})

	runner.AddCase("CONCAT_Empty",
		"SELECT CONCAT('', 'foo')",
		[][]interface{}{{"foo"}})

	runner.AddCase("REPLACE",
		"SELECT REPLACE('Hello world', 'l', 'L')",
		[][]interface{}{{"HeLLo WorLd"}})

	runner.AddCase("REPLACE_NoMatch",
		"SELECT REPLACE('abc', 'x', 'y')",
		[][]interface{}{{"abc"}})

	runner.AddCase("NestedFunctions",
		"SELECT STRLEN(UPPER('hello'))",
		[][]interface{}{{int64(5)}})

	runner.Run()
}

// ============================================================================
// TESTS: WHERE Clause Filtering
// ============================================================================

func TestFunctional_WhereClause(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// Simple filter
	runner.AddCase("WHERE_SimpleFilter",
		"SELECT int_col FROM types_test WHERE int_col = 2",
		[][]interface{}{{int64(2)}})

	// Range filter
	runner.AddCase("WHERE_RangeFilter",
		"SELECT int_col FROM types_test WHERE int_col >= 2 ORDER BY 0 ASC",
		[][]interface{}{{int64(2)}, {int64(3)}})

	// String filter
	runner.AddCase("WHERE_StringFilter",
		"SELECT varchar_col FROM types_test WHERE varchar_col = 'hello'",
		[][]interface{}{{"hello"}})

	// Complex filter
	runner.AddCase("WHERE_ComplexFilter",
		"SELECT int_col FROM types_test WHERE int_col > 1 AND int_col < 3",
		[][]interface{}{{int64(2)}})

	// Filter with function
	runner.AddCase("WHERE_FunctionFilter",
		"SELECT varchar_col FROM types_test WHERE STRLEN(varchar_col) > 4 ORDER BY 0 ASC",
		[][]interface{}{{"hello"}, {"world"}})

	// No matching rows
	runner.AddCase("WHERE_NoMatch",
		"SELECT int_col FROM types_test WHERE int_col > 100",
		[][]interface{}{})

	// TRUE/FALSE literals
	runner.AddCase("WHERE_TrueLiteral",
		"SELECT int_col FROM types_test WHERE TRUE ORDER BY 0 ASC",
		[][]interface{}{{int64(1)}, {int64(2)}, {int64(3)}})

	runner.AddCase("WHERE_FalseLiteral",
		"SELECT int_col FROM types_test WHERE FALSE",
		[][]interface{}{})

	runner.Run()
}

// ============================================================================
// TESTS: ORDER BY Clause
// ============================================================================

func TestFunctional_OrderBy(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// ASC
	runner.AddCase("OrderBy_ASC",
		"SELECT int_col FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{int64(1)}, {int64(2)}, {int64(3)}})

	// DESC
	runner.AddCase("OrderBy_DESC",
		"SELECT int_col FROM types_test ORDER BY 0 DESC",
		[][]interface{}{{int64(3)}, {int64(2)}, {int64(1)}})

	// String ordering
	runner.AddCase("OrderBy_String_ASC",
		"SELECT varchar_col FROM types_test ORDER BY 0 ASC",
		[][]interface{}{{"hello"}, {"test"}, {"world"}})

	runner.AddCase("OrderBy_String_DESC",
		"SELECT varchar_col FROM types_test ORDER BY 0 DESC",
		[][]interface{}{{"world"}, {"test"}, {"hello"}})

	// Multiple columns
	runner.AddCase("OrderBy_MultipleColumns",
		"SELECT int_col, varchar_col FROM types_test ORDER BY 1 ASC",
		[][]interface{}{{int64(1), "hello"}, {int64(3), "test"}, {int64(2), "world"}})

	runner.Run()
}

// ============================================================================
// TESTS: LIMIT Clause
// ============================================================================

func TestFunctional_Limit(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	runner.AddCase("Limit_One",
		"SELECT int_col FROM types_test ORDER BY 0 ASC LIMIT 1",
		[][]interface{}{{int64(1)}})

	runner.AddCase("Limit_Two",
		"SELECT int_col FROM types_test ORDER BY 0 ASC LIMIT 2",
		[][]interface{}{{int64(1)}, {int64(2)}})

	runner.AddCase("Limit_MoreThanRows",
		"SELECT int_col FROM types_test ORDER BY 0 ASC LIMIT 100",
		[][]interface{}{{int64(1)}, {int64(2)}, {int64(3)}})

	runner.AddCase("Limit_Zero",
		"SELECT int_col FROM types_test ORDER BY 0 ASC LIMIT 0",
		[][]interface{}{})

	runner.Run()
}

// ============================================================================
// TESTS: Literals
// ============================================================================

func TestFunctional_Literals(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// Integer literal
	runner.AddCase("IntegerLiteral",
		"SELECT 42 FROM types_test WHERE int_col = 1",
		[][]interface{}{{int64(42)}})

	// String literal
	runner.AddCase("StringLiteral",
		"SELECT 'hello world' FROM types_test WHERE int_col = 1",
		[][]interface{}{{"hello world"}})

	// Boolean literals
	runner.AddCase("TrueLiteral",
		"SELECT TRUE FROM types_test WHERE int_col = 1",
		[][]interface{}{{true}})

	runner.AddCase("FalseLiteral",
		"SELECT FALSE FROM types_test WHERE int_col = 1",
		[][]interface{}{{false}})

	// Literal arithmetic
	runner.AddCase("LiteralArithmetic",
		"SELECT 2 + 3 * 4 FROM types_test WHERE int_col = 1",
		[][]interface{}{{int64(14)}})

	runner.Run()
}

// ============================================================================
// TESTS: Complex Combined Queries
// ============================================================================

func TestFunctional_ComplexQueries(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "types_test")
	LoadTestData(t, dbClient, ctx, "types_test")

	runner := NewFunctionalTestRunner(t, dbClient, ctx)

	// Combination of multiple features
	runner.AddCase("FullQuery",
		"SELECT int_col * 10, UPPER(varchar_col) FROM types_test WHERE int_col >= 2 ORDER BY 0 DESC LIMIT 1",
		[][]interface{}{{int64(30), "TEST"}})

	runner.AddCase("FunctionInWhere_And_Select",
		"SELECT STRLEN(varchar_col), varchar_col FROM types_test WHERE STRLEN(varchar_col) = 5 ORDER BY 1 ASC",
		[][]interface{}{{int64(5), "hello"}, {int64(5), "world"}})

	runner.AddCase("ArithmeticInWhere",
		"SELECT int_col FROM types_test WHERE int_col + 10 > 12 ORDER BY 0 ASC",
		[][]interface{}{{int64(3)}})

	runner.Run()
}
