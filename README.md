[![CI-badge]][CI]
[![crate-badge]][crate]
[![docs-badge]][docs]

[CI-badge]: https://github.com/kojiishi/git-iblame/actions/workflows/rust-ci.yml/badge.svg
[CI]: https://github.com/kojiishi/git-iblame/actions/workflows/rust-ci.yml
[crate-badge]: https://img.shields.io/crates/v/git-iblame.svg
[crate]: https://crates.io/crates/git-iblame
[docs-badge]: https://docs.rs/git-iblame/badge.svg
[docs]: https://docs.rs/git-iblame/

# git-iblame

A command line tool to do `git blame` interactively.

# Install

## Prerequisites

* [Install Rust] if it's not installed yet.

[install Rust]: https://rustup.rs/

## From [`crates.io`][crate]

```shell-session
cargo install git-iblame
```

## From [github]

```shell-session
cargo install --git https://github.com/kojiishi/git-iblame
```

[github]: https://github.com/kojiishi/git-iblame

## From local checkout

After changing the current directory to the checkout directory:
```shell-session
cargo install --path .
```

# Usages

To start an interactive `git blame` session,
specify the path of the file in a git repository.
```shell-session
git-iblame <path-to-file>
```

The output is similar to `git blame`,
with the current line highlighted.
You can move the current line,
or traverse the git history of the current line.

Please see the help by pressing the `h` key
for the full commands and their key bindings.
Major commands are:
* **h**: Show the help.
* **q**: Quit the program.
* **s**: Show the commit at the current line.
* **c**: Copy the hash of the current line commit to the clipboard.
* **Enter**: Traverse to the parent commit of the commit at the current line.
* **Backspace**: Undo the last **Enter** key.
* **Up**/**Down**: Move the current line to the previous/next commit.
