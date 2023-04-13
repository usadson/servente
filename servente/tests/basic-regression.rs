// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! Basic integration testing for the servente executable. This requires
//! [cURL](https://curl.se/) to be installed, but it is practically installed
//! on every system: <https://curl.se/docs/companies.html>.

use std::{
    path::{Path, PathBuf},
    time::Duration,
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

static SERVENTE: OnceCell<Child> = OnceCell::const_new();

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

async fn invoke_curl(url: &str, args: &[&str]) -> Vec<u8> {
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

    assert!(res.status.success(), "cURL returned with status: {}, stderr: {}", res.status, String::from_utf8_lossy(&res.stderr));

    res.stdout
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

        servente
    }).await;

    let base_url = "https://localhost:8080/";
    ensure_file_matches(invoke_curl(base_url, args).await,
        "wwwroot/index.html");

    for entry in std::fs::read_dir(dir.join("wwwroot")).unwrap() {
        let entry = entry.unwrap();
        ensure_file_matches(invoke_curl(&format!("{base_url}{}", entry.file_name().to_string_lossy()), args).await,
            entry.path());
    }
}
