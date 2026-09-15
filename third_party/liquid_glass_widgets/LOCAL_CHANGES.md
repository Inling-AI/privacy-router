# Local Changes

Source: pub.dev `liquid_glass_widgets` 1.4.3, MIT licensed.
Only runtime sources, shaders, license and package manifest are vendored.

## Window-Level Split Navigation

`GlassNavigationShell.chromeInsets` positions the pinned controls inside a
detail pane while retaining a window-sized overlay. Insets are passed to
`GlassNavPinnedHost`; they affect positioning, not clipping or glass rendering.

The application puts its split layout, including the sidebar, inside this shell.
Route content remains pane-sized. Chrome paints above all pane surfaces, so
sidebar shadows cannot cover navigation controls and navigation shadows are not
cut off at the detail pane boundary. Existing callers default to zero insets.

No button materials, shapes, shaders or transition algorithms are changed.
When upgrading upstream, reapply these two small source changes or replace them
with an equivalent upstream API. Do not patch the global pub cache.
