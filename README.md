<h2>kumono - Media ripper for coomer.su and kemono.su</h2>

For a comparison with other tools, see [features](FEATURES.md).

- [Installation](#installation)
  - [AUR](#aur)
  - [Binaries](#binaries)
  - [From Source](#from-source)
- [Command Line](#command-line)
  - [Target Selection](#target-selection)
  - [Filtering by File Extension](#filtering-by-file-extension)
- [Related Userscripts](#related-userscripts)
- [Contribution](#contribution)
- [Support](#support)

## Installation

### AUR

Arch users may install one of our [AUR packages](AUR.md).

### Binaries

Pre-built binaries for Windows and Linux are available for every [release][releases].

### From Source

```fish
# linker dependencies
sudo apt-get install git rustup clang mold

# toolchain (incl. cargo)
rustup default stable

# build and install
cargo install --git=https://github.com/APT37/kumono
```

Make sure the cargo binary location is in your `$PATH`. This is usually `~/.cargo/bin`.

*Windows users may use the WSL.*

## Command Line

kumono automatically creates a download directory for each service/creator combination, e.g. `onlyfans/belledelphine` for `https://coomer.su/onlyfans/user/belledelphine`

```
Media ripper for coomer.su and kemono.su

Usage: kumono [OPTIONS] <URL>

Arguments:
  <URL>  Creator page or post / Discord server or channel

Options:
  -p, --proxy <PROXY>                            SOCKS5 proxy (IP:Port)
  -t, --threads <THREADS>                        Simultaneous downloads [default: 256]
  -i, --include <INCLUDE>...                     File extensions to include (comma separated)
  -e, --exclude <EXCLUDE>...                     File extensions to exclude (comma separated)
  -l, --list-extensions                          List of available file extensions (per target)
      --connect-timeout <CONNECT_TIMEOUT>        [default: 1]
      --read-timeout <READ_TIMEOUT>              [default: 5]
      --rate-limit-backoff <RATE_LIMIT_BACKOFF>  [default: 15]
      --server-error-delay <SERVER_ERROR_DELAY>  [default: 5]
  -s, --show-config                              Print configuration before execution
  -h, --help                                     Print help
  -V, --version                                  Print version

```

### Target Selection

```bash
# download all creator content
kumono https://coomer.su/onlyfans/user/belledelphine

# download only content from the given post
kumono https://coomer.su/onlyfans/user/belledelphine/post/1099631527

# download content from all channels of the given server
kumono https://kemono.su/discord/server/1196504962411282491

# download only content from the given channel
kumono https://kemono.su/discord/server/1196504962411282491/1196521501059469463
```

### Filtering by File Extension

```bash
# list available file types for a target
kumono https://coomer.su/onlyfans/user/belledelphine --list-extensions

jpg,m4v,mp4

# download only video files via inclusion
kumono https://coomer.su/onlyfans/user/belledelphine --include m4v,mp4

# download only video files via exclusion
kumono https://coomer.su/onlyfans/user/belledelphine --exclude jpg
```

## Related Userscripts

- [Kemono Browser][us1]: Adds a button at the bottom right of all kemono, coomer & nekohouse supported creator websites that redirects to the corresponding page.
- [Kemono FIX+Download][us2]: Embeds a "Download" button before each file element and starts downloading and saving it to your computer.
- [Kemer Downloader][us3]: One-click download of images (compressed download/single image download), create page data for json download, one-click open all current posts.

## Contribution

**Bug Reports and Feature Requests**

Feel free to open an issue if you have a bug to report or want to request a feature.

**Pull Requests**

You may open pull requests, but be aware that this is a repository mirror. PRs will not be merged directly - accepted changes will be incorporated in private and then pushed here.

## Support

Support is provided via the Discord linked in the repository description.

*Please do not open issues when you are merely seeking support. Your issues will be closed without comment and you may be banned from opening issues altogether.*

<!-- link definitions -->

[coomer]: https://coomer.su
[kemono]: https://kemono.su

[releases]: https://github.com/APT37/kumono/releases

[us1]: https://sleazyfork.org/en/scripts/483259-kemono-browser
[us2]: https://sleazyfork.org/en/scripts/519690-kemono-fix-download
[us3]: https://sleazyfork.org/en/scripts/472282-kemer-%E4%B8%8B%E8%BC%89%E5%99%A8
