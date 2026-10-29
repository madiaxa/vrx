# Build and release

## Development

Build commands for every platform and validation commands live here.

Install stable Rust 1.99+ with rustfmt and Clippy. The checked-in lockfile fixes the verified dependency resolution. No external runtime is required for reading. FFmpeg is optional for MP4 export.

```sh
cargo fmt --check
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked
```

Run `target/release/vrx` (`target\release\vrx.exe` on Windows). The `smoke` feature adds opt-in developer harnesses; ordinary releases should omit it. An Xcode command-line toolchain is needed on macOS, an MSVC C++ build toolchain on Windows, or a native compiler/linker on Linux.

## macOS

For a provided app archive, extract it outside a cloud/File Provider folder and move `VRX.app` into Applications. In this development environment the File Provider attached an empty FinderInfo attribute to the extracted bundle inside Documents, which caused strict signature verification to fail there; the same archive verified successfully after extraction into a local temporary directory. The archive itself contains no such extended attributes.

```sh
cargo build --release --locked --target aarch64-apple-darwin
```

For Intel, install `x86_64-apple-darwin` with rustup and compile on an appropriate SDK/toolchain; this was not tested here. A universal app requires both binaries combined with `lipo` and tested on both architectures. Retina scaling uses logical pixels through Iced.

The packager adds VRX.icns to macOS bundles. Native Windows MSVC builds also compile VRX.ico using the Windows SDK resource compiler. Prebuilt artwork variants are included; regenerating them with `python3 packaging/build_icons.py` requires macOS sips, not a dependency on Python image libraries.

For a local `.app`, place the binary at `VRX.app/Contents/MacOS/VRX` and use the supplied `packaging/Info.plist` at `VRX.app/Contents/Info.plist`. The delivered Apple Silicon bundle is for local testing. Public distribution requires an Apple Developer identity, hardened runtime signing, notarization with `notarytool`, stapling, and a clean-machine Gatekeeper test. No public signing or notarization has been performed. Command-line invocations can use the binary inside the bundle directly.

## Linux

An example Debian/Ubuntu development environment:

```sh
sudo apt-get install build-essential pkg-config libxkbcommon-dev libwayland-dev libvulkan-dev mesa-vulkan-drivers
cargo build --release --locked --target x86_64-unknown-linux-gnu
./target/x86_64-unknown-linux-gnu/release/vrx
```

The file dialog uses xdg-desktop-portal, so the desktop needs a working portal backend. Package the executable with a `.desktop` entry and icon for a distribution. Confirm runtime X11/Wayland, graphics, fontconfig/font dependencies on that distribution; no portable glibc compatibility is claimed. Linux build/launch was not executed in this environment.

## Windows

Use stable MSVC Rust and Visual Studio C++ build tools:

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
.\target\x86_64-pc-windows-msvc\release\vrx.exe
```

Test native file dialogs, clipboard, drag/drop, fullscreen, DPI changes and the platform config directory. Sign binaries/installers with the intended publisher identity for public distribution. Windows build/launch was not executed here.

## Software rendering

Both Iced WGPU and tiny-skia are built in. To request the software backend:

```sh
ICED_BACKEND=tiny-skia cargo run --release --locked
```

PowerShell: `$env:ICED_BACKEND='tiny-skia'`. Without a forced backend, Iced attempts the supported GPU backend and then its fallback. Software rendering still needs a functioning native window system; it does not provide a headless GUI.

## Unix desktop launcher

Linux/FreeBSD archives include `vrx.desktop` and `assets/icons/vrx-256.png`. Put the executable in a directory on PATH, the desktop entry in `~/.local/share/applications/`, and install the PNG as `~/.local/share/icons/hicolor/256x256/apps/vrx.png` (create directories first). This is optional desktop integration; the executable works from the terminal without it. Respect local desktop/FreeBSD package policies when installing globally.

## FreeBSD

VRX targets **FreeBSD amd64**, initially building releases on **FreeBSD 15.0**. The complete Rust project, GUI, examples, benchmarks and tests successfully type-checked for `x86_64-unknown-freebsd` from the development Mac. A FreeBSD executable has not been linked or run locally. Native VM CI and release packaging are included so a real FreeBSD build/test run can establish the next level of evidence.

Iced's X11 and Wayland features are explicitly enabled. WGPU and tiny-skia are both available; there is no browser or Linux compatibility layer. Rfd's selected portal backend explicitly supports FreeBSD. Rust provides the [FreeBSD target](https://doc.rust-lang.org/rustc/platform-support/freebsd.html); native graphics/session setup remains a platform responsibility.

### Build on a FreeBSD desktop

Use a supported amd64 FreeBSD installation, a working X11/Wayland desktop, and Rust **1.99+**. The Rust package in a quarterly repository can lag the required compiler; check `rustc --version`. If necessary install current stable via the [official rustup installer](https://rustup.rs/). A ports-provided Rust toolchain is also suitable when new enough.

Install development libraries and fonts as root:

```sh
pkg install -y pkgconf python3 libxkbcommon wayland libX11 libXcursor libXi libXrandr libXrender libXinerama libXfixes fontconfig freetype2 dejavu vulkan-loader
```

From the project directory as your normal user:

```sh
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:/usr/local/share/pkgconfig
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked --target x86_64-unknown-freebsd
./target/x86_64-unknown-freebsd/release/vrx book.md
```

The native linker and C toolchain come with the FreeBSD development/base system. Use the [FreeBSD desktop/graphics handbook](https://docs.freebsd.org/en/books/handbook/x11/) to configure the actual display driver. `vulkan-loader` is the loader, not a guarantee that the GPU has a working Vulkan driver. To request CPU rendering while validating the desktop:

```sh
env ICED_BACKEND=tiny-skia ./target/x86_64-unknown-freebsd/release/vrx
```

File dialogs need a session D-Bus and a portal backend. An example GTK backend installation is:

```sh
pkg install -y dbus xdg-desktop-portal xdg-desktop-portal-gtk
```

Use the portal appropriate to your existing desktop. In a minimal window manager, launch inside a D-Bus session and configure a portal backend rather than treating an unresponsive file chooser as a parser failure. Opening a filename from the CLI works independently of the chooser. The portal's upstream [backend documentation](https://flatpak.github.io/xdg-desktop-portal/docs/) explains selection. Clipboard and drag/drop require the corresponding native X11/Wayland session.

MP4 remains optional. Install an FFmpeg package built with libass and libx264 and inspect `ffmpeg -buildconf`; package options can vary. ASS/SRT export uses no external video library. Configuration follows XDG paths through the same `directories` implementation as other Unix desktops; `VRX_CONFIG_DIR` is available for isolated testing.

### GitHub CI and archives

`.github/workflows/freebsd.yml` runs a real FreeBSD 15.0 amd64 VM on an Ubuntu host using [vmactions/freebsd-vm](https://github.com/vmactions/freebsd-vm). Rust stable, fonts and native libraries are installed inside the VM. `packaging/freebsd-ci.sh` runs formatting, native tests, Clippy, release linking, and `vrx --version`. Build intermediates stay in the VM's `/tmp` so the action copies back only source/artifacts rather than the entire target tree.

Normal CI calls this workflow. Release CI calls it with `package: true`, creating a `vrx-v0.1.0-x86_64-unknown-freebsd.tar.gz` archive and SHA-256 file. Archive contents include the native executable, documentation, BUILD.json and dependency license notices. A desktop session is absent in this VM job: native tests and linking do not certify visible rendering or live clipboard/portal behavior. Follow [the desktop checklist](PERFORMANCE.md) on intended hardware.

For a local release archive after compiling:

```sh
python3 packaging/package_release.py --binary target/x86_64-unknown-freebsd/release/vrx --target x86_64-unknown-freebsd --output dist
```

The packager rejects a binary whose ELF ABI/architecture does not match FreeBSD amd64. It cannot make a macOS or Linux binary into a FreeBSD binary. FreeBSD 14.x and aarch64 are future validation targets, not verified compatibility claims for these archives.

## GitHub release builds

Three workflows are included:

| Workflow | Trigger | Result |
|---|---|---|
| `ci.yml` | Push / pull request | Formatting, tests, Clippy and builds on macOS/Linux/Windows; native FreeBSD VM checks |
| `freebsd.yml` | Called by CI/release, or manually | FreeBSD 15.0 amd64 tests/linking, optionally a release archive |
| `release.yml` | Manual run or pushed decimal `vMAJOR.MINOR.PATCH` tag | Native archives for macOS arm64, Linux x86_64, Windows x86_64 and FreeBSD amd64; tag runs also create a draft prerelease |

### Local packaging and validation

```sh
cargo build --release --locked
python3 packaging/package_release.py --binary target/release/vrx --target aarch64-apple-darwin --output dist
```

Use the correct platform target/binary, and `python` with `vrx.exe` on Windows. Python 3.9+ suffices; no Python packages are needed. macOS uses the system `codesign` tool. The archive contains a BUILD.json input-binary digest, packaged target, source commit when built on GitHub, and packaging timestamp. Its `.sha256` digest covers the final archive. Packaging does not provide a reproducible-build or third-party authenticity attestation.

To validate workflow syntax locally, install [actionlint](https://github.com/rhysd/actionlint) and run:

```sh
actionlint .github/workflows/ci.yml .github/workflows/freebsd.yml .github/workflows/release.yml
sh -n packaging/freebsd-ci.sh
```

Actions are referenced by maintained release tags. A repository with stricter supply-chain requirements can pin reviewed commit SHAs and enable dependency updates; keep those pins current.
