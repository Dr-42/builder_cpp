//! A library for building and packaging C and C++ projects.
//!
//! This library automatically configures various targets in your project
//! and gives an easy interface to grab packages from github.
//!
//! The library uses config_linux.toml or config_win32.toml file to configure the project.
//!
//! # Installation
//! To install this library, you need to have rust installed on your system.
//! ```no_run
//! cargo install builder_cpp
//! ```
//!
//! # Examples
//! To get the various flags that can be passed to builder_cpp
//! ```no_run
//! builder_cpp -h
//! ```
//! or
//! ```no_run
//! builder_cpp --help
//! ```
//!
//! # Sample toml files
//! Optional keys in toml are packages in build and deps in targets
//! Project contains an executable and a library from a github repo
//! ```toml
//! # config_linux.toml
//![build]
//!compiler = "g++"
//!packages = ["Dr-42/Nomu_Engine, master"]
//!
//![[targets]]
//!name = "main"
//!src = "./src"
//!include_dir = "./src"
//!type = "exe"
//!cflags = "-g -Wall "
//!libs = ""
//!deps = ["libengine"]
//!```
//! Projects contains a library and an executable
//!```toml
//! # config_win32.toml
//![build]
//!let compiler = "g++"
//!build_dir = "./bin"
//!obj_dir = "./obj_win"
//!
//![[targets]]
//!name = "libengine"
//!src = "./Engine/src/"
//!include_dir = "./Engine/src/include"
//!type = "dll"
//!cflags = "-g -Wall -Wunused `pkg-config --cflags freetype2` -std=c++17"
//!libs = "-lm -lglew32 -lglfw3 -lopengl32 -static-libstdc++ `pkg-config --libs freetype2`"
//!
//![[targets]]
//!name = "main"
//!src = "./Game/src/"
//!include_dir = "./Game/src"
//!type = "exe"
//!cflags = "-g -Wall"
//!libs = "-static-libstdc++"
//!deps = ["libengine"]
//!```

/// Contains code that handles various binary flags
pub mod bin_flags;
/// Contains code to build projects
pub mod builder;
/// Contains logger and config parser
pub mod utils;
/// Contains hashing related functions
pub mod hasher;
/// Handles global config
pub mod global_config;
