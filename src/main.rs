//! Minimal reproducer for an i_overlay 7.0.0 panic on a real degenerate input.
//!
//! Loads `culprit-min.iovs` — a delta-minimized version of the exact float input
//! that the `projected_visibility` crate handed to i_overlay when projecting one
//! architectural unit — and replays the same overlay call. The input contains
//! near-coincident / sub-pixel-sliver contours.
//!
//! Expected:
//!   * debug build  (`cargo run`)            -> panics in i_overlay OGC extraction:
//!         "assertion failed: overlay_rule.is_fill_top(link.fill)"
//!   * release build(`cargo run --release`)  -> the assertion is compiled out, so
//!         the fixed-point core corrupts instead and panics downstream with
//!         "index out of bounds: the len is N but the index is 9223372036854775807".
//!
//! Run:  cargo run        (debug, shows the real assertion)
//!       cargo run --release
//!
//! Optional: set `PV_IOVERLAY_OGC=0` to replay the same input with non-OGC
//! extraction while keeping the captured adapter scale and source geometry.

use std::io::{Cursor, Read};
use std::path::PathBuf;

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::ShapeType;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::{FloatOverlay, OverlayOptions};
use i_overlay::i_float::adapter::FloatPointAdapter;

type Point = [f64; 2];
type Contour = Vec<Point>;
type Shape = Vec<Contour>;
type Shapes = Vec<Shape>;
type Int = i64;

fn read_u32(c: &mut Cursor<&[u8]>) -> u32 {
    let mut b = [0u8; 4];
    c.read_exact(&mut b).unwrap();
    u32::from_le_bytes(b)
}
fn read_f64(c: &mut Cursor<&[u8]>) -> f64 {
    let mut b = [0u8; 8];
    c.read_exact(&mut b).unwrap();
    f64::from_le_bytes(b)
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("culprit-min.iovs"));
    let bytes = std::fs::read(&path).expect("read .iovs fixture");
    let mut c = Cursor::new(bytes.as_slice());

    let mut magic = [0u8; 4];
    c.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"IOVS");
    let _version = read_u32(&mut c);
    let rule_code = read_u32(&mut c);
    let scale = read_f64(&mut c); // IOVERLAY_UNION_FIXED_SCALE used when captured
    let source_count = read_u32(&mut c) as usize;

    let rule = match rule_code {
        0 => OverlayRule::Subject,
        1 => OverlayRule::Clip,
        2 => OverlayRule::Intersect,
        3 => OverlayRule::Union,
        4 => OverlayRule::Difference,
        5 => OverlayRule::InverseDifference,
        _ => OverlayRule::Xor,
    };

    let mut sources: Vec<(ShapeType, Shapes)> = Vec::new();
    for _ in 0..source_count {
        let type_code = read_u32(&mut c);
        let shape_type = if type_code == 0 {
            ShapeType::Subject
        } else {
            ShapeType::Clip
        };
        let nshapes = read_u32(&mut c) as usize;
        let mut shapes: Shapes = Vec::with_capacity(nshapes);
        for _ in 0..nshapes {
            let ncontours = read_u32(&mut c) as usize;
            let mut shape: Shape = Vec::with_capacity(ncontours);
            for _ in 0..ncontours {
                let npoints = read_u32(&mut c) as usize;
                let mut contour: Contour = Vec::with_capacity(npoints);
                for _ in 0..npoints {
                    let x = read_f64(&mut c);
                    let y = read_f64(&mut c);
                    contour.push([x, y]);
                }
                shape.push(contour);
            }
            shapes.push(shape);
        }
        sources.push((shape_type, shapes));
    }

    let total_points: usize = sources
        .iter()
        .flat_map(|(_, s)| s.iter())
        .flat_map(|sh| sh.iter())
        .map(|c| c.len())
        .sum();
    println!(
        "loaded {}: rule={rule:?}, scale={scale}, {} source(s), {} contour(s), {total_points} points",
        path.display(),
        sources.len(),
        sources
            .iter()
            .flat_map(|(_, s)| s.iter())
            .map(|sh| sh.len())
            .sum::<usize>(),
    );

    // Same adapter + options the projected_visibility crate uses.
    let all_points: Vec<Point> = sources
        .iter()
        .flat_map(|(_, shapes)| {
            shapes
                .iter()
                .flat_map(|shape| shape.iter())
                .flat_map(|contour| contour.iter().copied())
        })
        .collect();
    let adapter: FloatPointAdapter<Point, Int> =
        match FloatPointAdapter::with_iter_and_scale_checked(all_points.iter(), scale) {
            Ok(adapter) => {
                println!("adapter: accepted captured fixed scale {scale}");
                adapter
            }
            Err(err) => {
                println!("adapter: rejected captured fixed scale {scale}: {err:?}; using automatic scale");
                FloatPointAdapter::with_iter(all_points.iter())
            }
        };

    let mut options: OverlayOptions<f64, Int> = OverlayOptions::default();
    options.clean_result = true;
    options.ogc = std::env::var_os("PV_IOVERLAY_OGC").is_none_or(|value| value != "0");
    options.preserve_output_collinear = true;
    println!(
        "options: clean_result={}, ogc={}, preserve_output_collinear={}",
        options.clean_result, options.ogc, options.preserve_output_collinear
    );

    let mut overlay = FloatOverlay::new_custom(adapter, options, Default::default(), total_points);
    for (shape_type, shapes) in &sources {
        overlay = overlay.unsafe_add_source(shapes, *shape_type);
    }

    println!("calling i_overlay overlay({rule:?}, NonZero) ...");
    let result = overlay.overlay(rule, FillRule::NonZero);
    println!("OK: produced {} shape(s) (no panic)", result.len());
}
