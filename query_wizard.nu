# query_wizard.nu

# key_words are n1ql query keywords
const $key_words = [FROM SELECT WHERE AND ANY EVERY IN SATISFIES END LIMIT]

# operators are the operators that can be used in conditions in a where clause
const $operators = [!= <= >= > < = LIKE]

# cli_operators are the operators that can be suggested to the user as custom completions
# the only difference between $operators and ¢cli_operators is the = -> ==. This is because =
# is reserved in nushell and cannot be used as an argument to FROM
const $cli_operators = [!= <= >= > < == LIKE]

# collection_fields returns a table with two columns:
#     field - the name of the field in the documents in that collection
#     type - the type of the data contained in that field
export def collection_fields [context: string] {
    let $collection = ($context | split words | $in.1)
    let $name_space = (cb-env | ["`" $in.bucket "`" . $in.scope . $collection] | str join)
    let $infer_result = [infer $name_space] | str join " " | query $in
    let $fields = ($infer_result | get properties | columns)
    # For each field get its type from the results of the infer query and construct a table
    $fields | each {|it| $infer_result | get properties | get $it | get type | flatten | last | [[field type]; [$it $in]]} | flatten
}

# fields returns the list of valid next words given a partial query
export def fields [context: string] {
    let $last = ($context | split row " " | drop | last)
    if $last == * {
        return [WHERE]
    }

    let $query_words = ($context | split words)
    let $last_word = ($query_words | last)
    let $used_keywords = ($query_words | each {|it| if ($it in $key_words) {$it}})
    let $last_keyword = ($used_keywords | last)

    if $last_word == $last_keyword {
        match $last_word {
            FROM => {return (collections | get collection)}
            SELECT => {return (collection_fields $context | get field | prepend *)}
            WHERE => {return (collection_fields $context | get field | prepend ANY | prepend EVERY)}
            AND => {
                # Need to suggest list of fields after WHERE that have not been used in conditions yet
                let $after_where = ($context | split row WHERE | last)
                let $conditions = ($after_where | split row AND | drop)
                # Check what fields with spaces in have to be enclosed with this assumes it is `
                let $fields_in_conditions = ($conditions | each {|cond| $cli_operators | each {|op| if (($cond | split row $op | length) > 1) {$cond | split row $op | first | str trim | str trim --char '`' } } | first})
                return (collection_fields $context | get field | filter {|x| $x not-in $fields_in_conditions} | prepend [EVERY ANY])
            }
            ANY => {return}
            EVERY => {return}
            IN => {return (collection_fields $context| filter {|it| $it.type == array} | get field)}
            SATISFIES => {
                # When the last word is satisfies we need to return a list of fields of the objects in the array
                # appended to the alias of the array members given before IN

                # context will be "... review IN reviews SATISFIES " so we get alias, array from either side of "IN"
                let $in_index = ($context | split row " " | enumerate | each {|it| if ($it.item == IN) {$it.index}})
                let $alias = ($context | split words | get ($in_index.0 - 1))
                let $array = ($context | split words | get ($in_index.0 + 1))
                print $array

                let $collection = ($context | split words | get 1)

                # Gets the infer results for the JSON objects in the array
                let $array_object_fields = ([infer $collection] | str join " " | query $in | get properties | get $array | get items | get properties)



                return ($array_object_fields | columns | each {|it| $array_object_fields | get $it | columns | if ("properties" in $in) {$array_object_fields | get $it | get properties | columns | each {|prop| [$alias . $it . '`' $prop '`'] | str join }} else { [$alias . '`' $it '`'] | str join} } | flatten)
            }
            END => [AND LIMIT]
        }
    }

    let $after_last_keyword = ($context | split row $last_keyword | last)

    match $last_keyword {
        FROM => {return [SELECT]}
        SELECT => {
            # TO DO - make this work with fields with spaces in
            let $selected_fields = ($after_last_keyword | split row " ")
            let $remaining_fields = (collection_fields $context | get field | each {|it| if ($it not-in $selected_fields) {$it}} | flatten)
            return ($remaining_fields | prepend [WHERE])
        }
        WHERE => {
            # If an operator is last argument suggest nothing
            if ($last in $cli_operators) {
                return
            }

            let $after_where = ($after_last_keyword | split row " ")

             # WHERE x #operator y
             # If an operator has been given after where but is not the last, then suggest AND
             if (($after_last_keyword | split row " " | filter {|x| $x in $cli_operators} | length) != 0) {
                return [AND LIMIT]
             }

            # WHERE x
            return $cli_operators
        }
        AND => {
            # If an operator is last argument suggest nothing
            if ($last in $cli_operators) {
                return
            }

            # If an operator has been given after AND but is not the last, then suggest AND LIMIT
            if (($after_last_keyword | split row " " | filter {|x| $x in $cli_operators} | length) != 0) {
                # To do - should check if all fields have been used in conditions, if so only suggest LIMIT
                return [AND LIMIT]
            }

            # AND x => $cli_operators
            return $cli_operators
        }
        ANY => [IN]
        EVERY => [IN]
        IN => {
            # Cases
            # either
            # IN y => [SATISFIES]
            # IN x <operator> => Nothing
            # IN x <operator> y
            [SATISFIES]
        }
        SATISFIES => {
            # If an operator is last argument suggest nothing
            if ($last in $cli_operators) {
                return
            }

            # `Satisfies person.age <operator> <value> ` => END
            if (($after_last_keyword | split row " " | filter {|x| $x in $cli_operators} | length) != 0) {
                return [END]
            }

            # `SATISFIES person.age ` => $cli_operators
            return $cli_operators
        }
    }
}

# is_number determines if a string is actually a string representation of an int or float
def is_number [possible_string: string] {
    # Try to convert the input to an int to see if it is actually a string representation of an int/float
    let $possible_int = (do --ignore-shell-errors { $possible_string | into int })
    ($possible_int != null)
}

# parse_after_where is responsible for correctly formatting the condition values by adding speech marks around any strings
# this is done by iterating over all of the fields after WHERE and determining the strings
def parse_after_where [fields: list] {
    # Find the index of WHERE
    let $where_index = ($fields | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})

    # Get the query after WHERE, since this will hold all the conditions
    let $after_where = ($fields | skip ($where_index.0 + 1))

    # Get all of the values in the condition clauses, the values are determined as being the items that follow operators
    # return a table containing the index of the value in $after_where and the corresponding value
    let $condition_values = ($after_where | enumerate | each {|it| if ($it.item in $operators) {($after_where | get ($it.index + 1)) | [[index value]; [($it.index + 1) $in]]}}) | flatten

    mut $formatted_after_where = $after_where
    for item in $condition_values {
        # Iterate over all condition values and wrap any non-numbers (strings) in quote marks
        let $formatted_value = (if (is_number $item.value) {$item.value} else {['"' $item.value '"'] | str join})
        $formatted_after_where = ($formatted_after_where | update $item.index $formatted_value)
    }

    $formatted_after_where
}

# parse_return_fields takes the list of fields to be returned from the documents as a list and formats them into a single string by:
#   1) Wrapping all field names in backticks, required if they contain a space
#   2) Concatenates fields into a single comma seperated string
def format_return_fields [fields: list] {
    # We format the last field separately since we don't want the final field followed by a comma
    # Also we cannot wrap * or meta().id in backticks else the query will fail
    let $formatted_last_field = ($fields | last | if ($in in [* "meta().id"]) {$in} else {["`" $in "`"] | str join})
    let $other_fields = ($fields | drop)
    let $formatted_other_fields = ($other_fields | each {|it| if ($in in [* "meta().id"]) {[$it ","]} else {["`" $it "`"]} | str join})
    ($formatted_other_fields | append $formatted_last_field | str join " ")
}

# FROM is the top level function that users will interact with to generate a query
# It starts with FROM instead of SELECT since we need to know the collection first so that we can suggest appropriate fields after the SELECT
# The custom completions are (tab suggestions) are generated from the fields function based on the input so far
export def FROM [
field1?: string@fields
field2?: string@fields
field3?: string@fields
field4?: string@fields
field5?: string@fields
field6?: string@fields
field7?: string@fields
field8?: string@fields
field9?: string@fields
field10?: string@fields
field11?: string@fields
field12?: string@fields
field13?: string@fields
field14?: string@fields
field15?: string@fields
--print_query
] {
    let $inputs = [$field1 $field2 $field3 $field4 $field5 $field6 $field7 $field8 $field9 $field10 $field11 $field12 $field13 $field14 $field15]

    let $query = format_query $inputs

    if $print_query {
        return $query
    }

    query $query
}

def format_query [inputs: list] {
    # The brackets in meta().id get dropped so we need to add them back in
    # Also == needs to be replaced with =
    let $inputs = ($inputs | each {|it| if ($it == "meta.id") {"meta().id"} else if ($it == "==") {"="} else {$it}})

    # Find the index of WHERE and then split the input into before and after the WHERE clause
    let $where_index = ($inputs | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})
    let $after_where = ($inputs | skip ($where_index.0 + 1))

    # The conditions need to be parsed and any string condition values wrapped in speech marks
    let $parsed_after_where = (parse_after_where $inputs)

    # Extract the fields to be returned from the document and format them
    let $return_fields = ($inputs | drop ($inputs | length | $in - $where_index.0) | skip 2)
    let $formatted_return_fields = format_return_fields $return_fields

    # Finally construct the query from the starting section and the parsed conditions
    let $query = [SELECT $formatted_return_fields FROM $inputs.0 WHERE] | append $parsed_after_where | str join " "
    return $query
}

# TESTS
# These need to be executed in the travel-sample.inventory scope
# They are classified by the last key word present in the context
export def FROM_tests [] {
        # Expect list of collections after FROM
        let $context = "FROM "
        let $expected = [airline route landmark hotel airport]
        print $context
        assert $expected (fields $context)

        # Suggest SELECT after collection with special characters
        let $context = "FROM test_collection "
        let $expected = [SELECT]
        print $context
        assert $expected (fields $context)
}

export def SELECT_tests [] {
    # Suggest list of fields after SELECT
    let $context = "FROM route SELECT "
    let $expected = [* airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Suggest WHERE after *
    let $context = "FROM route SELECT * "
    let $expected = [WHERE]
    print $context
    assert $expected (fields $context)

    # Suggest list of fields and WHERE after SELECT meta().id
    let $context = "FROM route SELECT meta().id "
    let $expected = [WHERE airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Don't suggest used fields
    let $context = "FROM route SELECT airline distance schedule type "
    let $expected = [WHERE airlineid destinationairport equipment id sourceairport stops]
    print $context
    assert $expected (fields $context)

    #Correctly detect used fields when meta().id present
    let $context = "FROM route SELECT airline meta().id distance schedule type "
    let $expected = [WHERE airlineid destinationairport equipment id sourceairport stops]
    print $context
    assert $expected (fields $context)
}

export def WHERE_tests [] {
    # Suggest all fields after where
    let $context = "FROM route SELECT airline distance schedule type WHERE "
    let $expected = [EVERY ANY airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Suggest operators after field
    let $context = "FROM route SELECT * WHERE airline "
    let $expected = $cli_operators
    print $context
    assert $expected (fields $context)

     # Suggest operators after WHERE meta().id
    let $context = 'FROM hotel SELECT meta().id WHERE meta().id '
    let $expected = $cli_operators
    print $context
    assert $expected (fields $context)

    # Suggest operator after WHERE field with spaces
    let $context = "FROM route SELECT * WHERE `some field` "
    let $expected = $cli_operators
    print $context
    assert $expected (fields $context)

    # Suggest operator after WHERE field with underscore
    let $context = "FROM route SELECT * WHERE some_field "
    let $expected = $cli_operators
    print $context
    assert $expected (fields $context)

    # Suggest noting after operator
    let $context = "FROM route SELECT airline distance schedule type WHERE airline == "
    let $expected = null
    let $result = ((fields $context) == $expected)
    print $context
    if (not $result) {
        print (ansi red_bold) failed (ansi reset)
        print "EXPECTED: " $expected
        print "RESULT: " (fields $context)
    } else {
       print (ansi green_bold) passed (ansi reset)
    }

    # Suggest AND after condition value
    let $context = "FROM route SELECT airline distance schedule type WHERE airline == someAirline "
    let $expected = [AND LIMIT]
    print $context
    assert $expected (fields $context)

    # Condition value with spaces
    let $context = 'FROM route SELECT airline distance schedule type WHERE airline == "Best Airline" '
    let $expected = [AND LIMIT]
    print $context
    assert $expected (fields $context)
}

export def AND_tests [] {
    # Remove field once used in condition
    let $context = "FROM route SELECT airline distance schedule type WHERE airline == someAirline AND "
    let $expected = [EVERY ANY airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Remove fields once used in condition
    let $context = "FROM route SELECT airline distance schedule type WHERE airline == someAirline AND type == someType AND "
    let $expected = [EVERY ANY airlineid destinationairport distance equipment id schedule sourceairport stops]
    print $context
    assert $expected (fields $context)

    # Condition values with spaces
    let $context = 'FROM route SELECT airline distance schedule type WHERE airline == "Best Airline" AND type == "some things" AND '
    let $expected = [EVERY ANY airlineid destinationairport distance equipment id schedule sourceairport stops]
    print $context
    assert $expected (fields $context)
}

export def ANY_tests [] {
    # Empty list after ANY
    let $context = 'FROM hotel SELECT meta().id WHERE ANY '
    let $expected = null
    let $result = ((fields $context) == $expected)
    print $context
    if (not $result) {
        print (ansi red_bold) failed (ansi reset)
        print "EXPECTED: " $expected
        print "RESULT: " (fields $context)
    } else {
        print (ansi green_bold) passed (ansi reset)
    }

    # IN suggested after 'ANY x'
    let $context = 'FROM hotel SELECT meta().id WHERE ANY review '
    let $expected = [IN]
    print $context
    assert $expected (fields $context)
}

export def EVERY_tests [] {
    # Empty list after EVERY
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY '
    let $expected = null
    let $result = ((fields $context) == $expected)
    print $context
    if (not $result) {
        print (ansi red_bold) failed (ansi reset)
        print "EXPECTED: " $expected
        print "RESULT: " (fields $context)
    } else {
        print (ansi green_bold) passed (ansi reset)
    }

    # IN suggested after 'EVERY x'
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY review '
    let $expected = [IN]
    print $context
    assert $expected (fields $context)
}

export def IN_tests [] {
    # Suggest array fields after IN
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN '
    let $expected = [public_likes reviews]
    print $context
    assert $expected (fields $context)

    # Suggest SATISFIES after IN array field
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN reviews '
    let $expected = [SATISFIES]
    print $context
    assert $expected (fields $context)

    # Suggest SATISFIES after IN array field with spaces
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY something IN "some array"'
    let $expected = [SATISFIES]
    print $context
    assert $expected (fields $context)

    # Suggest SATISFIES after IN array field with underscore
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY something IN some_array'
    let $expected = [SATISFIES]
    print $context
    assert $expected (fields $context)
}

export def SATISFIES_tests [] {
    # Suggest nested fields after SATISFIES
    #let $context = 'FROM hotel SELECT meta().id WHERE ANY review IN reviews SATISFIES '
    #let $expected = [review.`author` review.`content` review.`date` review.ratings.`Business service` review.ratings.`Business service (e.g., internet access)` review.ratings.`Check in / front desk` review.ratings.`Cleanliness` review.ratings.`Location` review.ratings.`Overall`  review.ratings.`Rooms` review.ratings.`Service` review.ratings.`Sleep Quality` review.ratings.`Value`]
    #print $context
    #assert $expected (fields $context)

    # Suggest operators after nested field in SATISFIES clause
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN reviews SATISFIES review.ratings.`Service` '
    let $expected = $cli_operators
    print $context
    assert $expected (fields $context)

    # Suggest END after SATISFIES completed
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN reviews SATISFIES review.something == asdf '
    let $expected = [END]
    print $context
    assert $expected (fields $context)

    # Suggest END after SATISFIES condidtion with spaces
    let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN reviews SATISFIES review.something == "something else" '
    let $expected = [END]
    print $context
    assert $expected (fields $context)
}


export def fields_tests [] {
    FROM_tests
    SELECT_tests
    WHERE_tests
    AND_tests
    ANY_tests
    EVERY_tests
    IN_tests
    SATISFIES_tests
}

def assert [expected: list, result: list] {
    if (not ($expected == $result)) {
        print (ansi red_bold) failed (ansi reset)
        print "EXPECTED: " $expected
        print "RESULT: " $result
    } else {
        print (ansi green_bold) passed (ansi reset)
    }
}

def assert_string [expected: string, actual: string] {
     if (not ($expected == $actual)) {
        print (ansi red_bold) failed (ansi reset)
        print "EXPECTED: " $expected
        print "ACTUAL: " $actual
        print
    } else {
        print (ansi green_bold) passed (ansi reset)
    }
}

export def format_query_tests [] {

    let test_cases = [
    {
         # Don't wrap wildcard or meta().id in backticks
        input: [col SELECT * meta().id WHERE field == stringValue]
        expected: 'SELECT *, meta().id FROM col WHERE field = "stringValue"'
    }
    {
         # Don't wrap wildcard or meta().id in backticks with meta().id first
        input: [col SELECT meta().id * WHERE field = stringValue]
        expected: 'SELECT meta().id, * FROM col WHERE field = "stringValue"'
    }
    {
        # Field name with spaces
        input: [col SELECT 'space field' WHERE field == value]
        expected: 'SELECT `space field` FROM col WHERE field = "value"'
    }
    {
        # Condition value with spaces
        input: [col SELECT * WHERE field == 'some value']
        expected: 'SELECT * FROM col WHERE field = "some value"'
    }
    {
        # Multiple fields
        input: [col SELECT field1 field2 WHERE field3 == value]
        expected: 'SELECT `field1` `field2` FROM col WHERE field3 = "value"'
    }
    {
        # Don't wrap int or float condition values in quote marks
        # Note that we quote the numbers in the list because they will be passed to format_query as strings
        input: [col SELECT field WHERE field1 == '10' AND field2 == '10.5']
        expected: 'SELECT `field` FROM col WHERE field1 = 10 AND field2 = 10.5'
    }
    {
         # ANY IN basic test
         input: [col SELECT field WHERE ANY element IN array == value]
         expected: 'SELECT `field` FROM col WHERE ANY element IN array = "value"'
    }
    {
        # ANY IN - float
        input: [col SELECT field WHERE ANY element IN array < '10.123']
        expected: 'SELECT `field` FROM col WHERE ANY element IN array < 10.123'
    }
    {
        # ANY IN - string with spaces
        input: [col SELECT field WHERE ANY element IN array != 'some value']
        expected: 'SELECT `field` FROM col WHERE ANY element IN array != "some value"'
    }
    {
        # ANY IN AND - string with spaces
        input: [col SELECT field WHERE ANY element IN array != 'some value' AND field2 == value2]
        expected: 'SELECT `field` FROM col WHERE ANY element IN array != "some value" AND field2 = "value2"'
    }
    {
         # EVERY IN basic test
         input: [col SELECT field WHERE EVERY element IN array == value]
         expected: 'SELECT `field` FROM col WHERE EVERY element IN array = "value"'
    }
    {
        # EVERY IN - float
        input: [col SELECT field WHERE EVERY element IN array < '10.123']
        expected: 'SELECT `field` FROM col WHERE EVERY element IN array < 10.123'
    }
    {
        # EVERY IN - string with spaces
        input: [col SELECT field WHERE EVERY element IN array != 'some value']
        expected: 'SELECT `field` FROM col WHERE EVERY element IN array != "some value"'
    }
    {
        # LIMIT test
        input: [col SELECT * WHERE field == value LIMIT '1']
        expected: 'SELECT * FROM col WHERE field = "value" LIMIT 1'
    }
    {
        # SATISFIES basic test
        input: [col SELECT * WHERE ANY e IN array SATISFIES e.something == value]
        expected: 'SELECT * FROM col WHERE ANY e IN array SATISFIES e.something = "value"'
    }
    {
        # SATISFIES where nested field contains space
        input: [col SELECT * WHERE EVERY e IN array SATISFIES e.'some field' == '10']
        expected: "SELECT * FROM col WHERE EVERY e IN array SATISFIES e.'some field' = 10"
    }
    ]

    for test in $test_cases {
        print ($test.input | prepend "FROM" | str join " ")
        let $result = format_query $test.input
        assert_string $test.expected $result
    }
}
