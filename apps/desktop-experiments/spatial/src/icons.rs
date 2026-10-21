//! MIT Hugeicons Stroke Rounded, exported from @hugeicons/core-free-icons 4.3.5.
use gpui::{AssetSource, SharedString};
use std::borrow::Cow;
pub struct Icons;
macro_rules! assets {
    ($path:expr; $($name:literal),*) => { match $path {
        $(concat!($name,".svg")=>Some(include_bytes!(concat!("../assets/hugeicons/",$name,".svg")).as_slice()),)*
        _=>None,
    }};
}
impl AssetSource for Icons {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(assets!(path; "Search01","LayoutGrid","Command","ZoomInArea","ZoomOutArea","Pin","PinOff","ArrowExpand","ArrowShrink","Refresh","Undo02","Attachment01","Sent","Motion01").map(Cow::Borrowed))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
