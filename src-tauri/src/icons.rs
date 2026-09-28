//! Runtime-rendered tray icons (mouse glyph + battery level).

use tauri::image::Image;
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Transform};

const SIZE: u32 = 64;

const BODY: [u8; 4] = [232, 232, 232, 255];
const EDGE: [u8; 4] = [34, 34, 34, 255];
const GREEN: [u8; 4] = [63, 185, 80, 255];
const AMBER: [u8; 4] = [227, 179, 65, 255];
const RED: [u8; 4] = [248, 81, 73, 255];
const GRAY: [u8; 4] = [150, 150, 150, 255];

/// Static mouse glyph for the left tray icon.
pub fn mouse_icon() -> Image<'static> {
    let mut pm = Pixmap::new(SIZE, SIZE).expect("icon pixmap");

    // Body: dark outline under light fill (two nested rounded rects).
    fill(&mut pm, &rounded_rect(18.0, 6.0, 28.0, 52.0, 14.0), EDGE);
    fill(&mut pm, &rounded_rect(21.0, 9.0, 22.0, 46.0, 11.0), BODY);

    // Button split line.
    fill(&mut pm, &rect(31.0, 9.0, 2.0, 25.0), EDGE);

    // Scroll wheel.
    fill(&mut pm, &rounded_rect(30.0, 12.0, 4.0, 14.0, 2.0), EDGE);

    into_image(pm)
}

/// Battery glyph; `level` is a percentage, `None` shows "no data" slash.
pub fn battery_icon(level: Option<u8>) -> Image<'static> {
    let mut pm = Pixmap::new(SIZE, SIZE).expect("icon pixmap");

    // Case: dark outline under light fill.
    fill(&mut pm, &rounded_rect(6.0, 19.0, 50.0, 26.0, 6.0), EDGE);
    fill(&mut pm, &rounded_rect(9.0, 22.0, 44.0, 20.0, 4.0), BODY);

    // Terminal nub.
    fill(&mut pm, &rounded_rect(56.0, 27.0, 5.0, 10.0, 2.0), EDGE);

    match level {
        Some(pct) => {
            let pct = pct.min(100);
            let color = if pct >= 50 {
                GREEN
            } else if pct >= 20 {
                AMBER
            } else {
                RED
            };
            // Charge bar: area x 12..48 (36 px wide), y 25..39.
            let w = 36.0 * f32::from(pct) / 100.0;
            if w >= 1.0 {
                fill(&mut pm, &rounded_rect(12.0, 25.0, w, 14.0, 3.0), color);
            }
        }
        None => {
            // Diagonal "no data" slash.
            fill(
                &mut pm,
                &quad([[15.0, 37.0], [19.0, 39.5], [47.0, 24.5], [43.0, 22.0]]),
                GRAY,
            );
        }
    }

    into_image(pm)
}

fn into_image(pm: Pixmap) -> Image<'static> {
    let (w, h) = (pm.width(), pm.height());
    // tiny-skia stores premultiplied RGBA; tray icons expect straight RGBA.
    let mut rgba = pm.take();
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3];
        if a > 0 && a < 255 {
            px[0] = (u16::from(px[0]) * 255 / u16::from(a)).min(255) as u8;
            px[1] = (u16::from(px[1]) * 255 / u16::from(a)).min(255) as u8;
            px[2] = (u16::from(px[2]) * 255 / u16::from(a)).min(255) as u8;
        }
    }
    Image::new_owned(rgba, w, h)
}

fn paint(rgba: [u8; 4]) -> Paint<'static> {
    Paint {
        anti_alias: true,
        shader: tiny_skia::Shader::SolidColor(tiny_skia::Color::from_rgba8(
            rgba[0], rgba[1], rgba[2], rgba[3],
        )),
        ..Paint::default()
    }
}

fn fill(pm: &mut Pixmap, path: &Path, rgba: [u8; 4]) {
    pm.fill_path(path, &paint(rgba), FillRule::Winding, Transform::identity(), None);
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    PathBuilder::from_rect(Rect::from_xywh(x, y, w, h).expect("rect"))
}

/// Cubic control-point factor for a quarter-circle arc of radius `r`.
const KAPPA: f32 = 0.552_284_8;

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Path {
    let r = r.min(w / 2.0).min(h / 2.0);
    let k = r * KAPPA;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish().expect("rounded rect path")
}

/// Parallelogram through four points (used for the slash).
fn quad(pts: [[f32; 2]; 4]) -> Path {
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0][0], pts[0][1]);
    for p in &pts[1..] {
        pb.line_to(p[0], p[1]);
    }
    pb.close();
    pb.finish().expect("quad path")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_render_with_rgba_buffer() {
        for img in [mouse_icon(), battery_icon(Some(100)), battery_icon(None)] {
            let _ = img.rgba();
        }
    }

    #[test]
    fn battery_fill_scales_with_level() {
        // Guard against NaN/negative dimensions breaking pixmap fills.
        for level in [0u8, 1, 20, 49, 50, 99, 100, 255] {
            let _ = battery_icon(Some(level));
        }
    }

    /// Renders preview PNGs for visual inspection (`cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn render_preview_pngs() {
        let out = std::path::Path::new("/tmp/opencode/icons");
        std::fs::create_dir_all(out).unwrap();
        for (name, level) in [
            ("mouse", None),
            ("battery_100", Some(100u8)),
            ("battery_50", Some(50)),
            ("battery_15", Some(15)),
            ("battery_none", None),
        ] {
            let img = if name == "mouse" {
                mouse_icon()
            } else {
                battery_icon(level)
            };
            let mut pm = Pixmap::new(img.width(), img.height()).unwrap();
            pm.data_mut().copy_from_slice(img.rgba());
            let path = out.join(format!("{name}.png"));
            pm.save_png(&path).unwrap();
            println!("wrote {}", path.display());
        }
    }
}
