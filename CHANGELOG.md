# Change Log

All user visible changes to this project will be documented in this file.

## 1.0.0-beta.2 - Unreleased

 - Added support for `from bson` (also supports opening `bson` files directly).

## 1.0.0-beta.1 - 2021-07-15

 - Nushell pinned to 0.34
 - Added support for alternate addresses (enables all data commands for Couchbase Cloud)
 - Add `--with-meta` to `query indexes` and `query advise`
 - Added super simple `transactions list-atrs` support. needs a covered index for now to work.
 - Added support to configure cloud allow lists.
 - Added a `--silent` mode flag.
 - The `--clusters` option has been added to many more commands.
 - Custom port usage is now possible for bootstrap.
 - Hostnames are validated and parsed at startup.
 - Cloud secrets can now also be put in the credentials file.
 - Return an error if the `--clusters` flag does not return a single cluster.
 - Cloud support to the `nodes` command has been added.‚‚
 - The Message of the Day has been brought back.
 - Commands have been reordered so the `get` subcommands are now "at the toplevel.
 - Bundling the `fetch` plugin so now you can load any site/data you want.
 
## 1.0.0-alpha.2 - 2021-06-09

 - Nushell has been pinned to 0.32.0 for the next release.
 - fixed the history (now in the .cbsh dir as a `history.txt` file)
 - the linux builder has been switched from ubuntu 20.04 to 18.04 to be more conservative and target more linux users
 - renamed `[[clusters]]` to `[[cluster]]`, but kept the old style too for backwards compatibility
 - fix disabling tls in the config (property is now correctly called `tls-enabled`)
 - Added examples to: doc get
 - handle config syntax errors gracefully and log them nicely on startup
 - Added the `analytics links` command
 - Added the `analytics buckets` command
 - Added the `analytics pending-mutations` command
 - Support registering / unregistering clusters on the fly
 - Added new `[[cloud]]` config to support couchbase cloud
 - Supports getting, creating, updating and dropping buckets from couchbase cloud

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
