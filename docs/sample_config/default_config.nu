
export def build_collection_prompt [] {
    let content = (cb-env)
    let user = $"($content | get display_name)"
    let cluster = $"($content | get cluster)"
    let bucket = $"($content | get bucket)"
    let scope = $"($content | get scope)"
    let collection = $"($content | get collection)"
    let cluster_type = $"($content | get cluster_type)"

    let bucket_symbol = if $cluster_type == "provisioned" {
        "☁️"
    } else {
        "🗄"
    }

    let bucket_prompt = if $bucket == "" {
        ""
    } else {
       ' in ' + ($bucket_symbol) + ' ' + (ansi wb) + ($bucket)
    }

    let collection_prompt = if $bucket_prompt == "" {
        ""
    } else {
        let scope_name = if $scope == "" {
            '._default'
        } else {
            '.' + $scope
        }

        let col_name = if $collection == "" {
            '._default'
        } else {
            '.' + $collection
        }
        $"($scope_name + $col_name)"
    }

    let prompt = $"('👤 ' + (ansi ub) + ($user) + (ansi reset) + ' 🏠 ' + (ansi yb) + ($cluster) + (ansi reset) + ($bucket_prompt) + ($collection_prompt) + (ansi reset))

"

    $prompt
}
