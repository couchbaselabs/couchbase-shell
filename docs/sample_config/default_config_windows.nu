
def build_collection_prompt [] {
    let content = (cb-env)
    let user = $"($content | get display_name)"
    let cluster = $"($content | get database)"
    let bucket = $"($content | get bucket)"
    let scope = $"($content | get scope)"
    let collection = $"($content | get collection)"
    
    let collection_prompt = if $scope == "" and $collection == "" {
        ""
    } else {
        if $scope != "" and $collection == "" {
            '.' + ($scope) + '.<notset>'
        } else if $scope == "" and $collection != "" {
            '.<notset>.' + ($collection)
       } else {
            '.' + ($scope) + '.' + ($collection)
       }
    }
    
    let prompt = $"('(ansi ub) + ($user) + (ansi reset) + ' at ' + (ansi yb) + ($cluster) + (ansi reset) + ' in ' + (ansi wb) + ($bucket) + ($collection_prompt) + (ansi reset))"
    
    $prompt
}

let-env PROMPT_COMMAND = {build_collection_prompt}

let-env PROMPT_INDICATOR = " 
> "

let-env PROMPT_COMMAND_RIGHT = ""

let-env config = {
    show_banner: false
}
