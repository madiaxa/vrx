VRX v0.0.1 is the first native RSVP reader subdivision.

The personalization revision retains v0.0.1 and adds the supplied logo, configurable shortcuts, radix-aware numeric controls with balanced ternary, direct numeric editing, draggable positioning, optional focal highlighting, centered focus mode, independent GUI type scaling and separate punctuation pauses.

It includes indexed Unicode text and Markdown, shaped ORP anchoring, monotonic playback, adaptive pauses, keyboard navigation and configurable jumps/pace steps, themes and typography, saved file progress, a JSON-lines agent API, ASS/SRT export, and optional FFmpeg MP4 rendering.

Assets target Apple Silicon macOS, Linux x86_64, Windows x86_64, and FreeBSD 15.0 amd64. Archives include usage instructions, limitations, architecture, measurement evidence, and dependency notices. macOS builds use local ad-hoc signing, not Apple notarization; FFmpeg is not bundled. Consult BUILD.json for target and source commit.

Platform CI/build success and manual desktop verification are different. Review the current Actions run and perform the release checklist on each intended desktop before publishing. The checked-in measurements describe the development macOS system, not every downloaded binary.

Final changes retain v0.0.1: slash-preserving token splits, a simplified keymap, a green-by-default independent export canvas, toolbar text editing with in-editor Paste, and direct radix input. Contributor guidance is consolidated in docs/CONTRIBUTING.md; platform/GitHub build instructions are consolidated in docs/RELEASE.md. These final changes were compiled without running tests at the requester's direction.

The latest update preserves the edited GUI, implements ClearText in the editor, makes fractional sliders step in the selected radix, and shortens number formatting without losing stored precision. Focal guides now support shape/color/size/gap customization and scale with RSVP type and word fitting. Three focused regression tests passed; no full-suite or native smoke run was performed for this update.

VRX's original code is released under the Unlicense. Dependency and font licenses remain unchanged.
