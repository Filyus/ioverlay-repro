//! Minimal reproducer for an `i_overlay 7.0.0` OGC extraction panic.
//!
//! Expected:
//!   * `cargo run` panics in debug with:
//!     `assertion failed: overlay_rule.is_fill_top(link.fill)`
//!   * `cargo run --release` panics with:
//!     `index out of bounds: the len is 40 but the index is 9223372036854775807`
//!
//! Optional: set `PV_IOVERLAY_OGC=0` to replay the same input with non-OGC
//! extraction. That path returns a result instead of panicking.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::ShapeType;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::{FloatOverlay, OverlayOptions};
use i_overlay::i_float::adapter::FloatPointAdapter;

type Point = [f64; 2];
type Int = i64;

const FIXED_SCALE: f64 = 50_000.0;
const CONTOURS: &[&[Point]] = &[
    &[
        [24902.9222201258, 11129.9683052215],
        [24821.9592401258, 11107.1269052215],
        [24902.9218201258, 11129.9681852215],
        [24898.9601001258, 11128.8505052215],
    ],
    &[
        [20094.9253001258, 12125.6660652215],
        [20094.9253001258, 12125.6647652215],
        [29795.5156201258, 10942.5275852215],
    ],
    &[
        [24902.2200401258, 11129.7702052215],
        [24902.3098801258, 11129.7955452215],
        [24902.4788601258, 11129.8432252215],
    ],
    &[
        [24902.4819801258, 11129.8441052215],
        [24902.4832001258, 11129.8444452215],
        [24902.4821401258, 11129.8441452215],
    ],
];

fn main() {
    let shape: Vec<Vec<Point>> = CONTOURS.iter().map(|contour| contour.to_vec()).collect();
    let total_points = shape.iter().map(|contour| contour.len()).sum();
    let all_points: Vec<Point> = shape
        .iter()
        .flat_map(|contour| contour.iter().copied())
        .collect();
    let adapter = FloatPointAdapter::<Point, Int>::with_iter_and_scale_checked(
        all_points.iter(),
        FIXED_SCALE,
    )
    .unwrap();

    let mut options: OverlayOptions<f64, Int> = OverlayOptions::default();
    options.clean_result = true;
    options.ogc = std::env::var_os("PV_IOVERLAY_OGC").is_none_or(|value| value != "0");
    options.preserve_output_collinear = true;

    println!(
        "replaying {} contour(s), {total_points} points, fixed scale {FIXED_SCALE}",
        shape.len(),
    );
    println!(
        "options: clean_result={}, ogc={}, preserve_output_collinear={}",
        options.clean_result, options.ogc, options.preserve_output_collinear
    );

    let result = FloatOverlay::new_custom(adapter, options, Default::default(), total_points)
        .unsafe_add_source(&shape, ShapeType::Subject)
        .overlay(OverlayRule::Subject, FillRule::NonZero);

    println!("OK: produced {} shape(s) (no panic)", result.len());
}
