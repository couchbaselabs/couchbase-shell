# == Lists the ops per sec for each bucket, sorted by highest ==
#
# Sample Output:
# ───┬────────────────┬───────────
#  # │      name      │ opsPerSec 
# ───┼────────────────┼───────────
#  0 │ travel-sample  │     14998 
#  1 │ gamesim-sample │         0 
#  2 │ foo            │         0 
#  3 │ beer-sample    │         0 
# ───┴────────────────┴───────────
#
# Run with: ./cbsh --script examples/scripts/buckets_ops_per_sec.nu


# Get the names of all buckets on the active cluster
let bucket_names = (buckets | get name)

# For each name, load the full config and grab the basic stats (which
# contain the opsPerSec property)
let opsPerSec = ($bucket_names | each { |name|
    let basic_stats = (buckets config $name | get basicStats)
    
    # Return a simple table of two columns, the name and the ops per sec
    [[name opsPerSec]; [$name $basic_stats.opsPerSec]]
})

# Print the table, but sort by opsPerSec and make sure highest is on top
echo $opsPerSec | sort-by opsPerSec -r