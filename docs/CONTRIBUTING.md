# Contributing to VRX

Start with the [README](../README.md), [architecture](ARCHITECTURE.md), [API](API.md), and [known limitations](LIMITATIONS.md). [RELEASE.md](RELEASE.md) contains all build, validation, platform and GitHub release instructions. Both human and AI contributors use the same project structure and standards.

## Where to work

`Document::parse_cancellable` builds the indexed source; `Playback::advance` owns deadline transitions; `App::update` routes UI actions; `Reader::layout/draw` maintains shaped alignment. Configuration is in `src/config.rs`, editable preferences in `src/ui/settings.rs`, and video/subtitle output in `src/export.rs`. The public library stays independent of Iced. Match APIs to the actual locked dependencies and installed stable Rust, rather than relying on examples for a different Iced release.

## Invariants

- Preserve validated UTF-8/grapheme offsets, shared source storage, and indexed access. Slash-delimited tokens retain `/` on the preceding token. Document complex-script behavior instead of claiming unsupported segmentation or fixation.
- Keep typed playback states, absolute monotonic deadlines, generation invalidation, bounded pending work and prompt cancellation. Configured jumps and speed changes retain playback without losing or repeating words.
- Keep file I/O, parsing, segmentation, persistence and export outside views and word-advance paths. Paused playback has no recurring timer. Avoid polling, unbounded queues and a separately owned string per word.
- Shape proportional text and preserve the measured focal center across words, resize, fonts and DPI. Do not substitute character count for glyph geometry.
- Use safe Rust, checked access and recoverable errors. Application code forbids unsafe and explicit unwrap/expect/panic/todo/unimplemented/unreachable paths. Allocator, OS, driver and dependency failures remain outside that guarantee.
- Measure demonstrated bottlenecks before adding dependencies, runtimes, threads or storage complexity. Record actual hardware, input, method and limits when reporting performance.
- Keep document contents out of settings, logs and shared fixtures without explicit intent. Original input files are not modified by the editor.

## Submitting changes

Keep changes focused and documentation current. Use deterministic virtual-time tests for engine behavior; OS wake precision belongs in measurement tools, not fragile sub-millisecond assertions. Formatting, relevant tests and Clippy are part of the normal contribution process; respect an explicit request to omit tests for a particular change. Never describe unrun checks, configured CI, or compatibility expectations as verified results.

A useful contribution summary explains behavior, motivation, verification actually performed, and remaining limits. Native developer hooks belong behind the `smoke` feature and are excluded from ordinary release builds. Follow the existing dozenal human-version convention while keeping Cargo and OS metadata valid numeric versions.
