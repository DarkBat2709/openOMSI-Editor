use super::*;
impl Ui {
    fn route_value(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        value: &str,
        at: (f32, f32),
        width: f32,
        size: f32,
    ) {
        let mut value = value.to_owned();
        if self.text.width_raw(&value, size) > width {
            while !value.is_empty() && self.text.width_raw(&(value.clone() + "…"), size) > width {
                value.pop();
            }
            value.push('…');
        }
        let label = self
            .text
            .label_raw(r, scene, &value, (size as u32).max(1), WHITE);
        scene.overlays.push((
            label.tex,
            [at.0, at.1, at.0 + label.w as f32, at.1 + label.h as f32],
        ));
    }

    pub fn draw_route_editor(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        editor: Option<&mut crate::editor::Editor>,
        scale: f32,
        viewport: [f32; 4],
        cursor: (f32, f32),
    ) {
        use crate::route_editor::Command;
        let Some(w) = editor.and_then(|e| e.route_window.as_mut()) else {
            return;
        };
        let s = scale
            .max(0.5)
            .min(viewport[2] / 660.0)
            .min(viewport[3] / 710.0);
        let x = viewport[0] + (viewport[2] - 630.0 * s) / 2.0;
        let y = viewport[1] + (viewport[3] - 680.0 * s) / 2.0;
        let rect = |a: f32, b: f32, c: f32, d: f32| {
            [x + a * s, y + b * s, x + (a + c) * s, y + (b + d) * s]
        };
        w.buttons.clear();
        w.rect = Some(rect(0.0, 0.0, 630.0, 680.0));
        self.editor_surface(r, scene, w.rect.unwrap(), 5.0 * s, PANEL);
        self.catalog_text(
            r,
            scene,
            "Routes and lines",
            (x + 15.0 * s, y + 14.0 * s),
            600.0 * s,
            18.0 * s,
            WHITE,
        );
        let mut buttons = Vec::new();
        for (i, label) in [
            "Line",
            "Route name",
            "Departure (HH:MM)",
            "Duration (minutes)",
            "AI group (optional)",
        ]
        .into_iter()
        .enumerate()
        {
            let at = 48.0 + i as f32 * 36.0;
            self.catalog_text(
                r,
                scene,
                label,
                (x + 15.0 * s, y + (at + 6.0) * s),
                205.0 * s,
                12.0 * s,
                MUTED,
            );
            buttons.push((
                rect(230.0, at, 385.0, 29.0),
                w.fields[i].clone(),
                Command::Field(i),
            ));
        }
        self.catalog_text(
            r,
            scene,
            "Choose stops in travel order",
            (x + 15.0 * s, y + 239.0 * s),
            600.0 * s,
            13.0 * s,
            WHITE,
        );
        for (i, stop) in w.stops.iter().enumerate().skip(w.page * 6).take(6) {
            buttons.push((
                rect(15.0, 265.0 + (i % 6) as f32 * 31.0, 600.0, 27.0),
                format!("{} · #{}", stop.name, stop.object_id),
                Command::Stop(i),
            ));
        }
        if w.stops.is_empty() {
            self.catalog_text(
                r,
                scene,
                "No stops found. Place, name and save stops first.",
                (x + 15.0 * s, y + 280.0 * s),
                600.0 * s,
                12.0 * s,
                MUTED,
            );
        }
        buttons.extend([
            (
                rect(15.0, 457.0, 90.0, 28.0),
                "Previous".into(),
                Command::Page(-1),
            ),
            (
                rect(115.0, 457.0, 90.0, 28.0),
                "Next".into(),
                Command::Page(1),
            ),
            (
                rect(360.0, 457.0, 255.0, 28.0),
                "Remove last stop".into(),
                Command::Remove,
            ),
        ]);
        let order = w
            .chosen
            .iter()
            .map(|i| w.stops[*i].name.as_str())
            .collect::<Vec<_>>()
            .join(" → ");
        for (i, line) in wrap(&self.text, &order, 12.0 * s, 600.0 * s)
            .iter()
            .take(3)
            .enumerate()
        {
            self.catalog_text(
                r,
                scene,
                line,
                (x + 15.0 * s, y + (497.0 + i as f32 * 17.0) * s),
                600.0 * s,
                12.0 * s,
                WHITE,
            );
        }
        for (i, line) in wrap(&self.text, &omsi_ui::tr(&w.message), 11.0 * s, 600.0 * s)
            .iter()
            .take(3)
            .enumerate()
        {
            self.catalog_text(
                r,
                scene,
                line,
                (x + 15.0 * s, y + (557.0 + i as f32 * 16.0) * s),
                600.0 * s,
                11.0 * s,
                MUTED,
            );
        }
        buttons.extend([
            (
                rect(15.0, 628.0, 180.0, 34.0),
                "Close".into(),
                Command::Close,
            ),
            (
                rect(265.0, 628.0, 350.0, 34.0),
                "Save route and line".into(),
                Command::Save,
            ),
        ]);
        for (rect, label, c) in buttons {
            self.editor_surface(
                r,
                scene,
                rect,
                3.0 * s,
                if editor_contains(rect, cursor) {
                    LIT
                } else {
                    CHIP
                },
            );
            if matches!(c, Command::Field(_) | Command::Stop(_)) {
                self.route_value(
                    r,
                    scene,
                    &label,
                    (rect[0] + 7.0 * s, rect[1] + 6.0 * s),
                    rect[2] - rect[0] - 14.0 * s,
                    12.0 * s,
                );
            } else {
                self.catalog_text(
                    r,
                    scene,
                    &label,
                    (rect[0] + 7.0 * s, rect[1] + 6.0 * s),
                    rect[2] - rect[0] - 14.0 * s,
                    12.0 * s,
                    WHITE,
                );
            }
            w.buttons.push((rect, c));
        }
    }
}
