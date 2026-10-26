# CLI and contributor API

`vrx --help` lists commands. All file paths are native OS paths on the CLI; JSON protocol paths are JSON strings. Existing CLI export files are refused. The GUI's native save dialog handles overwrite selection. Library callers must coordinate writes to the same output path and check permission/overwrite policy themselves.

## JSON-lines protocol v1

Start `vrx api [book.md]`. The first line identifies protocol and version. Send one JSON object per line; read one response per request. Input requests are capped at 8 MiB and malformed requests return `{"ok":false,"error":"…"}` without terminating the session. Documents loaded from disk retain the normal 512 MiB source cap.

```json
{"command":"text","text":"One two three.\n\nNext paragraph."}
{"command":"adaptive","enabled":false}
{"command":"speed","wpm":300}
{"command":"play"}
{"command":"advance","nanoseconds":200000000}
{"command":"jump","words":2}
{"command":"pause"}
{"command":"status"}
{"command":"shutdown"}
```

| Command | Fields | Effect |
|---|---|---|
| `status` | — | Current state, token, timing, and diagnostic counters |
| `text` | `text: string` | Replace with UTF-8 plain text |
| `load` | `path: string` | Replace from txt/Markdown |
| `play`, `pause`, `restart` | — | Same lifecycle rules as the GUI |
| `seek` | `index: unsigned integer` | Zero-based clamped word index |
| `jump` | `words: signed integer` | Relative navigation, preserving playing state |
| `speed` | `wpm: unsigned integer` | Clamped to 20–10,000; retain current token |
| `adaptive` | `enabled: boolean` | Enable/disable pause multipliers |
| `advance` | `nanoseconds: unsigned integer` | Move the virtual clock forward and process exact due deadlines |
| `export` | `path: string`, optional `background: string` | Export full document with current timing and an independent RGB hex canvas (default `00FF00`) |
| `shutdown` | — | Invalidate outstanding timer and end the session |

Successful responses contain `ok`, `state`, `index`, `word`, `words`, `now_ns`, `deadline_ns`, `wpm`, `adaptive`, and `diagnostics`. `word` is null for empty documents. Index is zero-based. Unknown fields/commands are rejected. EOF also shuts down cleanly. The virtual clock starts at zero and does not advance with wall time. `advance` processes at most one million transitions per command; split larger requests. Export is synchronous in this protocol. There is no network listener and no cross-process GUI control.

CLI export syntax is `vrx export FILE OUTPUT [WPM] [RGB]`, for example `vrx export book.md reading.mp4 400 00FF00`. RGB is six hex digits; a leading `#` is accepted when quoted in a shell. Neither CLI nor protocol export depends on saved GUI appearance. SRT has no background styling.

## Rust API

`vrx::text::Document::parse` returns a validated indexed document. `Document::shared` returns an `Arc<Document>`. `Playback::new(document, Timing)` owns reading state. Call `play(now)`, schedule `ticket().deadline`, then call `advance(ticket, actual_now)` when due. `now` must always come from the same monotonic epoch and must not move backwards. `advance` does no I/O, segmentation, formatting, or heap allocation. Only the owner should mutate playback.

`Document::display` returns the bounded borrowed display text and relative ORP byte range. `Document::word` returns the complete token; `/` terminates a token and stays attached to it, without cutting graphemes. `Timing::remaining` estimates duration using sparse categories. Custom `OrpAlgorithm` implementations plug into parsing; a custom GUI selector must trigger background reindexing too. Implement `Importer::convert` for another input format and dispatch it from the importer. Format parsers should enforce their own resource limits.

`export::write_timeline` streams ASS/SRT into any `Write`; `export::export` handles a file or MP4. Set `Config::export_background` for an opaque MP4/ASS canvas; the default is `00FF00`, independent of `Config::background`. Supply an `AtomicBool` cancellation flag. `persistence::{load,save}` provide versioned settings; `save` must be serialized by the caller. `version::label` formats dozenal human release labels.

See `examples/headless.rs` for a compilable deterministic playback example. The core returns `Result<T, String>` to keep this small release's public error boundary straightforward; typed import/export errors can be introduced with a versioned API in a future subdivision.

GUI preferences support custom digit and balanced-ternary alphabets through `Config::notation()`, which returns a `CustomNotation` supporting integer/fraction formatting and parsing. The standalone `Notation(radix)` retains the default alphabet for callers that need established symbols. These presentation settings do not alter CLI/JSON numbers or exported timestamp syntax. `Playback::estimated_remaining(now)` includes the current interval and returns zero for finished/empty/error states.
