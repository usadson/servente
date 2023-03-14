// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    sync::Arc,
    time::Duration, path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use stretto::AsyncCache;
use tokio::io::AsyncReadExt;

use super::compression::ContentEncodedVersions;

/// The maximum size of a file that can be cached in memory.
const FILE_CACHE_MAXIMUM_SIZE: u64 = 50_000_000; // 50 MB

/// The default cache duration for files. This is 1 hour.
const DEFAULT_CACHE_DURATION: Duration = Duration::from_secs(60 * 60);

lazy_static! {
    pub static ref FILE_CACHE: AsyncCache<String, Arc<ContentEncodedVersions>> = AsyncCache::new(12960, 1e6 as i64, tokio::spawn).unwrap();
}

/// Caches all the applicable files on startup.
fn cache_files_on_startup(path: &Path) -> Result<(), std::io::Error> {
    for path in std::fs::read_dir(path)? {
        if let Ok(entry) = path {
            tokio::task::spawn(async move {
                maybe_cache_file(&entry.path()).await;
            });
        }
    }

    Ok(())
}

/// Initiated by a request that didn't have this file in cache. This function
/// will check for the right conditions and stores the file in the cache if
/// necessary.
pub async fn maybe_cache_file(path: &Path) {
    let start = std::time::Instant::now();
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return;
    };

    let path = path.to_owned();

    tokio::task::spawn(async move {
        if let Ok(metadata) = file.metadata().await {
            if metadata.len() > FILE_CACHE_MAXIMUM_SIZE {
                return;
            }

            let mut data = Vec::with_capacity(metadata.len() as usize);
            _ = file.read_to_end(&mut data).await;

            let cached = ContentEncodedVersions::create(data);
            FILE_CACHE.insert_with_ttl(path.to_string_lossy().to_string(), Arc::new(cached), 0, DEFAULT_CACHE_DURATION).await;
            println!("Cached file: {} in {} seconds", path.to_string_lossy(), (start.elapsed()).as_secs_f32());
        }
    });
}

fn remove_files_from_cache(paths: Vec<PathBuf>) {
    tokio::task::spawn(async move {
        for path in paths {
            FILE_CACHE.remove(&path.to_string_lossy().to_string()).await;
        }
    });
}

/// Starts the cache worker.
pub async fn start(path: &Path) {
    let path_for_startup = path.to_owned();
    tokio::task::spawn_blocking(move || {
        if let Err(err) = cache_files_on_startup(&path_for_startup) {
            #[cfg(debug_assertions)]
            eprintln!("Failed to cache files on startup: {}", err);

            #[cfg(not(debug_assertions))]
            { _ = err }
        }
    });

    #[cfg(feature = "watch")]
    start_watcher(path);
}

#[cfg(feature = "watch")]
fn start_watcher(path: &Path) {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        use notify::{RecursiveMode, Watcher};

        let Ok(mut watcher) = notify::recommended_watcher(|result| {
            if let Ok(event) = result {
                let event: notify::Event = event;

                if event.kind.is_remove() {
                    remove_files_from_cache(event.paths);
                    return;
                }

                if event.kind.is_create() || event.kind.is_modify() {
                    for path in event.paths {
                        tokio::task::spawn(async move {
                            maybe_cache_file(&path).await;
                        });
                    }
                }
            }
        }) else {
            return;
        };

        _ = watcher.watch(&path, RecursiveMode::Recursive);
    });
}
