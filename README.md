![zfetch Logo](https://raw.githubusercontent.com/zodium-project/zfetch-rs/refs/heads/stable/zfetch.png)

Official Fetch Script/Application on Zodium Project.

Why use zfetch ?
1. written in rust .
2. minimal .
3. looks good .
4. easy to configure ( has a tui menu to configure looks) .
5. has multiple build in themes .
6. resizes itself accoring to terminals dimentions.

## Command Line Arguments

| Argument | Description |
|----------|-------------|
| `-o, --os [name]` | Display OS art instead of the zfetch logo. Optionally specify an OS name to force that logo (e.g. `--os arch`) |
| `-i, --image [path]` | Display an image instead of ASCII art using Kitty graphics protocol. Optionally specify image path |
| `-c, --config` | Launch the TUI configuration editor |
| `-r, --refresh` | Force refresh of cached values (OS name and GPU) |
| `-u, --update` | Update config file to latest version while preserving user settings |

## Configuration
Configuration can be done either through the TUI interface or in the config file.

The config file is located at `~/.config/zfetch/config.toml` and is created automatically on first run.

The default config can also be found in `src/config.toml`.

## Contributing

PR's and Bug reports are welcome , contributions will be accepted .

## Installation

To install zfetch, clone this repo and use the following command from the root of the project:

```
cargo install --path .
```
or 
```
you can use the provided precompiled binary in release section.
```
## Credits 

Slowfetch => zfetch is a fork of slowfetch , you can find slowfetch at : https://github.com/tuibird/slowfetch

## Screenshots

![zfetch preview 1](https://raw.githubusercontent.com/zodium-project/zfetch-rs/refs/heads/stable/preview-1.png)

![zfetch preview 2](https://raw.githubusercontent.com/zodium-project/zfetch-rs/refs/heads/stable/preview-2.png)

![zfetch preview 3](https://raw.githubusercontent.com/zodium-project/zfetch-rs/refs/heads/stable/preview-3.png)