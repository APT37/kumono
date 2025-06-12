# kumono - Media ripper for [coomer](https://coomer.su)/[kemono](https://kemono.su)

## Installation

### Binaries

Windows and Linux binaries are built for every [release](https://github.com/APT37/kumono/releases).

### Arch Linux

Arch users may install via the [AUR](AUR.md).

### Building from source

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

## Usage

Downloaded content will be put into a directory called `<SERVICE>/<CREATOR>`, for example `onlyfans/belledelphine`.

```
Usage: kumono [OPTIONS] <URL>

Options:
  -p, --proxy <PROXY>                            SOCKS5 proxy (IP:Port)
  -t, --threads <THREADS>                        Simultaneous downloads [default: 256]
  -i, --include <INCLUDE>...                     File extensions to include (comma separated)
  -e, --exclude <EXCLUDE>...                     File extensions to exclude (comma separated)
  -l, --list-extensions                          List of available file extensions (per creator)
      --connect-timeout <CONNECT_TIMEOUT>        [default: 1]
      --read-timeout <READ_TIMEOUT>              [default: 5]
      --rate-limit-backoff <RATE_LIMIT_BACKOFF>  [default: 15]
      --server-error-delay <SERVER_ERROR_DELAY>  [default: 5]
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

## Contribution

**Bug Reports and Feature Requests**

Feel free to open an issue if you have a bug to report or want to request a feature.

**Pull Requests**

You may open pull requests, but be aware that this is a repository mirror. PRs will not be merged directly - accepted changes will be incorporated in private and then pushed here.

## Support

Support is provided via the Discord linked in the repository description.

*Please do not open issues when you are merely seeking support. Your issues will be closed without comment and you may be banned from opening issues altogether.*