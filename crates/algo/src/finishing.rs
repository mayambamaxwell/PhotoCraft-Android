//! Original, bounded highlight and RGB channel-shift effects.
use crate::{Ctx, Edge, FilterParams, Image};
use photocraft_color::{ColorMode, PixelFormat, SampleType};
use photocraft_geom::Rect;
use photocraft_raster::{from_rgba_into, to_rgba};
fn rgba(src: &Image, x: i32, y: i32, ctx: &Ctx) -> [f32; 4] {
    let mut p = [0.; 8];
    for (c, v) in p.iter_mut().enumerate().take(src.ch.min(8)) {
        *v = src.get(x, y, c);
    }
    to_rgba(&PixelFormat::new(ctx.mode, SampleType::F32, ctx.alpha), &p[..src.ch.min(8)])
}
fn highlights(src: &Image, ctx: &Ctx, threshold: f32) -> Image {
    let mut image = Image::new(src.rect, 4);
    for (i, p) in image.data.chunks_exact_mut(4).enumerate() {
        let x = src.rect.x0 + (i % src.rect.width().max(1) as usize) as i32;
        let y = src.rect.y0 + (i / src.rect.width().max(1) as usize) as i32;
        let a = rgba(src, x, y, ctx);
        let bright = (0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2]).max(0.);
        let weight = ((bright - threshold) / (1. - threshold).max(0.001)).clamp(0., 1.);
        p.copy_from_slice(&[a[0], a[1], a[2], a[3] * weight]);
    }
    image
}
pub(crate) fn run(params: &FilterParams, src: &Image, out: Rect, ctx: &Ctx) -> Vec<f32> {
    if src.ch == 0 || src.ch > 8 {
        return src.crop(out);
    }
    if matches!(params,FilterParams::ChromaticAberration{distance,..}if *distance==0.)
        || matches!(params,FilterParams::Bloom{brightness,..}|FilterParams::CrossFilter{brightness,..}if *brightness==0.)
    {
        return src.crop(out);
    }
    let fmt = PixelFormat::new(ctx.mode, SampleType::F32, ctx.alpha);
    match params {
        FilterParams::Bloom { radius, threshold, brightness } | FilterParams::CrossFilter { radius, threshold, brightness, .. } => {
            let bright = highlights(src, ctx, *threshold);
            let rgb = Ctx { bounds: ctx.bounds, mode: ColorMode::Rgb, alpha: true };
            let glow = if let FilterParams::CrossFilter { count, direction, .. } = params {
                let rays = (*count).clamp(2, 8).div_ceil(2);
                let mut sum = vec![0.; out.width() as usize * out.height() as usize * 4];
                for ray in 0..rays {
                    let angle = *direction + 180. * ray as f32 / rays as f32;
                    let image = crate::blur::motion(&bright, out, &rgb, angle, *radius * 2.);
                    for (dst, v) in sum.iter_mut().zip(image) {
                        *dst += v / rays as f32;
                    }
                }
                sum
            } else {
                crate::blur::gaussian(&bright, out, &rgb, *radius)
            };
            let mut output = vec![0.; out.width() as usize * out.height() as usize * src.ch];
            for (i, (dst, g)) in output.chunks_exact_mut(src.ch).zip(glow.chunks_exact(4)).enumerate() {
                let x = out.x0 + (i % out.width().max(1) as usize) as i32;
                let y = out.y0 + (i / out.width().max(1) as usize) as i32;
                let orig = rgba(src, x, y, ctx);
                let ga = (g[3] * brightness).clamp(0., 1.);
                let a = orig[3] + ga * (1. - orig[3]);
                let mut rgb = [0.; 4];
                rgb[3] = a;
                if a > 0. {
                    for c in 0..3 {
                        rgb[c] = (orig[c] * orig[3] + g[c] * ga) / a;
                    }
                }
                from_rgba_into(&fmt, rgb, dst);
            }
            output
        }
        FilterParams::ChromaticAberration { distance, angle, zoom, center_x, center_y, reverse } => {
            let mut output = vec![0.; out.width() as usize * out.height() as usize * src.ch];
            let cx = ctx.bounds.x0 as f32 + ctx.bounds.width() as f32 * center_x;
            let cy = ctx.bounds.y0 as f32 + ctx.bounds.height() as f32 * center_y;
            let rad = angle.to_radians();
            for (i, dst) in output.chunks_exact_mut(src.ch).enumerate() {
                let x = out.x0 + (i % out.width().max(1) as usize) as i32;
                let y = out.y0 + (i / out.width().max(1) as usize) as i32;
                let orig = rgba(src, x, y, ctx);
                let (dx, dy) =
                    if *zoom { ((x as f32 - cx) * distance / 100., (y as f32 - cy) * distance / 100.) } else { (rad.cos() * distance, rad.sin() * distance) };
                let mut result = orig;
                for (c, sign) in [(0, -1.), (2, 1.)] {
                    let sign = if *reverse { -sign } else { sign };
                    let mut px = [0.; 8];
                    src.sample(x as f32 + 0.5 + sign * dx, y as f32 + 0.5 + sign * dy, Edge::Repeat, ctx.bounds, ctx.alpha, &mut px[..src.ch.min(8)]);
                    result[c] = to_rgba(&fmt, &px[..src.ch.min(8)])[c];
                }
                from_rgba_into(&fmt, result, dst);
            }
            output
        }
        _ => src.crop(out),
    }
}
