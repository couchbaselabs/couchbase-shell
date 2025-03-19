# key_words are n1ql query keywords
const $key_words = [FROM SELECT WHERE AND LIMIT ANY EVERY IN SATISFIES END]

# cli_operators are the operators that can be suggested to the user as custom completions
# the only difference between $operators and ¢cli_operators is the = -> ==. This is because =
# is reserved in nushell and cannot be used as an argument to FROM
const $cli_operators = [!= <= >= > < == LIKE]

# fields returns the list of valid next words given a partial query
export def main [context: string] {
    let $last = ($context | split row " " | drop | last)

    let $query_words = ($context | split words)
    let $last_word = ($query_words | last)
    let $used_keywords = ($query_words | each {|it| if ($it in $key_words) {$it}})
    let $last_keyword = ($used_keywords | last)

    if ($last_word == $last_keyword) and ($last != *) {
        match $last_word {
            FROM => {return (collections | get collection)}
            SELECT => {
                let $collection = ($context | split words | $in.1)
                return (collection_fields $collection | get fields | prepend *)
            }
            WHERE => {
                let $collection = ($context | split words | $in.1)
                return (collection_fields $collection | get fields | prepend [ANY EVERY])
            }
            AND => {
                # Need to suggest list of fields after WHERE that have not been used in conditions yet
                let $after_where = ($context | split row WHERE | last)
                let $conditions = ($after_where | split row AND | drop)
                # Here we iterate over the condition clauses and try to separate with each operator
                # If they contain an operator then we split the condition on that operator and return the first word, this will be the name of the field
                let $fields_in_conditions = ($conditions | each {|cond| $cli_operators | each {|op| if ($cond | str contains $op) {$cond | split row $op | first | str trim} } | first})
                let $collection = ($context | split words | $in.1)
                return (collection_fields $collection | get fields | filter {|x| $x not-in $fields_in_conditions} | prepend [ANY EVERY])
            }
            ANY => {return}
            EVERY => {return}
            IN => {
                let $collection = ($context | split words | $in.1)
                let $array_fields = (collection_fields $collection| filter {|it| $it.type == array} | get fields)
                # There is an issue with the nushell completions where nothing is displayed if all the completions start the same
                # So we append an empty space to the list of array fields to work around this for now
                return { options: {sort: false}, completions: ($array_fields | append " ")}
            }
            SATISFIES => {
                # When the last word is satisfies we need to return a list of fields of the objects in the array
                # appended to the alias of the array members given before IN

                # context will be "... review IN reviews SATISFIES " so we get alias, array from either side of "IN"
                let $in_index = ($context | split row " " | enumerate | each {|it| if ($it.item == IN) {$it.index}})
                let $alias = ($context | split row " " | get ($in_index.0 - 1))
                let $array = ($context | split row " " | get ($in_index.0 + 1) | str trim --char "`")

                let $collection = ($context | split words | get 1)

                # Gets the infer results for the JSON objects in the array
                # If the array is not an array of objects this will error, which we ignore and result in array_object_fields being empty
                let $array_object_fields = (do --ignore-errors {([infer $collection] | str join " " | query $in | get properties | get $array | get items | get properties)})

                # If there are no objects then we are looking at an array of primitives and should just return the alias
                if ($array_object_fields | is-empty) {
                       return [$alias]
                }

                # There is an issue with the nushell completions where nothing is displayed if all the completions start the same
                # So we append an empty space to the list of array fields to work around this
                let $formatted_array_fields = ($array_object_fields | columns | each {|it| $array_object_fields | get $it | columns | if ("properties" in $in) {$array_object_fields | get $it | get properties | columns | each {|prop| [$alias . $it . '`' $prop '`'] | str join }} else { [$alias . '`' $it '`'] | str join} } | flatten | append " ")
                return { options: {sort: false}, completions: $formatted_array_fields}
            }
            END => {return [AND LIMIT]}
        }
    }

    let $after_last_keyword = ($context | split row $last_keyword | last)

    match $last_keyword {
        FROM => {return [SELECT]}
        SELECT => {
            # Find the list of fields already specified, wrapping them in backticks so they can be matched with the output of
            # collection_fields
            let $selected_fields = ($after_last_keyword | split row "`" | each {|field| if (not ($field | str contains "*")) {["`" $field "`"] | str join} else {"*"}})
            let $collection = ($context | split words | $in.1)
            let $remaining_fields = (collection_fields $collection | get fields | prepend * | each {|it| if ($it not-in $selected_fields) {$it}} | flatten)
            return ($remaining_fields | prepend [WHERE LIMIT])
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
        IN => [SATISFIES]
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

# collection_fields returns a table with two columns:
#     fields - the name of the field in the documents in that collection
#     type - the type of the data contained in that field
export def collection_fields [collection: string] {
    let $name_space = (cb-env | ["`" $in.bucket "`" . $in.scope . $collection] | str join)
    let $infer_result = [infer $name_space] | str join " " | query $in
    let $fields = ($infer_result | get properties | columns)
    # For each field get its type from the results of the infer query and construct a table
    $fields | each {|it| $infer_result | get properties | get $it | get type | flatten | last | [[fields type]; [(["`" $it "`"] | str join) $in]]} | flatten
}