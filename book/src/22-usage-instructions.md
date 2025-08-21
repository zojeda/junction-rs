# Building & Viewing the Book

## Install mdBook
```bash
cargo install mdbook
```

## Build
```bash
mdbook build book
```
Outputs HTML to `book/build/`.

## Serve (Live Reload)
```bash
mdbook serve book --open
```

## Update Workflow
1. Edit chapter markdown.
2. Run `mdbook build` or keep serve running.
3. Commit changes.

## CI Recommendation
Add a job to build the book and publish artifacts (e.g. GitHub Pages) if desired.

## Adding Chapters
- Create new file in `book/src/`
- Add entry to `SUMMARY.md`

## Linting Docs (Optional)
Consider tools like `markdownlint` to enforce consistency.

## Versioning
Tag releases after ensuring docs reflect all public API behaviors.
