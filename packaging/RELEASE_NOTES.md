# VRX 0.1.0

VRX is an early native RSVP reader with a reusable Rust library and CLI/JSON agent API. This working version uses standard decimal semantic versioning from Cargo; release tags use the v prefix.

- Radix/base and numeral-alphabet controls are grouped under a collapsed Radix notation dropdown in Personalize. Reading values still use the selected notation.
- Adaptive timing and Keybindings now have matching collapsed sections. Their headers and panels follow the theme accent, with a separate customizable Dropdown color override. Hidden invalid fields reopen their section when saving; collapsing shortcuts cancels recording.
- Personalize is 25% wider in windowed mode; fullscreen retains its previous widths. Native window-mode changes are reflected in the layout.
- About VRX centers its remaining text and controls and shows brief license information instead of the full license text. Complete licenses remain bundled.
- README wording has been audited against the implementation, including build prerequisites, platform-verification limits, download availability and progress persistence.

Retained features include Unicode text/Markdown input, in-window editing, shaped focal anchoring, configurable timing, countdowns, opacity and guide styling, custom alphabets, previews, confirmed preference reset/discard, native macOS About, saved file positions, timed subtitles and optional external-FFmpeg MP4 export.

VRX uses the Unlicense; dependencies and the OFL Axa Mono font retain their own licenses. macOS archives use local ad-hoc signing rather than public notarization. See docs/RELEASE.md for build instructions and the distinction between local verification, cross-target checks and native platform runs.
