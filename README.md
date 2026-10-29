<h1 align="center">VRX</h1>
<p align="center"><strong>Read Rapidly</strong></p>
<p align="center">
  <img alt="macOS · Windows · Linux · FreeBSD" src="https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux%20%C2%B7%20FreeBSD-native-2f7bf5?style=flat-square">
  <img alt="Status: early alpha" src="https://img.shields.io/badge/status-early%20alpha-d69e2e?style=flat-square">
  <img alt="Release status:" src="https://github.com/madiaxa/vrx/actions/workflows/release.yml/badge.svg">
</p>
<p>A native Rust and Iced RSVP reader for text and Markdown. Words use a shaped focal anchor, monotonic deadlines, and an event-driven interface. There is no embedded browser, document telemetry, or recurring idle playback timer.</p>
<h2>Get started</h2>
<p>See <a href="https://github.com/madiaxa/vrx/releases">GitHub Releases</a> for available published downloads; use only the source or platform assets actually attached to a release. For more information on platform dependencies, packaging, and GitHub release workflows, see <a href="docs/RELEASE.md">the release guide</a>.</p>
<p>Install stable Rust 1.99 or newer, a native compiler/linker, and the platform dependencies listed in the release guide. Then clone the repository and build with Cargo:</p>
<pre><code>git clone https://github.com/madiaxa/vrx
cd vrx
cargo build --release --locked
# See docs/RELEASE.md for platform dependencies and target architectures.
</code></pre>
<p>Once you have installed VRX, choose <strong>Open</strong>, drop a UTF-8 text/Markdown file onto the window, or use the <strong>pencil</strong> to edit the current reading text. The sample is already filled in. <strong>Paste</strong> inserts at the editor's selection; <strong>Clear text</strong> clears the draft. <strong>Apply changes</strong> starts changed text at the beginning. Cancel preserves the reading document and position. Original input files are never overwritten.</p>
<p>Press <strong>Space</strong> to read. In focus mode, the highlighted grapheme stays at the screen's center. With the focal point disabled, the whole word is centered instead. Long words and guides shrink together to fit the available area.</p>
<h2>Controls</h2>
<table>
<thead><tr><th>Default shortcut</th><th>Action</th></tr></thead>
<tbody>
<tr><td>Space</td><td>Play / pause; replay a finished document</td></tr>
<tr><td>Left / Right</td><td>Jump by the configured count, matching the arrow buttons</td></tr>
<tr><td>Cmd or Ctrl + Left / Right</td><td>Always move exactly one word while retaining playback</td></tr>
<tr><td>Shift + Left / Right</td><td>Previous / next word (alternate)</td></tr>
<tr><td>Cmd/Ctrl + Shift + Left / Right</td><td>Previous / next sentence start</td></tr>
<tr><td>Alt/Option + Left / Right</td><td>Previous / next paragraph start</td></tr>
<tr><td>Up / Down</td><td>Change speed by the configured WPM step without stopping</td></tr>
<tr><td>R</td><td>Restart, preserving active playback</td></tr>
<tr><td>H</td><td>Toggle focus mode</td></tr>
<tr><td>Escape or Cmd/Ctrl + comma</td><td>Open / finish Personalize; Escape closes the active popup</td></tr>
<tr><td>F11 / Shift + F11</td><td>Toggle / exit fullscreen</td></tr>
<tr><td>Cmd/Ctrl + O / V / E / N</td><td>Open / paste / export / edit text</td></tr>
<tr><td>Cmd/Ctrl + I</td><td>About VRX</td></tr>
</tbody>
</table>
<p>Shortcuts can be recorded, edited, or cleared in Personalize. Cmd/Ctrl + Left/Right are reserved for single-word navigation outside editable controls. Normal text-editing keys work inside inputs. Click the version number for the centered About popup, which shows version and brief license information, or use <strong>About VRX</strong> in the macOS application menu.</p>

<h2>Personalize</h2>
<ul>
<li>Set nominal WPM directly. Expand <strong>Adaptive timing</strong> for its enable switch and pause rules, or <strong>Keybindings</strong> for shortcuts, jump count, and speed step. These sections start collapsed, like Radix notation, and retain edits while collapsed. Numeric fields support direct entry.</li>
<li>Choose a reading font, weight and size up to 360 px, with a live miniature preview. Interface text size is independent.</li>
<li>Drag the focal anchor. Choose Classic, 40 percent, Center, or First grapheme. Center uses the left middle grapheme for even-length words; Classic uses 40 percent for its long-word branch.</li>
<li>Customize focal color, guide shape, size and gap. Side-letter opacity is adjustable when focal highlighting is on, with full opacity retained for whole-word shaping fallbacks.</li>
<li>Choose dark, light, high-contrast or custom colors. Every color input has a swatch. The warning color identifies destructive confirmation buttons. Dropdown headers and panels follow the active theme accent; set <strong>Dropdown color</strong> to override their tint in any theme, or leave it empty to follow the accent.</li>
<li>Use <strong>Discard changes</strong> to restore the snapshot from when the menu opened, or <strong>Restore all defaults</strong> to preview default preferences, or use <strong>Done</strong> to submit your changes.</li>

<h3>Numbers and countdown</h3>
<p>Expand <strong>Personalize → Radix notation</strong>, which is collapsed by default, to choose a decimal radix from 2 through 60. The default digit alphabet is <code>0123456789↊↋ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuv</code>. Change the symbols in that same dropdown section and apply them; symbols must be unique and cannot use reserved punctuation. Balanced ternary has a separate three-symbol alphabet in negative-one, zero, positive-one order, initially <code>T01</code>.</p>
<p>Fractional sliders advance in one fractional digit of the chosen base, rounded to the nearest stored thousandth. Typed pause weights keep thousandth precision. Changing notation alone does not change the actual speed or duration. JSON/CLI values, RGB hex, filenames, versions and subtitle timestamps retain their established notation. Axa Mono, bundled under the SIL Open Font License, displays numeric controls.</p>
<p>The countdown rounds up to whole seconds. By default it shows <code>6s remaining</code> under a minute and <code>5:04 remaining</code> for a minute or more. Choose clock, short labels, words, or no labels, or edit separate short/long templates. Available placeholders are <code>{minutes}</code>, <code>{seconds}</code>, <code>{seconds:02}</code>, and <code>{total_seconds}</code>. Padding uses the selected zero symbol. Completed documents show zero remaining.</p>

<h2>Timing and text</h2>
<p>Constant mode uses 60/WPM seconds per token. Adaptive mode uses the largest applicable multiplier, without multiplying pauses together. Independent pauses cover long words, commas, colons, semicolons, quotes, brackets, dashes, sentences and paragraphs, with an optional extra section pause.</p>
<p>The default Preserve every word policy extends the schedule after a genuine stall. Maintain pace explicitly permits skipping expired words and reports the count. Pause retains the current word and remaining interval; speed changes preserve the interval fraction.</p>
<p>Slashes and em dashes terminate a token while staying attached: <code>true/false—maybe</code> becomes <code>true/</code>, <code>false—</code>, <code>maybe</code>. Grapheme clusters remain intact. Contextual scripts and unavailable per-grapheme ligature geometry use whole-word fallback. Unspaced languages requiring dictionary segmentation have documented limitations.</p>
<p>Timer wake, UI dispatch and CPU rendering submission are different from physical presentation. High speeds may exceed a display's refresh capacity. See <a href="docs/PERFORMANCE.md">measurement procedures</a> and <a href="docs/LIMITATIONS.md">known limitations</a>; no unmeasured timing, accessibility or cross-platform guarantee is claimed.</p>

<h2>Export</h2>
<p>Export opens an in-window preview using the sample <strong>VRX</strong>. Change that export's background and choose an output file. Personalize's <strong>Default export background color</strong> supplies the initial color, independently of the reader theme. The default is opaque chroma green, <code>00FF00</code>.</p>
<ul>
<li><strong>ASS:</strong> compact timed text, typography, focal color/opacity and an opaque vector canvas.</li>
<li><strong>SRT:</strong> portable plain timed text; no background or styling.</li>
<li><strong>MP4:</strong> 1920×1080, 60 fps, H.264/yuv420p. Requires an external FFmpeg build with libass and libx264; set <code>VRX_FFMPEG</code> if it is not on PATH.</li>
</ul>
<p>Exports cover the whole document with the selected timing and whole-word positioning. They do not reproduce desktop guide marks or measured per-grapheme anchoring. ASS timestamps have centisecond resolution; MP4 is frame-quantized. Fonts can differ in video editors.</p>

<h2>CLI / API</h2>
<pre><code>vrx --help
vrx inspect book.md
vrx export book.md reading.mp4 400 00FF00
vrx diagnose 30
vrx api book.md</code></pre>
<p>Inspection, diagnostics, export completion and the JSON-lines agent protocol use machine-readable output. The API has a deterministic virtual clock and does not remotely control a running GUI. See <a href="docs/API.md">the API guide</a> and <a href="examples/headless.rs">the Rust example</a>. Python is used for release packaging and optional icon generation, not application runtime.</p>

<h2>Privacy and persistence</h2>
<p>Versioned settings store preferences and up to 16 recent paths, fingerprints and token positions in the platform configuration directory. Document content and editor drafts are never stored there. Export edited text before closing to retain a timed-text copy. Progress is saved through the Play/Pause controls, document replacement, about every 30 seconds during playback, and normal close. Forced termination can lose changes since the last save. Reopening a file resumes only when its fingerprint matches. <code>VRX_CONFIG_DIR</code> selects another configuration directory.</p>
<p>Preferences have live previews, but background saves retain the opening settings snapshot until Done commits it. Saves run off the presentation path and use atomic replacement where supported. Missing/corrupt configuration recovers to defaults with a notice. The editor opening limit is 16 MiB; document import is capped at 512 MiB.</p>

<h2>Contribute and license</h2>
<p>Read <a href="docs/CONTRIBUTING.md">Contributing</a>, <a href="docs/ARCHITECTURE.md">Architecture</a>, <a href="docs/RELEASE.md">Build and release</a>, and <a href="docs/LIMITATIONS.md">Limitations</a>. Rust application code forbids unsafe and explicit panic/unwrap/expect paths; allocator, OS, driver and dependency failures cannot be guaranteed away.</p>
<p>VRX's original code uses <a href="LICENSE">the Unlicense</a>. Dependencies retain their own licenses; Axa Mono uses <a href="assets/OFL.txt">the SIL Open Font License</a>.</p>
