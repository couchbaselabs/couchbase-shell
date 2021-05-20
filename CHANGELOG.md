# Change Log

All user visible changes to this project will be documented in this file.

## 1.0.0-alpha.1 - 2021-05-20

 - Removed libcouchbase, cbshell is now pure rust and optimized for shell-type workloads.
 - TLS is turned on by default.
 - Reduced binary sizes, including windows.

## 0.5.0 - 2021-03-19

 - Updated documentation and examples
 - Bump nushell to 0.26
 - Allow to fetch query index definitions via (`query indexes --definitions`)
 - Add a `tutorial` command
 - Support for scope level query and analytics queries
 - (breaking) changed the cluster config format from list to map (#81)
 - Removed unused experimental UI for now
 - Added a custom `help` command
 - The `map` command has been removed since it does not work properly under windows
 - Added support for bucket management
 - Added support for scope and collection management
 - Log SDK output to a file and add a `sdklog` command (stored in `.cbsh`)
 - Added a `error` column to `doc get`
 - Added collection support to `doc` commands

## 0.4.0 - 2020-10-13

 - Added Windows support
 - Added a custom prompt
 - Some commands can now be interrupted with a `CTRL+C` command
 - Added simple `clusters health` check against a single cluster, two checks
 - Overall fixes and enhancements
 - Various Docs enhancements
 - `doc get --flatten` now works on nested rows as well
 - Added `users roles` subcommand
 - Added `clusters health` subcommand
 - Bumped nushell to 0.20
 - Added support for memcached buckets

## 0.3.0 - 2020-07-01

 - Renamed `kv` to `doc`
 - Added `data stats` command to display KV raw stats
 - Added `search` command to run an FTS query
 - Add `ping` command to ping all services
 - Fixed a bug where cloud node and buckets would not work
 - Bumped nushell to 0.16

## 0.2.0 - 2020-05-26

 - Added `whoami` command
 - Added `map` command
 - Converted `kv upsert` and `fake` to full streaming
 - Bumped dependencies (including nushell)
 - Added `expiry` option to `kv upsert`
 - Override `version` command to show cbsh version
 - Add simple `query advise` subcommand
 - Add user management through `users`, `users get` and `users upsert`
 - Added `analytics dataverses` and `analytics datasets`

## 0.1.0 - 2020-05-20

 * Build `libcouchbase` statically and with OpenSSL
 * More `fake` functions added
 * Added `kv-upsert`, `kv-insert` and `kv-replace`
 * Renamed `.cbshrc` to just  `.cbsh` (since it is not an rc file)
 * Added support for dynamic bucket usage

## 0.0.1 - 2020-04-03

 * Initial Release
