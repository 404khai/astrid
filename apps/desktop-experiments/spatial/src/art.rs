//! Offline identity artwork: Blobatar 2.7.0 (MIT), public mock seeds only.
use gpui::{Image, ImageFormat};
use std::sync::{Arc, OnceLock};
fn raster(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default())?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow::anyhow!("allocate artwork"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}
fn svg_image(bytes: &[u8]) -> Arc<Image> {
    // GPUI 0.2.2's Arc<Image> SVG decoder omits RGBA→BGRA conversion.
    // Its PNG decoder converts correctly. Rasterize once and cache the image.
    let png = raster(bytes).unwrap_or_else(|error| {
        eprintln!("Artwork failed: {error}");
        include_bytes!("../assets/astrid-logo.png").to_vec()
    });
    Arc::new(Image::from_bytes(ImageFormat::Png, png))
}
pub fn avatar(id: usize) -> Arc<Image> {
    static IMAGES: OnceLock<Vec<Arc<Image>>> = OnceLock::new();
    IMAGES.get_or_init(|| {
        [
            include_bytes!("../assets/avatars/0.svg").as_slice(),
            include_bytes!("../assets/avatars/1.svg").as_slice(),
            include_bytes!("../assets/avatars/2.svg").as_slice(),
            include_bytes!("../assets/avatars/3.svg").as_slice(),
            include_bytes!("../assets/avatars/4.svg").as_slice(),
        ]
        .into_iter()
        .map(svg_image)
        .collect()
    })[id % 5]
        .clone()
}
pub fn scene() -> Arc<Image> {
    static IMAGE: OnceLock<Arc<Image>> = OnceLock::new();
    IMAGE
        .get_or_init(|| svg_image(include_bytes!("../assets/scenes/night-street.svg")))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_scene_and_avatars_rasterize_to_png_for_correct_gpui_channels() {
        assert_eq!(scene().format(), ImageFormat::Png);
        for id in 0..5 {
            assert_eq!(avatar(id).format(), ImageFormat::Png);
            assert_eq!(folder(id).format(), ImageFormat::Png);
        }
        let bytes = raster(include_bytes!("../assets/scenes/night-street.svg")).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
}
pub fn logo() -> Arc<Image> {
    static IMAGE: OnceLock<Arc<Image>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../assets/astrid-logo.png").to_vec(),
            ))
        })
        .clone()
}

/// Individual interactive folder plates, derived from the supplied rounded
/// folder/tab silhouette. Cached PNGs avoid GPUI 0.2.2 SVG channel conversion.
pub fn folder(id: usize) -> Arc<Image> {
    static IMAGES: OnceLock<Vec<Arc<Image>>> = OnceLock::new();
    IMAGES.get_or_init(|| {
        [("#7eabf0", "#4260b8"), ("#dfb8fa", "#9576c9"), ("#c0e98b", "#649d87"), ("#c4aafa", "#8777c4"), ("#ffdb9f", "#ed9069")]
            .iter().map(|(top,bottom)| {
                let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="880" height="390" viewBox="0 0 880 390"><defs><linearGradient id="f" x2="0" y2="1"><stop stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient></defs><path d="M2 65 Q2 47 20 47 L66 47 L66 17 Q66 2 84 2 L224 2 Q236 2 245 14 L270 47 L860 47 Q878 47 878 65 L878 368 Q878 388 858 388 L22 388 Q2 388 2 368 Z" fill="url(#f)" stroke="#ffffff" stroke-opacity=".28" stroke-width="2"/><path d="M18 69 H862" stroke="#ffffff" stroke-opacity=".24"/></svg>"##);
                svg_image(svg.as_bytes())
            }).collect()
    })[id % 5].clone()
}
