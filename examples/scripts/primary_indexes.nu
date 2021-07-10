# == Prints the names of all primary indexes ==
#
# This is a very simple example and showcases how to use variables. Of course
# this can easily be coerced into a one-liner, but then how would we show
# variable usage? ;-)
#
# Run with: ./cbsh --script examples/scripts/primary_indexes.nu

# Get all query ndexes from the current active cluster
let indexes = (query indexes)

# Print the name of all indexes where the primary column is true
$indexes | where primary == $true | select name
