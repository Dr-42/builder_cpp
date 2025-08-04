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

# Limitations

- [x] Only supports clang and gcc compilers

## Usage

Write a config_win32.toml for windows and config_linux.toml for linux

To create a new project

```console
builder_cpp init <project-name> [--c|--cpp]
```

For help

```console
builder_cpp --help
```

The help command will show you the following

```sh
$ builder_cpp -h
A simple build tool for building C and C++ applications

Usage: builder_cpp [OPTIONS] [-- [BIN_ARGS]...] [COMMAND]

Commands:
  init    Initialize a new project Defaults to C++ if no language is specified
  config  Configuration settings
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [BIN_ARGS]...  Arguments to pass to the executable when running

Options:
  -b, --build             Build your project
  -c, --clean             Clean the obj and bin intermediates
  -r, --run               Run the executable
      --gen-cc            Generate compile_commands.json
      --gen-vsc           Generate .vscode/c_cpp_properties.json
      --clean-packages    Clean packages
      --update-packages   Update packages
      --restore-packages  Restore packages
  -h, --help              Print help
  -V, --version           Print version
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
[Heim_Engine](https://github.com/Dr-42/Heim_Engine)
[Imeye](https://github.com/Dr-42/imeye)
And lots more.
