# query_wizard.nu
const $operators = [LIKE = != > < >= <=]

def collection_fields [context: string] {
    let $collection = ($context | split words | $in.1)
    let $name_space = (cb-env | ["`" $in.bucket "`" . $in.scope . $collection] | str join)
    [infer $name_space] | str join " " | query $in | get properties | columns
}

def fields [context: string] {
     let $last = ($context | split row " " | drop | last)
     if ("WHERE" in ($context | split words)) {
        if ($last == "WHERE") {
            collection_fields $context
        } else {
            if ($last in ((collection_fields $context | append "meta().id"))) { $operators } else {
                if ($last not-in $operators) {
                    let $last_word = ($context | split words | last)
                    if ($last_word != AND) and ($last_word not-in (collection_fields $context)) {
                        [AND LIMIT]
                    } else {
                        let $where_index = ($context | split words | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})
                        let $after_where = ($context | split words | skip ($where_index.0 + 1))
                        collection_fields $context | each {|it| if ($it not-in $after_where) {$it}} | flatten
                    }
                }
            }
        }
     } else {
         match $last {
            FROM => {
                collections | get collection
            }
            * => [WHERE]
            WHERE => {}
            _ => {
                let $length = ($context | split words | length)
                match $length {
                    2 => [SELECT]
                    3 => {collection_fields $context | prepend *}
                    _ => {
                       let $used_fields = ($context | split words | skip 3)
                       let $unused_fields = (collection_fields $context | each {|it| if ($it not-in $used_fields) {$it}} | flatten | prepend WHERE)
                       if (($unused_fields | length) == 0) {
                            [WHERE]
                       } else {
                            $unused_fields
                       }
                    }
                }
            }
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

def assert [expected: list, result: list] {
    if (not ($expected == $result)) {
        print (ansi red_bold) failed (ansi reset)
        print "EXPECTED: " $expected
        print "RESULT: " $result
    } else {
        print (ansi green_bold) passed (ansi reset)
    }
}

export def fields_tests [] {

    # All fields test
    let $context = "FROM route SELECT "
    let $expected = [* airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Wildcard test
     let $context = "FROM route SELECT * "
     let $expected = [WHERE]
     print $context
     assert $expected (fields $context)

    # Don't suggest used fields
    let $context = "FROM route SELECT airline distance schedule type "
    let $expected = [WHERE airlineid destinationairport equipment id sourceairport stops]
    print $context
    assert $expected (fields $context)

    # Suggest all fields after where
    let $context = "FROM route SELECT airline distance schedule type WHERE "
    let $expected = [airline airlineid destinationairport distance equipment id schedule sourceairport stops type]
    print $context
    assert $expected (fields $context)

    # Suggest operators after WHERE field
    let $context = "FROM route SELECT airline distance schedule type WHERE airline "
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
    let $expected = [airlineid destinationairport distance equipment id schedule sourceairport stops]
    print $context
    assert $expected (fields $context)

    # Condition value with spaces
     let $context = 'FROM route SELECT airline distance schedule type WHERE airline = "Best Airline" '
     let $expected = [AND LIMIT]
     print $context
     assert $expected (fields $context)

     let $context = 'FROM route SELECT airline distance schedule type WHERE airline = "Best Airline" AND type = "some things" AND '
     let $expected = [airlineid destinationairport distance equipment id schedule sourceairport stops]
     print $context
     assert $expected (fields $context)

     # meta().id as condition value
     let $context = 'FROM hotel SELECT meta().id WHERE meta().id '
     let $expected = $operators
      print $context
      assert $expected (fields $context)
}
