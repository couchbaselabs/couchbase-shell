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
         expected: [airline route landmark hotel airport]
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
        expected: [* "`airline`" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]
    }
    {
        # Suggest WHERE and all other fields after *
        input: "FROM route SELECT * "
        expected:  {options: {sort: false}, completions: [WHERE LIMIT "ORDER BY" "`airline`" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]}
    }
    {
        # Suggest list of fields and WHERE after SELECT meta().id
        input: "FROM route SELECT meta().id "
        expected:  {options: {sort: false}, completions: [WHERE LIMIT "ORDER BY" * "`airline`" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]}
    }
    {
        # Don't suggest used fields
        input: "FROM route SELECT `airline` `distance` `schedule` `type` "
        expected: {options: {sort: false}, completions: [WHERE LIMIT "ORDER BY" * "`airlineid`" "`destinationairport`" "`equipment`" "`id`" "`sourceairport`" "`stops`"]}
    }
    {
        # Correctly detect used fields when meta().id present
        input: "FROM route SELECT `airline` meta().id `distance` `schedule` `type` "
        expected: {options: {sort: false}, completions: [WHERE LIMIT "ORDER BY" * "`airlineid`" "`destinationairport`" "`equipment`" "`id`" "`sourceairport`" "`stops`"]}
    }
    {
        # Handle fields with spaces in the names
        input: "FROM route SELECT `airline` `space field` `distance` `schedule` `type` "
        expected:  {options: {sort: false}, completions: [WHERE LIMIT "ORDER BY" * "`airlineid`" "`destinationairport`" "`equipment`" "`id`" "`sourceairport`" "`stops`"]}
    }
]

const WHERE_tests = [
    {
        # Suggest all fields after WHERE
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE "
        expected: { options: {sort: false}, completions: ["ANY" "EVERY" "`airline`" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]}
    }
    {
        # Suggest operators after field
        input: "FROM route SELECT * WHERE `airline` "
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
         input: "FROM route SELECT * WHERE `some_field` "
         expected: $cli_operators
    }
    {
        # Suggest nothing after operator
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` == "
        expected: null
    }
    {
        # Suggest AND after condition
        input: "FROM route SELECT * WHERE distance < 100 "
        expected: [AND LIMIT]
    }
]

const AND_tests = [
    {
        # Suggest AND after a condition
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` == someAirline "
        expected: [AND LIMIT]
    }
    {
        # Suggest AND after a condition
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` < someAirline "
        expected: [AND LIMIT]
    }
    {
        # Suggest nothing after operator
        input: "FROM route SELECT * WHERE `airline` == someAirline AND `type` != "
        expected: null
    }
    {
        # Remove field once used in condition
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` == someAirline AND "
        expected: { options: {sort: false}, completions: ["ANY" "EVERY" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]}
    }
    {
        # Remove field once used in condition with different operator
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` < someAirline AND "
        expected: { options: {sort: false}, completions: ["ANY" "EVERY" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]}
    }
    {
        # Remove fields once used in condition
        input: "FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` == someAirline AND `type` == someType AND "
        expected: { options: {sort: false}, completions: ["ANY" "EVERY" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`"]}
    }
    {
        # Condition values/fields with spaces
        input: 'FROM route SELECT `airline` `distance` `schedule` `type` WHERE `airline` > "Best Airline" AND `some field` < "some things" AND '
        expected: { options: {sort: false}, completions: ["ANY" "EVERY" "`airlineid`" "`destinationairport`" "`distance`" "`equipment`" "`id`" "`schedule`" "`sourceairport`" "`stops`" "`type`"]}
    }
    {
        # Suggest operator after AND field with spaces
        input: "FROM route SELECT * WHERE `airline` == someAirline AND `some field` "
        expected: $cli_operators
    }
]

const LIMIT_tests = [
    {
        input: "FROM route SELECT * WHERE `airline` == someAirline LIMIT "
        expected: null
    }
]

const ANY_tests = [
    {
        input: 'FROM hotel SELECT meta().id WHERE ANY '
        expected: null
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE ANY r '
        expected: [IN]
    }
]

const EVERY_tests = [
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY '
        expected: null
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY r '
        expected: [IN]
    }
]

const IN_tests = [
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY review IN '
        expected: { options: {sort: false}, completions: ["`public_likes`" "`reviews`" " "]}
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY r IN reviews '
        expected: [SATISFIES]
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY something IN "some array"'
        expected: [SATISFIES]
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE ANY something IN some_array'
        expected: [SATISFIES]
    }
]

const SATISFIES_tests = [
    {
        # This test is flaky due to the output of the INFER query used being non-deterministic
        input: 'FROM hotel SELECT * WHERE ANY r IN `reviews` SATISFIES '
        expected:  { options: {sort: false}, completions: ["r.`author`" "r.`content`" "r.`date`" "r.ratings.`Business service (e.g., internet access)`"  "r.ratings.`Check in / front desk`" "r.ratings.`Cleanliness`" "r.ratings.`Location`" "r.ratings.`Overall`" "r.ratings.`Rooms`" "r.ratings.`Service`" "r.ratings.`Sleep Quality`" "r.ratings.`Value`" " "]}
    }
    {
        input: 'FROM hotel SELECT * WHERE ANY l IN `public_likes` SATISFIES '
        expected: [l]
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE ANY r IN `reviews` SATISFIES r.`author` '
        expected: $cli_operators
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY r IN reviews SATISFIES r.`author` == asdf '
        expected: [END]
    }
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY r IN reviews SATISFIES r.`author` == "some name" '
        expected: [END]
    }
]

const END_tests = [
    {
        input: 'FROM hotel SELECT meta().id WHERE EVERY r IN reviews SATISFIES r.`author` == asdf END '
        expected: [AND LIMIT]
    }
]

const ORDER_BY_tests = [
    {
        input: 'FROM landmark SELECT `city` ORDER BY '
        expected: { options: {sort: false}, completions: ["`city`" " "]}
    }
    {
        input: 'FROM landmark SELECT `some field` ORDER BY '
        expected: { options: {sort: false}, completions: ["`some field`" " "]}
    }
    {
        input: 'FROM landmark SELECT `city` ORDER BY `city` '
        expected: { options: {sort: false}, completions: [ASC DESC LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` ORDER BY '
        expected: { options: {sort: false}, completions: ["`city`" "`name`" " "]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` * ORDER BY '
        expected:  { options: {sort: false}, completions: ["`city`" "`name`" " "]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` WHERE `country` == "France" ORDER BY '
        expected: { options: {sort: false}, completions: ["`city`" "`name`" " "]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` WHERE `country` == France ORDER BY `city` '
        expected: { options: {sort: false}, completions: ["`name`" ASC DESC LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` WHERE `country` == France ORDER BY `city` `name`'
        expected: { options: {sort: false}, completions: ["`address`" ASC DESC LIMIT]}
    }
    # TO DO - SATISFIES AND ORDER BY tests
]

const ASC_tests = [
    {
        input: 'FROM landmark SELECT `city` ORDER BY `city` ASC '
        expected: { options: {sort: false}, completions: [LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` ORDER BY `city` ASC '
        expected: { options: {sort: false}, completions: ["`name`" LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` ORDER BY `city` ASC `name` '
        expected: { options: {sort: false}, completions: ["`address`" ASC DESC LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` ORDER BY `city` `name` ASC '
        expected: { options: {sort: false}, completions: ["`address`" LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` WHERE `some field` == "some value" ORDER BY `city` ASC '
        expected: { options: {sort: false}, completions: ["`name`" "`address`" LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` WHERE `country` == France ORDER BY `city` ASC `name` ASC '
        expected: { options: {sort: false}, completions: [LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `country` WHERE `country` == France ORDER BY `city` ASC '
        expected: { options: {sort: false}, completions: ["`country`" LIMIT]}
    }
]

const DESC_tests = [
    {
        input: 'FROM landmark SELECT `city` ORDER BY `city` DESC '
        expected: { options: {sort: false}, completions: [LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` ORDER BY `city` DESC '
        expected: { options: {sort: false}, completions: ["`name`" LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` ORDER BY `city` DESC `name` '
        expected: { options: {sort: false}, completions: ["`address`" ASC DESC LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` ORDER BY `city` `name` DESC '
        expected: { options: {sort: false}, completions: ["`address`" LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` `address` WHERE `some field` == "some value" ORDER BY `city` DESC '
        expected: { options: {sort: false}, completions: ["`name`" "`address`" LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `name` WHERE `country` == France ORDER BY `city` ASC `name` DESC '
        expected: { options: {sort: false}, completions: [LIMIT]}
    }
    {
        input: 'FROM landmark SELECT `city` `country` WHERE `country` == France ORDER BY `city` DESC '
        expected: { options: {sort: false}, completions: ["`country`" LIMIT]}
    }
]

export def main [] {
    let tests = [
        ...$FROM_tests
        ...$SELECT_tests
        ...$WHERE_tests
        ...$AND_tests
        ...$LIMIT_tests
        ...$ANY_tests
        ...$EVERY_tests
        ...$IN_tests
        ...$SATISFIES_tests
        ...$END_tests
        ...$ORDER_BY_tests
        ...$ASC_tests
        ...$DESC_tests
    ]

    for test in $tests {
        print $test.input
        assert $test.expected (fields $test.input)
    }
}