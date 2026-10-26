# Performance methodology

The earlier slash/editor/export revision was not tested or benchmarked at the requester's direction. The latest ClearText/slider/focal update ran only three focused tests and a release build; no new timing, native rendering, or full-suite measurements were collected. The figures below are earlier v0.0.1 engineering baselines, not results for the final source. See [RELEASE.md](RELEASE.md) for build and validation commands.

## Historical baseline

Measured on 2026-10-25: Apple M4, 24 GB RAM, macOS 27.0.1, Rust 1.99.1, optimized builds. Criterion indexed a mixed Unicode fixture in 3.684 ms (~10,000 words), 37.545 ms (~100,000), and 375.393 ms (~1,000,000), consistent with linear scaling. Synthetic engine transitions averaged 8.56–8.80 ns, excluding waiting, shaping, GUI and I/O. `benches/baselines/core.json` retains the estimates and confidence intervals.

Earlier native Iced measurements collected 30 samples at 200, 400, 800, 1,200, 2,000 and 4,000 WPM. Wake-lateness p99 was approximately 3.8–5.5 ms; at 4,000 WPM, wake-to-UI p99 reached 15.9 ms. Raw timestamps and summaries are in `benches/baselines/native-timing.json`. A 15 ms requested slot can outrun a 60 Hz display; no physical visibility guarantee follows from zero engine skips. Earlier paused snapshots reported 0.0% CPU and about 95 MiB RSS; they were not a soak, leak or whole-application allocation profile.

## What to measure

Separate nominal deadline, timer wake, UI dispatch, CPU draw-command submission, GPU completion and physical display presentation. VRX records the first four where available. CPU submission is not scanout. The opt-in `smoke` feature includes a native timing harness selected by `VRX_BENCH_DIR`; ordinary releases exclude it. The CLI `diagnose` command measures OS deadlines without a GUI. Both are explicit measurement tools, never idle polling.

Use at least 1,000 transitions per speed for p50/p95/p99 estimates; the historical 30-sample p99 is essentially an observed maximum. Record background load, refresh rate, renderer, font, diagnostics state and hardware. Include 200/400/800/1,200/2,000 WPM and a faster stress rate. Real scanout needs platform present timestamps or a high-speed camera.

## Manual profiling and rendering checklist

1. Step through `tests/fixtures/render.txt` with proportional fonts, ligatures, combining marks, emoji and contextual scripts. Check ORP geometry, whole-word fallback, focal-disabled text, clipping and 20/76/360 px sizes.
2. Resize, toggle H/fullscreen, drag the anchor, and increase GUI text size. Check separate RSVP sizing, short-window scrolling, dark/light/contrast/custom themes, 100/150/200% DPI and mixed-DPI monitors.
3. Profile large import/cancellation while interacting. Confirm worker preprocessing and bounded pending requests. Include malformed/empty files and slash-delimited Unicode text.
4. Change pace and configured jump sizes while playing; pause/resume/seek/restart and replace the document with a timer pending. Use virtual-time tests for logical invariants, rather than demanding OS sub-millisecond accuracy.
5. Inspect editor prefill, selection/cursor paste, cancellation, Apply changes, late clipboard replies, and original-file preservation. Export MP4/ASS with green and a custom canvas while changing reader themes; SRT remains unstyled.
6. Sample CPU-time deltas and RSS for at least 60 seconds paused, with no blinking input; run a load/unload soak before claiming no leaks. Use Instruments, heaptrack/perf, or WPR/WPA to distinguish import, playback, shaping and UI allocations.
7. Measure cold start externally from process launch to first presentation, using at least 30 samples and recording reboot/font-cache state. Keep warm launches separate.
8. Check progress recovery, corrupt settings, native dialogs/clipboard/drop, export cancellation, missing FFmpeg and video-editor import. Native linking does not certify desktop rendering or screen-reader integration.

No latest performance target, cross-platform runtime, extended soak or zero-allocation GUI claim is made without new measurements.
