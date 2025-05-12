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

An interactive enhanced [`git blame`] command line tool.

The `git-iblame` allows you to find
changes to the lines of your interests interactively,
not only the last change as [`git blame`] can do,
but also all the past changes,
up to the initial commit.

Features:

* The interactive "current line" specifies the line of your interests.
* Easy and instant traversals to older or newer trees of the line.
* Inspect the commit that modified the line.
* Deleted lines are also shown for inspections.

The `git-iblame` is built for responsive interaction.
Lengthy operations such as
computing all the history of a file runs in background.
Old annotations which takes time to read from the disk
come up incrementally while you are browsing.

To make the traversals of the history responsive,
especially for large repositories,
the `git-iblame` has its own file history engine.
This engine is built on top of the fundamental git operations
without using the logic in the [`git blame`].

When traversing to older or newer trees,
the `git-iblame`'s engine can re-compute the history for the trees instantly
from its own data structure in memory.

[`git blame`]: https://git-scm.com/docs/git-blame
[git2]: https://docs.rs/git2/latest/git2/
[libgit2]: https://libgit2.org/

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
* **d**: Show the diff of the current file of the commit at the current line.
* **c**: Copy the hash of the current line commit to the clipboard.
* **→** (**Right**): Traverse to the parent commit of the commit at the current line.
* **←** (**Left**): Undo the last traversal;
  i.e., traverse back to the last newer tree.

# Change History

Please see [release notes].

[release notes]: https://github.com/kojiishi/git-iblame/releases
