//! Direct map editing, independent of the selected driving duty.
use super::{Launcher, theme::*, ui::ButtonKind};
use glam::Vec2;
use omsi_launcher_lib as core;
use omsi_ui::{Rect, Weight};

pub fn draw(l: &mut Launcher, area: Rect) {
    let subtitle = if l.state.in_game() { "Please end the current session before starting a map in the editor." }
        else { "Create a new map or edit an existing map directly." };
    let body = l.page_title(area, "Editor", subtitle);
    let r = Rect::new(body.x, body.y, body.w.min(820.0), 380.0);
    l.ui.panel(r);
    let inner = l.ui.heading(Rect::new(r.x + 20.0, r.y + 16.0, r.w - 40.0, r.h - 32.0), "New empty map", Some("add"));
    let mut y = inner.y;
    for (id, label, value, placeholder) in [
        ("editor-name", "Map name", &mut l.pages.editor_name, "My new map"),
        ("editor-author", "Author (optional)", &mut l.pages.editor_author, "Name or nickname"),
        ("editor-description", "Description", &mut l.pages.editor_description, "Short description (optional)"),
    ] {
        l.ui.label(Rect::new(inner.x, y, 150.0, ROW), label);
        l.ui.text_input(id, Rect::new(inner.x + 150.0, y, inner.w - 150.0, ROW), value, placeholder, None);
        y += ROW + 10.0;
    }
    let content = l.pages.editor_content.clone();
    let destination = content.as_ref().map(|p| p.join("maps").join(l.pages.editor_name.trim())).map(|p| p.display().to_string())
        .unwrap_or_else(|| "Please configure the game folder under Setup first.".into());
    y += l.ui.paragraph(&format!("Save location: {destination}"), Vec2::new(inner.x, y), inner.w, 12.5, Weight::Regular, TEXT_DIM) + 8.0;
    y += l.ui.paragraph("Start with one flat tile (300 × 300 m), a ground texture and a free camera. Add more tiles in the editor.", Vec2::new(inner.x, y), inner.w, 12.5, Weight::Regular, TEXT_DIM) + 12.0;
    if l.ui.button("editor-create", Rect::new(inner.x, y, inner.w.min(310.0), 42.0), "Create and edit map", Some("add"), ButtonKind::Primary) && !l.state.in_game() {
        let result = content.ok_or_else(|| anyhow::anyhow!("Please configure Setup first."))
            .and_then(|p| core::editor_maps::create(&p, std::path::Path::new(&l.state.config.root), &l.pages.editor_name, &l.pages.editor_author, &l.pages.editor_description));
        match result {
            Ok(map) => {
                if let Ok(maps) = core::list_maps() { l.state.maps = maps; }
                l.pages.editor_map = l.state.maps.iter().position(|m| m.file == map).unwrap_or(0);
                l.state.launch_editor(map);
            }
            Err(e) => l.state.set_status(format!("{e:#}"), true),
        }
    }
    let r = Rect::new(body.x, r.bottom() + 16.0, r.w, 160.0);
    l.ui.panel(r);
    let inner = l.ui.heading(Rect::new(r.x + 20.0, r.y + 16.0, r.w - 40.0, r.h - 32.0), "Edit existing map", Some("folder_open"));
    let labels: Vec<String> = l.state.maps.iter().map(|m| if m.friendly.is_empty() { m.name.clone() } else { m.friendly.clone() }).collect();
    l.pages.editor_map = l.pages.editor_map.min(labels.len().saturating_sub(1));
    if labels.is_empty() {
        l.ui.label(Rect::new(inner.x, inner.y, inner.w, ROW), "No maps found. Please check Setup.");
    } else {
        l.ui.select("editor-existing-map", Rect::new(inner.x, inner.y, inner.w, ROW), &mut l.pages.editor_map, &labels);
        if l.ui.button("editor-open", Rect::new(inner.x, inner.y + ROW + 12.0, inner.w.min(310.0), 42.0), "Open in editor", Some("edit"), ButtonKind::Primary) && !l.state.in_game() {
            let map = l.state.maps[l.pages.editor_map].file.clone();
            l.state.launch_editor(map);
        }
    }
}
