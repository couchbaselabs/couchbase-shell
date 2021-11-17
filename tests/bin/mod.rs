use bytes::Buf;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, fs};
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::tcp::ReadHalf;
use tokio::net::TcpListener;
use uuid::Uuid;

#[cfg(target_os = "windows")]
const CAVES_BINARY: &str = "gocaves-windows.exe";
#[cfg(target_os = "macos")]
const CAVES_BINARY: &str = "gocaves-macos";
#[cfg(target_os = "linux")]
const CAVES_BINARY: &str = "gocaves-linux";

const CAVES_URL: &str = "https://github.com/couchbaselabs/gocaves/releases/download";
const CAVES_VERSION: &str = "v0.0.1-38";

async fn fetch_caves(path: &PathBuf) {
    let response =
        reqwest::get(format!("{}/{}/{}", CAVES_URL, CAVES_VERSION, CAVES_BINARY).as_str())
            .await
            .unwrap();

    if !response.status().is_success() {
        panic!("Response failed: {}", response.status())
    }

    let mut file = File::create(path).await.expect("Failed to create file");

    let content = response
        .bytes()
        .await
        .expect("Failed to read response into bytes");

    file.write_all(content.chunk())
        .await
        .expect("Failed to write response data to file");
    drop(file);

    set_permissions(path);
}

#[cfg(target_os = "windows")]
fn set_permissions(_path: &PathBuf) {}

#[cfg(not(target_os = "windows"))]
fn set_permissions(path: &PathBuf) {
    let meta = fs::metadata(path).expect("Failed to get file metadata");
    let mut perms = meta.permissions();
    perms.set_mode(0o744);
    fs::set_permissions(&path, perms).expect("Failed to set file permissions");
}

#[derive(Serialize, Deserialize)]
struct CreateConfig {
    #[serde(rename(serialize = "type"))]
    typ: String,
    id: String,
}

fn start_caves(path: &PathBuf, port: u16) -> Child {
    let mut cmd = Command::new(path);

    // Caves outputs a lot of info, we need to redirect this so that our tests don't pick it up
    // on stdout.
    cmd.arg("-control-port")
        .arg(format!("{}", port))
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("Failed to spawn child process for caves")
}

#[tokio::main]
async fn main() {
    let path = std::env::temp_dir().join(Path::new(CAVES_BINARY));
    if path.exists() {
        println!(
            "Found existing caves binary for {} at {}",
            CAVES_VERSION,
            path.to_string_lossy()
        );
    } else {
        println!(
            "Fetching caves {} to {}",
            CAVES_VERSION,
            path.to_string_lossy()
        );
        fetch_caves(&path).await;
        println!("Fetched caves");
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind tcp listener");
    let port = listener
        .local_addr()
        .expect("Failed to get local addr from listener")
        .port();

    println!("Caves starting");
    let mut caves = start_caves(&path, port);
    println!("Caves started");

    let (mut stream, _) = listener
        .accept()
        .await
        .expect("Failed to accept connection");
    println!("Caves connected");

    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    let hello_msg = read_from_stream(&mut buf_reader).await;
    println!("Received hello {}", hello_msg);

    let data = CreateConfig {
        typ: "createcluster".to_string(),
        id: Uuid::new_v4().to_string(),
    };

    println!("Sending createcluster request");
    let mut cluster_req_data =
        serde_json::to_vec(&data).expect("Failed to serialize request data to json");
    cluster_req_data.push(0);
    writer
        .write_all(&cluster_req_data)
        .await
        .expect("Failed to write data");
    println!("Sent createcluster request");

    let create_msg = read_from_stream(&mut buf_reader).await;
    println!("Received create cluster response {}", &create_msg);

    let addr = parse_create_cluster_response(create_msg);
    println!("Setting hostnames to {}", &addr);

    env::set_var("CBSH_HOSTNAMES", addr);
    // env::set_var("RUST_LOG", "debug");

    let output = Command::new("cargo")
        .arg("test")
        .spawn()
        .expect("Failed to spawn process for cargo test");

    let result = output
        .wait_with_output()
        .expect("Failed to wait for output for cargo test");

    drop(listener);
    match caves.kill() {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to kill gocaves instance: {}", e);
        }
    };

    assert!(result.status.success());
}

fn parse_create_cluster_response(msg: String) -> String {
    let j: HashMap<String, Value> =
        serde_json::from_str(msg.as_str()).expect("Failed to parse json response");

    let addrs = j
        .get("mgmt_addrs")
        .expect("Response did not have mgmt_addrs field");
    let mut hosts: Vec<String> = Vec::new();
    for addr in addrs.as_array().unwrap() {
        let addr_str = addr.as_str().unwrap();
        hosts.push(
            addr_str
                .strip_prefix("http://")
                .unwrap_or(addr_str)
                .to_string(),
        );
    }
    assert!(!hosts.is_empty());

    hosts[0].clone()
}

async fn read_from_stream(buf_reader: &mut BufReader<ReadHalf<'_>>) -> String {
    let mut buf = vec![];
    buf_reader
        .read_until(0, &mut buf)
        .await
        .expect("Failed to read from stream");

    let terminator_removed = &buf[0..buf.len() - 1];

    let msg = String::from_utf8_lossy(terminator_removed);
    msg.into()
}
