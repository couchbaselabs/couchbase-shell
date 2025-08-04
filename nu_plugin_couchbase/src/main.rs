use anyhow::{Context, Result};
use log::{error, warn};
use nu_plugin::{MsgPackSerializer, serve_plugin};
use tokio::runtime::Runtime;
use nu_couchbase::plugin::CouchbasePlugin;

fn main() -> Result<()> {
    env_logger::init();
    serve_plugin(
        &CouchbasePlugin::new(Runtime::new().context("Failed to create tokio runtime")?),
        MsgPackSerializer {},
    );
    Ok(())
}