use utils.nu assert
use format_query.nu

const SELECT_tests = [
    {
        # Most basic query
        input: [col SELECT *]
        expected: 'FROM col SELECT *'
    }
    {
        # correctly format list of fields
        input: [col SELECT field1 field2]
        expected: 'FROM col SELECT `field1` , `field2`'
    }
    {
        # Don't wrap * or meta().id in backticks
        input: [col SELECT * meta().id]
        expected: 'FROM col SELECT * , meta().id'
    }
    {
        # Same as above, with meta().id first
        input: [col SELECT meta().id *]
        expected: 'FROM col SELECT meta().id , *'
    }
    {
        #Handle field name with a space
        input: [col SELECT 'space field' field2]
        expected: 'FROM col SELECT `space field` , `field2`'
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
        expected: 'SELECT `field1` , `field2` FROM col WHERE `field3` = "value"'
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
        input: [col SELECT field WHERE field1 == value AND field2 == '10.5']
        expected: 'SELECT `field` FROM col WHERE `field1` = "value" AND `field2` = 10.5'
    }
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
        expected: 'FROM col SELECT * LIMIT 10'
    }
    {
        input: [col SELECT * WHERE field == value LIMIT 10]
        expected: 'SELECT * FROM col WHERE `field` = "value" LIMIT 10'
    }
    {
        input: [col SELECT * WHERE field == value AND field2 != "other value" LIMIT 10]
        expected: 'SELECT * FROM col WHERE `field` = "value" AND `field2` != "other value" LIMIT 10'
    }
]

const ORDER_BY_tests = [
    {
        input: [col SELECT field ORDER BY field]
        expected: 'SELECT `field` FROM col ORDER BY `field`'
    }
    {
        input: [col SELECT 'some field' ORDER BY 'some field' LIMIT '10']
        expected: 'SELECT `some field` FROM col ORDER BY `some field` LIMIT 10'
    }
    {
        input: [col SELECT field ORDER BY field ASC]
        expected: 'SELECT `field` FROM col ORDER BY `field` ASC'
    }
    {
        input: [col SELECT field ORDER BY field DESC]
        expected: 'SELECT `field` FROM col ORDER BY `field` DESC'
    }
    {
        input: [col SELECT field 'some field' ORDER BY field ASC LIMIT '10']
        expected: 'SELECT `field`, `some field` FROM col ORDER BY `field` ASC LIMIT 10'
    }
    {
        input: [col SELECT field * 'some field' ORDER BY 'some field' ASC]
        expected: 'SELECT `field`, *, `some field` FROM col ORDER BY `some field` ASC'
    }
    {
        input: [col SELECT field1 'some field' ORDER BY field1 'some field']
        expected: 'SELECT `field1`, `some field` FROM col ORDER BY `field1` , `some field`'
    }
    {
        input: [col SELECT field1 field2 WHERE field3 == value ORDER BY field1]
        expected: 'SELECT `field1`, `field2` FROM col WHERE `field3` = "value" ORDER BY `field1`'
    }
    {
        input: [col SELECT field1 field2 WHERE 'some field' == 'some value' ORDER BY field1 ASC field2 DESC]
        expected: 'SELECT `field1`, `field2` FROM col WHERE `field3` = "value" ORDER BY `field1` ASC , `field2` DESC'
    }
    {
        input: [col SELECT field1 field2 field3 ORDER BY field1 ASC field2 DESC field3 LIMIT '10']
        expected: 'SELECT `field1`, `field2`, field3` FROM col ORDER BY `field1` ASC , `field2` DESC , `field3` LIMIT 10'
    }
    # TO do tests with WHERE AND, also with SATISFIES clauses
]

export def main [] {
    let tests = [
        #...$SELECT_tests
        #...$WHERE_tests
        #...$AND_tests
        #...$SATISFIES_tests
        #...$LIMIT_tests
        ...$ORDER_BY_tests
    ]

    for test in $tests {
        print $test.expected
        assert $test.expected (format_query $test.input)
    }
}