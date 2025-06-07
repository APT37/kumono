## kumono coomer/kemono downloader

### Installation

You need to compile the source code yourself. A quick rundown:

```fish
# install dependencies
sudo apt-get install git rustup clang mold

# install cargo
rustup default stable

# clone the repository
git clone <REPO_URL>

# enter the directory
cd <REPO_DIR>

# compile and install kumono
cargo install --force --path .
```

Make sure the cargo binary location is in your `$PATH`. This is usually `~/.cargo/bin`.

*Windows users need to use the WSL. Native support is not planned.*

### Usage and Features

Run `kumono <SERVICE> <USER_ID>` and the given creators's content will be downloaded into `<SERVICE>/<USER_ID>`.

For more options please refer to `--help`.

### Contribution

**Bugs Reports and Feature Requests**

Feel free to open an issue if you have a bug to report or want to request a feature and suggest something be changed. You may also discuss this in linked Discord.

**Pull Requests**

This is a repository mirror, so they will not be merged directly; accepted changes will be incorporated in private and afterwards pushed here.

### Support

Support is mainly provided via the the Discord linked in the repository description.

*Please do not open issues when merely seeking support. Your issues will be closed without comment and you may be banned from opening issues altogether.*