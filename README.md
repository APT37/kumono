<h2>kumono - Media ripper for <a href="https://coomer.su">coomer.su</a> and <a href="https://kemono.su">kemono.su</a></h2>

For a comparison with other tools, see [features](FEATURES.md).

Support is provided in the [discussions][discussions] section.

- [Installation](#installation)
  - [AUR Packages](#aur-packages)
  - [Pre-built Binaries](#pre-built-binaries)
  - [Source Code](#source-code)
- [Command Line](#command-line)
  - [Available Options](#available-options)
  - [Target Selection](#target-selection)
  - [Extension Selection](#extension-selection)
  - [Download Archive](#download-archive)
- [Contribution](#contribution)

## Installation

### AUR Packages

You need an AUR helper like [paru][paru] to install one of these packages.

[![][kmn-aur-ver]][kmn-aur] [![][kmn-bin-aur-ver]][kmn-bin-aur] [![][kmn-git-aur-ver]][kmn-git-aur]

### Pre-built Binaries

[![][build-status]][build-runs] [![][release-date]][latest-release]

Windows and Linux binaries are built for every release.

[![][linux-badge]][linux-dl] [![][windows-badge]][windows-dl]

### Source Code

[![][crate-ver]][crate-url] [![][crate-deps]][crate-deps-url]

```fish
# 1. build dependencies
sudo apt-get install git rustup

# 2. Rust toolchain
rustup default stable

# 3a. latest release (via crates.io)
cargo install kumono

# 3b. latest commit (via GitHub)
cargo install --git https://github.com/APT37/kumono
```

Make sure the cargo binary location is in your `$PATH`. This is usually `~/.cargo/bin`.

*Windows users may use the WSL.*

## Command Line

<img src="kumono.gif">

### Available Options

Downloads for `https://coomer.su/onlyfans/user/belledelphine` will go into `{output-path}/onlyfans/belledelphine`

```
Media ripper for coomer.su and kemono.su

Usage: kumono [OPTIONS] [URLS]...

Arguments:
  [URLS]...  Creator page or post / Discord server or channel

Options:
  -p, --proxy <PROXY>                            Proxy URL (scheme://host:port[/path])
  -t, --threads <THREADS>                        Simultaneous downloads (1-4096) [default: 256]
  -o, --output-path <OUTPUT_PATH>                Base directory for downloads [default: kumono]
  -l, --list-extensions                          List of available file extensions (per target)
  -i, --include <INCLUDE>                        File extensions to include (comma separated)
  -e, --exclude <EXCLUDE>                        File extensions to exclude (comma separated)
  -d, --download-archive                         Log hashes, skip moved/deleted file download
  -m, --max-retries <MAX_RETRIES>                [default: 5]
  -r, --retry-delay <RETRY_DELAY>                [default: 1]
      --connect-timeout <CONNECT_TIMEOUT>        [default: 1]
      --read-timeout <READ_TIMEOUT>              [default: 5]
      --rate-limit-backoff <RATE_LIMIT_BACKOFF>  [default: 15]
      --server-error-delay <SERVER_ERROR_DELAY>  [default: 5]
  -s, --show-config                              Print configuration
  -h, --help                                     Print help
  -V, --version                                  Print version
```

### Target Selection

```bash
# whole creator + linked profiles
kumono https://coomer.su/onlyfans/user/belledelphine/links

# whole creator
kumono https://coomer.su/onlyfans/user/belledelphine

# single page
kumono https://coomer.su/onlyfans/user/belledelphine?o=50

# single post
kumono https://coomer.su/onlyfans/user/belledelphine/post/1099631527

# whole server
kumono https://kemono.su/discord/server/1196504962411282491

# single channel
kumono https://kemono.su/discord/server/1196504962411282491/1196521501059469463

# multiple targets
kumono https://coomer.su/onlyfans/user/belledelphine https://kemono.su/discord/server/1196504962411282491
```

### Extension Selection

```bash
# list available file types for a target
kumono https://coomer.su/onlyfans/user/belledelphine --list-extensions

jpg,m4v,mp4

# download only video files via inclusion
kumono https://coomer.su/onlyfans/user/belledelphine --include m4v,mp4

# download only video files via exclusion
kumono https://coomer.su/onlyfans/user/belledelphine --exclude jpg
```

### Download Archive

When using the `--download-archive` option, `kumono` will create log files for each target in `{output-path}/db` to save hashes of previously downloaded files.

*Using this option will also add the hashes of existing files from previous runs for the same target to the respective log file.*

## Contribution

Feel free to open an issue if you have a bug to report or want to request a feature.

Please use proper code formatting when creating a pull request.

<!-- link definitions -->

[discussions]: https://github.com/APT37/kumono/discussions/categories/support

[paru]: https://github.com/Morganamilo/paru

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

[linux-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=Linux&color=darkgreen
[windows-badge]: https://img.shields.io/github/v/tag/APT37/kumono?logo=github&label=Windows&color=darkblue

[windows-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono.exe
[linux-dl]: https://github.com/APT37/kumono/releases/latest/download/kumono

[crate-ver]: https://img.shields.io/crates/v/kumono?logo=rust&label=Crates.io&color=red
[crate-url]: https://crates.io/crates/kumono

[crate-deps]: https://img.shields.io/deps-rs/kumono/latest?logo=rust&label=Dependencies
[crate-deps-url]: https://crates.io/crates/kumono/dependencies
