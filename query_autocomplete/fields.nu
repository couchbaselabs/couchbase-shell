# key_words are n1ql query keywords
const $key_words = [FROM SELECT WHERE]

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
                return (collection_fields $collection | prepend *)
            }
            WHERE => {
                let $collection = ($context | split words | $in.1)
                return (collection_fields $collection)
            }
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
            let $remaining_fields = (collection_fields $collection | prepend * | each {|it| if ($it not-in $selected_fields) {$it}} | flatten)
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
    }
}

# collection_fields returns a list of the fields in a collection
# each field is wrapped in backticks to avoid thinking single field names containing spaces are multiple fields
export def collection_fields [collection: string] {
    let $name_space = (cb-env | ["`" $in.bucket "`" . $in.scope . $collection] | str join)
    let $infer_result = [infer $name_space] | str join " " | query $in
    ($infer_result | get properties | columns | each {|field| ["`" $field "`"] | str join})
}