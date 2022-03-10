extern crate notify;

use crate::file_index::FileIndex;
use crate::file_view::FileView;
use crate::index_store::IndexStore;
use crate::utils::subs;
use notify::{raw_watcher, Op, RawEvent, RecursiveMode, Watcher};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::{fs, path};

pub struct FsWatcher {
  index_store: Arc<IndexStore>,
  path: String,
}

impl FsWatcher {
  pub fn new(index_store: Arc<IndexStore>, path: String) -> FsWatcher {
    FsWatcher { index_store, path }
  }

  pub fn start(&mut self) {
    let (tx, rx) = channel();
    let mut watcher = raw_watcher(tx).unwrap();
    let wt_res = watcher.watch(self.path.as_str(), RecursiveMode::Recursive);
    if wt_res.is_err() {
      println!("watch {} err ", self.path);
      return;
    }

    let arc = self.index_store.clone();
    std::thread::spawn(move || {
      arc.commit();
      std::thread::sleep(Duration::from_secs(1));
    });

    loop {
      match rx.recv() {
        Ok(RawEvent {
          path: Some(path),
          op: Ok(op),
          cookie: _,
        }) => {
          if Op::REMOVE & op == Op::REMOVE {
            self.index_store.del(path.to_str().unwrap().to_string())
          };
          let result = path.metadata();
          match result {
            Ok(meta) => {
              let abs_path = path.to_str().unwrap().to_string();
              let name = Self::get_filename(&path);

              let created_at = Self::parse_ts(meta.created().unwrap());
              let mod_at = Self::parse_ts(meta.modified().unwrap());

              #[cfg(windows)]
              let size = meta.file_size();
              #[cfg(unix)]
              let size = meta.size();

              let is_dir = meta.is_dir();
              if is_dir {
                if let Some(path_str) = path.to_str() {
                  self.save_subs(path_str);
                }
              }
              if let Some(p) = path.parent() {
                if let Some(parent_str) = p.to_str() {
                  self.save_subs(parent_str);
                }
              }

              self.index_store.add_doc(FileIndex{ abs_path: &abs_path, name })

            }
            Err(_) => {}
          }
        }
        Ok(event) => println!("broken event: {:?}", event),
        Err(e) => println!("watch error: {:?}", e),
      }
    }
  }

  fn save_subs(&mut self, parent_str: &str) {
    let subs = subs(parent_str);
    for sub in subs {
      let sub_path = path::Path::new(sub.as_str());
      let name = sub_path
        .file_name()
        .map(|x| x.to_str().unwrap())
        .unwrap_or_default()
        .to_string();

      if let Ok(m) = sub_path.metadata() {
        self.index_store.add_doc(FileIndex { abs_path: sub.clone().as_str(), name });

      }
    }
  }

  fn get_filename(path: &PathBuf) -> String {
    let name = path
      .file_name()
      .map(|x| x.to_str().unwrap())
      .unwrap_or_default()
      .to_string();
    name
  }

  fn parse_ts(time: SystemTime) -> u64 {
    let created_at = time
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_millis() as u64;
    created_at
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use notify::watcher;
  use std::time::Duration;

  #[test]
  fn t1() {

    // let mut watcher = FsWatcher::new(
    //   ,
    //   "/Users/jeff/CLionProjects/orangemac/src-tauri/target".to_string(),
    // );
    // watcher.start();
  }
  #[test]
  fn t2() {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
      .watch(
        "/Users/jeff/CLionProjects/orangemac/src-tauri/target/hi",
        RecursiveMode::Recursive,
      )
      .unwrap();

    loop {
      match rx.recv() {
        Ok(event) => println!("{:?}", event),
        Err(e) => println!("watch error: {:?}", e),
      }
    }
  }
  #[test]
  fn t3() {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering raw events.
    // The notification back-end is selected based on the platform.
    let mut watcher = raw_watcher(tx).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch("/", RecursiveMode::Recursive).unwrap();

    loop {
      match rx.recv() {
        Ok(RawEvent {
          path: Some(path),
          op: Ok(op),
          cookie,
        }) => {
          println!("{:?} {:?} ({:?})", op, path, cookie)
        }
        Ok(event) => println!("broken event: {:?}", event),
        Err(e) => println!("watch error: {:?}", e),
      }
    }
  }
}
