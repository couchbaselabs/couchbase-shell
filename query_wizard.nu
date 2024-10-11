# query_wizard.nu
const $key_words = [FROM SELECT WHERE AND ANY EVERY IN SATISFIES END LIMIT]
const $operators = [!= <= >= > < = LIKE]

# Returns a table with two columns, field and type
export def collection_fields [context: string] {
    let $collection = ($context | split words | $in.1)
    let $name_space = (cb-env | ["`" $in.bucket "`" . $in.scope . $collection] | str join)
    let $infer_result = [infer $name_space] | str join " " | query $in
    let $fields = ($infer_result | get properties | columns)
    # For each field get its type from the results of the infer query and construct a table
    $fields | each {|it| $infer_result | get properties | get $it | get type | flatten | last | [[field type]; [$it $in]]} | flatten
}

# Returns list of next valid strings given a partial query
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
                let $fields_in_conditions = ($conditions | each {|cond| $operators | each {|op| if (($cond | split row $op | length) > 1) {$cond | split row $op | first | str trim | str trim --char '`' } } | first})
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
            if ($last in $operators) {
                return
            }

            let $after_where = ($after_last_keyword | split row " ")

             # WHERE x #operator y
             # If an operator has been given after where but is not the last, then suggest AND
             if (($after_last_keyword | split row " " | filter {|x| $x in $operators} | length) != 0) {
                return [AND LIMIT]
             }

            # WHERE x
            return $operators
        }
        AND => {
            # If an operator is last argument suggest nothing
            if ($last in $operators) {
                return
            }

            # If an operator has been given after AND but is not the last, then suggest AND LIMIT
            if (($after_last_keyword | split row " " | filter {|x| $x in $operators} | length) != 0) {
                # To do - should check if all fields have been used in conditions, if so only suggest LIMIT
                return [AND LIMIT]
            }

            # AND x => $operators
            return $operators
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
            if ($last in $operators) {
                return
            }

            # `Satisfies person.age <operator> <value> ` => END
            if (($after_last_keyword | split row " " | filter {|x| $x in $operators} | length) != 0) {
                return [END]
            }

            # `SATISFIES person.age ` => $operators
            return $operators
        }
    }
}

def parse_after_where [fields: list] {
    let $where_index = ($fields | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})
    let $after_where = ($fields | skip ($where_index.0 + 1))

    let $condition_values = ($after_where | enumerate | each {|it| if ($it.item in $operators) {($after_where | get ($it.index + 1))}})
    let $parsed_after_where = (($after_where | enumerate | each {|it| if ($it.item not-in $condition_values) { if ($it.item in $operators) { [$it.item, ($after_where | get ($it.index + 1) | do --ignore-shell-errors {$in | into int} | length | if ($in == 0) {['"' ($after_where | get ($it.index + 1)) '"'] | str join} else {($after_where | get ($it.index + 1))})]} else {$it.item}}}) | flatten)
    $parsed_after_where
}

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
    # The brackets in meta().id get dropped so we need to add them back in
    let $inputs = [$field1 $field2 $field3 $field4 $field5 $field6 $field7 $field8 $field9 $field10 $field11 $field12 $field13 $field14 $field15] | each {|it| if ($it == "meta.id") {"meta().id"} else {$it}}
    let $where_index = ($inputs | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})
    let $after_where = ($inputs | skip ($where_index.0 + 1))

    let $condition_values = ($after_where | enumerate | each {|it| if ($it.item in $operators) {($after_where | get ($it.index + 1))}})
    let $parsed_after_where = (parse_after_where $inputs)

    let $select_fields = ($inputs | drop ($inputs | length | $in - $where_index.0) | skip 2)

    let $select_section = [SELECT] | append ($select_fields | enumerate | each {|it| if ($it.index != ($select_fields | length | $in - 1)) { [$it.item `,`] | str join } else { $it.item }}) | str join " "
    let $query = [$select_section FROM $field1 WHERE] | append $parsed_after_where | str join " "

    if $print_query {
        print $query
    }
    query $query
}


# TESTS
# These need to be executed in the travel-sample.inventory scope
export def fields_tests [] {

    # Suggest collections after FROM
    let $context = "FROM "
    let $expected = [route landmark hotel airport airline]
    print $context
    assert $expected (fields $context)

    # Suggest SELECT after collection with special characters
    let $context = "FROM test_collection "
    let $expected = [SELECT]
    print $context
    assert $expected (fields $context)

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

    # Suggest operators after WHERE meta().id
    let $context = 'FROM hotel SELECT meta().id WHERE meta().id '
    let $expected = $operators
    print $context
    assert $expected (fields $context)

    # Suggest all fields after where
    let $context = "FROM route SELECT airline distance schedule type WHERE "
    let $expected = [EVERY ANY airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Suggest operators after WHERE field
    let $context = "FROM route SELECT * WHERE airline "
    let $expected = $operators
    print $context
    assert $expected (fields $context)

    # Suggest operator after WHERE field with spaces
    let $context = "FROM route SELECT * WHERE `some field` "
    let $expected = $operators
    print $context
    assert $expected (fields $context)

    # Suggest operator after WHERE field with underscore
     let $context = "FROM route SELECT * WHERE some_field "
     let $expected = $operators
     print $context
     assert $expected (fields $context)

    # Suggest noting after operator
    let $context = "FROM route SELECT airline distance schedule type WHERE airline = "
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
    let $context = "FROM route SELECT airline distance schedule type WHERE airline = someAirline "
    let $expected = [AND LIMIT]
    print $context
    assert $expected (fields $context)

    # Remove field once used in condition
    let $context = "FROM route SELECT airline distance schedule type WHERE airline = someAirline AND type = someType AND "
    let $expected = [EVERY ANY airlineid destinationairport distance equipment id schedule sourceairport stops]
    print $context
    assert $expected (fields $context)

    # Condition value with spaces
     let $context = 'FROM route SELECT airline distance schedule type WHERE airline = "Best Airline" '
     let $expected = [AND LIMIT]
     print $context
     assert $expected (fields $context)

     let $context = 'FROM route SELECT airline distance schedule type WHERE airline = "Best Airline" AND type = "some things" AND '
     let $expected = [EVERY ANY airlineid destinationairport distance equipment id schedule sourceairport stops]
     print $context
     assert $expected (fields $context)

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

      # Empty list after ANY
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

      # IN suggested after EVERY
      let $context = 'FROM hotel SELECT meta().id WHERE ANY review '
      let $expected = [IN]
      print $context
      assert $expected (fields $context)

      # IN suggested after EVERY
      let $context = 'FROM hotel SELECT meta().id WHERE EVERY review '
      let $expected = [IN]
      print $context
      assert $expected (fields $context)

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

      # SATISFIES after array field with multiple words
      let $context = "FROM hotel SELECT meta().id WHERE ANY like IN public_likes "
      let $expected = [SATISFIES]
      print $context
      assert $expected (fields $context)

      # Suggest nested fields after SATISFIES
      #let $context = 'FROM hotel SELECT meta().id WHERE ANY review IN reviews SATISFIES '
      #let $expected = [review.`author` review.`content` review.`date` review.ratings.`Business service` review.ratings.`Business service (e.g., internet access)` review.ratings.`Check in / front desk` review.ratings.`Cleanliness` review.ratings.`Location` review.ratings.`Overall`  review.ratings.`Rooms` review.ratings.`Service` review.ratings.`Sleep Quality` review.ratings.`Value`]
      #print $context
      #assert $expected (fields $context)

      # Suggest operators after nested field in SATISFIES clause
      let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN reviews SATISFIES review.ratings.`Service` '
      let $expected = $operators
      print $context
      assert $expected (fields $context)

      # Suggest END after SATISFIES completed
      let $context = 'FROM hotel SELECT meta().id WHERE EVERY review IN reviews SATISFIES review.something LIKE asdf '
      let $expected = [END]
      print $context
      assert $expected (fields $context)

    # Value for SATISFIES condition contains spaces
    let $context = "FROM hotel SELECT meta().id WHERE ANY like IN public_likes = 'Julius Tromp I' "
    let $expected = [END]
    print $context
    assert $expected (fields $context)
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
