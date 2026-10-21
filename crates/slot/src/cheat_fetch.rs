//! Cheat files fetched for the carts that have none, with Home Wi-Fi on, on a worker of their
//! own. Each platform's list of the database's files is kept on the card for a week; a cart
//! the list holds nothing for is noted, so it is not asked after again until the list is new.
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime};

use slot_labels::cheats::{matches, Cheats};
use slot_labels::Error;
use slot_store::{Cart, Platform};

const CACHE: &str = "System/Cache/Cheats";
const INDEX_DAYS: u64 = 7;
const RETRY: Duration = Duration::from_secs(30);
const PACE: Duration = Duration::from_millis(500);

pub(crate) struct CheatFetch {
    // Dropping it wakes and ends the worker; a transfer in flight ends at its timeout.
    _stop: Sender<()>,
}

impl CheatFetch {
    pub fn spawn(root: PathBuf, carts: Vec<Cart>) -> Self {
        let (stop, stopped) = mpsc::channel();
        if let Err(e) = thread::Builder::new()
            .name("slot-cheats".into())
            .spawn(move || run(&root, carts, &stopped, &Cheats::default()))
        {
            eprintln!("slot: cheats: worker failed to start: {e}");
        }
        Self { _stop: stop }
    }
}

trait Source {
    fn index(&self, platform: Platform) -> Result<Vec<String>, Error>;
    fn file(&self, platform: Platform, name: &str) -> Result<Vec<u8>, Error>;
}

impl Source for Cheats {
    fn index(&self, platform: Platform) -> Result<Vec<String>, Error> {
        Cheats::index(self, platform)
    }
    fn file(&self, platform: Platform, name: &str) -> Result<Vec<u8>, Error> {
        Cheats::file(self, platform, name)
    }
}

fn index_path(root: &Path, platform: Platform) -> PathBuf {
    root.join(CACHE)
        .join(format!("{}.txt", platform.dir_name()))
}

fn checked_path(root: &Path, platform: Platform) -> PathBuf {
    root.join(CACHE)
        .join(format!("{}-checked.txt", platform.dir_name()))
}

/// The list kept on the card while it is under a week old. A clock that reads earlier than
/// the list was written (never set, or reset) keeps it too.
fn kept_index(root: &Path, platform: Platform) -> Option<Vec<String>> {
    let path = index_path(root, platform);
    let age = std::fs::metadata(&path).ok()?.modified().ok()?;
    let old = SystemTime::now()
        .duration_since(age)
        .is_ok_and(|d| d > Duration::from_secs(INDEX_DAYS * 24 * 3600));
    if old {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.lines().map(str::to_string).collect())
}

fn checked(root: &Path, platform: Platform) -> Vec<String> {
    std::fs::read_to_string(checked_path(root, platform))
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn note_checked(root: &Path, platform: Platform, stem: &str) {
    let mut list = checked(root, platform);
    list.push(stem.to_string());
    let _ = slot_store::atomic_write(
        &checked_path(root, platform),
        (list.join("\n") + "\n").as_bytes(),
    );
}

/// The platforms a cart's cheats may be filed under: its own first, then the other Game Boy,
/// as the database files a dual-mode game under one of them.
fn shelves(platform: Platform) -> &'static [Platform] {
    match platform {
        Platform::Gb => &[Platform::Gb, Platform::Gbc],
        Platform::Gbc => &[Platform::Gbc, Platform::Gb],
        Platform::Gba => &[Platform::Gba],
    }
}

/// Waits out a pause; false when the worker is told to stop meanwhile.
fn wait(stop: &Receiver<()>, d: Duration) -> bool {
    stop.recv_timeout(d) == Err(mpsc::RecvTimeoutError::Timeout)
}

fn run(root: &Path, carts: Vec<Cart>, stop: &Receiver<()>, source: &dyn Source) {
    let _ = std::fs::create_dir_all(root.join(CACHE));
    let mut indexes: HashMap<Platform, Vec<String>> = HashMap::new();
    let mut failures = 0u8;
    let mut done: Vec<(Platform, String)> = Vec::new();
    for cart in carts {
        // A game on two shelves (its own and Recents or Favorites) is asked after once.
        if done.contains(&(cart.platform, cart.stem.clone())) {
            continue;
        }
        if crate::cheats::has_files(root, cart.platform, &cart.stem)
            || checked(root, cart.platform).contains(&cart.stem)
        {
            continue;
        }
        let mut found = Vec::new();
        let mut offline = false;
        for &shelf in shelves(cart.platform) {
            if let Entry::Vacant(slot) = indexes.entry(shelf) {
                let index = match kept_index(root, shelf) {
                    Some(index) => index,
                    None => loop {
                        match source.index(shelf) {
                            Ok(index) => {
                                let _ = slot_store::atomic_write(
                                    &index_path(root, shelf),
                                    (index.join("\n") + "\n").as_bytes(),
                                );
                                // A new list may hold what the old one did not.
                                let _ = std::fs::remove_file(checked_path(root, shelf));
                                break index;
                            }
                            Err(Error::Network(e)) => {
                                failures += 1;
                                eprintln!("slot: cheats: {e}");
                                if failures >= 3 || !wait(stop, RETRY) {
                                    return;
                                }
                            }
                            Err(e) => {
                                eprintln!("slot: cheats: {}: {e}", shelf.dir_name());
                                break Vec::new();
                            }
                        }
                    },
                };
                slot.insert(index);
            }
            let names = matches(&indexes[&shelf], &cart.stem);
            if !names.is_empty() {
                found = names.into_iter().map(|n| (shelf, n)).collect();
                break;
            }
        }
        if found.is_empty() {
            note_checked(root, cart.platform, &cart.stem);
            done.push((cart.platform, cart.stem));
            continue;
        }
        let dir = root.join("Cheats").join(cart.platform.dir_name());
        let _ = std::fs::create_dir_all(&dir);
        for (shelf, name) in found {
            let target = dir.join(&name);
            if target.exists() {
                continue;
            }
            match source.file(shelf, &name) {
                Ok(bytes) => {
                    if let Err(e) = slot_store::atomic_write(&target, &bytes) {
                        eprintln!("slot: cheats: {}: {e}", target.display());
                    }
                }
                Err(Error::Network(e)) => {
                    eprintln!("slot: cheats: {name}: {e}");
                    offline = true;
                    break;
                }
                Err(e) => eprintln!("slot: cheats: {name}: {e}"),
            }
            if !wait(stop, PACE) {
                return;
            }
        }
        if offline {
            failures += 1;
            if failures >= 3 || !wait(stop, RETRY) {
                return;
            }
        }
        done.push((cart.platform, cart.stem));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        index: Vec<String>,
        asked: RefCell<Vec<String>>,
        offline: bool,
    }

    impl Source for Fake {
        fn index(&self, _: Platform) -> Result<Vec<String>, Error> {
            self.asked.borrow_mut().push("index".into());
            if self.offline {
                return Err(Error::Network("offline".into()));
            }
            Ok(self.index.clone())
        }
        fn file(&self, _: Platform, name: &str) -> Result<Vec<u8>, Error> {
            self.asked.borrow_mut().push(name.into());
            Ok(b"cheats = 1\ncheat0_desc = \"Max money\"\ncheat0_code = \"x\"\n".to_vec())
        }
    }

    fn cart(root: &Path, platform: Platform, stem: &str) -> Cart {
        Cart {
            platform,
            stem: stem.into(),
            rom: root.join(format!("{stem}.gba")),
            label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
            marker: None,
        }
    }

    #[test]
    fn a_new_cart_gets_its_files_once_and_a_cart_with_none_is_not_asked_again() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let fake = Fake {
            index: vec![
                "Game (USA) (Code Breaker).cht".into(),
                "Other (USA).cht".into(),
            ],
            asked: RefCell::default(),
            offline: false,
        };
        let carts = vec![
            cart(root, Platform::Gba, "Game (USA)"),
            cart(root, Platform::Gba, "Game (USA)"),
            cart(root, Platform::Gba, "Homebrew"),
        ];
        let (_stop, stopped) = mpsc::channel();
        run(root, carts.clone(), &stopped, &fake);
        assert_eq!(
            *fake.asked.borrow(),
            ["index", "Game (USA) (Code Breaker).cht"]
        );
        assert!(crate::cheats::has_files(root, Platform::Gba, "Game (USA)"));
        assert!(!crate::cheats::load(root, Platform::Gba, "Game (USA)").is_empty());

        fake.asked.borrow_mut().clear();
        run(root, carts, &stopped, &fake);
        assert!(
            fake.asked.borrow().is_empty(),
            "nothing new to ask: {:?}",
            fake.asked.borrow()
        );
    }

    #[test]
    fn a_worker_told_to_stop_does_not_try_again() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let fake = Fake {
            index: Vec::new(),
            asked: RefCell::default(),
            offline: true,
        };
        let (stop, stopped) = mpsc::channel();
        // A stop already sent ends the waits between tries at once.
        drop(stop);
        run(
            root,
            vec![cart(root, Platform::Gba, "Game")],
            &stopped,
            &fake,
        );
        assert_eq!(fake.asked.borrow().len(), 1);
    }
}
