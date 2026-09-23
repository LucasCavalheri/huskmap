//! Rasterize `packaging/icons/huskmap.svg` with Skia into the hicolor PNG sizes.
//!
//! Run with `HUSKMAP_WRITE_ICONS=1` to refresh `packaging/icons/*.png`; otherwise the PNGs are
//! rendered to the target dir and checked against the committed ones for size only.
#![cfg(feature = "desktop")]

use std::path::PathBuf;
use std::time::Duration;

use freya::prelude::*;
use freya_testing::prelude::TestingRunner;

const SVG: &[u8] = include_bytes!("../../packaging/icons/huskmap.svg");

fn render(size: f32) -> Vec<u8> {
    let (mut t, _) = TestingRunner::new(
        move || {
            rect()
                .width(Size::px(size))
                .height(Size::px(size))
                .child(
                    SvgViewer::new(("huskmap-icon", SVG))
                        .show_loader(false)
                        .parallel(false)
                        .width(Size::px(size))
                        .height(Size::px(size)),
                )
                .into_element()
        },
        Size2D::new(size, size),
        |_| {},
        1.0,
    );
    t.sync_and_update();
    t.poll(Duration::from_millis(16), Duration::from_millis(200));
    t.render().as_bytes().to_vec()
}

#[test]
fn icon_sizes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../packaging/icons");
    let write = std::env::var_os("HUSKMAP_WRITE_ICONS").is_some();
    let out = if write {
        root.clone()
    } else {
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("icons")
    };
    std::fs::create_dir_all(&out).unwrap();
    for size in [32u32, 48, 64, 128, 256, 512] {
        let png = render(size as f32);
        assert_eq!(&png[1..4], b"PNG");
        let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
        assert_eq!(width, size);
        std::fs::write(out.join(format!("huskmap-{size}.png")), &png).unwrap();
    }
}
