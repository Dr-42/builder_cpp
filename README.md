# BUILDER_CPP

A simple build tool for building C and C++ applications

The tool is still in development.

## Usage
Write a config_win32.toml for windows and config_linux.toml for linux

## Features

- [x] Multithreaded
- [x] Can generate compile_commnds.json
- [x] Auto add project libraries to other targets

Sample file
```
[build]
compiler = "g++"
build_dir = "./Nomu_Engine/bin"
obj_dir = "./Nomu_Engine/obj_win"

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

All keys mentioned are mandatory.

Install with cargo install
For subcommands run with -h flag

To see a real project being built with the tool
	[Nomu_Engine](https://github.com/Dr-42/Nomu_Engine)
