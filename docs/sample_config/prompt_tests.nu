use default_config.nu build_collection_prompt
use std assert

# Mock the cb-env command used by build_collection_prompt
def cb-env [] {{display_name: my-name cluster: my-cluster bucket: my-bucket scope: my-scope collection: my-collection cluster_type: provisioned}}
let prompt = build_collection_prompt
let expected_prompt = $"('👤 ' + (ansi bb) + 'my-name' + (ansi reset) + ' 🏠 ' + (ansi yb) + 'my-cluster' + (ansi reset) + ' in ☁️ ' + (ansi wb) + 'my-bucket.my-scope.my-collection' + (ansi reset))
"
assert equal ($prompt) $expected_prompt

use default_config_windows.nu build_collection_prompt
let prompt = build_collection_prompt
let expected_windows_prompt = $"('my-name' + ' at ' + 'my-cluster' + ' in ' + 'my-bucket.my-scope.my-collection')
"
assert equal ($prompt) $expected_windows_prompt