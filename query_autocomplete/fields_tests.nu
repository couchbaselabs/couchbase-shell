use fields.nu
use utils.nu assert

const $cli_operators = [!= <= >= > < == LIKE]

# These need to be executed in the travel-sample.inventory scope
# They are classified by the last key word present in the context
# To run the tests do `use query_autocomplete *` then `test_all`
const FROM_tests = [
     {
         # Expect list of collections after FROM
         input: "FROM "
         expected: [route landmark hotel airport airline]
     }
     {
         # Suggest SELECT after collection with special characters
         input: "FROM test_collection "
         expected: [SELECT]
     }
]

const SELECT_tests = [
    {
        # Suggest list of fields after SELECT
        input: "FROM route SELECT "
        expected: [* airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    }
    {
        # Suggest WHERE and all other fields after *
        input: "FROM route SELECT * "
        expected: [WHERE airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    }
    {
        # Suggest list of fields and WHERE after SELECT meta().id
        input: "FROM route SELECT meta().id "
        expected: [WHERE * airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    }
    {
        # Don't suggest used fields
        input: "FROM route SELECT airline distance schedule type "
        expected: [WHERE * airlineid destinationairport equipment id sourceairport stops]
    }
    {
        # Correctly detect used fields when meta().id present
        input: "FROM route SELECT airline meta().id distance schedule type "
        expected: [WHERE * airlineid destinationairport equipment id sourceairport stops]
    }
    {
        # Handle fields with spaces in the names
        input: "FROM route SELECT airline 'space field' distance schedule type "
        expected: [WHERE * airlineid destinationairport equipment id sourceairport stops]
    }
]

const WHERE_tests = [
    {
        # Suggest all fields after WHERE
        input: "FROM route SELECT airline distance schedule type WHERE "
        expected: [airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    }
    {
        # Suggest operators after field
        input: "FROM route SELECT * WHERE airline "
        expected: $cli_operators
    }
    {
        # Suggest operators after WHERE meta().id
        input: "FROM hotel SELECT * WHERE meta().id "
        expected: $cli_operators
    }
    {
        # Suggest operator after WHERE field with spaces
        input: "FROM route SELECT * WHERE `some field` "
        expected: $cli_operators
    }
    {
         # Suggest operator after WHERE field with underscore
         input: "FROM route SELECT * WHERE some_field "
         expected: $cli_operators
    }
    {
        # Suggest nothing after operator
        input: "FROM route SELECT airline distance schedule type WHERE airline == "
        expected: null
    }
]

export def main [] {
    let tests = [
        ...$FROM_tests
        ...$SELECT_tests
        ...$WHERE_tests
    ]

    for test in $tests {
        print $test.input
        assert $test.expected (fields $test.input)
    }
}