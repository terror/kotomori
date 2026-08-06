## kotomori

[![release](https://img.shields.io/github/release/terror/kotomori.svg?label=release&style=flat&labelColor=1d1d1d&color=424242&logo=github)](https://github.com/terror/kotomori/releases/latest)
[![crates.io](https://img.shields.io/crates/v/kotomori.svg?style=flat&labelColor=1d1d1d&color=424242&logo=rust)](https://crates.io/crates/kotomori)
[![build](https://img.shields.io/github/actions/workflow/status/terror/kotomori/ci.yaml?branch=master&style=flat&labelColor=1d1d1d&color=424242&logo=GitHub%20Actions&logoColor=white&label=build)](https://github.com/terror/kotomori/actions/workflows/ci.yaml)
[![codecov](https://img.shields.io/codecov/c/gh/terror/kotomori?style=flat&labelColor=1d1d1d&color=424242&logo=Codecov&logoColor=white)](https://codecov.io/gh/terror/kotomori)
[![downloads](https://img.shields.io/github/downloads/terror/kotomori/total.svg?style=flat&labelColor=1d1d1d&color=424242)](https://github.com/terror/kotomori/releases)

**kotomori** (言の森) is a coding agent implemented in Rust, with a focus on
performance and simplicity.

<img width="1667" alt="val" src="screenshot.png" />

If you need help with `kotomori` please feel free to open an issue. Feature
requests and bug reports are always welcome!

## Installation

`kotomori` should run on any system, including Linux, MacOS, and Windows.

The easiest way to install it is by using
[cargo](https://doc.rust-lang.org/cargo/index.html), the Rust package manager:

```bash
cargo install kotomori
```

Otherwise, see below for the complete package list:

#### Cross-platform

<table>
  <thead>
    <tr>
      <th>Package Manager</th>
      <th>Package</th>
      <th>Command</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><a href=https://www.rust-lang.org>Cargo</a></td>
      <td><a href=https://crates.io/crates/kotomori>kotomori</a></td>
      <td><code>cargo install kotomori</code></td>
    </tr>
    <tr>
      <td><a href=https://brew.sh>Homebrew</a></td>
      <td><a href=https://github.com/terror/homebrew-tap>terror/tap/kotomori</a></td>
      <td><code>brew install terror/tap/kotomori</code></td>
    </tr>
  </tbody>
</table>

### Pre-built binaries

Pre-built binaries for Linux, MacOS, and Windows can be found on
[the releases page](https://github.com/terror/kotomori/releases).

## Prior Art

This project was inspired by tools like [pi](https://github.com/earendil-works/pi) and [opencode](https://github.com/anomalyco/opencode).
