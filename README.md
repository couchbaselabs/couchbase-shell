![CI](https://github.com/couchbaselabs/couchbase-shell/workflows/CI/badge.svg)

# cbsh - a new couchbase shell!

cbsh is a modern and fun shell for Couchbase Server and Cloud.

*Note that while the project is maintained by Couchbase, it is not covered under the EE support contract. We are providing community support through this bug tracker.*

# Usage

You can either build it from source if you want the latest and greatest or download (semi regular) binaries. Check the
[releases](https://github.com/couchbaselabs/couchbase-shell/releases) page for the latest binaries available for
your platform. For now we only build for linux and OSX.

For the latest and greatest, build for yourself:

Prerequisites:

 - Make sure to have a recent rust version installed (recommended: [rustup](https://rustup.rs/))
 - Depending on your platform you'll need some libraries installed through homebrew or apt etc.
 - This should work for ubuntu (`sudo apt-get install git libssl-dev pkg-config cmake libevent-dev libxcb-composite0-dev libx11-dev llvm-dev libclang-dev clang`)
 - On macOS make sure you brew install automake and brew install libtool
 - By default cbshell connects to the locahost instance of couchbase server, ensure that a local instance of couchbase server is installed and running.


Next, clone the repository and then run `cargo run`. The first time it will take some time since it builds all the dependencies, but you should end up in a shell. You can use `cargo run -- -h` to see all the available flags:

```
$ ./cbsh --help
The Couchbase Shell 0.5.0
Alternative Shell and UI for Couchbase Server and Cloud

USAGE:
    cbsh [FLAGS] [OPTIONS]

FLAGS:
    -h, --help        Prints help information
        --no-motd     
    -p, --password    
        --stdin       
    -V, --version     Prints version information

OPTIONS:
        --bucket <bucket>            
        --cert-path <cert-path>      
        --cluster <cluster>          
        --collection <collection>    
    -c, --command <command>          
        --hostnames <hostnames>      
        --scope <scope>              
        --script <script>            
    -u, --username <username>    
```

Note that if you want to spawn the highly experimental ui, use the `--ui` flag.

# Supported Commands

This is heavily in flux right now, but you can try these commands (always try with `--help` if you are unsure about args and flags).

 - `query <statement>`: Perform a N1QL query
 - `query indexes`: list query indexes
 - `query advise`: ask the query advisor
 - `analytics <statement>`: Perform an analytics query
 - `analytics dataverses`: List all dataverses
 - `analytics datasets`: List all datasets
 - `use`: Change the active bucket or cluster on the fly
 - `doc get`: Perform a KV get operation
 - `doc upsert`: Perform a KV upsert operation
 - `doc insert`: Perform a KV insert operation
 - `doc replace`: Perform a KV replace operation
 - `doc remove`: Removes a KV document
 - `nodes`: List all nodes in the active cluster
 - `buckets`: List all buckets in the active cluster
 - `clusters`: List and manage (active) clusters
 - `fake`: Generate fake/mock data
 - `users`: List all users
 - `users get`: show a specific user
 - `users upsert`: create a new user or replace one
 - `data stats`: displays kv service statistics
 - `search`: runs a query against a search index

# Config & Multiple Clusters

While quickly connecting with command line arguments is convenient, if you manage multiple clusters it can get tedious. 
For this reason, the shell supports managing multiple clusters at the same time. 
This works by adding a `.cbsh/config` file to either the path where you run the binary from, or more practical, from your home directory (`~/.cbsh/config`).

The format of the rc file is `toml`, and the default looks pretty much like this:

```toml
# Allows us to evolve in the future without breaking old config files
version = 1

[[clusters]]
identifier = "default"
hostnames = ["couchbase://127.0.0.1"]
default-bucket = "default"
username = "Administrator"
password = "password"
```

You can modify it so that it contains all the clusters you need:

```toml
# Allows us to evolve in the future without breaking old config files
version = 1

[[clusters]]
identifier = "cluster1"
hostnames = ["couchbase://10.143.193.101"]
default-bucket = "default"
username = "user1"
password = "pw1"

[[clusters]]
identifier = "cluster2"
hostnames = ["couchbase://10.143.193.102"]
default-bucket = "default"
username = "user2"
password = "pw2"
```

You can use the `clusters` command to list them:

```
❯ clusters
───┬────────┬──────────┬────────────────────────────┬───────────────
 # │ active │ name     │ connstr                    │ username 
───┼────────┼──────────┼────────────────────────────┼───────────────
 0 │ Yes    │ cluster1 │ couchbase://10.143.193.101 │ Administrator 
 1 │ No     │ cluster2 │ couchbase://10.143.193.102 │ Administrator 
───┴────────┴──────────┴────────────────────────────┴───────────────
```

By default the first alphabetically first one (in this case `cluster1`) will be active, but you can override this on the command line with `./cbsh --cluster=cluster2` for example. This allows you to store all cluster references in one rc file and then activate the one you need.

You can switch the cluster by identifier while being in the shell `clusters --activate identifier`.

# Installing into bin

If you just want to use it and don't want to bother compiling all the time, you can use `cargo install --path .` to install it into your cargo bin path.

```
/couchbase-shell$ cargo install --path .
  Installing couchbase-shell v0.1.0 (/couchbase-shell)
    Updating git repository `https://github.com/couchbaselabs/couchbase-rs`
    Updating crates.io index
   Compiling libc v0.2.66
   Compiling proc-macro2 v1.0.8
   *** SNIP ***
   Compiling heim-process v0.0.9
   Compiling heim v0.0.9
   Compiling couchbase-shell v0.1.0 (/couchbase-shell)
    Finished release [optimized] target(s) in 8m 10s
  Installing /.cargo/bin/cbsh
   Installed package `couchbase-shell v0.1.0 (/couchbase-shell)` (executable `cbsh`)
```

Grab a quick coffee or tea since this will take some time to compile (since it compiles it in *release* mode) but then it is available in your regular path like this:

```
/couchbase-shell$ cbsh
/couchbase-shell(master)> 
```
