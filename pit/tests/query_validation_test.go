package tests

import (
	"context"
	"testing"

	"github.com/smogork/ISBD-MIMUW/pit"
)

// ============================================================================
// TESTS: Column Reference Validation
// ============================================================================

func TestQueryValidation_ColumnReferences(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")
	_ = SetupTestTable(t, dbClient, ctx, "types_test") // needed for multi-table test

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("ExistingColumn", "SELECT name FROM people")
	runner.AddSuccessCase("CorrectTablePrefix", "SELECT people.name FROM people")
	runner.AddSuccessCase("MultipleColumns", "SELECT name, surname, age FROM people")

	runner.AddFailureCase("NonExistentColumn", "SELECT nonexistent FROM people", "column")
	runner.AddFailureCase("WrongTablePrefix", "SELECT other.name FROM people", "table")
	runner.AddFailureCase("OneInvalidColumn", "SELECT name, invalid_col FROM people", "column")
	runner.AddFailureCase("ColumnsFromTwoTables", "SELECT people.name, types_test.int_col FROM people, types_test", "table")

	runner.Run()
}

// ============================================================================
// TESTS: Arithmetic Operator Type Validation
// ============================================================================

func TestQueryValidation_ArithmeticOperators(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("INT64_Plus_INT64", "SELECT age + 1 FROM people")
	runner.AddSuccessCase("INT64_Plus_INT64_Columns", "SELECT age + id FROM people")
	runner.AddSuccessCase("INT64_Minus_INT64", "SELECT age - 10 FROM people")
	runner.AddSuccessCase("INT64_Multiply_INT64", "SELECT age * 2 FROM people")
	runner.AddSuccessCase("INT64_Divide_INT64", "SELECT age / 2 FROM people")
	runner.AddSuccessCase("ComplexArithmetic", "SELECT (age + 10) * 2 - id FROM people")

	runner.AddFailureCase("VARCHAR_Plus_INT64", "SELECT name + 1 FROM people", "type")
	runner.AddFailureCase("INT64_Minus_VARCHAR", "SELECT age - name FROM people", "type")
	runner.AddFailureCase("VARCHAR_Multiply_INT64", "SELECT name * 2 FROM people", "type")
	runner.AddFailureCase("VARCHAR_Divide_INT64", "SELECT name / 2 FROM people", "type")

	runner.Run()
}

// ============================================================================
// TESTS: Logical Operator Type Validation
// ============================================================================

func TestQueryValidation_LogicalOperators(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("BOOL_AND_BOOL", "SELECT (age > 10) AND (age < 50) FROM people")
	runner.AddSuccessCase("BOOL_OR_BOOL", "SELECT (age > 10) OR (age < 5) FROM people")
	runner.AddSuccessCase("NOT_BOOL", "SELECT NOT (age > 10) FROM people")
	runner.AddSuccessCase("ComplexLogical", "SELECT (age > 10 AND age < 50) OR (name = 'Jan') FROM people")

	runner.AddFailureCase("INT64_AND_BOOL", "SELECT age AND TRUE FROM people", "type")
	runner.AddFailureCase("VARCHAR_OR_BOOL", "SELECT name OR TRUE FROM people", "type")
	runner.AddFailureCase("NOT_INT64", "SELECT NOT age FROM people", "type")
	runner.AddFailureCase("NOT_VARCHAR", "SELECT NOT name FROM people", "type")

	runner.Run()
}

// ============================================================================
// TESTS: Comparison Operator Type Validation
// ============================================================================

func TestQueryValidation_ComparisonOperators(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("INT64_Equal_INT64", "SELECT age = 30 FROM people")
	runner.AddSuccessCase("VARCHAR_Equal_VARCHAR", "SELECT name = 'Jan' FROM people")
	runner.AddSuccessCase("INT64_NotEqual_INT64", "SELECT age <> 30 FROM people")
	runner.AddSuccessCase("INT64_LessThan_INT64", "SELECT age < 30 FROM people")
	runner.AddSuccessCase("VARCHAR_LessThan_VARCHAR", "SELECT name < 'M' FROM people")
	runner.AddSuccessCase("INT64_LessEqual_INT64", "SELECT age <= 30 FROM people")
	runner.AddSuccessCase("INT64_GreaterThan_INT64", "SELECT age > 30 FROM people")
	runner.AddSuccessCase("INT64_GreaterEqual_INT64", "SELECT age >= 30 FROM people")
	runner.AddSuccessCase("ColumnToColumn_SameType", "SELECT name = surname FROM people")

	runner.AddFailureCase("INT64_Equal_VARCHAR", "SELECT age = 'Jan' FROM people", "type")
	runner.AddFailureCase("VARCHAR_NotEqual_INT64", "SELECT name <> 30 FROM people", "type")
	runner.AddFailureCase("INT64_LessThan_VARCHAR", "SELECT age < 'M' FROM people", "type")
	runner.AddFailureCase("ColumnToColumn_DifferentType", "SELECT name = age FROM people", "type")

	runner.Run()
}

// ============================================================================
// TESTS: Function Type Validation
// ============================================================================

func TestQueryValidation_Functions(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("STRLEN_VARCHAR", "SELECT STRLEN(name) FROM people")
	runner.AddSuccessCase("CONCAT_VARCHAR_VARCHAR", "SELECT CONCAT(name, surname) FROM people")
	runner.AddSuccessCase("UPPER_VARCHAR", "SELECT UPPER(name) FROM people")
	runner.AddSuccessCase("LOWER_VARCHAR", "SELECT LOWER(name) FROM people")
	runner.AddSuccessCase("NestedFunctions", "SELECT STRLEN(CONCAT(name, surname)) FROM people")
	runner.AddSuccessCase("FunctionInArithmetic", "SELECT STRLEN(name) + STRLEN(surname) FROM people")

	runner.AddFailureCase("STRLEN_INT64", "SELECT STRLEN(age) FROM people", "type")
	runner.AddFailureCase("CONCAT_VARCHAR_INT64", "SELECT CONCAT(name, age) FROM people", "type")
	runner.AddFailureCase("CONCAT_INT64_VARCHAR", "SELECT CONCAT(age, name) FROM people", "type")
	runner.AddFailureCase("UPPER_INT64", "SELECT UPPER(age) FROM people", "type")
	runner.AddFailureCase("LOWER_INT64", "SELECT LOWER(age) FROM people", "type")
	runner.AddFailureCase("InvalidNestedFunction", "SELECT STRLEN(STRLEN(name)) FROM people", "type")

	runner.Run()
}

// ============================================================================
// TESTS: WHERE Clause Validation
// ============================================================================

func TestQueryValidation_WhereClause(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("WHERE_BoolExpression", "SELECT name FROM people WHERE age > 30")
	runner.AddSuccessCase("WHERE_ComplexBool", "SELECT name FROM people WHERE age > 30 AND name = 'Jan'")
	runner.AddSuccessCase("WHERE_WithOr", "SELECT name FROM people WHERE age > 30 OR age < 10")
	runner.AddSuccessCase("WHERE_WithNot", "SELECT name FROM people WHERE NOT (age > 30)")
	runner.AddSuccessCase("WHERE_FunctionResult_Bool", "SELECT name FROM people WHERE STRLEN(name) > 3")
	runner.AddSuccessCase("WHERE_TrueLiteral", "SELECT name FROM people WHERE TRUE")
	runner.AddSuccessCase("WHERE_FalseLiteral", "SELECT name FROM people WHERE FALSE")

	runner.AddFailureCase("WHERE_INT64", "SELECT name FROM people WHERE age", "bool")
	runner.AddFailureCase("WHERE_VARCHAR", "SELECT name FROM people WHERE name", "bool")
	runner.AddFailureCase("WHERE_FunctionResult_NotBool", "SELECT name FROM people WHERE STRLEN(name)", "bool")
	runner.AddFailureCase("WHERE_Arithmetic_NotBool", "SELECT name FROM people WHERE age + 10", "bool")

	runner.Run()
}

// ============================================================================
// TESTS: ORDER BY Validation
// ============================================================================

func TestQueryValidation_OrderBy(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("OrderBy_ValidIndex_0", "SELECT name FROM people ORDER BY 0 ASC")
	runner.AddSuccessCase("OrderBy_ValidIndex_1", "SELECT name, age FROM people ORDER BY 1 DESC")
	runner.AddSuccessCase("OrderBy_MultipleColumns", "SELECT name, age, surname FROM people ORDER BY 0 ASC, 1 DESC")
	runner.AddSuccessCase("OrderBy_WithLimit", "SELECT name, age FROM people ORDER BY 1 DESC LIMIT 10")

	runner.AddFailureCase("OrderBy_IndexOutOfRange", "SELECT name FROM people ORDER BY 5 ASC", "index")
	runner.AddFailureCase("OrderBy_IndexOutOfRange_ExactlyOne", "SELECT name FROM people ORDER BY 1 ASC", "index")
	runner.AddFailureCase("OrderBy_MultipleColumns_OneInvalid", "SELECT name, age FROM people ORDER BY 0 ASC, 5 DESC", "index")

	runner.Run()
}

// ============================================================================
// TESTS: Unary Minus Operator
// ============================================================================

func TestQueryValidation_UnaryMinus(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("Minus_INT64", "SELECT -age FROM people")
	runner.AddSuccessCase("Minus_Expression", "SELECT -(age + 10) FROM people")

	runner.AddFailureCase("Minus_VARCHAR", "SELECT -name FROM people", "type")

	runner.Run()
}

// ============================================================================
// TESTS: Literal Values
// ============================================================================

func TestQueryValidation_Literals(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("IntegerLiteral", "SELECT 42 FROM people")
	runner.AddSuccessCase("StringLiteral", "SELECT 'hello' FROM people")
	runner.AddSuccessCase("TrueLiteral", "SELECT TRUE FROM people")
	runner.AddSuccessCase("FalseLiteral", "SELECT FALSE FROM people")
	runner.AddSuccessCase("LiteralArithmetic", "SELECT 1 + 2 + 3 FROM people")
	runner.AddSuccessCase("LiteralStringConcat", "SELECT CONCAT('Hello', ' World') FROM people")
	runner.AddSuccessCase("LiteralStringReplace", "SELECT REPLACE('Hello world!', ' World', ' Interface') FROM people")
	runner.AddSuccessCase("MixedLiteralAndColumn", "SELECT age + 100 FROM people")

	runner.AddFailureCase("LiteralStringReplace", "SELECT REPLACE('Hello world!', 1, TRUE) FROM people", "type")

	runner.Run()
}

// ============================================================================
// TESTS: Complex Queries
// ============================================================================

func TestQueryValidation_ComplexQueries(t *testing.T) {
	RequireInterfaceVersion(t, 2)
	dbClient := pit.DbClient(BaseURL)
	ctx := context.Background()
	_ = SetupTestTable(t, dbClient, ctx, "people")

	runner := NewValidationTestRunner(t, dbClient, ctx)

	runner.AddSuccessCase("CompleteQuery",
		"SELECT name, UPPER(surname), age + 10 FROM people WHERE age > 20 ORDER BY 2 DESC LIMIT 100")
	runner.AddSuccessCase("QueryWithAllFeatures",
		"SELECT CONCAT(name, CONCAT(' ', surname)), STRLEN(name) + STRLEN(surname), age * 2 FROM people WHERE age >= 18 AND STRLEN(name) > 2 ORDER BY 1 ASC, 2 DESC LIMIT 50")
	runner.AddSuccessCase("DeepNesting",
		"SELECT UPPER(LOWER(UPPER(name))) FROM people")
	runner.AddSuccessCase("ComplexWhere",
		"SELECT name FROM people WHERE (age > 20 AND age < 40) OR (STRLEN(name) > 5 AND name <> 'Jan')")

	runner.Run()
}
