// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! Basic integration testing for the servente executable. This requires
//! [cURL](https://curl.se/) to be installed, but it is practically installed
//! on every system: <https://curl.se/docs/companies.html>.

use std::{
    path::{Path, PathBuf},
    time::Duration,
    sync::RwLock,
};

use rstest::rstest;
use tokio::{
    io::{
        AsyncBufReadExt,
        BufReader,
    },
    process::{
        Child,
        Command,
    },
    time::{timeout, sleep},
    sync::OnceCell,
};

use assert_cmd::prelude::*;

const CURL_STATUS_UNSUPPORTED_PROTOCOL: i32 = 1;
const CURL_STATUS_FAILED_TO_INITIALIZE: i32 = 2;
const CURL_STATUS_FEATURE_NOT_ENABLED: i32 = 4;

static SERVENTE: OnceCell<RwLock<Child>> = OnceCell::const_new();

fn cleanup() {
    _ = SERVENTE.get().unwrap().write().unwrap().start_kill();
}

/// Spawn servente as a background process.
fn spawn_servente<P>(working_directory: P) -> Child
        where P: AsRef<Path> {
    let working_directory = working_directory.as_ref();
    Command::from(std::process::Command::cargo_bin("servente").unwrap())
        .current_dir(working_directory)
        .kill_on_drop(true)
        // .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect(&format!("Failed to start servente in dir: {}", working_directory.display()))
}

async fn invoke_curl(url: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new("curl");

    command.arg("-k") // Insecure, since the certificate is self-signed
        .arg("-v") // Verbose
        .arg(url)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true);

    for arg in args {
        command.arg(arg);
    }

    let res = timeout(Duration::from_secs(8), async {
        command.spawn()
            .expect("Failed to invoke cURL")
            .wait_with_output()
            .await
    });

    let res = res.await
        .expect("cURL timed out!")
        .expect("Failed to wait_with_output");

    let status_message = format!("cURL returned with status: {}, stderr: {}", res.status, String::from_utf8_lossy(&res.stderr));

    // Not all systems ship with the same feature set. In this case, the test
    // should be ignored, but it definitely should print something in this case.
    //
    // In the future, we might implement some kind of hard error based on the
    // environment flags, something like: SERVENTE_CURL_ERROR_HARD_FAILURE.
    match res.status.code() {
        Some(CURL_STATUS_FAILED_TO_INITIALIZE) => {
            return Err(format!("Ignoring test: cURL failed to initialize: {}", status_message));
        }
        Some(CURL_STATUS_FEATURE_NOT_ENABLED) => {
            return Err(format!("Ignoring test: cURL feature not enabled: {}", status_message));
        }
        Some(CURL_STATUS_UNSUPPORTED_PROTOCOL) => {
            return Err(format!("Ignoring test: cURL unsupported protocol: {}", status_message));
        }
        _ => (),
    }

    assert!(res.status.success(), "{status_message}");

    Ok(res.stdout)
}

fn ensure_file_matches<P>(response_body: Vec<u8>, file: P)
        where P: AsRef<Path> {
    let file_contents = std::fs::read(file)
            .expect("Failed to retrieve homepage/index.html");
    assert_eq!(response_body, file_contents);
}

#[rstest]
#[case(&["--http1.0"])]
#[case(&["--http1.1"])]
#[case(&["--http1.1", "--raw"])]
#[case(&["--http1.1", "--compressed"])]
#[case(&["--http1.1", "--tr-encoding"])]
#[case(&["--http1.1", "--tlsv1.2"])]
#[case(&["--http1.1", "--tlsv1.3"])]
#[case(&["--http2"])]
#[case(&["--http2", "--tlsv1.2"])]
#[case(&["--http2", "--tlsv1.3"])]
#[tokio::test]
async fn homepage(#[case] args: &[&str]) {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests/homepage");

    std::env::set_current_dir(&dir)
        .expect("Failed to set the correct CWD");

    SERVENTE.get_or_init(|| async {
        let mut servente = spawn_servente(&dir);
        sleep(Duration::from_secs(3)).await;
        let buf = servente.stdout.take().unwrap();
        let mut buf = BufReader::new(buf);

        let mut string = String::new();
        timeout(Duration::from_secs(5), async {
            loop {
                buf.read_line(&mut string).await.unwrap();
                if string.contains("[servente] Ready.") {
                    break;
                }
                string.clear();
            }
        }).await.expect("Failed to start servente!");

        std::panic::set_hook(Box::new(|_| {
            cleanup();
            let _ = std::panic::take_hook();
        }));

        servente.into()
    }).await;

    let base_url = "https://localhost:8080/";
    match invoke_curl(base_url, args).await {
        Ok(response_body) => ensure_file_matches(response_body, "wwwroot/index.html"),
        Err(string) => eprintln!("{string}"),
    }

    for entry in std::fs::read_dir(dir.join("wwwroot")).unwrap() {
        let entry = entry.unwrap();
        match invoke_curl(&format!("{base_url}{}", entry.file_name().to_string_lossy()), args).await {
            Ok(response_body) => ensure_file_matches(response_body, entry.path()),
            Err(string) => eprintln!("{string}"),
        }
    }
}
