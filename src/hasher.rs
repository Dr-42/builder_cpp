//! This module contains functions for hashing files and checking if they have changed.
use crate::utils::log::{log, LogLevel};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

// Hashes a file and returns the hash as a string.
fn hash_file(path: &str) -> String {
    let mut file = File::open(path).unwrap();
    const CHUNK_SIZE: usize = 1024 * 1024;

    let mut limit = file
        .metadata()
        .unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Failed to get length for file: {}", path),
            );
            log(LogLevel::Error, &format!("Error: {}", why));
            std::process::exit(1);
        })
        .len();
    let mut buffer = [0; CHUNK_SIZE];
    let mut hasher = Sha1::new();

    while limit > 0 {
        let read_size = if limit < CHUNK_SIZE as u64 {
            limit as usize
        } else {
            CHUNK_SIZE
        };
        let read = file.read(&mut buffer[0..read_size]).unwrap();
        if read == 0 {
            break;
        }
        limit -= read as u64;
        hasher.update(&buffer[0..read]);
    }
    let result = hasher.finalize();
    let mut hash = String::new();
    for byte in result {
        hash.push_str(&format!("{:02x}", byte));
    }
    hash
}

/// Returns the hash of a file if it exists in the path_hash.
/// Otherwise returns None.
/// # Arguments
/// * `path` - The path of the file to get the hash of.
/// * `path_hash` - The hashmap of paths and hashes.
pub fn get_hash(path: &str, path_hash: &HashMap<String, String>) -> Option<String> {
    if path_hash.contains_key(path) {
        Some(path_hash.get(path).unwrap().to_string())
    } else {
        None
    }
}

/// Loads the hashes from a file and returns them as a hashmap.
/// # Arguments
/// * `path` - The path of the file to load the hashes from.
pub fn load_hashes_from_file(path: &str) -> HashMap<String, String> {
    let mut path_hash: HashMap<String, String> = HashMap::new();
    let path = Path::new(path);
    if !path.exists() {
        return path_hash;
    }
    let mut file = OpenOptions::new().read(true).open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        let mut split = line.split(' ');
        let path = split.next().unwrap();
        let hash = split.next().unwrap();
        path_hash.insert(path.to_string(), hash.to_string());
    }
    path_hash
}

/// Saves the hashes to a file.
/// # Arguments
/// * `path` - The path of the file to save the hashes to.
/// * `path_hash` - The hashmap of paths and hashes.
pub fn save_hashes_to_file(path: &str, path_hash: &HashMap<String, String>) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .unwrap_or_else(|_| {
            log(LogLevel::Error, &format!("Failed to open file: {}", path));
            std::process::exit(1);
        });
    for (path, hash) in path_hash {
        let line = format!("{} {}\n", path, hash);
        file.write_all(line.as_bytes()).unwrap();
    }
}

/// Checks if a file has changed.
/// # Arguments
/// * `path` - The path of the file to check.
/// * `path_hash` - The hashmap of paths and hashes.
pub fn is_file_changed(path: &str, path_hash: &HashMap<String, String>) -> bool {
    let hash = get_hash(path, path_hash);
    if hash.is_none() {
        return true;
    }
    let hash = hash.unwrap();
    let new_hash = hash_file(path);
    hash != new_hash
}

/// Saves the hash of a file to the hashmap.
/// # Arguments
/// * `path` - The path of the file to save the hash of.
/// * `path_hash` - The hashmap of paths and hashes.
pub fn save_hash(path: &str, path_hash: &mut HashMap<String, String>) {
    let new_hash = hash_file(path);
    let hash = get_hash(path, path_hash);
    if hash.is_none() {
        path_hash.insert(path.to_string(), new_hash);
        return;
    }
    let hash = hash.unwrap();
    if hash != new_hash {
        log(
            LogLevel::Info,
            &format!("File changed, updating hash for file: {}", path),
        );
        path_hash.insert(path.to_string(), new_hash);
    }
}
