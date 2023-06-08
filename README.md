gh-mirror
=========

This little program uses the `gh` GitHub CLI to get a list of all repositories owned by a GitHub
user, and then makes a clone of them all using `git clone --mirror` (as a bare repo).

When run again, it will perform an update of previously-cloned repositories using
`git remote update` and clone any new ones.

It also sets up a pre-receive hook in the clones which prevents accidentally pushing commits to
them. They are intended to be read-only mirrors, pushes are probably not something you want to do.

Before using this, you need to have the `gh` command installed, and have run `gh auth login`.
