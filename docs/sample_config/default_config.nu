
def build_collection_prompt [] {
    let content = (cb-env)
    let user = $"($content | get display_name)"
    let cluster = $"($content | get cluster)"
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

    let bucket_symbol = if $cluster_type == "provisioned" {
        "☁️"
    } else {
        "🗄"
    }

    let prompt = $"('👤 ' + (ansi ub) + ($user) + (ansi reset) + ' 🏠 ' + (ansi yb) + ($cluster) + (ansi reset) + ' in ' + ($bucket_symbol) + ' ' + (ansi wb) + ($bucket_name) + ($collection_prompt) + (ansi reset))

"

    $prompt
}

$env.PROMPT_COMMAND = {build_collection_prompt}

$env.PROMPT_COMMAND_RIGHT = ""
$env.config = {
    show_banner: false
}
