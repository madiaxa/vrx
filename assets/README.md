# VRX artwork and numeric font

`logo.png` is the original logo supplied by the user, preserved unchanged with its white background. Generated ICNS, ICO and raw window-icon pixels are size variants of that artwork. macOS bundles use VRX.icns; native Windows MSVC builds embed VRX.ico; native window-manager icons use the same logo. The artwork is not part of the reading interface.

Clockette Mono is the OFL font used by Clockette for Pitman dozenal numerals. It is reused unchanged for VRX numeric controls so ↊ and ↋ retain full cap height and a consistent advance. The font is embedded; its SIL Open Font License is included in OFL.txt. RSVP words still use the independently selected reading font. GUI numbers follow Clockette's digit alphabet and T/0/1 balanced ternary; no Clockette executable or runtime dependency is bundled.
