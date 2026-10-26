# VRX v0.0.1

Read Rapidly. A native Rust / Iced 0.14 RSVP reader for text and Markdown. Normal reading can use a stable focal anchor; focus mode centers the whole word. No browser, network service, document telemetry, or recurring idle timer. The supplied artwork is the application icon; branding within the reading interface is text only.

## Run

For the packaged Apple Silicon app, extract the release zip in a local folder, move `VRX.app` to Applications, and open it. The app is locally signed, not publicly notarized. Use the included CLI binary at `VRX.app/Contents/MacOS/VRX` for agent commands.

To build from source or produce releases through GitHub, use [RELEASE.md](docs/RELEASE.md), covering macOS, Linux, Windows and FreeBSD. Development uses Rust 1.99+ and locked Iced 0.14 dependencies. Earlier macOS runs and a FreeBSD cross-target check are documented separately; native Linux/Windows/FreeBSD desktop operation remains unverified here. The latest ClearText, slider and focal-guide changes passed three focused regression tests and a native release build; the full suite and GUI smoke tests were not rerun.

Open a UTF-8 `.txt`, `.md`, or `.markdown` file, drop it onto the window, or click the **✎ pencil between Open and Export** to edit the current reading text in an in-window pop-up. The welcome sample is already filled in. The editor's **Paste** button inserts clipboard text at the cursor or replaces its selection, to the left of **Apply changes**. **Clear text** empties the edit buffer; **Cancel** still keeps the original reading text. **Apply changes** starts changed text at its beginning; **Cancel** (or Escape) keeps the original text and position. Editing pauses playback, and never overwrites an imported file. Press **Space** to read. In **Personalize**, adjust nominal WPM, adaptive pauses, font, RSVP type size up to 360 px, independent GUI text size, weight, draggable word position, colors, jump size, speed step, radix, and every reader shortcut. The dark, light, contrast, and custom color presets are saved. Start at a comfortable pace; speed is a preference, not a goal.

## Controls

| Key | Action |
|---|---|
| Space | Play / pause; replay a finished document |
| ← / → | Jump by the configured word count; identical to the arrow buttons |
| Shift + ← / → | Previous / next single word |
| Cmd/Ctrl + ← / → | Previous / next sentence start |
| Alt/Option + ← / → | Previous / next paragraph start |
| ↑ / ↓ | Change WPM by the configured step without stopping |
| R | Restart; keep playing if already playing |
| H | Toggle focus mode; center the whole RSVP word in the window |
| Escape | Open / close Personalize (including from focus mode) |
| F11 | Toggle fullscreen |
| Shift + F11 | Exit fullscreen |
| Cmd/Ctrl + O / V / E | Open / paste / export |
| Cmd/Ctrl + N | Edit current text in the in-window pop-up |
| Cmd/Ctrl + , | Personalize (alternate) |

Every application shortcut can be changed or cleared in **Personalize → Keybindings**. Type a chord and press Enter (separate alternatives with `;`), or click Record and press the combination. Conflicts are reported; clear the old assignment first. Restore defaults is always available. Standard text-field editing keys retain their normal behavior while typing. Inside the text editor, Enter adds a line, Escape cancels, and reader shortcuts are suspended. Fullscreen and focus remain keyboard actions. The Maximize binding has been removed; old saved entries are ignored when loading settings.

Disable **Enable focal point / ORP highlight** to remove both the accent character and guide marks. The whole word is then centered at the selected position. Drag the target in Personalize's two-dimensional pad, or drag directly in the reading area when the panel is alongside it. Positions can also be entered numerically as percentages. Focus mode ignores the saved position and centers the complete word without changing that preference. Choose **Ticks**, **Dots**, or **Lines**, adjust guide size and gap, and enter a focal RGB hex color (empty follows the theme accent). The color applies to ORP and guides. Guide dimensions and spacing follow the reading font, independently of GUI text size, and shrink together with words that need to fit. Focal color also applies to ASS/MP4 highlighting; guide shapes remain desktop-only.

Click any numeric field to edit it, including pause multipliers, positions, font sizes, step counts and WPM. Valid values apply as you type; Enter validates and finishes editing, and Done validates and saves the menu. The GUI text-size setting scales interface text and controls while compensating RSVP font measurements, so it does not change the word's requested reading size.

**Number systems:** choose base 2–60. Numeric GUI values and inputs use Clockette's alphabet `0123456789↊↋ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuv`; base 3 always uses balanced ternary `T`, `0`, `1` (`1T` means decimal two). Fractional pause multipliers use the selected base too. The radix is a directly editable decimal number field, with no slider or dropdown. RGB hex, release versions, filenames, JSON/CLI machine values and exported subtitle timestamps keep their established notation. Radix changes never change actual WPM or pause duration. Pause sliders step by one fractional digit of the chosen base (for example, one twelfth in base 12); typed values retain thousandth precision. Numeric formatting uses the shortest spelling that preserves that precision, rather than twelve fractional digits. A selected fraction is stored as the nearest thousandth for timing, so nondecimal fractions can have a small rounding error (at most 0.0005 in the multiplier).

Seek with the progress slider. Text inputs capture typing normally; reader shortcuts apply when a text field is not handling the key. Tab navigation of all controls and native screen-reader announcements are limited by this Iced release; see [limitations](docs/LIMITATIONS.md).

## Timing

The tokenizer keeps slash delimiters: `true/false/maybe` reads as `true/`, `false/`, `maybe`. Consecutive or leading slashes remain visible tokens. Slash-based grapheme clusters are kept intact.

Constant mode gives every token `60 / WPM` seconds. Adaptive mode uses the **largest** applicable pause multiplier, rather than multiplying pauses together. Long words start at 1.15×, commas 1.3×, clauses 1.5×, sentence endings 1.8×, and paragraph endings 2×. Independent colon, semicolon, quotation-mark, parentheses/bracket and dash pauses are also editable. A multiplier of 1 adds no pause. Section pauses add the configured number of milliseconds. Effective throughput can therefore be lower than nominal WPM.

Default **Preserve every word** advances one token per accepted deadline. Ordinary wake-up jitter stays on the absolute schedule; if the following word's entire slot expires, the schedule extends. **Maintain pace** explicitly permits skipping expired words and reports the count. It limits catch-up work to 4,096 words before rebasing. Pause retains the word and its remaining interval. Speed changes scale the remaining fraction of the current word; navigation grants the destination a fresh interval.

An OS timer waking is not proof that pixels became visible. GPU submission, compositor behavior, display refresh, and system load add latency. No guaranteed physical presentation time is claimed. [Measured results and repeatable procedures](docs/PERFORMANCE.md) cover both OS deadlines and the actual Iced subscription.

## Export

Choose **Export**, then save `.ass`, `.srt`, or `.mp4`. MP4 and ASS use a full-frame, opaque **00FF00 chroma green** background by default. Set **Personalize → Export → Export background** to another six-digit RGB hex color; it is saved independently of the reader's background and theme. SRT is plain timed text and cannot encode a background.

- **ASS**: compact timed text with typography and ORP color, suitable for subtitle/video tools using libass. It is resolution-independent and cheap to export.
- **SRT**: widely supported plain timed text; no color or font styling.
- **MP4**: 1920×1080, 60 fps, H.264, yuv420p. Requires an external FFmpeg build with `libass` and `libx264`. Set `VRX_FFMPEG` to its executable path if it is not on `PATH`. VRX does not install or bundle it.

Exports cover the entire document with the current timing settings, independent of live playback. ASS/MP4 center each complete shaped word at the configured focal position; they do not reproduce the desktop widget's measured per-grapheme anchor. ASS has 10 ms timestamp resolution, SRT 1 ms, and MP4 is quantized to frames. High speeds can exceed their representational limits. See [limitations](docs/LIMITATIONS.md).

## CLI and Rust API

```sh
vrx --help
vrx inspect book.md
vrx export book.md reading.ass 400
vrx export book.md reading.mp4 400 00FF00
vrx diagnose 30
vrx api book.md
```

`inspect`, `diagnose`, export completion, and the agent protocol produce JSON on stdout. Errors go to stderr and return a failing exit code. The agent API uses a deterministic virtual clock; it does not remote-control a running GUI. See [API reference](docs/API.md) and [library example](examples/headless.rs).

## Privacy and persistence

Settings and the last 16 file paths, fingerprints, and token indices are stored in the platform application configuration directory as versioned JSON. Document contents, clipboard text, and edits made in the pop-up are never saved there. Export edited text before closing if it needs to be retained. Reopening the same file resumes only if its fingerprint matches. Progress is saved on pause, on replacement, every 30 seconds during playback, and on normal close. Forced termination can lose progress since the last save. Export files deliberately contain the reading text.

`VRX_CONFIG_DIR` selects another settings directory. Missing/corrupt settings load defaults and show a notice. Writes run in the background and use same-directory temporary files, fsync, and rename. One save runs at a time, with one coalesced follow-up. Multiple app processes sharing a settings directory use last-writer-wins semantics.

## Contribute

See the [contributor guide](docs/CONTRIBUTING.md), [architecture](docs/ARCHITECTURE.md), [API](docs/API.md), [performance methodology](docs/PERFORMANCE.md), [limitations](docs/LIMITATIONS.md), and [build/release instructions](docs/RELEASE.md). Recoverable failures return errors or show a notice; OS termination, allocator, driver and dependency failures cannot be guaranteed away.

Release labels use dozenal digits `0123456789↊↋` in `v0.y.z`. This release is **v0.0.1**. Cargo uses numeric SemVer `0.0.1` because Cargo does not accept the turned Unicode digits. `version::label` converts future release components to the human format.

VRX is released under the [Unlicense](LICENSE), dedicating its original code to the public domain. Third-party dependencies and bundled fonts retain their own licenses and notices.
