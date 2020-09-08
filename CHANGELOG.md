# Change Log

All user visible changes to this project will be documented in this file.

## 1.0.0 - In Progress

 - Added simple `clusters health` check against a single cluster, two checks

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
