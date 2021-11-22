# Couchbase Shell - Shell Yeah!
![CI](https://github.com/couchbaselabs/couchbase-shell/workflows/CI/badge.svg)

Couchbase Shell (`cbsh`) is a modern, productive and fun shell for Couchbase Server and Cloud.

*Note that while the project is maintained by Couchbase, it is not covered under the EE support contract. We are providing community support through this bug tracker.*

The documentation is available [here](https://couchbase.sh/docs/).

## Quickstart

First, download the archive for your operating system.

 - [Linux 1.0.0-beta.3](https://github.com/couchbaselabs/couchbase-shell/releases/download/v1.0.0-beta.3/cbsh-1.0.0-beta.3-linux.tar.gz)
 - [macOS 1.0.0-beta.3](https://github.com/couchbaselabs/couchbase-shell/releases/download/v1.0.0-beta.3/cbsh-1.0.0-beta.3-mac.zip)
 - [Windows 1.0.0-beta.3](https://github.com/couchbaselabs/couchbase-shell/releases/download/v1.0.0-beta.3/cbsh-1.0.0-beta.3-windows.zip)

You do not need any extra dependencies to run `cbsh`, it comes "batteries included".

**macOS Users**: You will need to grant the binary permissions through `Security & Privacy` settings the first time you run it. 

After extracting the archive, run the `cbsh` binary in your terminal.

```
❯ ./cbsh --version
The Couchbase Shell 1.0.0-beta.3
```

## Basic Usage

Once the binary is available, you can connect to a cluster on the fly and run a simple command to list the (user-visible) buckets.

```
❯ ./cbsh --hostnames 127.0.0.1 -u username -p                       
Password: 
👤 username 🏠 default in 🗄 <not set>
> buckets
───┬─────────┬───────────────┬───────────┬──────────┬──────────────────────┬───────────┬───────────────┬────────┬───────
 # │ cluster │     name      │   type    │ replicas │ min_durability_level │ ram_quota │ flush_enabled │ status │ cloud 
───┼─────────┼───────────────┼───────────┼──────────┼──────────────────────┼───────────┼───────────────┼────────┼───────
 0 │ default │ beer-sample   │ couchbase │        1 │ none                 │  209.7 MB │ false         │        │ false 
 1 │ default │ default       │ couchbase │        1 │ none                 │  104.9 MB │ true          │        │ false 
 2 │ default │ targetBucket  │ couchbase │        0 │ none                 │  104.9 MB │ true          │        │ false 
 3 │ default │ travel-sample │ couchbase │        1 │ none                 │  209.7 MB │ false         │        │ false 
───┴─────────┴───────────────┴───────────┴──────────┴──────────────────────┴───────────┴───────────────┴────────┴───────
```

While passing in command-line arguments is fine if you want to connect quickly, using the dotfile `~/.cbsh/config` for configuration is much more convenient. Here is a simple config which connects to a cluster running on localhost:

```toml
version = 1

[[cluster]]
identifier = "my-local-cb-node"
hostnames = ["127.0.0.1"]
default-bucket = "travel-sample"
username = "Administrator"
password = "password"
```

After the config is in place, you can run `./cbsh` without any arguments and it will connect to that cluster after start automatically. 

The downloaded archive contains an `example` directory which also contains sample configuration files for more information. Also, please see the [docs](https://couchbase.sh/docs/) for full guidance, including information about how to work with multiple clusters at the same time.

# cbsh commands

On top of [nushell](https://www.nushell.sh/) built-in commands, the following couchbase commands are available:

 - `addresses` - List all whitelisted addresses (cloud)
 - `addresses add` - Add a whitelist address (cloud)
 - `addresses drop` - Remove a whitelist address (cloud)
 - `analytics <statement>` - Perform an analytics query
 - `analytics dataverses` - List all dataverses
 - `analytics datasets` - List all datasets
 - `analytics indexes` - List all analytics indexes
 - `analytics links` - List all analytics links
 - `analytics buckets` - List all analytics buckets
 - `analytics pending-mutations` - List pending mutations
 - `buckets` - Fetches buckets through the HTTP API
 - `buckets config` - Shows the bucket config (low level)
 - `buckets create` - Creates a bucket
 - `buckets drop` - Drops buckets through the HTTP API
 - `buckets flush` - Flushes buckets through the HTTP API
 - `buckets get` - Fetches a bucket through the HTTP API
 - `buckets load-sample` - Load a sample bucket
 - `buckets update` - Updates a bucket
 - `clouds` - List (active) clouds (cloud)
 - `clouds clusters` - List all cloud clusters in the active organization  (cloud)
 - `clouds clusters-create` - Create a cloud cluster in the active organization  (cloud)
 - `clouds clusters-drop` - Remove a cloud cluster in the active organization  (cloud)
 - `clouds clusters-Get` - Get a cloud cluster in the active organization  (cloud)
 - `clouds status` - List all clouds in the active organization (cloud)
 - `clusters` - List (active) clusters
 - `clusters health` - Performs health checks on the target cluster(s)
 - `collections` - Fetches collections through the HTTP API
 - `collections create` - Creates collections through the HTTP API
 - `collections drop` - Removes a collection
 - `doc get` - Perform a KV get operation
 - `doc insert` - Perform a KV insert operation
 - `doc remove` - Removes a KV document 
 - `doc replace` - Perform a KV replace operation
 - `doc upsert` - Perform a KV upsert operation
 - `fake` - Generate fake/mock data
 - `nodes` - List all nodes in the active cluster
 - `ping` - Ping available services in the cluster
 - `projects` - List all projects (cloud)
 - `projects create` - Create a new project (cloud)
 - `projects drop` - Remove a project (cloud)
 - `query <statement>` - Perform a N1QL query
 - `query indexes` - list query indexes
 - `query advise` - Ask the query advisor
 - `use` - Change the active bucket or cluster on the fly
 - `scopes` - Fetches scopes through the HTTP API
 - `scopes create` - Creates scopes through the HTTP API
 - `scopes drop` - Removes a scope
 - `search` - Runs a query against a search index
 - `transations list-atrs` - List all active transaction records (requires an index - create index id3 on `travel-sample`(meta().id, meta().xattrs.attempts)) 
 - `tutorial` - Runs you through a tutorial of both nushell and cbshell
 - `use` - lists the currently active bucket, collection, etc.
 - `use bucket` - Sets the active bucket based on its name
 - `use cloud` - Sets the active cloud based on its identifier
 - `use cloud-organization` - Sets the active cloud organization based on its identifier
 - `use cluster` - Sets the active cluster based on its identifier
 - `use collection` - Sets the active collection based on its name
 - `use project` - Sets the active cloud project based on its name
 - `use scope` - Sets the active scope based on its name
 - `users` - List all users
 - `users roles` - List roles available on the cluster
 - `users get` - Show a specific user
 - `users upsert` - Create a new user or replace one
 - `version` - Shows the version of the shell
 - `whoami` - Shows roles and domain for the connected user

## Building From Source

If you want to build from source, make sure you have a modern rust version and cargo installed (ideally through the [rustup](https://rustup.rs/) toolchain).

After that, you can build and/or run through `cargo build` / `cargo run`. By default it will build in debug mode, so if you want to build a binary and test the performance, make sure to include `--release`.

### Installing as a binary through cargo

If you just want to use it and don't want to bother compiling all the time, you can use `cargo install --path .` to install it into your cargo bin path (run from the checked out source directory).

```
❯ cargo install --path .
  Installing couchbase-shell v1.0.0-beta.3 (/Users/michaelnitschinger/couchbase/code/rust/couchbase-shell)
    Updating crates.io index
  Downloaded plist v1.2.1
  Downloaded onig v6.3.0
  Downloaded string_cache v0.8.2
  Downloaded num-bigint v0.4.2
  ...

```

Grab a quick coffee or tea since this will take some time to compile.
```
