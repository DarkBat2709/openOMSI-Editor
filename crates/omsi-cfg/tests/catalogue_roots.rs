use std::path::PathBuf;
#[test]
fn catalogue_resolves_each_root_without_overlay_redirection() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp = std::env::temp_dir().join(format!("catalogue-roots-{stamp}"));
    let addon = temp.join("addon"); let original = temp.join("original");
    for root in [&addon, &original] {
        for folder in ["sPLINES", "sceneryobjects", "TEXTURE"] {
            std::fs::create_dir_all(root.join(folder)).unwrap();
        }
    }
    omsi_cfg::add_content_root(addon.clone());
    omsi_cfg::add_content_root(original.clone());
    for (request, actual) in [("Splines", "sPLINES"), ("Sceneryobjects", "sceneryobjects"), ("Texture", "TEXTURE")] {
        assert_eq!(omsi_cfg::resolve_path(&original, request), addon.join(actual));
        assert_eq!(omsi_cfg::resolve_in_content_root(&original, request), Some(original.join(actual)));
        assert_eq!(omsi_cfg::resolve_in_content_root(&addon, request), Some(addon.join(actual)));
    }
    assert_eq!(omsi_cfg::resolve_in_content_root(&original, "missing"), None::<PathBuf>);
    omsi_cfg::remove_content_root(&addon); omsi_cfg::remove_content_root(&original);
    std::fs::remove_dir_all(temp).unwrap();
}
