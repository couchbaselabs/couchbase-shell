export use fields.nu
export use fields_tests.nu
export use format_query.nu
export use format_query_tests.nu


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



