# `mdbook-backlinks`

This is a rewrite of https://crates.io/crates/mdbook-backlinks by https://crates.io/users/ratsclub
and https://crates.io/users/kmaasrud . I could not find a repo to contribute back to so I made this
one. I mostly modified it to work with non-wikilink links.

## Usage

Install the `mdbook-backlinks` binary and add this to your `book.toml`:
```toml
[preprocessor.backlinks]
```

Note that the `mdbook-backlinks` on crates.io is different from this repo and only supports wikilink
links. This repo supports all markdown links.
