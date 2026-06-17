# i_overlay OGC extraction repro

Minimal reproducer for an `i_overlay 7.0.0` panic on a real degenerate input.

The fixture `culprit-min.iovs` is a minimized version of float input captured from
`projected_visibility`. It contains 4 contours / 13 points and replays the same
fixed-scale `FloatOverlay` call:

- `OverlayRule::Subject`
- `FillRule::NonZero`
- fixed adapter scale `50000`
- `clean_result = true`
- `ogc = true`
- `preserve_output_collinear = true`

## Run

Debug build:

```powershell
cargo run
```

Expected panic:

```text
assertion failed: overlay_rule.is_fill_top(link.fill)
```

Release build:

```powershell
cargo run --release
```

Expected panic:

```text
index out of bounds: the len is 40 but the index is 9223372036854775807
```

The same input passes when OGC extraction is disabled:

```powershell
$env:PV_IOVERLAY_OGC='0'
cargo run
```

Expected result:

```text
OK: produced 53 shape(s) (no panic)
```
