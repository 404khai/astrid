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
