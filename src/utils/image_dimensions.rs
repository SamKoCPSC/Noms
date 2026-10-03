//! Canvas sizing shared with native regression tests.

/// Fit a decoded image inside a square bound without enlarging the source.
/// A real image needs at least one pixel on each axis for canvas encoding.
pub(super) fn calculate_dimensions(src_w: u32, src_h: u32, max_dim: u32) -> (u32, u32) {
    // The decoder rejects empty images; keep invalid inputs empty rather than
    // inventing a usable canvas when this helper is called independently.
    if src_w == 0 || src_h == 0 || max_dim == 0 {
        return (0, 0);
    }
    if src_w <= max_dim && src_h <= max_dim {
        return (src_w, src_h);
    }
    let ratio = src_w as f64 / src_h as f64;
    if src_w > src_h {
        (max_dim, ((max_dim as f64 / ratio) as u32).max(1))
    } else {
        (((max_dim as f64 * ratio) as u32).max(1), max_dim)
    }
}

#[cfg(test)]
mod tests {
    use super::calculate_dimensions;

    #[test]
    fn thin_images_keep_one_pixel_on_the_short_axis() {
        for limit in [800, 1920] {
            assert_eq!(calculate_dimensions(4096, 1, limit), (limit, 1));
            assert_eq!(calculate_dimensions(1, 4096, limit), (1, limit));
        }
    }

    #[test]
    fn ordinary_landscape_portrait_and_square_sizes_are_preserved() {
        for (source, limit, expected) in [
            ((3000, 1000), 1920, (1920, 640)),
            ((1000, 3000), 1920, (640, 1920)),
            ((2000, 2000), 1920, (1920, 1920)),
            ((3000, 2000), 800, (800, 533)),
            ((2000, 3000), 800, (533, 800)),
        ] {
            assert_eq!(calculate_dimensions(source.0, source.1, limit), expected);
        }
    }

    #[test]
    fn small_and_exact_boundary_images_are_not_enlarged() {
        for (width, height, limit) in [(100, 100, 800), (1920, 1080, 1920), (1, 1, 1920)] {
            assert_eq!(calculate_dimensions(width, height, limit), (width, height));
        }
    }

    #[test]
    fn maximum_source_dimensions_do_not_overflow_or_collapse() {
        assert_eq!(calculate_dimensions(u32::MAX, 1, 1920), (1920, 1));
        assert_eq!(calculate_dimensions(1, u32::MAX, 800), (1, 800));
        assert_eq!(calculate_dimensions(u32::MAX, u32::MAX, 1920), (1920, 1920));
    }

    #[test]
    fn positive_dimensions_stay_inside_bounds_without_upscaling() {
        for width in [1, 2, 319, 800, 1920, 4096, u32::MAX] {
            for height in [1, 2, 320, 800, 1920, 4096, u32::MAX] {
                for limit in [1, 800, 1920] {
                    let (w, h) = calculate_dimensions(width, height, limit);
                    assert!(
                        (1..=width.min(limit)).contains(&w),
                        "{width}x{height} -> {w}x{h}"
                    );
                    assert!(
                        (1..=height.min(limit)).contains(&h),
                        "{width}x{height} -> {w}x{h}"
                    );
                }
            }
        }
    }

    #[test]
    fn empty_sources_or_bounds_do_not_create_a_canvas_size() {
        for (width, height, limit) in [(0, 320, 800), (480, 0, 1920), (480, 320, 0)] {
            assert_eq!(calculate_dimensions(width, height, limit), (0, 0));
        }
    }

    #[test]
    fn browser_pipeline_uses_the_native_tested_sizing_contract() {
        let source = include_str!("image_processing_client.rs");
        assert!(source.contains("use super::image_dimensions::calculate_dimensions;"));
        assert!(!source.contains("fn calculate_dimensions("));
        assert!(source.contains(
            "calculate_dimensions(img.natural_width(), img.natural_height(), MAX_DIMENSION)"
        ));
        assert!(source.contains(
            "calculate_dimensions(img.natural_width(), img.natural_height(), THUMB_SIZE)"
        ));
    }
}
