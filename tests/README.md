What's going on here?
---------------------
Rust special cases the tests directory which makes root modules within in really difficult to work with, however nushell hardcodes `tests/fixtures` into creating a playground and errors without it.
This puts us in the position of having to call our tests directory tests but then having to create a subdirectory within it so it's easier to work with.
