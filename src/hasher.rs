//! This module contains functions for hashing files and checking if they have changed.
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use crate::utils::{log, LogLevel};
use std::collections::HashMap;
use md5;

// Hashes a chain of bytes and returns the hash as a string.
fn hash_u8(v: &[u8]) -> String {
    let digest = md5::compute(v);
    return format!("{:x}", digest);
}

// Hashes a file and returns the hash as a string.
fn hash_file(path: &str) -> String {
    let mut file = File::open(path).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();
    return hash_u8(&contents);
}

/// Returns the hash of a file if it exists in the path_hash.
/// Otherwise returns None.
/// # Arguments
/// * `path` - The path of the file to get the hash of.
/// * `path_hash` - The hashmap of paths and hashes.
pub fn get_hash(path: &str, path_hash: &HashMap<String, String>) -> Option<String> {
    if path_hash.contains_key(path) {
        return Some(path_hash.get(path).unwrap().to_string());
    }
    return None;
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
        let mut split = line.split(" ");
        let path = split.next().unwrap();
        let hash = split.next().unwrap();
        path_hash.insert(path.to_string(), hash.to_string());
    }
    return path_hash;
}

/// Saves the hashes to a file.
/// # Arguments
/// * `path` - The path of the file to save the hashes to.
/// * `path_hash` - The hashmap of paths and hashes.
pub fn save_hashes_to_file(path: &str, path_hash: &HashMap<String, String>) {
    let mut file = OpenOptions::new().write(true).create(true).open(path).unwrap_or_else(|_| {
        log(LogLevel::Error, &format!("Failed to open file: {}", path));
        std::process::exit(1);
    });
    for (path, hash) in path_hash {
        let line = format!("{} {}\n", path, hash);
        file.write(line.as_bytes()).unwrap();
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
    let result = hash != new_hash;
    result
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
        log(LogLevel::Info, &format!("File changed, updating hash for file: {}", path));
        path_hash.insert(path.to_string(), new_hash);
        return;
    }
}
