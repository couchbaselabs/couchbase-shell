# operators are the operators that can be used in conditions in a where clause
const $operators = [!= <= >= > < = LIKE]

# format_query takes the list of inputs generated by the FROM custom completions and turns them into a valid n1ql query string
export def main [inputs: list] {
    # The brackets in meta().id get dropped so we need to add them back in
    # Also == needs to be replaced with =
    let $inputs = ($inputs | each {|it| if ($it == "meta.id") {"meta().id"} else if ($it == "==") {"="} else {$it}})

    # Find the index of WHERE and then split the input into before and after the WHERE clause
    mut $where_index = ($inputs | length)
    for $it in ($inputs | enumerate) {
        if $it.item == WHERE {
            $where_index = $it.index
            break
        }
    }
    # let $where_index = ($inputs | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})
    mut $where_clause = []
    if $where_index != ($inputs | length) {
        let $after_where = ($inputs | skip ($where_index + 1))

        # The conditions need to be parsed and any string condition values wrapped in speech marks
        let $parsed_after_where = (format_after_where $inputs)
        $where_clause = ([WHERE] | append $parsed_after_where)
    }

    # Extract the fields to be returned from the document and format them
    let $return_fields = ($inputs | drop ($inputs | length | $in - $where_index) | skip 2)
    let $formatted_return_fields = format_return_fields $return_fields

    # Finally construct the query from the starting section and the parsed conditions
    let $query = [SELECT $formatted_return_fields FROM $inputs.0] | append $where_clause | str join " "
    return $query
}

# parse_after_where is responsible for correctly formatting the condition values by adding speech marks around any strings
# this is done by iterating over all of the fields after WHERE and determining the strings
def format_after_where [fields: list] {
    # Find the index of WHERE
    let $where_index = ($fields | enumerate | each {|it| if ($it.item == WHERE) {$it.index}})

    # Get the strings after WHERE, since this will hold all the conditions
    let $after_where = ($fields | skip ($where_index.0 + 1))

    # Get all of the values in the condition clauses, the values are determined as being the items that follow operators
    # return a table containing the index of the value in $after_where and the corresponding value
    let $condition_values = ($after_where | enumerate | each {|it| if ($it.item in $operators) {($after_where | get ($it.index + 1)) | [[index value]; [($it.index + 1) $in]]}}) | flatten

    # Get all of the fields in the condition clauses, the values are determined as being the items that proceed operators
    # return a table containing the index of the field names in $after_where and the corresponding field name wrapped in backticks
    # unless the field name already contains backticks then it is part of a SATISFIES clause and should not be wrapped again
    mut $condition_fields = []
    for it in ($after_where | enumerate) {
        if ($it.item in $operators)  {
            let $field = ($after_where | get ($it.index - 1))
            if (not ($field | str contains "`")) {
                  let $formatted = [[index value]; [($it.index - 1) (["`" $field "`"] | str join)]]
                  $condition_fields = ($condition_fields | append $formatted)
            }
        }
    }

    mut $formatted_after_where = $after_where
    for item in $condition_values {
        # Iterate over all condition values and wrap any non-numbers (strings) in quote marks
        let $formatted_value = (if (is_number $item.value) {$item.value} else {['"' $item.value '"'] | str join})
        $formatted_after_where = ($formatted_after_where | update $item.index $formatted_value)
    }

    for $item in $condition_fields {
        $formatted_after_where = ($formatted_after_where | update $item.index $item.value)
    }

    $formatted_after_where
}

# format_return_fields takes the list of fields to be returned from the documents as a list and formats them into a single string by:
#   1) Wrapping all field names in backticks, required if they contain a space
#   2) Concatenates fields into a single comma seperated string
def format_return_fields [fields: list] {
    # We format the last field separately since we don't want the final field followed by a comma
    # Also we cannot wrap * or meta().id in backticks else the query will fail
    let $formatted_last_field = ($fields | last | if ($in in [* "meta().id"]) {$in} else {["`" $in "`"] | str join})
    let $other_fields = ($fields | drop)
    let $formatted_other_fields = ($other_fields | each {|it| if ($in in [* "meta().id"]) {[$it ","]} else {["`" $it "`" ","]} | str join})
    ($formatted_other_fields | append $formatted_last_field | str join " ")
}

# is_number determines if a string is actually a string representation of an int or float
def is_number [possible_string: string] {
    # Try to convert the input to an int to see if it is actually a string representation of an int/float
    let $possible_int = (do --ignore-errors { $possible_string | into int })
    ($possible_int != null)
}