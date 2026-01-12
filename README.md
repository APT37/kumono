<h2>kumono - Media ripper for <a href="https://coomer.st">coomer</a> and <a href="https://kemono.cr">kemono</a></h2>

[![][build-status]][build-runs] [![][release-date]][latest-release]

For a comparison with other tools, see [features](FEATURES.md).

Support is provided in the [discussions][discussions] section.

- [Installation](#installation)
  - [Binaries](#binaries)
  - [Packages (Arch)](#packages-arch)
  - [Cargo (Debian)](#cargo-debian)
- [Command Line](#command-line)
  - [Available Options](#available-options)
  - [Target Selection](#target-selection)
  - [Extension Selection](#extension-selection)
  - [Download Archive](#download-archive)
- [Legal Disclaimer](#legal-disclaimer)

## Installation

### Binaries

[![][windows-x64-badge]][windows-x64-dl] [![][windows-arm-badge]][windows-arm-dl]

[![][macos-x64-badge]][macos-x64-dl] [![][macos-arm-badge]][macos-arm-dl]

[![][linux-x64-badge]][linux-x64-dl] [![][linux-arm-badge]][linux-arm-dl]

### Packages (Arch)

You may use an AUR helper like [**paru**][paru] to install one of these packages.

[![][kmn-aur-ver]][kmn-aur] [![][kmn-bin-aur-ver]][kmn-bin-aur] [![][kmn-git-aur-ver]][kmn-git-aur]

### Cargo (Debian)

[![][crate-ver]][crate-url] [![][crate-deps]][crate-deps-url]

```fish
# 1. build dependencies
sudo apt-get install git rustup

# 2. Rust toolchain
rustup install stable --profile minimal

# 3a. latest release (via crates.io)
cargo install kumono

# 3b. latest commit (via GitHub)
cargo install --git https://github.com/APT37/kumono
```

Make sure the cargo binary location (usually `~/.cargo/bin`) is in your `$PATH`.

*Windows users may use the WSL, possibly via **[MobaXterm][mobax]**.* 

## Command Line

<img src="kumono.gif">

### Available Options

Downloads for `https://coomer.st/onlyfans/user/belledelphine` will go into `{output-path}/onlyfans/belledelphine` (the default value for `{output-path}` is `kumono`).

```
Media ripper for coomer and kemono

Usage: kumono [OPTIONS] [URLS]...

Arguments:
  [URLS]...  Creator page or post / Discord server or channel

Options:
  -p, --proxy <PROXY>                            Proxy URL (scheme://host:port[/path])
  -t, --threads <THREADS>                        Simultaneous downloads (1-512) [default: 256]
  -f, --input-file <INPUT_FILE>                  List of target URLs
  -o, --output-path <OUTPUT_PATH>                Base directory for downloads [default: kumono]
  -l, --list-extensions                          List of available file extensions (per target)
  -i, --include <INCLUDE>                        File extensions to include (comma separated)
  -e, --exclude <EXCLUDE>                        File extensions to exclude (comma separated)
  -d, --download-archive                         Log hashes, skip moved/deleted file download
  -m, --max-retries <MAX_RETRIES>                [default: 5]
  -r, --retry-delay <RETRY_DELAY>                [default: 1]
      --connect-timeout <CONNECT_TIMEOUT>        [default: 1]
      --read-timeout <READ_TIMEOUT>              [default: 180]
      --rate-limit-backoff <RATE_LIMIT_BACKOFF>  [default: 15]
      --server-error-delay <SERVER_ERROR_DELAY>  [default: 5]
  -s, --show-config                              Print configuration values
  -h, --help                                     Print help
  -V, --version                                  Print version
```

### Target Selection

```bash
# whole creator + linked profiles
kumono https://coomer.st/onlyfans/user/belledelphine/links

# whole creator
kumono https://coomer.st/onlyfans/user/belledelphine

# single page
kumono https://coomer.st/onlyfans/user/belledelphine?o=50

# single post
kumono https://coomer.st/onlyfans/user/belledelphine/post/1099631527

# whole server
kumono https://kemono.cr/discord/server/1196504962411282491

# single channel
kumono https://kemono.cr/discord/server/1196504962411282491/1196521501059469463

# multiple targets
kumono https://coomer.st/onlyfans/user/belledelphine https://kemono.cr/discord/server/1196504962411282491
```

### Extension Selection

```bash
# list available file types for a target
kumono https://coomer.st/onlyfans/user/belledelphine --list-extensions

jpg,m4v,mp4

# download only video files via inclusion
kumono https://coomer.st/onlyfans/user/belledelphine --include m4v,mp4

# download only video files via exclusion
kumono https://coomer.st/onlyfans/user/belledelphine --exclude jpg
```

### Download Archive

When using the `--download-archive` option, `kumono` will create log files for each target in `{output-path}/db` to save hashes of previously downloaded files.

*Using this option will also add the hashes of existing files from previous runs for the same target to the respective log file.*

## Legal Disclaimer

This project does not condone or support piracy in any form. We respect the intellectual property rights of creators and encourage users to access content through legal and authorized channels. The project aims to promote creativity, innovation, and the responsible use of digital resources. Any content shared or discussed within the scope of this project is intended for educational and informational purposes only. Users are urged to respect copyright laws and support creators by purchasing or accessing their work legally.

<!-- link definitions -->

[discussions]: https://github.com/APT37/kumono/discussions/categories/support

[paru]: https://github.com/Morganamilo/paru#description

[kmn-aur]: https://aur.archlinux.org/packages/kumono
[kmn-bin-aur]: https://aur.archlinux.org/packages/kumono-bin
[kmn-git-aur]: https://aur.archlinux.org/packages/kumono-git

[kmn-aur-ver]: https://img.shields.io/aur/version/kumono?logo=archlinux&label=kumono
[kmn-bin-aur-ver]: https://img.shields.io/aur/version/kumono-bin?logo=archlinux&label=kumono-bin
[kmn-git-aur-ver]: https://img.shields.io/aur/version/kumono-git?logo=archlinux&label=kumono-git

[build-status]: https://img.shields.io/github/actions/workflow/status/APT37/kumono/build-release.yml?logo=github&label=CI
[build-runs]: https://github.com/APT37/kumono/actions/workflows/build-release.yml

[release-date]: https://img.shields.io/github/release-date/APT37/kumono?logo=github&label=Latest%20Release
[latest-release]: https://github.com/APT37/kumono/releases/latest

[windows-x64-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=Windows%20x64&color=cornflowerblue
[windows-arm-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=Windows%20ARM&color=cornflowerblue
[macos-x64-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=macOS%20x64&color=lightslategray
[macos-arm-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=macOS%20ARM&color=lightslategray
[linux-x64-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=Linux%20x64&color=forestgreen
[linux-arm-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=Linux%20ARM&color=forestgreen

[windows-x64-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono-windows-x64.exe
[windows-arm-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono-windows-arm64.exe
[macos-x64-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono-macos-x64
[macos-arm-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono-macos-arm64
[linux-x64-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono-linux-x64
[linux-arm-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono-linux-arm64

[crate-ver]: https://img.shields.io/crates/v/kumono?logo=rust&label=Crates.io&color=red
[crate-url]: https://crates.io/crates/kumono

[crate-deps]: https://img.shields.io/deps-rs/kumono/latest?logo=rust&label=Dependencies
[crate-deps-url]: https://crates.io/crates/kumono/dependencies

[mobax]: https://mobaxterm.mobatek.net/