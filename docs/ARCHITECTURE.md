# Architecture and implementation decisions

| Area | Files | Responsibilities |
|---|---|---|
| Text | `src/text/` | One owned UTF-8 buffer; indexed tokens; grapheme ORPs; sparse sentence/paragraph/category indexes |
| Import | `src/import/` | UTF-8 validation, file limits, readable Markdown, cancellation; `Importer` extension trait |
| Playback | `src/engine/` | Lifecycle, rational durations, deadlines, pause/seek/speed, overruns, diagnostics |
| Native app | `src/app.rs`, `src/message.rs` | Message-driven orchestration, background tasks, keyboard/window events, persistence |
| Typography | `src/ui/reader.rs` | Cached shaped paragraph, ORP geometry, scaling, clipping, guides |
| Numbers / shortcuts | `src/numerals.rs`, `src/keybinds.rs` | Clockette-compatible radix formatting/input and portable configurable chords |
| Settings | `src/config.rs`, `src/ui/settings.rs`, `src/ui/theme.rs` | Sanitized preferences, editable numeric drafts, native controls and palettes |
| Storage | `src/persistence.rs` | Versioned JSON, bounded history, same-directory atomic replacement |
| Export | `src/export.rs` | Streaming ASS/SRT timelines and cancellable external FFmpeg rendering |
| Automation | `src/cli.rs`, `src/lib.rs` | JSON CLI and public GUI-independent Rust API |
| Verification | `tests/`, `benches/`, `src/native_bench.rs` | Deterministic/property tests, Criterion, opt-in native measurements |

The library has no dependency on Iced types. Iced is used only by the executable. A single package avoids workspace boilerplate while keeping the engine usable by other Rust programs. `unsafe_code = forbid` applies to both library and executable. There are no application panic/unwrap/expect calls; dependencies and allocator failures remain outside that promise.

## Document invariants

Each token stores six `usize` byte offsets, grapheme count, boundary category, and whole-word fallback flag. This is 56 bytes on the verified 64-bit target, plus shared source and sparse indexes. `usize` matches the addressable source size; import is capped at 512 MiB. Every offset is constructed at a validated UTF-8/grapheme boundary, and public lookup uses checked slices. No per-token `String` exists. Quote, parentheses/bracket, colon, semicolon and dash flags fit within the existing token padding; token size remains 56 bytes on the development target. Import and indexing run on Iced's worker pool. There is one active import and one replaceable pending request. Cancellation is checked during indexing; synchronous filesystem reads and the Markdown parser cannot be interrupted mid-call.

Tokenization splits on whole graphemes whose first scalar is Rust Unicode whitespace, and after a slash- or em-dash-based grapheme while retaining it in the preceding token (`true/false/maybe` becomes `true/`, `false/`, `maybe`). Decorative combining marks attached to a whitespace base are normalized away with that separator, so no source grapheme is cut. A single line break separates words; a blank line (including CRLF/CR/Unicode separators) or U+2029 ends a paragraph. Other punctuation stays attached to the word. Sentence ends include `. ! ? … 。 ！ ？ ؟ । ॥` with common closing quotes/brackets ignored for classification. This is a documented heuristic: abbreviations and decimals can create false sentence boundaries. Markdown parser events strip markup/destinations, keep code and alt text, and emit paragraph boundaries. HTML blocks are omitted; this is not a general HTML importer.

Classic ORP counts graphemes, including attached punctuation, and retains the short-word table, using approximately 40% for words of 14 or more graphemes. Center chooses the left middle grapheme for an even count; the separate percentage strategy is also 40%, with a migration alias for old thirty_percent configurations. Strategies implement `OrpAlgorithm`, so alternate algorithms can be indexed without changing rendering. A token exceeding 256 graphemes gets a precomputed display excerpt around its ORP; navigation and export retain the whole word. No segmentation happens in `view` or `advance`.

Sparse indexes provide O(1) word access and O(log n) sentence/paragraph lookup. Sparse categories combining boundary class, long-word status and a five-bit punctuation mask make remaining-time estimates O(log n) without rebuilding timing weights after a preference change. No speculative memory mapping or chunking was added; the measured million-word baseline is documented separately.

## Playback invariants

`State` is explicitly Empty, Ready, Playing, Paused, Finished, or Error. Only Playing has a timer ticket. Ready/Paused/Finished can play; Finished restarts at zero. Empty/Error ignore play. Seek clamps safely and retains active playback. Replacement invalidates timers, resets diagnostics, and produces Ready/Empty. Shutdown pauses and invalidates outstanding work. The final word receives its interval before Finished.

Callers pass elapsed monotonic `Duration` values from one fixed `Instant` epoch. Tests pass virtual time. Durations use integer nanoseconds and a carried division remainder: `60e9 × weight / (WPM × 1000)`. No repeated floating-point addition exists. Pausing retains remaining duration; resuming starts a new deadline at current monotonic time. Speed changes preserve the elapsed fraction and never change the token index.

Tickets contain generation and absolute elapsed deadline. Both must match current playback. Thus messages from pause, seek, speed changes, restart, or document replacement cannot advance the new session. Preserve advances at most once per event and rebases only if the next word's entire slot expired. MaintainPace may consume multiple expired slots, caps work at 4,096, and counts skipped words. No word-event queue can grow unbounded.

## Runtime and drawing

Iced 0.14's default reactive redraw strategy suits a mostly static reader. A `Subscription::run_with` keyed by `(ticket, epoch)` owns a one-shot `futures_timer::Delay`. Each accepted deadline changes the key; pause removes the subscription and drops its delay. There is at most one scheduled word event, no periodic GUI poll, and no subscription while paused other than filtered input/window events. The UI event records the actual timer wake separately from UI dispatch. No dedicated playback thread was justified by the measurements.

Shaping costs cannot be eliminated: every new word needs glyph layout. The ordinary Iced text widget cannot align a selected grapheme's measured visual center, so the small `Reader` widget uses Iced's actual `Paragraph::with_spans` and `span_bounds`. Three borrowed spans shape the complete word together. Its widget tree caches one paragraph and its focal geometry across rebuilds. Font/color/word changes invalidate that cache; resizing only changes its transform. Proportional font widths, combining marks, and shaping come from Iced's backend. A bounded word excerpt prevents pathological shaping costs. Long words scale down around the same focal anchor and clip to the reading area.

Arabic/Hebrew, Indic and several other contextual scripts use a single uncolored, whole-word paragraph centered at the focal anchor. A color-independent pair of guide marks remains available when the focal point is enabled. This avoids claiming precise grapheme fixation for complex shaping or bidi layouts. Ligature clusters can have a shared visual bound or assign their glyph metadata to another span. In that case, Iced may return no bounds for an ORP inside the cluster (for example the second f in office with Ubuntu fonts); the reader centers the whole word through the tested focal-center fallback. The shaping test requires bounds for complete clusters, not for every character inside a ligature. See the manual checklist.

Imports, exports, and saves run as Iced tasks on its thread pool. Normal production adds no application-owned scheduler threads; futures-timer and platform libraries manage their own helpers. Save work is coalesced; exports are limited to one at a time. Closing cancels import/export work and waits for the final save and any child FFmpeg process to finish cancellation. The CLI API is single-threaded and deterministic. Frame submission diagnostics do not measure compositor scanout.

## Compatibility evidence

Installed Rust was checked with `rustup check`. Actual Cargo.lock sources were read and compilation verified `iced::application(boot, update, view)`, Task, filtered event subscriptions, window mode/screenshot actions, advanced Paragraph APIs, and reactive rendering. The initial target resolves Iced 0.14.0 with compatible widget/winit patch releases. WGPU, tiny-skia, X11 and Wayland are explicitly enabled. Full x86_64-unknown-freebsd type-checking (including tests/examples/benches and the GUI) passed from macOS; native FreeBSD linking and desktop validation remain separate. The toolchain emits a future-incompatibility warning for transitive `block 0.1.6` through WGPU's Metal implementation; VRX does not suppress it or modify vendor code.

## Personalization revision (version unchanged)

`numerals` is independent of the GUI and formats/parses bases 2–60 using Clockette's digit alphabet; base 3 is balanced T/0/1. Fractional formatting chooses the shortest spelling that round-trips to integer storage units, including thousandth pause weights. Pause sliders use integer ticks in units of 1/radix and convert to the nearest thousandth; changing notation alone does not quantize stored settings. Only presentation notation changes; engine durations, offsets, JSON/CLI values and subtitle timestamps do not. `keybinds` stores portable chords and rejects conflicts; its deserializer drops the retired Maximize entry from older settings without discarding other bindings. The native layer records combinations even if a text field held focus, and unfocuses text controls for recording/finished numeric editing.

The reader disables all focal styling through one preference. Focus mode anchors the focal grapheme at the center of the client window. Whole-word centering is reserved for a disabled focal point, contextual-script/ligature fallback, and exported previews. Interface scaling uses Iced's scale-factor API, while the reader divides its shaping size by that GUI factor to retain its independent physical reading size. The live reader and settings anchor pad share a normalized drag handler; neither uses anchor sliders. Vertical fit now considers the available space above and below the selected point.

Numbers in Personalize are editable text fields with radix-aware validation. The radix itself is a decimal input field, validated from 2 through 60 and applied without replacing other settings drafts. Additional punctuation pauses use precomputed token flags and the largest applicable weight, and remaining-time estimates use sparse groups, so no per-word parsing or full-document rescans occur during playback. Older configurations receive defaults and inherit old clause weights for new colon/semicolon controls. The working package version is 0.1.0, derived from Cargo and independent of GUI number-system preferences.

## In-window text editor

`App::editor` owns an optional Iced text-editor buffer and preparation generation. An opaque stacked layer blocks interaction with the reading controls; keyboard routing suspends reader actions while editing. Opening pauses playback, prepares the current source on the worker pool, then focuses the editor. Cancelling invalidates late preparations. Applying changed text uses the same background document pipeline as paste; unchanged text closes without reindexing or moving the reading position. ClearText replaces only the editor buffer and increments its generation, so older clipboard/preparation replies cannot refill it. Cancel preserves the reading document; Apply can replace it with an empty document. Original files are never written. Editor buffers are temporary and not persisted. A 16 MiB opening bound avoids making a whole-book editing buffer part of the playback path; larger files remain readable.

## Export canvas and clipboard

`Config::export_background` is a separate RGB color, defaulting to 00FF00. Neither the reader theme nor its custom background affects it. MP4 creates a full-frame color source; ASS includes a full-frame vector rectangle on layer -1 underneath layer-0 word events. SRT remains unstyled. The CLI and JSON API accept an optional export background. Validation rejects malformed colors before invoking FFmpeg.

The top toolbar pencil opens the editor. Its Paste button reads the clipboard asynchronously, inserts at the selection/cursor, and tags the reply with the editor generation so a cancelled session cannot paste into a later one. Apply changes replaces changed text through background indexing.

## Scalable focal customization

The GUI-independent configuration stores a guide shape (ticks/dots/lines), size percentage, gap as a percentage of the RSVP font, and an optional focal RGB color override. Empty color follows the theme accent. Older configurations receive defaults through the existing schema. Sanitization bounds the size/gap and rejects malformed color overrides.

The reader caches shaping with the resolved focal color. Guide geometry is derived from the effective reading font, before applying the same fit transform as the paragraph. Vertical and horizontal fit account for the guide extent; guide size and spacing therefore follow text size and clipping constraints, while GUI scaling remains independent. Geometry is drawn with existing quads and rounded corners, without new subscriptions, threads or shaping work. ASS/MP4 uses the same ORP color override but does not draw desktop guide marks.

## Dialogs, preferences and native menu

Personalize captures an opening configuration snapshot. Live edits update the renderer and timing, but background saves serialize the snapshot until Done validates and commits. Discard restores that snapshot; restore-defaults updates the live draft while preserving the opening snapshot. Confirmation popups use the configurable warning color and never delete reading content or recent files. Alphabet/template/color drafts are validated before commit.

Export captures the document Arc and a cloned per-export configuration, a 16:9 VRX thumbnail and an independent background draft. Native file-picker replies carry a generation and cannot start an export after cancellation or dialog replacement. The default export background is a separate persisted preference. About and confirmations use the same opaque in-window dialog layer. About centers all text and controls and shows only brief license information; full licenses are distributed as files. Reader keyboard shortcuts and file drops are suspended under these dialogs.

On macOS, the target-only muda dependency owns the native application menu. Installation happens in App::update after the window exists, on the main thread. Menu callbacks use a bounded channel and an awaiting subscription, with no timer poll or application-owned thread. About opens the same in-window popup; Quit follows the normal final-save/cancellation path. The safe APIs were checked against the installed crate and its [upstream documentation](https://docs.rs/muda/0.21.2/muda/). Other targets do not compile this menu module.

Countdown templates live in the GUI-independent countdown module. Subminute values omit the zero-minute prefix, clock seconds are padded with the selected zero symbol, and estimates include the unconsumed current interval. Finished/empty/error documents return zero. Formats are length-limited and accept only named placeholders. Number-system changes affect labels, not stored timing or API values.

Side opacity is cached with the foreground/focal color. If a ligature lacks its own focal bounds, the complete word is reshaped without fading so zero side opacity cannot hide it. Each normal word measures focal bounds once. Export color conversions are computed once per timeline rather than once per word. These are inspection-based allocation reductions; no new performance benchmark is claimed.

Personalize's base and numeral-alphabet controls live in a collapsed-by-default disclosure section; expansion is transient UI state. Panel widths depend on the actual window mode: windowed 525/650, fullscreen 420/520. Window resize/focus events request the current Iced window mode. Generation tags reject stale replies, and explicit fullscreen actions retain their requested target. This avoids both idle polling and mode-query races. Release labels are decimal and obtained from Cargo package metadata.

Adaptive timing and Keybindings use the same disclosure component as Radix notation. Their expansion flags are transient; validation opens the affected section, and collapsing Keybindings cancels shortcut recording. The optional persisted `dropdown_color` override controls header/panel tint in every theme; an empty value follows the active accent. Resetting shortcuts preserves other preference drafts and section states.
