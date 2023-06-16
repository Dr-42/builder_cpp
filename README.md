# BUILDER_CPP

A simple build tool for building C and C++ applications

The tool is still in development. Do not use for production code.

## Installation

The tool requires cargo for installation
```console
cargo install builder_cpp
```
For subcommands run with -h flag

## Features

- [x] Create new project
- [x] Multithreaded
- [x] Can generate compile_commands.json
- [x] Can generate .vscode/c_cpp_properties.json
- [x] Auto add project libraries to other targets
- [x] Get libraries as packages from github

## Usage
Write a config_win32.toml for windows and config_linux.toml for linux

To create a new project 
```console
builder_cpp --init <project-name> [--c|--cpp]
```

For help
```console
builder_cpp --help
```

The help command will show you the following
```sh
[LOG] Usage: $ builder_cpp <options>
[LOG] Options:
[LOG]   -c              Clean the build directory
[LOG]   -r              Run the executable
[LOG]   -b              Build the project
[LOG]   -h              Show this help message
[LOG]
[LOG]   --help                  Show this help message
[LOG]   --init <project name> [--c|--cpp]       Initialize the project. Default is C++
[LOG]   --bin-args <args>       Pass arguments to the executable
[LOG]   --gen-cc                Generate compile_commands.json
[LOG]   --gen-vsc               Generate .vscode directory
[LOG]   --clean-packages        Clean the package binaries
[LOG]   --update-packages       Update the packages
[LOG]   --restore-packages      Restore the packages
[LOG]   --version               Show the version
[LOG] Environment variables:
[LOG]   BUILDER_CPP_LOG_LEVEL
[LOG]           Valid values are: Debug, Log, Info, Warn, Error
```

Sample file with a library and an executable

```toml
[build]
compiler = "g++"

[[targets]]
name = "libengine"
src = "./Nomu_Engine/Engine/src/"
include_dir = "./Nomu_Engine/Engine/src/include"
type = "dll"
cflags = "-g -Wall -Wunused `pkg-config --cflags freetype2` -std=c++17"
libs = "-lm -lglew32 -lglfw3 -lopengl32 -static-libstdc++ `pkg-config --libs freetype2`"

[[targets]]
name = "main"
src = "./Nomu_Engine/Game/src/"
include_dir = "./Nomu_Engine/Game/src"
type = "exe"
cflags = "-g -Wall"
libs = "-static-libstdc++"
deps = ["libengine"]
```
Sample file with an added package and an executable
```toml
[build]
compiler = "g++"
packages = ["Dr-42/Nomu_Engine, master"]

[[targets]]
name = "main"
src = "./src"
include_dir = "./src"
type = "exe"
cflags = "-g -Wall "
libs = ""
deps = ["libengine"]
```

Optional keys in toml are packages in build and deps in targets

To see a real project being built with the tool
	[Nomu_Engine](https://github.com/Dr-42/Nomu_Engine)
