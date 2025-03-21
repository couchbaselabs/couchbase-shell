use utils.nu assert
use format_query.nu

const SELECT_tests = [
    {
        # Most basic query
        input: [col SELECT *]
        expected: 'SELECT * FROM col'
    }
    {
        # correctly format list of fields
        input: [col SELECT field1 field2]
        expected: 'SELECT `field1`, `field2` FROM col'
    }
    {
        # Don't wrap * or meta().id in backticks
        input: [col SELECT * meta().id]
        expected: 'SELECT *, meta().id FROM col'
    }
    {
        # Same as above, with meta().id first
        input: [col SELECT meta().id *]
        expected: 'SELECT meta().id, * FROM col'
    }
    {
        #Handle field name with a space
        input: [col SELECT 'space field' field2]
        expected: 'SELECT `space field`, `field2` FROM col'
    }
]

const WHERE_tests = [
    {
        # Basic query with WHERE clause
        input: [col SELECT * WHERE field == value]
        expected: 'SELECT * FROM col WHERE `field` = "value"'
    }
    {
        # Multiple fields
        input: [col SELECT field1 field2 WHERE field3 == value]
        expected: 'SELECT `field1`, `field2` FROM col WHERE `field3` = "value"'
    }
    {
        # Condition value with spaces
        input: [col SELECT * WHERE field == 'some value']
        expected: 'SELECT * FROM col WHERE `field` = "some value"'
    }
    {
        # LIKE with wildcard
        input: [col SELECT * WHERE field LIKE '%alue']
        expected: 'SELECT * FROM col WHERE `field` LIKE "%alue"'
    }
]

const AND_tests = [
    {
        # Don't wrap int or float condition values in quote marks
        # Note that we quote the numbers in the list because they will be passed to format_query as strings
        input: [col SELECT field WHERE field1 == '10' AND field2 == '10.5']
        expected: 'SELECT `field` FROM col WHERE `field1` = 10 AND `field2` = 10.5'
    }
]

const SATISFIES_tests = [
    {
        # Don't wrap the element.`field` pair in more backticks
        input: [col SELECT field WHERE ANY element IN array SATISFIES "element.`field`" == value END]
        expected: 'SELECT `field` FROM col WHERE ANY element IN array SATISFIES element.`field` = "value" END'
    }
    {
        # Same as above with int value
        input: [col SELECT field WHERE ANY element IN array SATISFIES "element.`field`" == '11' END]
        expected: 'SELECT `field` FROM col WHERE ANY element IN array SATISFIES element.`field` = 11 END'
    }
    {
        # It is fine to wrap the element when it is not referencing a nested field
        input: [col SELECT field WHERE ANY element IN array SATISFIES element == value END]
        expected: 'SELECT `field` FROM col WHERE ANY element IN array SATISFIES `element` = "value" END'
    }
]

const LIMIT_tests = [
    {
        input: [col SELECT * LIMIT 10]
        expected: 'SELECT * FROM col LIMIT 10'
    }
    {
        input: [col SELECT * WHERE field == value LIMIT 10]
        expected: 'SELECT * FROM col WHERE `field` = "value" LIMIT 10'
    }
]

export def main [] {
    let tests = [
        ...$SELECT_tests
        ...$WHERE_tests
        ...$AND_tests
        ...$SATISFIES_tests
        ...$LIMIT_tests
    ]

    for test in $tests {
        print $test.expected
        assert $test.expected (format_query $test.input)
    }
}