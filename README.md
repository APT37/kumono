# kumono - Media ripper for [coomer](https://coomer.su)/[kemono](https://kemono.su)

## Installation

### Binaries

Windows and Linux binaries are built for every [release](https://github.com/APT37/kumono/releases).

### Arch Linux

Arch users may install via the [AUR](/AUR.md).

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

## Usage and Features

Downloaded content will be put in a dircetory called `<SERVICE>/<USER_ID>`, for example `onlyfans/belledelphine`.

```
Usage: kumono [OPTIONS] <SERVICE> <USER_ID>

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

*For more information please refer to `--help`.*

## Contribution

**Bug Reports and Feature Requests**

Feel free to open an issue if you have a bug to report or want to request a feature and suggest something be changed. You may also discuss this in linked Discord.

**Pull Requests**

This is a repository mirror, so pull requests will not be merged directly; accepted changes will be incorporated in private and afterwards pushed here.

## Support

Support is mainly provided via the Discord linked in the repository description.

*Please do not open issues when merely seeking support. Your issues will be closed without comment and you may be banned from opening issues altogether.*