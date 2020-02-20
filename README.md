# cbsh - a new couchbase shell!

This project is a work in progress, come back later.

# Usage

If you want to run it right now, the bad news is you have to build it yourself.

Prerequisites:

 - Make sure to have a recent rust version installed (recommended: [rustup](https://rustup.rs/))
 - Install libcouchbase 3.0.0

Next, clone the repository and then run `cargo run`. The first time it will take some time since it builds all the dependencies, but you should end up in a shell. You can use `cargo run -- -h` to see all the available flags:

```
couchbase-shell$ cargo run -- -h
   Compiling couchbase-shell v0.1.0 (/Users/michaelnitschinger/couchbase/code/rust/couchbase-shell)
    Finished dev [unoptimized + debuginfo] target(s) in 13.54s
     Running `target/debug/cbsh -h`
The Couchbase Shell 0.1.0
Alternative Shell and UI for Couchbase Server and Cloud

USAGE:
    cbsh [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
        --ui         
    -V, --version    Prints version information

OPTIONS:
    -c, --connstring <connection-string>     [default: couchbase://localhost]
    -p, --password <password>                [default: password]
    -u, --username <username>                [default: Administrator]
````

Note that if you want to spawn the ui, use the `--ui` flag.