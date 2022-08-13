# mure

A command line tool for creating and managing multiple repositories.

## Installation

```bash
cargo install mure
```

## Usage

### directory structure

```
$HOME/.mure.toml ... configuration file
$HOME/.dev ... development directory
$HOME/.dev/repo ... repositories directory
```

When you clone a repository, it will be clone into the `$HOME/.dev/repo/github.com/{owner}/{repo}` directory.

### `mure init`

Generate `.mure.toml` file in home directory.

```toml
[core]
base_dir = "~/.dev"

[github]
username = "kitsuyui"
```

### mure clone

`mure clone` clone the repository to the common directory.
And makes symbolic links to the working repository.

```bash
mure clone <url>
```

### mure issues

`mure issues` shows the list of issues and pull requests of all repositories.

Example:

<img width="1023" alt="example-mure-issues" src="https://user-images.githubusercontent.com/2596972/184259022-cb428537-f12e-41b0-8b49-a72565afa167.png">

### mure refresh

`mure refresh` updates the repository.

## License

BSD-3-Clause
