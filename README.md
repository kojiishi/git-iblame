# git-iblame

A command line tool to do `git blame` interactively.

# Install

## Prerequisites

* [Install Rust] if you haven't done so yet.

[install Rust]: https://rustup.rs/

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
Following commands are available:
* **Enter**: Run `git blame` on one older commit
  of the current commit.
* **Up**/**Down**: Move the current line to
  the previous/next commit.
* **Home**/**End**/**PgUp**/**PgDown**: Move the current line.
* **Number + Enter**: Go to the line.
* **c**: Copy the hash of the current commit to the clipboard.
* **q**: Quit the session.
