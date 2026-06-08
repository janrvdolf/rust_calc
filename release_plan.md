# Release Plan

## Where to host executables

The standard place is a **GitHub Release**. You don't commit executables into the repository itself — instead you attach them as binary assets to a tagged release.

## How it works

1. You create a git tag (e.g. `v1.0.0`) and push it.
2. On GitHub you go to **Releases → Draft a new release**, pick that tag, and upload your compiled binaries as attachments.
3. Each attached file gets a stable download URL in the form:
   ```
   https://github.com/<user>/<repo>/releases/latest/download/<filename>
   ```
   That is exactly the URL pattern already in your README's commented-out Deployment section.

## Why not commit binaries to the repo?

- Git stores every version of every file forever — large binaries bloat the repo permanently.
- GitHub has a 100 MB file size limit and warns above 50 MB.
- Releases are designed for distributable artifacts; the repo is for source.

## Automating it with GitHub Actions

The typical workflow is to let CI build and upload the binaries automatically when you push a tag:

```yaml
# .github/workflows/release.yml
on:
  push:
    tags: ["v*"]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
      - uses: softprops/action-gh-release@v2
        with:
          files: target/release/rust_calc
```

You'd add separate jobs for `macos-latest` and `windows-latest` to produce all three platform binaries, naming them `rust_calc-linux-x86_64`, `rust_calc-macos-aarch64`, and `rust_calc-windows-x86_64.exe` — matching what's already written in your README.
