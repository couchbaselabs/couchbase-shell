
def build_collection_prompt [] {
    let content = (cb-env)
    let user = $"($content | get display_name)"
    let cluster = $"($content | get database)"
    let bucket = $"($content | get bucket)"
    let scope = $"($content | get scope)"
    let collection = $"($content | get collection)"
    let cluster_type = $"($content | get cluster_type)"
    
    let bucket_name = if $bucket == "" {
        "<not-set>"
    } else {
        $bucket
    }
    
    let collection_prompt = if $bucket_name == "" or ($scope == "" and $collection == "") {
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
    
    let prompt = $"(($user) + ' at ' + ($cluster) + ' in ' + ($bucket_name) + ($collection_prompt))"
    
    $prompt
}

let-env PROMPT_COMMAND = {build_collection_prompt}

let-env PROMPT_INDICATOR = " 
> "

let-env PROMPT_COMMAND_RIGHT = ""

let-env config = {
    show_banner: false
}
