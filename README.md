# mure

A command line tool for creating and managing multiple repositories.

## Installation

```bash
cargo install mure
```

## Environment variables

This application requires the following environment variable.

- `GH_TOKEN`

`GH_TOKEN` is GitHub access token.

(I haven't set it up to automatically read the contents of .env yet.)

## Usage

### directory structure

```
$HOME/.mure.toml ... configuration file
$HOME/.dev ... development directory
$HOME/.dev/repo ... repositories directory
```

When you clone a repository, it will be clone into the `$HOME/.dev/repo/github.com/{owner}/{repo}` directory.

## requirements

- `GH_TOKEN` environment variable is required for authentication.

### `mure init`

Generate `.mure.toml` file in home directory.

```toml
[core]
base_dir = "~/.dev"

[github]
username = "kitsuyui"

[shell]
cd_shims = "mucd"
```

### Set up shell environment for mure

Add following script to your shell configuration file such as `~/.bashrc`, `~/.zshrc` or etc.

```sh
eval $(mure init --shell)
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

#### Options

`--query` option is available for advanced search like `--query 'user:kitsuyui'`
See this page for more about advanced search: https://docs.github.com/en/search-github/searching-on-github/searching-for-repositories

Default search query is `user:{username} is:public fork:false archived:false`

#### Customization

You can customize the output format by setting `github.queries` in `.mure.toml`.
For example, if you want to show both of [my (user:kitsuyui) repositories](https://github.com/kitsuyui?tab=repositories) and the [organization gitignore-in](https://github.com/orgs/gitignore-in/repositories)'s repositories, you can set `github.queries` like this:

```toml
[github]
queries = [
  "user:kitsuyui",
  "owner:gitignore-in",
]
```

### mure refresh

`mure refresh` updates the repository.

### mucd

`mucd` is a command line shims for changing directory shortcut.
mucd enables you to change directory into the repository.

```shell
mucd something  # => Same as `cd $HOME/.dev/something`
```

You can change the name of the shim by set `shell.cd_shims` in `.mure.toml` to another name.

### mure path

`mure path` shows the path of the repository for given repository name.
(Internally, `mure path` is used for `mucd` command.)

### Setup shell completion

```sh
mure completion --shell zsh > /usr/local/Homebrew/completions/zsh/_mure
ln -svf /usr/local/Homebrew/completions/zsh/_mure /usr/local/share/zsh/site-functions/_mure
autoload -Uz compinit && compinit
```

## License

BSD-3-Clause
