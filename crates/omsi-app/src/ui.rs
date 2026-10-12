//! The game's own interface, drawn over the picture in Inter with a dark outline (the
//! HUD's `.oft` fonts are OMSI's and stay for the time and speed): the chat of a LAN
//! session, the name of what the cursor points at next to the cursor, and the other
//! players' name tags above their buses.
//!
//! Every text is rendered once into a small texture and kept while it is shown; the
//! overlays are rectangles in physical pixels (`Scene::overlays`).

use ab_glyph::{Font, FontVec, PxScale, ScaleFont, VariableFont};
use omsi_render::{Renderer, Scene, TextureId};

mod menu;
mod perf_panel;
mod text;

pub use text::TextCache;
use menu::*;
use text::*;

// ---------------------------------------------------------------------------------------
// the chat

/// How many rows the chat shows (a long line takes several), and how many lines it keeps to
/// scroll back.
const CHAT_SHOWN: usize = 8;
pub const CHAT_KEEP: usize = 200;

/// A notification: a card over the navigator that goes by itself - from the server
/// (`notify`), the plugins, the update check, and what the game tells the driver to do
/// (`NoteWatch`).
#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
    /// Seconds it still shows, of `total`.
    pub left: f32,
    pub total: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoticeKind {
    Info,
    Warn,
    Alert,
    /// A new openOMSI version (`update_watch`).
    Update,
    /// Something the driver has to do (release the parking brake, start the engine, give
    /// the change): in the accent.
    Action,
    /// Something done (the trip's end, a setting saved).
    Success,
}

impl NoticeKind {
    /// The kind's colour (the launcher's: neutral for information, the accent for what to
    /// do, red for an alert, green for done, blue for an update) and its Material icon.
    pub fn look(self) -> ([u8; 3], &'static str) {
        match self {
            NoticeKind::Info => ([184, 184, 184], "info"),
            NoticeKind::Warn => ([224, 164, 58], "warning"),
            NoticeKind::Alert => ([229, 83, 75], "error"),
            NoticeKind::Update => ([96, 160, 232], "system_update"),
            NoticeKind::Action => ([232, 160, 48], "touch_app"),
            NoticeKind::Success => ([76, 183, 130], "check_circle"),
        }
    }

    /// How long a card of this kind stays when the game shows it on its own (seconds): what
    /// to do and an alert a little longer, to be read while driving.
    pub fn secs(self) -> f32 {
        match self {
            NoticeKind::Action => 7.0,
            NoticeKind::Alert => 8.0,
            NoticeKind::Warn => 6.0,
            _ => 5.0,
        }
    }
}

/// How many notifications show at once (the newest; an older one fades to make room).
pub const NOTICES_SHOWN: usize = 4;

/// How long a card takes to come in and to go (seconds).
const NOTICE_IN: f32 = 0.3;
const NOTICE_OUT: f32 = 0.4;

impl Notice {
    /// `notify`'s argument: `<id> <seconds> <info|warn|alert> <text>` → the id and the notice
    /// (seconds held to 2 .. 30; an unknown kind is information).
    pub fn parse(arg: &str) -> Option<(String, Notice)> {
        let mut it = arg.trim().splitn(4, ' ');
        let id = it.next().filter(|s| !s.is_empty())?.to_string();
        let secs: f32 = it.next()?.parse().ok().filter(|s: &f32| s.is_finite())?;
        let kind = match it.next()? {
            "warn" => NoticeKind::Warn,
            "alert" => NoticeKind::Alert,
            "action" => NoticeKind::Action,
            "success" | "ok" => NoticeKind::Success,
            _ => NoticeKind::Info,
        };
        let text = it.next().map(str::trim).filter(|t| !t.is_empty())?.to_string();
        let total = secs.clamp(2.0, 30.0);
        Some((id, Notice { kind, text, left: total, total }))
    }

    /// A notice of the game's own: `kind`'s time.
    pub fn new(kind: NoticeKind, text: impl Into<String>) -> Notice {
        let total = kind.secs();
        Notice { kind, text: text.into(), left: total, total }
    }

    /// How far it is there (0 to 1, eased): coming in over `NOTICE_IN`, going over its last
    /// `NOTICE_OUT` - its opacity, and how much room it takes in the stack, so that the
    /// cards round it slide instead of jumping.
    pub fn alpha(&self) -> f32 {
        let k = (self.left / NOTICE_OUT).min((self.total - self.left) / NOTICE_IN).clamp(0.0, 1.0);
        k * k * (3.0 - 2.0 * k)
    }
}

/// A new notice in the list. The same text still on the screen stays its time again instead
/// of standing twice; with more than `NOTICES_SHOWN` the oldest fades out to make room.
pub fn push_notice(list: &mut Vec<Notice>, n: Notice) {
    if let Some(o) = list.iter_mut().find(|o| o.kind == n.kind && o.text == n.text && o.left > NOTICE_OUT) {
        // (its age kept: it does not fade in once more)
        o.total = o.total - o.left + n.total;
        o.left = n.total;
        return;
    }
    list.push(n);
    let mut live = list.iter().filter(|o| o.left > NOTICE_OUT).count();
    for o in list.iter_mut() {
        if live <= NOTICES_SHOWN {
            break;
        }
        if o.left > NOTICE_OUT {
            o.left = NOTICE_OUT;
            live -= 1;
        }
    }
    while list.len() > NOTICES_SHOWN + 3 {
        list.remove(0);
    }
}

/// Turns what the game tells the driver every frame (why the bus does not move, the change
/// due, a message) into notifications: a line becomes a card when it starts to be true, once,
/// and again only after it was gone for `NoteWatch::REARM` seconds - the throttle let go and
/// pressed again a moment later is the same reason, not a new one.
#[derive(Default)]
pub struct NoteWatch {
    /// When each line (by its key) was last there.
    seen: hashbrown::HashMap<String, f32>,
    now: f32,
}

impl NoteWatch {
    pub const REARM: f32 = 3.0;

    /// This frame's lines: (key, text, kind). The key names the condition (its text may
    /// change with a value in it, a pressure or a sum, without being a new one).
    pub fn frame(&mut self, dt: f32, lines: &[(String, String, NoticeKind)], list: &mut Vec<Notice>) {
        self.now += dt.max(0.0);
        for (key, text, kind) in lines {
            let fresh = self.seen.get(key).is_none_or(|&t| self.now - t > Self::REARM);
            self.seen.insert(key.clone(), self.now);
            if fresh && !text.trim().is_empty() {
                push_notice(list, Notice::new(*kind, text.clone()));
            }
        }
        let now = self.now;
        self.seen.retain(|_, t| now - *t <= Self::REARM);
    }
}

/// The part of a line of the game that names its condition: up to its first bracket or
/// colon ("Parking brake is on  (. releases it)", "Air pressure is low (4.1 bar ...)").
pub fn note_key(text: &str) -> String {
    text.split(['(', ':']).next().unwrap_or(text).trim().to_string()
}

/// What the chat widget needs from the session each frame.
pub struct ChatView<'a> {
    /// Every line, oldest first ("Name: text" or "* notice").
    pub lines: &'a [String],
    /// The line being typed (the input box is open).
    pub typing: Option<&'a str>,
    /// Why the last line was not sent.
    pub error: Option<&'a str>,
}

/// The chat's own state: shown or hidden (V), the scroll position and whether the cursor is
/// over it.
#[derive(Default)]
pub struct ChatWidget {
    pub hidden: bool,
    /// Lines scrolled back from the newest.
    pub scroll: usize,
    /// The chat's box on the screen (physical pixels) as drawn last: hovering over it shows
    /// the input box, a click there opens it.
    pub rect: [f32; 4],
    pub hovered: bool,
    pub caret_t: f32,
}

impl ChatWidget {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.rect[0] && x <= self.rect[2] && y >= self.rect[1] && y <= self.rect[3]
    }

    /// The mouse wheel over the chat (or while typing) scrolls the history.
    pub fn wheel(&mut self, lines: usize, amount: f32) {
        let max = lines.saturating_sub(CHAT_SHOWN);
        let s = self.scroll as i32 + amount.round() as i32;
        self.scroll = s.clamp(0, max as i32) as usize;
    }
}

/// How much larger than designed the interface is drawn, on top of the screen's scale
/// `dpi`: on a window taller than 1080 logical pixels as much as it is taller (up to twice),
/// times the player's `size` (`Settings::ui_scale`). Laid out for 1080p, the texts were half
/// their size on a 4K screen at 100 % display scaling; a window of 1080 lines or fewer is
/// drawn as it always was. With `window` off (`Settings::ui_scale_window`) it does not grow
/// with the window at all.
pub fn size_factor(height_px: f32, dpi: f32, size: f32, window: bool) -> f32 {
    let grown = if window { (height_px / dpi.max(0.5) / 1080.0).clamp(1.0, 2.0) } else { 1.0 };
    grown * size
}

/// How strongly the interface's backgrounds are drawn for the opacity setting
/// (`Settings::ui_opacity`): 1 at its default of 85 %, as designed; lower, the picture shows
/// through them (never less than 0.3), higher a little darker. The texts stay solid.
pub fn backdrop(opacity: f32) -> f32 {
    (opacity / 0.85).clamp(0.3, 1.3)
}

/// Which menu is open: its layout follows from it.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum MenuKind {
    /// The game menu itself.
    #[default]
    Game,
    /// A settings window (options, vehicle, world): a sidebar of pages and rows with switches,
    /// sliders and buttons. A row's label is `name\u{1f}kind\u{1f}value\u{1f}description\u{1f}fraction`
    /// (kind: `s` switch, `v` slider, `c` stepper, `o` opens a list, `a` button, `i` information).
    Options,
    /// The lines of the map's timetable.
    Lines,
    /// One line's tours.
    Tours,
    /// Any other list (drivers, fleet numbers, destinations, liveries ...).
    List,
}

/// The timetable beside a list of lines or tours: a title, a line of facts and rows of
/// (what, time).
pub struct Preview {
    pub title: String,
    pub meta: String,
    pub rows: Vec<(String, String)>,
    /// The row chosen, when the rows can be chosen (the stops to start a tour from).
    pub chosen: Option<usize>,
    /// The button under the rows, when there is one.
    pub button: Option<String>,
    /// The time of the timetable the rows are of, with buttons to put it ahead (the trip of
    /// a tour to take on later), when it can be set.
    pub time: Option<String>,
}


/// The timetable window of the trip driven: its line and terminus, the tour and trip, how
/// late, the stops, and the trip after it.
pub struct TimetableView {
    pub line: String,
    pub terminus: String,
    /// "Tour 1 · Trip 17/72".
    pub meta: String,
    /// Seconds late (early: negative).
    pub delay: f64,
    pub rows: Vec<TimetableRow>,
    /// The next trip: its line and terminus, and when it leaves.
    pub next: Option<(String, String)>,
}

/// A stop of the timetable window: its name, arrival and departure ("" where there is none),
/// and 0 served, 1 next, 2 ahead.
pub struct TimetableRow {
    pub name: String,
    pub arr: String,
    pub dep: String,
    pub state: u8,
}

/// A drop-down open over a row of a settings window: the entries, the one the keyboard is
/// on, the first one shown and the one in force now.
pub struct DropdownView<'a> {
    pub row: usize,
    pub items: Vec<&'a str>,
    pub sel: usize,
    pub top: usize,
    pub current: Option<usize>,
}

/// Everything the interface draws in a frame.
pub struct Frame<'a> {
    /// Physical pixels per logical one.
    pub scale: f32,
    /// How much larger than designed (`size_factor`): everything below is drawn this much
    /// larger on top of `scale`.
    pub ui_scale: f32,
    /// How strongly the backgrounds are drawn (`backdrop`): the menu's and the timetable's
    /// panels, the chat's box, the plates under the notes, the tooltip and the frame rate.
    pub opacity: f32,
    pub width: f32,
    pub height: f32,
    pub cursor: (f32, f32),
    /// The mouse steering's cross, while the system cursor is hidden and held (where the
    /// crosshair cursor stood before): drawn here instead.
    pub steer_cross: Option<(f32, f32)>,
    /// The tooltip goes above the cursor rather than right of it: the cursor is a finger,
    /// whose hand covers what is below and beside it.
    pub tooltip_above: bool,
    /// An OpenXR headset is drawing this frame.
    pub vr: bool,
    /// The name of what the cursor points at (a switch, a part), shown next to it.
    pub tooltip: Option<String>,
    /// The chat, when a LAN session runs and the chat is not switched off.
    pub chat: Option<ChatView<'a>>,
    /// The chat's own size on top of `ui_scale` (`Settings::chat_size`).
    pub chat_size: f32,
    /// What the driver has to act on (why the bus does not move, a passenger's wish, the
    /// change due, a service done), top left.
    pub notes: &'a [String],
    /// The frame rate, top right (the `show_fps` setting).
    pub fps: Option<f32>,
    pub perf: Option<&'a crate::perf_overlay::PerfOverlay>,
    pub perf_detailed: bool,
    pub perf_touch: Option<Option<f32>>,
    /// The game stands paused.
    pub paused: bool,
    /// The game menu is open, with this line chosen (labels from `GAME_MENU`).
    pub menu: Option<(usize, &'a [(&'a str, &'a str)])>,
    /// The first line shown when a finger scrolled the menu (`App::menu_top`).
    pub menu_top: Option<f32>,
    /// Ids of the game menu's lines that are greyed out and cannot be chosen (the timetable
    /// without an active route).
    pub menu_disabled: &'a [&'a str],
    /// The timetable window (Insert).
    pub timetable: Option<TimetableView>,
    /// The information bar along the top: its parts between [`INFO_SEP`]s.
    pub info: Option<String>,
    /// The room the information bar has along the top: from x to x, from y down (on a
    /// touch screen the gap between its buttons); none: the window's width.
    pub info_room: Option<[f32; 3]>,
    /// A tutorial page: title, text, picture, page number and count.
    pub tutorial: Option<(&'a str, &'a str, Option<&'a std::path::Path>, usize, usize)>,
    /// Name tags: a screen position (the point above a bus), the name and a second line.
    pub tags: Vec<((f32, f32), String, String, f32)>,
    /// The server's notifications, oldest first.
    pub notices: &'a [Notice],
    /// Where the navigator is on the screen (the notifications stand over it, or under it
    /// in a top corner); none: they go to the top middle.
    pub notice_anchor: Option<[f32; 4]>,
    /// What kind of menu the lines belong to.
    pub menu_kind: MenuKind,
    /// The open list's title and the small line above it (the line a tour list is of).
    pub menu_head: Option<(String, String)>,
    /// The timetable of the chosen line or tour, beside the list.
    pub menu_preview: Option<Preview>,
    /// The first stop shown of the timetable beside the tours, when the wheel has scrolled it
    /// (`None`: the stop chosen is kept in view).
    pub pane_first: Option<usize>,
    /// The pages of an open settings window (their titles) and the one shown.
    pub menu_tabs: Option<(Vec<String>, usize)>,
    /// The keyboard chose the menu's line last: that line is shown lit (else only the one
    /// under the mouse is).
    pub menu_kbd: bool,
    /// The drop-down open over a row of the settings window.
    pub dropdown: Option<DropdownView<'a>>,
}

pub struct Ui {
    origin_x: f32,
    editor_hud_rect: Option<[f32; 4]>,
    editor_help_rect: Option<[f32; 4]>,
    editor_help_expanded: bool,
    editor_help_scroll: usize,
    editor_help_scroll_max: usize,
    editor_hint_mode: Option<(bool, bool)>,
    editor_dock_rect: Option<[f32; 4]>,
    editor_dock_header: Option<[f32; 4]>,
    editor_window_kind: Option<&'static str>,
    editor_window_positions: hashbrown::HashMap<&'static str, [f32; 2]>,
    editor_dock_reset: Option<[f32; 4]>,
    pub editor_reload_rect: Option<[f32; 4]>,
    pub editor_audit_rect: Option<[f32;4]>,
    pub tile_open_rect: Option<[f32; 4]>,
    pub terrain_open_rect: Option<[f32; 4]>,
    pub roundabout_open_rect: Option<[f32;4]>,
    pub junction_open_rect: Option<[f32;4]>,
    junction_image:Option<(u64,TextureId)>,
    pub terrain_tool_rects: Vec<([f32; 4], crate::terrain_editor::Command)>,
    editor_hint_rect: Option<[f32; 4]>,
    editor_dock_position: Option<[f32; 2]>,
    editor_dock_drag: Option<(f32, f32)>,
    editor_dock_bounds: [f32; 4],
    editor_notes_top: f32,
    notes_bottom: f32,
    editor_tools_rect: Option<[f32; 4]>,
    editor_connection_rect: Option<[f32; 4]>,
    pub spline_connect_rect: Option<[f32; 4]>,
    pub spline_transition_rect: Option<[f32; 4]>,
    pub spline_cancel_rect: Option<[f32; 4]>,
    pub text: TextCache,
    pub chat: ChatWidget,
    /// Where the game menu's lines were drawn this frame (physical pixels), for the mouse.
    pub menu_rects: Vec<[f32; 4]>,
    /// Per line of `menu_rects`, where the arrows round its value are (a list's setting,
    /// `game_lists::ADJUST`): `[from, to, plus]` - a click from `from` to `to` steps it
    /// down, one right of `plus` up.
    pub menu_arrows: Vec<Option<[f32; 3]>>,
    pub menu_scroll_thumb: Option<[f32; 4]>,
    pub menu_scroll_track: Option<[f32; 4]>,
    /// Where the controls of the settings rows were drawn (a slider's track, a stepper), one
    /// entry per line in `menu_rects`: a click there sets the value.
    pub menu_ctl: Vec<Option<[f32; 4]>>,
    /// The sidebar of a settings window: one box per page, the way back last.
    pub menu_side: Vec<[f32; 4]>,
    /// The rows of the timetable beside a line's tours (the stops to start from), the stop the
    /// first of them is, and the button that starts the trip.
    pub menu_pane: Vec<[f32; 4]>,
    pub menu_pane_start: usize,
    pub menu_pane_go: Option<[f32; 4]>,
    /// The whole timetable pane beside the tours: the wheel over it scrolls its stops.
    pub menu_pane_box: Option<[f32; 4]>,
    /// Its scroll bar when it has more stops than it shows: the track, the thumb (widened
    /// to be hit), how many stops there are and how many it shows - for the mouse to drag.
    pub menu_pane_scroll: Option<([f32; 4], [f32; 4], usize, usize)>,
    /// The two arrows beside the time of a tour: the trip before, the next one.
    pub menu_time: Vec<[f32; 4]>,
    /// The colours and positions of the menu's parts that ease to their new state (a line's
    /// light, a switch's knob ...), by what they belong to.
    anim: std::collections::HashMap<u64, f32>,
    /// Seconds since the last frame, for `ease`.
    anim_dt: f32,
    /// Overlay entries belonging to the game menu.
    pub menu_overlay_range: std::ops::Range<usize>,
    /// The pointer texture, positioned separately for each headset eye.
    pub vr_cursor_overlay: Option<usize>,
    pub vr_tooltip_overlay: Option<usize>,
    /// The first line of the menu shown (a long menu scrolls: `menu_rects[k]` is line
    /// `menu_start + k`).
    pub menu_start: usize,
    /// How many lines the menu shows at once, and how high one is (physical pixels): a
    /// finger's drag is turned into lines with it.
    pub menu_rows: usize,
    pub menu_row_h: f32,
    /// The entries of the drop-down shown (their rects), the first of them, and how many fit.
    pub dd_rects: Vec<[f32; 4]>,
    pub dd_top: usize,
    pub dd_rows: usize,
    /// The drop-down's scroll bar when it has more entries than it shows: its track and
    /// its thumb (widened to be hit), for the mouse to drag.
    pub dd_scroll: Option<([f32; 4], [f32; 4])>,
    /// Pictures shown in the interface (a tutorial page's), by file.
    images: hashbrown::HashMap<std::path::PathBuf, Option<(TextureId, u32, u32)>>,
    catalog_images: hashbrown::HashMap<String, TextureId>,
    pub catalog_open_rect: Option<[f32; 4]>,
    pub spline_tool_rects: Vec<([f32; 4], crate::editor::Action)>,
    /// Where the information bar was drawn (within the panel, before `origin_x`), for the
    /// navigator to keep out of its way.
    pub info_rect: Option<[f32; 4]>,
    perf: perf_panel::PerfPanel,
}

/// Between the information bar's parts.
pub(crate) const INFO_SEP: &str = "   |   ";

fn editor_scale(scale: f32, viewport: [f32; 4]) -> f32 {
    // HUD + dock buttons + one hint row + all nine spline tools need
    // about 675 logical pixels including margins. Leave room for rounding.
    scale.max(0.5).min((viewport[2] / 544.0).max(0.1))
        .min((viewport[3] / 700.0).max(0.1))
}

fn editor_contains(rect: [f32; 4], cursor: (f32, f32)) -> bool {
    cursor.0 >= rect[0] && cursor.0 <= rect[2]
        && cursor.1 >= rect[1] && cursor.1 <= rect[3]
}

fn editor_shortcuts(spline: bool, expanded: bool) -> Vec<(&'static str, &'static str)> {
    let mut keys = vec![
        ("WASD / Q E", "Camera"), ("Click / Enter", "Selection"),
        ("I J K L", "Move"), ("U / O", "Height"), ("N / M", "Rotate"),
        ("Ctrl+S", "Save"), ("T", if spline { "Objects" } else { "Splines" }),
    ];
    if spline {
        if expanded {
            keys.extend([
                ("F6", "Fit to ground"), ("F8", "Smooth shoulder"),
                ("Tab", "Next hit"), ("Shift", "Fine movement"),
                ("+ / -", "Length"), (", / .", "Curve"), ("Home / End", "Gradient"),
                ("B", "Straight / Cancel"), ("Shift+F8", "Smooth wider"),
                ("C", "Extend"), ("G", "Connect"), ("Ctrl+G", "Replace connection"),
                ("Shift+G", "New road"), ("R / Shift+R", "Branch right / left"),
                ("H", "Snap end"), ("F7", "Split"), ("Del", "Delete"),
                ("Ctrl+Z", "Undo"), ("P / X", "Catalogue / Objects"), ("Esc", "Close / Cancel"),
            ]);
        }
    } else {
        keys.extend([("Del", "Delete / Restore"),("C", "Copy and place"),("Ctrl+C / Ctrl+V", "Copy / Paste")]);
        if expanded {
            keys.extend([
                ("Tab", "Next hit"), ("Shift", "Fine movement"),
                ("Drag", "Move"), ("Mouse wheel", "Rotate"), ("Shift+Wheel", "Height"),
                ("Ctrl+R", "Place last object"), ("V", "Variant"), ("Backspace", "Reset"),
                ("P / X", "Catalogue / Objects"),
                ("Home / End", "Builder pitch"), ("Page Up / Down", "Builder bank"),
                ("F", "Level terrain"), ("[ / ]", "Brush size"), ("Esc", "Close / Cancel"),
            ]);
        }
    }
    keys
}

struct EditorHudLayout {
    panel: [f32; 4],
    selection: [f32; 4],
}

fn editor_hud_layout(viewport: [f32; 4], s: f32) -> EditorHudLayout {
    let x = viewport[0] + 12.0 * s;
    let y = viewport[1] + 8.0 * s;
    let right = viewport[0] + viewport[2] - 12.0 * s;
    EditorHudLayout {
        panel: [x, y, right, y + 66.0 * s],
        selection: [x + 10.0 * s, y + 24.0 * s, right - 10.0 * s, y + 60.0 * s],
    }
}

struct EditorDockLayout {
    panel: [f32; 4],
    bounds: [f32; 4],
    header: [f32; 4],
    catalog: [f32; 4],
    builders: [f32; 4],
    tiles: [f32; 4],
    terrain: [f32; 4],
    toggle: [f32; 4],
    reload: [f32; 4],
    audit: [f32;4],
    reset: [f32; 4],
    hints: [f32; 4],
    rows: usize,
    max_scroll: usize,
    footer_top: f32,
}

/// The footer and catalogue stay visible; only the key list scrolls. A normalized
/// position keeps a moved panel reachable when the window or UI scale changes.
fn editor_dock_layout(viewport: [f32; 4], s: f32, hud_bottom: f32,
    count: usize, footer: f32, position: Option<[f32; 2]>) -> EditorDockLayout {
    let min_x = viewport[0] + 12.0 * s;
    let min_y = hud_bottom + 10.0 * s;
    let available = viewport[1] + viewport[3] - min_y - 12.0 * s;
    let fixed = (232.0 + if footer > 0.0 { 8.0 + footer } else { 0.0 }) * s;
    let rows = (((available - fixed) / (24.0 * s)).floor().max(1.0) as usize).min(count);
    let height = fixed + rows as f32 * 24.0 * s;
    let width = 304.0 * s;
    let max_x = (viewport[0] + viewport[2] - width - 12.0 * s).max(min_x);
    let max_y = (viewport[1] + viewport[3] - height - 12.0 * s).max(min_y);
    let position = position.unwrap_or([0.0, 0.0]);
    let x = min_x + position[0].clamp(0.0, 1.0) * (max_x - min_x);
    let y = min_y + position[1].clamp(0.0, 1.0) * (max_y - min_y);
    let right = x + width;
    let hint_top = y + 222.0 * s;
    EditorDockLayout {
        panel: [x, y, right, y + height],
        bounds: [min_x, min_y, max_x, max_y],
        header: [x, y, right, y + 28.0 * s],
        catalog: [x + 10.0 * s, y + 32.0 * s, right - 10.0 * s, y + 58.0 * s],
        builders: [x + 10.0 * s, y + 64.0 * s, right - 10.0 * s, y + 90.0 * s],
        tiles: [x + 10.0 * s, y + 96.0 * s, x + 145.0 * s, y + 122.0 * s],
        terrain: [x + 151.0 * s, y + 96.0 * s, right - 10.0 * s, y + 122.0 * s],
        reload: [x + 10.0 * s, y + 128.0 * s, right - 10.0 * s, y + 154.0 * s],
        audit: [x + 10.0 * s, y + 160.0 * s, right - 10.0 * s, y + 186.0 * s],
        toggle: [x + 10.0 * s, y + 192.0 * s, right - 62.0 * s, y + 214.0 * s],
        reset: [right - 56.0 * s, y + 192.0 * s, right - 10.0 * s, y + 214.0 * s],
        hints: [x + 10.0 * s, hint_top, right - 10.0 * s, hint_top + rows as f32 * 24.0 * s],
        rows,
        max_scroll: count.saturating_sub(rows),
        footer_top: hint_top + rows as f32 * 24.0 * s + 8.0 * s,
    }
}

struct TerrainDockLayout { scale:f32, panel:[f32;4], header:[f32;4], bounds:[f32;4] }

fn terrain_dock_layout(viewport:[f32;4],scale:f32,hud_bottom:f32,position:Option<[f32;2]>) -> TerrainDockLayout {
    let top=hud_bottom+10.0;
    let s=scale.max(0.5).min((viewport[2]/544.0).max(0.1))
        .min(((viewport[1]+viewport[3]-top-12.0)/668.0).max(0.1));
    let min_x=viewport[0]+12.0*s;let min_y=top;
    let max_x=(viewport[0]+viewport[2]-316.0*s).max(min_x);
    let max_y=(viewport[1]+viewport[3]-680.0*s).max(min_y);
    let position=position.unwrap_or([0.0,0.0]);
    let x=min_x+position[0].clamp(0.0,1.0)*(max_x-min_x);
    let y=min_y+position[1].clamp(0.0,1.0)*(max_y-min_y);
    TerrainDockLayout {scale:s,panel:[x,y,x+304.0*s,y+668.0*s],header:[x,y,x+304.0*s,y+28.0*s],
        bounds:[min_x,min_y,max_x,max_y]}
}

/// The information bar's parts in rows no wider than `room` (as `width` measures a text),
/// each as many parts as fit: in one line it ran off both sides of a narrow window, and on a
/// phone it lay under the buttons along the top (#1164).
pub(crate) fn info_rows(line: &str, room: f32, width: impl Fn(&str) -> f32) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for part in line.split(INFO_SEP) {
        match rows.last_mut() {
            Some(row) if width(&format!("{row}{INFO_SEP}{part}")) <= room => {
                row.push_str(INFO_SEP);
                row.push_str(part);
            }
            _ => rows.push(part.to_string()),
        }
    }
    rows
}

pub(crate) fn shift_overlays(scene: &mut Scene, start: usize, x: f32) {
    for (_, rect) in &mut scene.overlays[start..] {
        rect[0] += x;
        rect[2] += x;
    }
}

impl Ui {
    /// Lay out at panel resolution, then move both pixels and mouse targets to
    /// that panel's position in the spanning window.
    pub fn draw_at(&mut self, r: &Renderer, scene: &mut Scene, f: &Frame, dt: f32, origin_x: f32) {
        self.shift_hitboxes(-self.origin_x);
        let start = scene.overlays.len();
        self.draw(r, scene, f, dt);
        shift_overlays(scene, start, origin_x);
        self.shift_hitboxes(origin_x);
        self.origin_x = origin_x;
    }

    fn shift_hitboxes(&mut self, x: f32) {
        for rect in self
            .menu_rects
            .iter_mut()
            .chain(self.menu_side.iter_mut())
            .chain(self.menu_pane.iter_mut())
            .chain(self.menu_time.iter_mut())
            .chain(self.dd_rects.iter_mut())
            .chain(self.menu_ctl.iter_mut().flatten())
            .chain(self.menu_scroll_thumb.iter_mut())
            .chain(self.menu_scroll_track.iter_mut())
            .chain(self.menu_pane_go.iter_mut())
            .chain(self.menu_pane_box.iter_mut())
        {
            rect[0] += x;
            rect[2] += x;
        }
        for arrows in self.menu_arrows.iter_mut().flatten() {
            for at in arrows {
                *at += x;
            }
        }
        if let Some((track, thumb)) = &mut self.dd_scroll {
            for rect in [track, thumb] {
                rect[0] += x;
                rect[2] += x;
            }
        }
        if let Some((track, thumb, _, _)) = &mut self.menu_pane_scroll {
            for rect in [track, thumb] {
                rect[0] += x;
                rect[2] += x;
            }
        }
        self.chat.rect[0] += x;
        self.chat.rect[2] += x;
    }
    pub fn new() -> Option<Ui> {
        Some(Ui { catalog_images:Default::default(), catalog_open_rect:None, spline_tool_rects:Vec::new(), origin_x: 0.0, editor_hud_rect: None, editor_help_rect: None, editor_help_expanded: false, editor_help_scroll: 0, editor_help_scroll_max: 0, editor_hint_mode: None, editor_dock_rect: None, editor_dock_header: None, editor_window_kind: None, editor_window_positions: Default::default(), editor_dock_reset: None, editor_reload_rect: None, editor_audit_rect: None, tile_open_rect: None, terrain_open_rect: None, junction_open_rect:None, roundabout_open_rect:None, junction_image:None, terrain_tool_rects: Vec::new(), editor_hint_rect: None, editor_dock_position: None, editor_dock_drag: None, editor_dock_bounds: [0.0; 4], editor_notes_top: 0.0, notes_bottom: 0.0, editor_tools_rect: None, editor_connection_rect: None, spline_connect_rect: None, spline_transition_rect: None, spline_cancel_rect: None, text: TextCache::new()?, chat: ChatWidget::default(), menu_rects: Vec::new(), menu_arrows: Vec::new(), menu_scroll_thumb: None, menu_scroll_track: None, menu_ctl: Vec::new(), dd_rects: Vec::new(), dd_top: 0, dd_rows: 8, dd_scroll: None, menu_side: Vec::new(), menu_pane: Vec::new(), menu_pane_start: 0, menu_pane_go: None, menu_pane_box: None, menu_pane_scroll: None, menu_time: Vec::new(), anim: Default::default(), anim_dt: 0.0, menu_overlay_range: 0..0, vr_cursor_overlay: None, vr_tooltip_overlay: None, menu_start: 0, menu_rows: 0, menu_row_h: 1.0, images: Default::default(), info_rect: None, perf: Default::default() })
    }

    fn catalog_text(&mut self, r: &Renderer, scene: &mut Scene, text: &str, at: (f32, f32), width: f32, size: f32, color: [u8; 4]) {
        let clipped = clip_to(&self.text, text, size, width);
        let label = self.text.label(r, scene, &clipped, (size as u32).max(1), color);
        scene.overlays.push((label.tex, [at.0, at.1, at.0 + label.w as f32, at.1 + label.h as f32]));
    }

    fn catalog_plate(&mut self, r: &Renderer, scene: &mut Scene, color: [u8; 4], rect: [f32; 4]) {
        if color[3] == 255 && rect[3] - rect[1] > 8.0 {
            self.editor_surface(r, scene, rect, 5.0, color);
        } else {
            let tex = self.text.solid(r, scene, color);
            scene.overlays.push((tex, rect));
        }
    }

    /// Editor surfaces share the in-game menu palette (also matched to launcher/theme.rs).
    fn editor_surface(&mut self, r: &Renderer, scene: &mut Scene,
        rect: [f32; 4], radius: f32, color: [u8; 4]) {
        let edge = ((rect[3] - rect[1]) / 36.0).clamp(0.5, 1.25);
        let radius = radius.max(4.0 * edge);
        self.text.rounded(r, scene, rect, radius, BORDER);
        self.text.rounded(r, scene, [rect[0]+edge, rect[1]+edge, rect[2]-edge, rect[3]-edge],
            (radius-edge).max(0.0), color);
        if color == SELECTED {
            self.text.rounded(r, scene, [rect[0]+edge, rect[1]+4.0*edge,
                rect[0]+3.0*edge, rect[3]-4.0*edge], edge, ACCENT);
        }
    }

    /// Only selection data stays at the top; keyboard hints live in the movable dock.
    pub fn draw_editor_hud(&mut self, r: &Renderer, scene: &mut Scene,
        editor: Option<(bool, bool, &str, bool)>, scale: f32, viewport: [f32; 4], _cursor: (f32, f32)) {
        self.editor_window_kind = None;
        self.editor_dock_header = None;
        self.editor_hud_rect = None;
        self.editor_notes_top = 0.0;
        let Some((spline, terrain, description, _)) = editor else {
            self.editor_dock_drag = None;
            self.editor_dock_rect = None;
            return;
        };
        let s = editor_scale(scale, viewport);
        let layout = editor_hud_layout(viewport, s);
        let flat = self.text.flat;
        self.text.flat = true;
        self.editor_surface(r, scene, layout.panel, 5.0 * s, PANEL);
        let title = if terrain { "Terrain editor 0.7.23-pre" } else if spline { "Spline editor 0.7.23-pre" } else { "Object editor 0.7.23-pre" };
        self.put(r, scene, title, ((13.0 * s) as u32).max(1) | BOLD,
            MUTED, layout.panel[0] + 10.0 * s, layout.panel[1] + 12.0 * s);
        self.editor_surface(r, scene, layout.selection, 3.0 * s, PANEL_ALT);
        let description = omsi_ui::tr(description);
        let mut parts = description.split(" · ");
        let mut selected = parts.next().unwrap_or_default().to_string();
        if let Some(name) = parts.next() { selected.push_str(" · "); selected.push_str(name); }
        let details = parts.collect::<Vec<_>>().join(" · ");
        let x = layout.selection[0] + 8.0 * s;
        let room = layout.selection[2] - x - 8.0 * s;
        self.catalog_text(r, scene, &selected, (x, layout.selection[1] + 2.0 * s), room, 13.0 * s, WHITE);
        self.catalog_text(r, scene, &details, (x, layout.selection[1] + 19.0 * s), room, 11.0 * s, MUTED);
        self.text.flat = flat;
        self.editor_hud_rect = Some(layout.panel);
        self.editor_notes_top = layout.panel[3] - viewport[1] + 8.0 * s;
    }

    pub fn draw_spline_audit(&mut self,r:&Renderer,scene:&mut Scene,audit:Option<&mut crate::audit_events::Audit>,scale:f32,viewport:[f32;4],cursor:(f32,f32)){
        use crate::audit_events::Command;
        let Some(a)=audit else{return;};a.buttons.clear();
        let top=self.editor_hud_rect.map_or(viewport[1]+12.0,|r|r[3]+10.0);
        let s=scale.min((viewport[2]/1000.0).max(0.1)).min(((viewport[1]+viewport[3]-top-12.0)/704.0).max(0.1));
        let (x,y)=self.editor_window_origin("audit",viewport,[620.0*s,700.0*s],[viewport[0]+viewport[2]-632.0*s,top],[620.0*s,36.0*s]);
        let rect=|left:f32,top:f32,w:f32,h:f32|[x+left*s,y+top*s,x+(left+w)*s,y+(top+h)*s];
        let panel=rect(0.0,0.0,620.0,700.0);a.rect=Some(panel);
        let flat=self.text.flat;self.text.flat=true;
        self.editor_surface(r,scene,panel,5.0*s,PANEL);
        self.catalog_text(r,scene,"Spline connections · entire map",(x+12.0*s,y+12.0*s),590.0*s,20.0*s,WHITE);
        let mut buttons=vec![
            (rect(12.0,46.0,110.0,30.0),"Check again".to_string(),Command::Scan,!a.busy()),
            (rect(128.0,46.0,135.0,30.0),if a.markers{"Markers: ON"}else{"Markers: OFF"}.into(),Command::ToggleMarkers,true),
            (rect(269.0,46.0,235.0,30.0),if a.show_good{"Show all ends"}else{"Only open / suspect ends"}.into(),Command::ToggleGood,true),
            (rect(510.0,46.0,98.0,30.0),"Close".into(),Command::Close,true),
        ];
        for (i,(label,color))in [("Red: errors",[255,80,80,0]),("Yellow: open",[255,215,60,0]),("Grey: unchecked",MUTED),("Green: connected",[60,240,110,0])].into_iter().enumerate(){
            self.catalog_text(r,scene,label,(x+(12.0+i as f32*150.0)*s,y+86.0*s),148.0*s,12.0*s,color);
        }
        let n=a.report.counts();let summary=format!("{} splines · Ends: {} errors / {} open / {} unchecked / {} connected",a.report.roads,n[0],n[1],n[2],n[3]);
        self.catalog_text(r,scene,&summary,(x+12.0*s,y+110.0*s),596.0*s,12.0*s,WHITE);
        let message=if !a.report.unread_tiles.is_empty(){format!("INCOMPLETE: {} tiles not read · {}",a.report.unread_tiles.len(),a.message)}else{a.message.clone()};
        self.catalog_text(r,scene,&message,(x+12.0*s,y+133.0*s),596.0*s,11.0*s,[220,206,165,0]);
        let indices=a.indices();a.page=a.page.min(indices.len().saturating_sub(1)/8);
        for (row,i)in indices.clone().skip(a.page*8).take(8).enumerate(){
            let e=&a.report.entries[i];let area=rect(12.0,160.0+row as f32*54.0,596.0,50.0);
            let col=e.status.color().map(|v|(v*255.0)as u8);let color=[col[0],col[1],col[2],0];
            self.editor_surface(r,scene,area,3.0*s,if crate::audit_events::contains(area,cursor){LIT}else{PANEL_ALT});
            let title=format!("{} · ID {} · {} · Tile ({}, {})",e.status.name(),e.key.1,if e.end==0{"Start"}else{"End"},e.key.0.0,e.key.0.1);
            self.catalog_text(r,scene,&title,(area[0]+7.0*s,area[1]+5.0*s),580.0*s,13.0*s,color);
            self.catalog_text(r,scene,&e.reason,(area[0]+7.0*s,area[1]+26.0*s),580.0*s,11.0*s,WHITE);
            if e.at.is_finite(){a.buttons.push((area,Command::Jump(i)));}
        }
        if !a.busy()&&indices.is_empty(){self.catalog_text(r,scene,"No results for the selected filter.",(x+20.0*s,y+184.0*s),560.0*s,14.0*s,WHITE);}
        buttons.push((rect(12.0,606.0,100.0,30.0),"Back".into(),Command::Previous,a.page>0));
        buttons.push((rect(508.0,606.0,100.0,30.0),"Next".into(),Command::Next,(a.page+1)*8<indices.len()));
        self.catalog_text(r,scene,&format!("Page {} / {} · Click a row to go there",a.page+1,indices.len().div_ceil(8).max(1)),(x+122.0*s,y+614.0*s),378.0*s,12.0*s,WHITE);
        self.catalog_text(r,scene,"Yellow may be an intentional end. Nothing is changed automatically.",(x+12.0*s,y+650.0*s),596.0*s,11.0*s,SOFT);
        self.catalog_text(r,scene,"15 cm endpoint tolerance · Markers in 3D · Esc closes this list",(x+12.0*s,y+674.0*s),596.0*s,11.0*s,SOFT);
        for(area,title,command,enabled)in buttons{
            self.editor_surface(r,scene,area,3.0*s,if !enabled{TRACK_OFF}else if crate::audit_events::contains(area,cursor){LIT}else{CHIP});
            self.catalog_text(r,scene,&title,(area[0]+6.0*s,area[1]+7.0*s),area[2]-area[0]-12.0*s,12.0*s,WHITE);
            if enabled{a.buttons.push((area,command));}
        }
        self.text.flat=flat;
    }

    pub fn draw_editor_dock(&mut self, r: &Renderer, scene: &mut Scene,
        editor: Option<(bool, bool)>, tools: Option<bool>, connection: Option<&(String, bool)>,
        replace: bool, transition: bool, objects:Option<(bool,bool,bool,bool,bool)>,terrain: Option<&crate::terrain_editor::TerrainEditor>, scale: f32, viewport: [f32; 4], cursor: (f32, f32)) {
        self.editor_window_kind = None;
        self.editor_dock_rect = None;
        self.editor_dock_header = None;
        self.editor_dock_reset = None;
        self.editor_reload_rect = None;
        self.editor_audit_rect = None;
        self.editor_help_rect = None;
        self.editor_hint_rect = None;
        self.catalog_open_rect = None;
        self.tile_open_rect = None;
        self.terrain_open_rect = None;
        self.junction_open_rect = None; self.roundabout_open_rect = None;
        self.terrain_tool_rects.clear();
        self.editor_tools_rect = None;
        self.editor_connection_rect = None;
        self.spline_tool_rects.clear();
        self.spline_connect_rect = None; self.spline_transition_rect = None;
        self.spline_cancel_rect = None;
        let Some((spline, expanded_allowed)) = editor else {
            self.editor_dock_drag = None;
            return;
        };
        if let Some(terrain)=terrain {
            self.draw_terrain_dock(r,scene,terrain,scale,viewport,cursor);return;
        }
        let s = editor_scale(scale, viewport);
        let expanded = self.editor_help_expanded && expanded_allowed;
        if self.editor_hint_mode != Some((spline, expanded)) {
            self.editor_help_scroll = 0;
            self.editor_hint_mode = Some((spline, expanded));
        }
        let shortcuts = editor_shortcuts(spline, expanded);
        let footer = if connection.is_some() { 198.0 } else if tools.is_some() { 320.0 } else if objects.is_some() {256.0} else { 0.0 };
        let hud_bottom = self.editor_hud_rect.map_or(viewport[1], |rect| rect[3]);
        let layout = editor_dock_layout(viewport, s, hud_bottom, shortcuts.len(), footer, self.editor_dock_position);
        self.editor_help_scroll_max = layout.max_scroll;
        self.editor_help_scroll = self.editor_help_scroll.min(layout.max_scroll);
        let flat = self.text.flat;
        self.text.flat = true;
        self.editor_surface(r, scene, layout.panel, 5.0 * s, PANEL);
        self.editor_surface(r, scene, layout.header, 5.0 * s, PANEL_ALT);
        self.put(r, scene, "Tools · drag here", ((13.0 * s) as u32).max(1) | BOLD,
            SOFT, layout.header[0] + 10.0 * s, layout.header[1] + 14.0 * s);
        let catalog=layout.catalog;
        let mut junction=layout.builders;junction[2]=junction[0]+135.0*s;
        let roundabout=[junction[2]+8.0*s,junction[1],layout.builders[2],junction[3]];
        for (rect, title) in [(catalog, "Catalogue (P)"),(junction,"Build junction"),(roundabout,"Build roundabout"), (layout.tiles, "New tile"), (layout.terrain, "Edit terrain"),
            (layout.reload, "Reload map …"), (layout.audit,"Check spline connections …"), (layout.toggle, if expanded { "Fewer commands" } else { "All commands" }), (layout.reset, "Left")] {
            let enabled = rect != layout.toggle || expanded_allowed;
            self.editor_surface(r, scene, rect, 3.0 * s, if !enabled { PANEL_ALT }
                else if editor_contains(rect, cursor) { LIT } else { CHIP });
            self.put(r, scene, title, ((12.0 * s) as u32).max(1), WHITE,
                rect[0] + 8.0 * s, (rect[1] + rect[3]) * 0.5);
        }
        for (row, (key, label)) in shortcuts.iter().skip(self.editor_help_scroll).take(layout.rows).enumerate() {
            let y = layout.hints[1] + row as f32 * 24.0 * s;
            let rect = [layout.hints[0], y, layout.hints[0] + 114.0 * s, y + 20.0 * s];
            self.editor_surface(r, scene, rect, 3.0 * s, CHIP);
            self.put(r, scene, key, ((11.0 * s) as u32).max(1) | BOLD,
                WHITE, rect[0] + 6.0 * s, y + 10.0 * s);
            let x = rect[2] + 8.0 * s;
            let label = clip_to(&self.text, label, 12.0 * s, layout.hints[2] - x - 8.0 * s);
            self.put(r, scene, &label, ((12.0 * s) as u32).max(1), WHITE, x, y + 10.0 * s);
        }
        if layout.max_scroll > 0 {
            let h = layout.hints[3] - layout.hints[1];
            let thumb_h = h * layout.rows as f32 / shortcuts.len() as f32;
            let top = layout.hints[1] + (h - thumb_h) * self.editor_help_scroll as f32 / layout.max_scroll as f32;
            self.catalog_plate(r, scene, TRACK_OFF,
                [layout.hints[2] - 3.0 * s, top, layout.hints[2], top + thumb_h]);
        }
        let origin = [layout.panel[0] + 10.0 * s, layout.footer_top];
        self.draw_spline_tools(r, scene, tools, s, origin, cursor);
        self.draw_object_tools(r,scene,objects,s,origin,cursor);
        self.draw_spline_connection(r, scene, connection, replace, transition, s, origin, cursor);
        self.text.flat = flat;
        self.editor_dock_rect = Some(layout.panel);
        self.editor_dock_header = Some(layout.header);
        self.editor_dock_reset = Some(layout.reset);
        self.editor_reload_rect = Some(layout.reload);
        self.editor_audit_rect = Some(layout.audit);
        self.editor_help_rect = expanded_allowed.then_some(layout.toggle);
        self.editor_hint_rect = Some(layout.hints);
        self.catalog_open_rect = Some(catalog);
        self.junction_open_rect = Some(junction);
        self.roundabout_open_rect = Some(roundabout);
        self.tile_open_rect = Some(layout.tiles);
        self.terrain_open_rect = Some(layout.terrain);
        self.editor_dock_bounds = layout.bounds;
    }

    /// Place a dialog and register only its title area as a drag handle.
    /// Normalized positions remain valid after resizing or changing UI scale.
    fn editor_window_origin(&mut self, kind: &'static str, viewport: [f32;4],
        size: [f32;2], default: [f32;2], header: [f32;2]) -> (f32,f32) {
        let min_x=viewport[0]; let min_y=viewport[1];
        let max_x=(min_x+viewport[2]-size[0]).max(min_x);
        let max_y=(min_y+viewport[3]-size[1]).max(min_y);
        let (x,y)=self.editor_window_positions.get(kind).map_or(
            (default[0].clamp(min_x,max_x),default[1].clamp(min_y,max_y)),
            |p|(min_x+p[0].clamp(0.0,1.0)*(max_x-min_x),min_y+p[1].clamp(0.0,1.0)*(max_y-min_y)));
        self.editor_window_kind=Some(kind);
        self.editor_dock_bounds=[min_x,min_y,max_x,max_y];
        self.editor_dock_rect=Some([x,y,x+size[0],y+size[1]]);
        self.editor_dock_header=Some([x,y,x+header[0],y+header[1]]);
        self.editor_dock_reset=None;
        self.editor_help_rect=None;
        (x,y)
    }

    pub fn editor_window_drag_press(&mut self, cursor:(f32,f32)) -> bool {
        if !self.editor_dock_header.is_some_and(|r|editor_contains(r,cursor)) {return false;}
        let Some(r)=self.editor_dock_rect else{return false;};
        self.editor_dock_drag=Some((cursor.0-r[0],cursor.1-r[1]));
        true
    }

    pub fn editor_hud_press(&mut self, cursor: (f32, f32)) -> bool {
        if self.editor_help_rect.is_some_and(|rect| editor_contains(rect, cursor)) {
            self.editor_help_expanded = !self.editor_help_expanded;
            self.editor_help_scroll = 0;
            return true;
        }
        if self.editor_dock_reset.is_some_and(|rect| editor_contains(rect, cursor)) {
            self.editor_dock_position = None;
            self.editor_dock_drag = None;
            return true;
        }
        if self.editor_dock_header.is_some_and(|rect| editor_contains(rect, cursor)) {
            if let Some(rect) = self.editor_dock_rect {
                self.editor_dock_drag = Some((cursor.0 - rect[0], cursor.1 - rect[1]));
            }
            return true;
        }
        self.editor_hud_rect.is_some_and(|rect| editor_contains(rect, cursor))
    }

    pub fn end_editor_dock_drag(&mut self) -> bool {
        self.editor_dock_drag.take().is_some()
    }

    pub fn editor_dock_move(&mut self, cursor: (f32, f32)) -> bool {
        let (Some((ox, oy)), Some(panel)) = (self.editor_dock_drag, self.editor_dock_rect) else { return false; };
        let [min_x, min_y, max_x, max_y] = self.editor_dock_bounds;
        let x = (cursor.0 - ox).clamp(min_x, max_x);
        let y = (cursor.1 - oy).clamp(min_y, max_y);
        let position=[(x-min_x)/(max_x-min_x).max(1.0),(y-min_y)/(max_y-min_y).max(1.0)];
        if let Some(kind)=self.editor_window_kind {self.editor_window_positions.insert(kind,position);}
        else {self.editor_dock_position=Some(position);}
        let (dx, dy) = (x - panel[0], y - panel[1]);
        for rect in self.editor_dock_rect.iter_mut().chain(self.editor_dock_header.iter_mut())
            .chain(self.editor_audit_rect.iter_mut()).chain(self.editor_reload_rect.iter_mut()).chain(self.editor_dock_reset.iter_mut()).chain(self.editor_help_rect.iter_mut())
            .chain(self.editor_hint_rect.iter_mut()).chain(self.catalog_open_rect.iter_mut())
            .chain(self.tile_open_rect.iter_mut())
            .chain(self.terrain_open_rect.iter_mut())
            .chain(self.junction_open_rect.iter_mut())
            .chain(self.roundabout_open_rect.iter_mut())
            .chain(self.editor_tools_rect.iter_mut()).chain(self.editor_connection_rect.iter_mut())
            .chain(self.spline_transition_rect.iter_mut()).chain(self.spline_connect_rect.iter_mut()).chain(self.spline_cancel_rect.iter_mut()) {
            rect[0] += dx; rect[2] += dx; rect[1] += dy; rect[3] += dy;
        }
        for (rect, _) in &mut self.spline_tool_rects {
            rect[0] += dx; rect[2] += dx; rect[1] += dy; rect[3] += dy;
        }
        for (rect,_) in &mut self.terrain_tool_rects {
            rect[0]+=dx;rect[2]+=dx;rect[1]+=dy;rect[3]+=dy;
        }
        true
    }

    pub fn editor_dock_scroll(&mut self, cursor: (f32, f32), amount: f32) -> bool {
        if !self.editor_dock_rect.is_some_and(|rect| editor_contains(rect, cursor)) { return false; }
        if amount.is_finite() && amount != 0.0 && self.editor_hint_rect.is_some_and(|rect| editor_contains(rect, cursor)) {
            let step = amount.abs().ceil() as usize;
            self.editor_help_scroll = if amount > 0.0 { self.editor_help_scroll.saturating_sub(step) }
                else { self.editor_help_scroll.saturating_add(step).min(self.editor_help_scroll_max) };
        }
        true
    }

    pub fn editor_panel_contains(&self, cursor: (f32, f32)) -> bool {
        self.editor_dock_rect.is_some_and(|rect| editor_contains(rect, cursor))
            || self.editor_hud_rect.is_some_and(|rect|editor_contains(rect,cursor))
    }

    fn draw_terrain_dock(&mut self,r:&Renderer,scene:&mut Scene,terrain:&crate::terrain_editor::TerrainEditor,
        scale:f32,viewport:[f32;4],cursor:(f32,f32)) {
        use crate::terrain_editor::{Command,Field,Mode,Tool};
        let layout=terrain_dock_layout(viewport,scale,self.editor_hud_rect.map_or(viewport[1],|r|r[3]),self.editor_dock_position);
        let s=layout.scale;let [x,y,_,_]=layout.panel;
        let rect=|a:f32,b:f32,w:f32,h:f32|[x+a*s,y+b*s,x+(a+w)*s,y+(b+h)*s];
        let flat=self.text.flat;self.text.flat=true;
        self.editor_surface(r,scene,layout.panel,5.0*s,PANEL);
        self.editor_surface(r,scene,layout.header,5.0*s,PANEL_ALT);
        self.put(r,scene,"Terrain · drag here",((13.0*s) as u32).max(1)|BOLD,SOFT,x+10.0*s,y+14.0*s);
        let mut buttons=vec![(rect(10.0,34.0,135.0,26.0),"Back to building".to_string(),Command::Exit,true,false),
            (rect(10.0,70.0,140.0,26.0),"Heights".into(),Command::Mode(Mode::Heights),true,terrain.mode==Mode::Heights),
            (rect(154.0,70.0,140.0,26.0),"Textures".into(),Command::Mode(Mode::Textures),true,terrain.mode==Mode::Textures)];
        let new_tile=rect(151.0,34.0,143.0,26.0);
        self.editor_surface(r,scene,new_tile,3.0*s,CHIP);
        self.put(r,scene,"New tile",((12.0*s) as u32).max(1),WHITE,new_tile[0]+8.0*s,(new_tile[1]+new_tile[3])*0.5);
        self.tile_open_rect=Some(new_tile);
        let fields=if terrain.mode==Mode::Heights {
            for (i,tool) in [Tool::Raise,Tool::Lower,Tool::Level,Tool::Smooth,Tool::Height].into_iter().enumerate() {
                let area=rect(10.0+(i%2) as f32*144.0,106.0+(i/2) as f32*30.0,140.0,26.0);
                buttons.push((area,tool.title().into(),Command::Tool(tool),true,terrain.tool==tool));
            }
            buttons.push((rect(154.0,166.0,140.0,26.0),"Fine smoothing".into(),Command::FineSmooth,true,terrain.tool==Tool::Smooth&&terrain.strength<=0.2));
            vec![Field::Radius,Field::Strength,Field::Softness,Field::Height,Field::TileStep]
        } else {
            self.catalog_text(r,scene,&format!("Layer {} · {}",terrain.texture_layer,if terrain.texture_layer==0 {"Base texture"} else {"Ground texture"}),
                (x+10.0*s,y+109.0*s),284.0*s,12.0*s,SOFT);
            let name=terrain.texture_name.rsplit(['/', '\\']).next().unwrap_or(&terrain.texture_name);
            self.catalog_text(r,scene,name,(x+10.0*s,y+133.0*s),284.0*s,14.0*s,WHITE);
            buttons.extend([
                (rect(10.0,164.0,36.0,28.0),"←".into(),Command::Layer(-1),true,false),
                (rect(52.0,164.0,196.0,28.0),"Texture catalogue …".into(),Command::TextureCatalog,true,false),
                (rect(254.0,164.0,40.0,28.0),"→".into(),Command::Layer(1),true,false),
                (rect(10.0,204.0,140.0,28.0),"Paint".into(),Command::Erase(false),true,!terrain.texture_erase),
                (rect(154.0,204.0,140.0,28.0),"Eraser".into(),Command::Erase(true),terrain.texture_layer>0,terrain.texture_erase),
                (rect(10.0,368.0,284.0,28.0),"Eyedropper: pick texture".into(),Command::SampleTexture,true,terrain.sample_texture),
            ]);
            self.catalog_text(r,scene,"Hold left mouse button to paint",(x+10.0*s,y+416.0*s),284.0*s,13.0*s,SOFT);
            self.catalog_text(r,scene,"Mouse wheel: brush size",(x+10.0*s,y+439.0*s),284.0*s,12.0*s,[142,142,142,255]);
            self.catalog_text(r,scene,"Paint base texture: remove overlays",(x+10.0*s,y+465.0*s),284.0*s,11.0*s,MUTED);
            vec![Field::Radius,Field::Strength,Field::Softness]
        };
        let start=if terrain.mode==Mode::Heights {204.0} else {250.0};
        for (i,field) in fields.into_iter().enumerate() {
            let row=start+i as f32*32.0;
            self.catalog_text(r,scene,field.title(),(x+10.0*s,y+(row+7.0)*s),133.0*s,11.0*s,SOFT);
            let step=match field {Field::Radius=>1.0,Field::Softness=>5.0,_=>0.1};
            buttons.push((rect(146.0,row,25.0,28.0),"−".into(),Command::Adjust(field,-step),true,false));
            buttons.push((rect(269.0,row,25.0,28.0),"+".into(),Command::Adjust(field,step),true,false));
            let editing=terrain.input.as_ref().filter(|input|input.field==field);
            let text=editing.map(|input|format!("{}│",input.text)).unwrap_or_else(||format!("{:.2}",terrain.value(field)));
            buttons.push((rect(177.0,row,86.0,28.0),text,Command::Edit(field),true,editing.is_some()));
        }
        if terrain.mode==Mode::Heights {
            buttons.extend([
                (rect(10.0,374.0,140.0,28.0),"Sample height".into(),Command::SampleHeight,true,terrain.sample_height),
                (rect(154.0,374.0,140.0,28.0),"Select tile".into(),Command::PickTile,true,terrain.pick_tile),
                (rect(10.0,432.0,140.0,28.0),"Lower tile".into(),Command::TileMove(-1.0),terrain.tile.is_some(),false),
                (rect(154.0,432.0,140.0,28.0),"Raise tile".into(),Command::TileMove(1.0),terrain.tile.is_some(),false),
                (rect(10.0,464.0,284.0,28.0),"Set entire tile to target height".into(),Command::TileLevel,terrain.tile.is_some(),false),
                (rect(10.0,496.0,284.0,26.0),if terrain.blend {"Blend with neighbours: 20 m"} else {"Blending OFF · possible steps"}.into(),Command::Blend,true,terrain.blend),
            ]);
            let selected=terrain.tile.map(|k|format!("Selected tile: ({}, {})",k.0,k.1)).unwrap_or_else(||"No tile selected yet".into());
            self.catalog_text(r,scene,&selected,(x+10.0*s,y+408.0*s),284.0*s,12.0*s,SOFT);
        }
        buttons.extend([
            (rect(10.0,528.0,140.0,28.0),"Undo".into(),Command::Undo,terrain.can_undo(),false),
            (rect(154.0,528.0,140.0,28.0),"Redo".into(),Command::Redo,terrain.can_redo(),false),
            (rect(10.0,562.0,140.0,28.0),"Save (Ctrl+S)".into(),Command::Save,true,false),
        ]);
        for (area,title,command,enabled,selected) in buttons {
            let primary = matches!(&command, Command::Save);
            self.editor_surface(r,scene,area,3.0*s,if !enabled {PANEL_ALT}
                else if primary {if editor_contains(area,cursor) {ACCENT_HOT} else {ACCENT}}
                else if selected {SELECTED} else if editor_contains(area,cursor) {LIT} else {CHIP});
            self.catalog_text(r,scene,&title,(area[0]+5.0*s,area[1]+6.0*s),area[2]-area[0]-10.0*s,12.0*s,if !enabled {OFF_INK} else if primary {ON_ACCENT} else {WHITE});
            if enabled {self.terrain_tool_rects.push((area,command));}
        }
        let reload=rect(154.0,562.0,140.0,28.0);
        self.editor_surface(r,scene,reload,3.0*s,CHIP);
        self.catalog_text(r,scene,"Reload map …",(reload[0]+5.0*s,reload[1]+6.0*s),130.0*s,12.0*s,WHITE);
        self.editor_reload_rect=Some(reload);
        let audit=rect(10.0,596.0,284.0,28.0);
        self.editor_surface(r,scene,audit,3.0*s,CHIP);
        self.catalog_text(r,scene,"Check spline connections …",(audit[0]+5.0*s,audit[1]+6.0*s),274.0*s,12.0*s,WHITE);
        self.editor_audit_rect=Some(audit);
        let reset=rect(234.0,0.0,60.0,28.0);
        self.editor_surface(r,scene,reset,3.0*s,CHIP);
        self.put(r,scene,"Left",((12.0*s) as u32).max(1),WHITE,reset[0]+8.0*s,(reset[1]+reset[3])*0.5);
        let message=if terrain.input.is_some() {"Type number · Enter applies · Esc cancels"} else {&terrain.message};
        for (i,line) in wrap(&self.text,message,11.0*s,284.0*s).iter().take(2).enumerate() {
            self.catalog_text(r,scene,line,(x+10.0*s,y+(634.0+i as f32*15.0)*s),284.0*s,11.0*s,SOFT);
        }
        self.editor_dock_rect=Some(layout.panel);self.editor_dock_header=Some(layout.header);self.editor_dock_reset=Some(reset);
        self.editor_dock_bounds=layout.bounds;self.text.flat=flat;
    }

    fn draw_object_tools(&mut self,r:&Renderer,scene:&mut Scene,state:Option<(bool,bool,bool,bool,bool)>,s:f32,origin:[f32;2],cursor:(f32,f32)) {
        use crate::editor::Action;
        let Some((copy,paste,last,repeat,placing))=state else {return;};let [x,y]=origin;
        self.editor_surface(r,scene,[x,y,x+284.0*s,y+256.0*s],3.0*s,PANEL);
        self.catalog_text(r,scene,"Copy / place objects",(x+8.0*s,y+5.0*s),268.0*s,13.0*s,MUTED);
        for (a,b,w,title,action,enabled) in [
            (6.0,26.0,132.0,"Copy (Ctrl+C)",Action::ClipboardCopy,copy),
            (144.0,26.0,134.0,"Paste (Ctrl+V)",Action::Paste,paste),
            (6.0,58.0,272.0,"Place last object again (Ctrl+R)",Action::RepeatObject,last),
            (6.0,90.0,272.0,if repeat {"Repeat placement: ON"} else {"Repeat placement: OFF"},Action::PlacementRepeat,true),
            (6.0,122.0,272.0,"Finish placement (Esc)",Action::CancelPlacement,placing),
            (6.0,154.0,272.0,"Edit text …",Action::ObjectText,copy),
            (6.0,186.0,272.0,"Select / edit AI paths",Action::TrafficManage,true),
            (6.0,218.0,272.0,"Bridge / tunnel / underground …",Action::ConstructionWindow,true),
        ] {
            let rect=[x+a*s,y+b*s,x+(a+w)*s,y+(b+26.0)*s];
            self.editor_surface(r,scene,rect,3.0*s,if !enabled {PANEL_ALT} else if editor_contains(rect,cursor) {LIT} else {CHIP});
            self.catalog_text(r,scene,title,(rect[0]+6.0*s,rect[1]+6.0*s),(w-12.0)*s,11.0*s,WHITE);
            if enabled {self.spline_tool_rects.push((rect,action));}
        }
    }

    /// Fixed dock footer: hints can scroll without moving these actions.
    fn draw_spline_tools(&mut self, r: &Renderer, scene: &mut Scene, enabled: Option<bool>,
        s: f32, origin: [f32; 2], cursor: (f32, f32)) {
        use crate::editor::Action;
        let Some(enabled) = enabled else { return; };
        let [x, y] = origin;
        let panel = [x, y, x + 284.0 * s, y + 320.0 * s];
        self.editor_tools_rect = Some(panel);
        self.editor_surface(r, scene, panel, 3.0 * s, PANEL);
        self.catalog_text(r, scene, if enabled { "Road shoulders / terrain" } else { "Select a road first" },
            (x + 8.0 * s, y + 5.0 * s), 268.0 * s, 13.0 * s, MUTED);
        for (row, (action, title)) in [
            (Action::SmoothRoad(false), "Smooth shoulder (F8)"),
            (Action::SmoothRoad(true), "Smooth wider (Shift+F8)"),
            (Action::FitTerrain, "Fit to ground (F6)"),
            (Action::RoadsideWindow, "Objects along the road"),
            (Action::SidewalkWindow, "Sidewalk along the road"),
            (Action::TrafficManage, "Select / edit AI paths"),
            (Action::TrafficPaths, "AI paths along the road"),
            (Action::TrafficOverlay, "Show / hide AI paths"),
            (Action::ConstructionWindow, "Bridge / tunnel / underground …"),
        ].into_iter().enumerate() {
            let enabled=enabled||matches!(action,Action::TrafficManage|Action::TrafficOverlay|Action::ConstructionWindow);
            let top = y + (27.0 + row as f32 * 32.0) * s;
            let rect = [x + 6.0 * s, top, x + 278.0 * s, top + 27.0 * s];
            let color = if !enabled { PANEL_ALT }
                else if editor_contains(rect, cursor) { LIT } else { CHIP };
            self.editor_surface(r, scene, rect, 3.0 * s, color);
            self.put(r, scene, title, ((12.0 * s) as u32).max(1), WHITE,
                rect[0] + 8.0 * s, top + 13.5 * s);
            self.spline_tool_rects.push((rect, action));
        }
    }

    pub fn draw_roadside_window(&mut self, r: &Renderer, scene: &mut Scene,
        window: Option<&mut crate::roadside_objects::Window>, scale: f32, viewport: [f32; 4], cursor: (f32, f32)) {
        use crate::roadside_objects::{Command, Field};
        let Some(window) = window else { return; }; window.rects.clear();
        let top = self.editor_hud_rect.map_or(viewport[1], |r| r[3]) + 10.0;
        let s = scale.max(0.5).min((viewport[2] / 544.0).max(0.1))
            .min(((viewport[1] + viewport[3] - top - 12.0) / 712.0).max(0.1));
        let min_x = viewport[0] + 12.0 * s; let min_y = top;
        let max_x = (viewport[0] + viewport[2] - 372.0 * s).max(min_x);
        let max_y = (viewport[1] + viewport[3] - 724.0 * s).max(min_y);
        let position = self.editor_dock_position.unwrap_or([0.0, 0.0]);
        let x = min_x + position[0].clamp(0.0, 1.0) * (max_x - min_x);
        let y = min_y + position[1].clamp(0.0, 1.0) * (max_y - min_y);
        let rect = |a: f32, b: f32, w: f32, h: f32| [x + a * s, y + b * s, x + (a + w) * s, y + (b + h) * s];
        self.editor_dock_rect = Some(rect(0.0, 0.0, 360.0, 712.0)); window.rect = self.editor_dock_rect;
        self.editor_dock_header = Some(rect(0.0, 0.0, 360.0, 28.0));
        self.editor_dock_bounds = [min_x, min_y, max_x, max_y];
        self.editor_help_rect = None; self.editor_dock_reset = None; self.editor_hint_rect = None;
        self.catalog_open_rect = None; self.tile_open_rect = None; self.terrain_open_rect = None; self.junction_open_rect = None; self.roundabout_open_rect = None;
        self.editor_tools_rect = None; self.editor_connection_rect = None; self.spline_connect_rect = None; self.spline_transition_rect = None; self.spline_cancel_rect = None;
        self.spline_tool_rects.clear(); self.terrain_tool_rects.clear();
        let flat = self.text.flat; self.text.flat = true;
        self.editor_surface(r, scene, self.editor_dock_rect.unwrap(), 5.0 * s, PANEL);
        self.editor_surface(r, scene, self.editor_dock_header.unwrap(), 5.0 * s, PANEL_ALT);
        self.put(r, scene, "Objects along the road · drag here", ((13.0 * s) as u32).max(1) | BOLD,
            SOFT, x + 10.0 * s, y + 14.0 * s);
        let mut buttons = vec![(rect(10.0, 36.0, 340.0, 28.0), "Choose object in catalogue …".to_string(), Command::Catalog, true, false)];
        let name = window.file.rsplit(['/', '\\']).next().unwrap_or(&window.file);
        self.catalog_text(r, scene, name, (x + 12.0 * s, y + 70.0 * s), 332.0 * s, 12.0 * s, MUTED);
        for (i, (sides, title)) in [(1, "Left"), (2, "Right"), (3, "Both sides")].into_iter().enumerate() {
            buttons.push((rect(10.0 + i as f32 * 115.0, 94.0, 110.0, 28.0), title.into(), Command::Sides(sides), true, window.settings.sides == sides));
        }
        buttons.extend([
            (rect(10.0, 130.0, 165.0, 28.0), "Selected spline".into(), Command::Connected(false), true, !window.settings.connected),
            (rect(185.0, 130.0, 165.0, 28.0), "Connected road".into(), Command::Connected(true), true, window.settings.connected),
        ]);
        for (i, field) in [Field::Interval, Field::Margin, Field::Start, Field::Range, Field::Height, Field::Rotation, Field::JunctionGap].into_iter().enumerate() {
            let row = 166.0 + i as f32 * 34.0;
            self.catalog_text(r, scene, field.title(), (x + 12.0 * s, y + (row + 8.0) * s), 196.0 * s, 12.0 * s, SOFT);
            let step = match field { Field::Interval | Field::Start | Field::Range | Field::Rotation => 5.0, Field::Height => 0.1, _ => 0.5 };
            buttons.push((rect(210.0, row, 26.0, 28.0), "−".into(), Command::Adjust(field, -step), true, false));
            buttons.push((rect(324.0, row, 26.0, 28.0), "+".into(), Command::Adjust(field, step), true, false));
            let input = window.input.as_ref().filter(|input| input.field == field);
            let title = input.map(|input| format!("{}│", input.text)).unwrap_or_else(|| format!("{:.2}", window.value(field)));
            buttons.push((rect(242.0, row, 76.0, 28.0), title, Command::Edit(field), true, input.is_some()));
        }
        buttons.extend([
            (rect(10.0, 412.0, 165.0, 28.0), "Height: road".into(), Command::Ground(false), true, !window.settings.ground),
            (rect(185.0, 412.0, 165.0, 28.0), "Height: terrain".into(), Command::Ground(true), true, window.settings.ground),
            (rect(10.0, 448.0, 165.0, 28.0), "Leave driveway clear".into(), Command::PickGap, true, window.gap_pick.is_some()),
            (rect(185.0, 448.0, 165.0, 28.0), "Clear custom gaps".into(), Command::ClearGaps, !window.settings.manual_gaps.is_empty() || window.gap_pick.is_some(), false),
            (rect(10.0, 532.0, 222.0, 32.0), "Place preview".into(), Command::Apply,
                window.error.is_none() && !window.preview.points.is_empty() && window.input.is_none() && window.gap_pick.is_none(), false),
            (rect(242.0, 532.0, 108.0, 32.0), "Refresh".into(), Command::Preview, true, false),
            (rect(10.0, 572.0, 165.0, 28.0), "Undo (Ctrl+Z)".into(), Command::Undo, window.can_undo, false),
            (rect(185.0, 572.0, 165.0, 28.0), "Remove own row".into(), Command::Remove, window.start.is_some(), false),
            (rect(10.0, 608.0, 165.0, 28.0), "Save (Ctrl+S)".into(), Command::Save, true, false),
            (rect(185.0, 608.0, 165.0, 28.0), "Close (Esc)".into(), Command::Close, true, false),
        ]);
        let summary = format!("{} objects · {:.1} m · {} splines", window.preview.points.len(), window.preview.length, window.preview.segments);
        let skipped = format!("{} positions free / already occupied · {} gaps", window.preview.skipped, window.preview.gaps.len());
        self.catalog_text(r, scene, &summary, (x + 12.0 * s, y + 486.0 * s), 336.0 * s, 12.0 * s, MUTED);
        self.catalog_text(r, scene, &skipped, (x + 12.0 * s, y + 504.0 * s), 336.0 * s, 11.0 * s, MUTED);
        let message = if window.input.is_some() { "Enter a number · Enter applies · Esc cancels input" }
            else { window.error.as_deref().unwrap_or(&window.message) };
        for (i, line) in wrap(&self.text, message, 12.0 * s, 336.0 * s).iter().take(4).enumerate() {
            self.catalog_text(r, scene, line, (x + 12.0 * s, y + (650.0 + i as f32 * 15.0) * s), 336.0 * s, 12.0 * s, SOFT);
        }
        for (area, title, command, enabled, selected) in buttons {
            let color = if !enabled { PANEL_ALT } else if selected { SELECTED }
                else if editor_contains(area, cursor) { LIT } else { CHIP };
            self.editor_surface(r, scene, area, 3.0 * s, color);
            self.catalog_text(r, scene, &title, (area[0] + 6.0 * s, area[1] + 7.0 * s), area[2] - area[0] - 12.0 * s, 12.0 * s, WHITE);
            if enabled { window.rects.push((area, command)); }
        }
        self.text.flat = flat;
    }

    pub fn draw_traffic_window(&mut self,r:&Renderer,scene:&mut Scene,window:Option<&mut crate::traffic_editor::Window>,show_paths:bool,scale:f32,viewport:[f32;4],cursor:(f32,f32)) {
        use crate::traffic_editor::Command;
        let Some(w)=window else{return;};w.rects.clear();
        let top=self.editor_hud_rect.map_or(viewport[1],|r|r[3])+10.0;
        let height=if w.manager.is_some(){480.0}else{786.0};
        let s=scale.max(0.5).min((viewport[2]/440.0).max(0.1)).min(((viewport[1]+viewport[3]-top-12.0)/height).max(0.1));
        let kind=if w.manager.is_some(){"path-manager"}else if w.connector.is_some(){"path-connector"}else{"paths"};
        let (x,y)=self.editor_window_origin(kind,viewport,[410.0*s,height*s],[viewport[0]+12.0*s,top],[410.0*s,32.0*s]);
        let rect=|a:f32,b:f32,ww:f32,h:f32|[x+a*s,y+b*s,x+(a+ww)*s,y+(b+h)*s];
        w.rect=Some(rect(0.0,0.0,410.0,height));self.editor_dock_rect=w.rect;
        self.editor_help_rect=None;self.editor_tools_rect=None;
        self.editor_connection_rect=None;self.spline_tool_rects.clear();self.terrain_tool_rects.clear();
        self.catalog_open_rect=None;self.tile_open_rect=None;self.terrain_open_rect=None;
        self.junction_open_rect=None;self.roundabout_open_rect=None;
        let flat=self.text.flat;self.text.flat=true;
        self.editor_surface(r,scene,w.rect.unwrap(),5.0*s,PANEL);
        if w.manager.is_some(){self.draw_path_manager(r,scene,w,s,x,y,cursor);self.text.flat=flat;return;}
        if let Some(c)=w.connector.as_ref() {
            self.put(r,scene,"Additional AI connection",((15.0*s)as u32).max(1)|BOLD,WHITE,x+12.0*s,y+18.0*s);
            let source=c.source.map(|t|format!("Start: path {} / lane {}",t.key.id,t.key.path+1)).unwrap_or("Select start: yellow path end".into());
            let target=c.destination.map(|t|format!("Destination: path {} / lane {}",t.key.id,t.key.path+1)).unwrap_or("Select destination: purple path start".into());
            let buttons=vec![
                (rect(10.0,40.0,390.0,34.0),source,Command::ConnectorSource,true),
                (rect(10.0,84.0,390.0,34.0),target,Command::ConnectorDestination,c.source.is_some()),
                (rect(10.0,132.0,390.0,30.0),format!("Vehicles: {}",if c.bus_only{"Buses only"}else{"All vehicles"}),Command::ForwardBus,true),
                (rect(310.0,172.0,40.0,28.0),"−".into(),Command::Width(-0.25),true),
                (rect(360.0,172.0,40.0,28.0),"+".into(),Command::Width(0.25),true),
                (rect(10.0,420.0,390.0,32.0),"Select start and destination again".into(),Command::ConnectorReset,true),
                (rect(10.0,466.0,390.0,34.0),"Add connection (Enter)".into(),Command::Apply,c.picking.is_none()&&w.error.is_none()&&!w.preview.pieces.is_empty()),
                (rect(10.0,512.0,390.0,30.0),"Back to AI paths along the road".into(),Command::Connector,true),
                (rect(10.0,556.0,390.0,30.0),if c.source.is_some()&&c.destination.is_some(){"Add connection + save (Ctrl+S)"}else{"Save map (Ctrl+S)"}.into(),Command::PathSave,true),
                (rect(10.0,594.0,390.0,30.0),"Close (Esc)".into(),Command::Close,true),
            ];
            self.catalog_text(r,scene,&format!("Lane width: {:.2} m",c.width),(x+12.0*s,y+184.0*s),290.0*s,12.0*s,SOFT);
            for (i,text) in ["Straight-ahead paths are retained.","Yellow: start · Purple: destination · Green: drag curve", "Connect path endpoints, not the middle of a path.", "Preview only until you add the connection."].iter().enumerate(){
                self.catalog_text(r,scene,text,(x+12.0*s,y+(224.0+i as f32*22.0)*s),386.0*s,12.0*s,SOFT);
            }
            let status=c.status();let message=w.error.as_deref().unwrap_or(&status);
            for (i,line) in wrap(&self.text,message,12.0*s,382.0*s).iter().take(5).enumerate(){
                self.catalog_text(r,scene,line,(x+12.0*s,y+(326.0+i as f32*17.0)*s),382.0*s,12.0*s,WHITE);
            }
            self.catalog_text(r,scene,"Ctrl+S: add the completed connection and save",(x+12.0*s,y+642.0*s),386.0*s,12.0*s,SOFT);
            self.catalog_text(r,scene,"Then reload the map so AI traffic uses the path.",(x+12.0*s,y+664.0*s),386.0*s,11.0*s,SOFT);
            for(area,title,command,enabled) in buttons {
                self.editor_surface(r,scene,area,3.0*s,if !enabled{PANEL_ALT}else if editor_contains(area,cursor){LIT}else{CHIP});
                self.catalog_text(r,scene,&title,(area[0]+6.0*s,area[1]+8.0*s),area[2]-area[0]-12.0*s,12.0*s,WHITE);
                if enabled{w.rects.push((area,command));}
            }
            self.text.flat=flat;return;
        }
        self.put(r,scene,"AI paths along the road",((15.0*s)as u32).max(1)|BOLD,WHITE,x+12.0*s,y+18.0*s);
        let settings=&w.settings;
        let mut buttons=vec![
            (rect(10.0,40.0,390.0,30.0),if settings.connected{"Scope: connected spline chain"}else{"Scope: selected spline"}.to_string(),Command::Connected,true),
            (rect(10.0,78.0,390.0,30.0),if settings.two_way{if settings.parallel{"Two lanes in the same direction"}else{"Two opposing lanes"}}else{"One lane / one-way road"}.to_string(),Command::TwoWay,true),
            (rect(10.0,116.0,390.0,30.0),if settings.reverse{"Direction: reversed"}else{"Direction: along the reference"}.to_string(),Command::Reverse,true),
            (rect(10.0,154.0,390.0,30.0),format!("Lane 1: {}",if settings.forward_bus{"Buses only"}else{"All vehicles"}),Command::ForwardBus,true),
            (rect(10.0,192.0,390.0,30.0),format!("Lane 2: {}",if settings.backward_bus{"Buses only"}else{"All vehicles"}),Command::BackwardBus,settings.two_way),
        ];
        for (i,(label,value,minus,plus)) in [
            ("Lane width",settings.width,Command::Width(-0.25),Command::Width(0.25)),
            ("Lateral offset",settings.offset,Command::Offset(-0.25),Command::Offset(0.25)),
            ("Height above spline",settings.height,Command::Height(-0.05),Command::Height(0.05)),
        ].into_iter().enumerate(){let row=234.0+i as f32*36.0;
            let label=omsi_ui::tr(label);
            self.catalog_text(r,scene,&format!("{label}: {value:.2} m"),(x+12.0*s,y+(row+8.0)*s),290.0*s,12.0*s,SOFT);
            buttons.push((rect(310.0,row,40.0,28.0),"−".into(),minus,true));
            buttons.push((rect(360.0,row,40.0,28.0),"+".into(),plus,true));
        }
        self.catalog_text(r,scene,"Blue: all vehicles · Orange: buses only",(x+12.0*s,y+354.0*s),386.0*s,12.0*s,SOFT);
        self.catalog_text(r,scene,if w.free {"Green: drag · Purple: connection · Release: snap"}else{"Arrows show travel direction. Bus schedules are separate."},(x+12.0*s,y+374.0*s),386.0*s,11.0*s,MUTED);
        let message=w.error.as_deref().unwrap_or(&w.preview.message);
        for (i,line) in wrap(&self.text,message,12.0*s,382.0*s).iter().take(3).enumerate(){
            self.catalog_text(r,scene,line,(x+12.0*s,y+(398.0+i as f32*17.0)*s),382.0*s,12.0*s,SOFT);
        }
        buttons.extend([
            (rect(10.0,456.0,390.0,30.0),if w.free {"Freeform active: left-drag points"}else{"Edit freeform points"}.into(),Command::Free,true),
            (rect(10.0,490.0,190.0,30.0),format!("Select lane {}",if w.active_lane==0{2}else{1}),Command::Lane,w.free&&settings.two_way),
            (rect(210.0,490.0,190.0,30.0),"Reset point".into(),Command::ResetNode,w.free&&w.selected_node.is_some()),
            (rect(10.0,524.0,190.0,30.0),"Lower point (0.1 m)".into(),Command::NodeHeight(-0.1),w.free&&w.selected_node.is_some()),
            (rect(210.0,524.0,190.0,30.0),"Raise point (0.1 m)".into(),Command::NodeHeight(0.1),w.free&&w.selected_node.is_some()),
            (rect(10.0,564.0,390.0,30.0),if w.replace {"Mode: replace existing AI paths"}else{"Mode: add missing AI paths"}.into(),Command::Replace,true),
            (rect(10.0,604.0,390.0,30.0),if show_paths {"AI paths: visible - hide"}else{"AI paths: hidden - show"}.into(),Command::Overlay,true),
            (rect(10.0,644.0,390.0,32.0),if w.replace {"Replace AI paths (Enter)"}else{"Create AI paths (Enter)"}.into(),Command::Apply,w.error.is_none()&&!w.preview.pieces.is_empty()),
            (rect(10.0,724.0,190.0,30.0),"Create additional AI connection".into(),Command::Connector,true),
            (rect(210.0,724.0,190.0,30.0),"Select / edit AI paths".into(),Command::Manage,true),
            (rect(10.0,686.0,190.0,30.0),"Refresh preview".into(),Command::Refresh,true),
            (rect(210.0,686.0,190.0,30.0),"Close (Esc)".into(),Command::Close,true),
        ]);
        self.catalog_text(r,scene,"Then: Ctrl+Z undo · Ctrl+S save",(x+12.0*s,y+765.0*s),386.0*s,11.0*s,MUTED);
        for(area,title,command,enabled) in buttons {
            self.editor_surface(r,scene,area,3.0*s,if !enabled{PANEL_ALT}else if editor_contains(area,cursor){LIT}else{CHIP});
            self.catalog_text(r,scene,&title,(area[0]+6.0*s,area[1]+8.0*s),area[2]-area[0]-12.0*s,12.0*s,WHITE);
            if enabled{w.rects.push((area,command));}
        }
        self.text.flat=flat;
    }

    pub fn draw_construction_window(&mut self, r:&Renderer, scene:&mut Scene, editor:Option<&mut crate::editor::Editor>, scale:f32, viewport:[f32;4], cursor:(f32,f32)) {
        use crate::construction_tools::Command;
        let Some(ed)=editor else{return;}; let Some(w)=ed.construction_window.as_mut() else{return;};
        w.buttons.clear();
        let flat=self.text.flat; self.text.flat=true;
        let top=self.editor_hud_rect.map_or(viewport[1],|r|r[3])+10.0;
        let s=scale.max(0.5).min((viewport[2]/384.0).max(0.1)).min(((viewport[1]+viewport[3]-top-12.0)/560.0).max(0.1));
        let (x,y)=self.editor_window_origin("construction",viewport,[360.0*s,560.0*s],[viewport[0]+12.0*s,top],[360.0*s,32.0*s]);
        let rect=|a:f32,b:f32,width:f32,height:f32|[x+a*s,y+b*s,x+(a+width)*s,y+(b+height)*s];
        let panel=rect(0.0,0.0,360.0,560.0);w.rect=Some(panel);self.editor_dock_rect=Some(panel);
        self.spline_tool_rects.clear();self.terrain_tool_rects.clear();
        self.editor_surface(r,scene,panel,5.0*s,PANEL);
        self.catalog_text(r,scene,"Bridge / tunnel / underground",(x+10.0*s,y+10.0*s),340.0*s,14.0*s,WHITE);
        let mut buttons=vec![(rect(10.0,42.0,165.0,28.0),"Bridge: +5 m".into(),Command::Bridge,w.key.is_some()),
            (rect(185.0,42.0,165.0,28.0),"Tunnel: -5 m".into(),Command::Tunnel,w.key.is_some())];
        for (i,label) in ["Start height (m)","End height (m)","Start gradient (%)","End gradient (%)","Work plane height (m)"].into_iter().enumerate() {
            let at=82.0+i as f32*46.0;
            self.catalog_text(r,scene,label,(x+12.0*s,y+at*s),190.0*s,12.0*s,WHITE);
            let value=w.input.as_ref().filter(|(f,_,_)|*f==i).map(|(_,t,_)|t.clone()).unwrap_or_else(||format!("{:.2}",w.values[i]));
            buttons.push((rect(200.0,at-4.0,28.0,28.0),"−".into(),Command::Adjust(i,-0.5),true));
            buttons.push((rect(234.0,at-4.0,80.0,28.0),value,Command::Edit(i),true));
            buttons.push((rect(320.0,at-4.0,28.0,28.0),"+".into(),Command::Adjust(i,0.5),true));
        }
        buttons.extend([
            (rect(10.0,316.0,340.0,28.0),if ed.construction_plane.is_some(){"Work plane: ON"}else{"Work plane: OFF"}.into(),Command::Plane,true),
            (rect(10.0,352.0,340.0,28.0),if ed.hide_terrain{"Terrain: hidden (editor only)"}else{"Terrain: visible"}.into(),Command::Hide,true),
            (rect(10.0,492.0,222.0,32.0),"Apply road heights".into(),Command::Apply,w.key.is_some()),
            (rect(240.0,492.0,110.0,32.0),"Close (Esc)".into(),Command::Close,true)]);
        for (rect,title,command,enabled) in buttons {
            self.editor_surface(r,scene,rect,3.0*s,if !enabled{PANEL_ALT}else if editor_contains(rect,cursor){LIT}else{CHIP});
            self.catalog_text(r,scene,&title,(rect[0]+6.0*s,rect[1]+6.0*s),rect[2]-rect[0]-12.0*s,12.0*s,WHITE);
            if enabled{w.buttons.push((rect,command));}
        }
        self.catalog_text(r,scene,"Road only: linked ends need checking",(x+12.0*s,y+394.0*s),336.0*s,11.0*s,MUTED);
        self.catalog_text(r,scene,"Deck, walls and portals: choose catalogue assets",(x+12.0*s,y+418.0*s),336.0*s,11.0*s,MUTED);
        for (i,line) in wrap(&self.text,&w.message,11.0*s,336.0*s).iter().take(3).enumerate() {
            self.catalog_text(r,scene,line,(x+12.0*s,y+(442.0+i as f32*15.0)*s),336.0*s,11.0*s,WHITE);
        }
        self.catalog_text(r,scene,"Work plane stays active after closing this panel",(x+12.0*s,y+534.0*s),336.0*s,10.0*s,MUTED);
        self.text.flat=flat;
    }

    pub fn draw_sidewalk_window(&mut self, r: &Renderer, scene: &mut Scene,
        window: Option<&mut crate::sidewalk::Window>, scale: f32, viewport: [f32; 4], cursor: (f32, f32)) {
        use crate::sidewalk::{Command, Field};
        let Some(window) = window else { return; }; window.rects.clear();
        let top = self.editor_hud_rect.map_or(viewport[1], |r| r[3]) + 10.0;
        let s = scale.max(0.5).min((viewport[2] / 544.0).max(0.1))
            .min(((viewport[1] + viewport[3] - top - 12.0) / 712.0).max(0.1));
        let min_x = viewport[0] + 12.0 * s; let min_y = top;
        let max_x = (viewport[0] + viewport[2] - 372.0 * s).max(min_x);
        let max_y = (viewport[1] + viewport[3] - 724.0 * s).max(min_y);
        let position = self.editor_dock_position.unwrap_or([0.0, 0.0]);
        let x = min_x + position[0].clamp(0.0, 1.0) * (max_x - min_x);
        let y = min_y + position[1].clamp(0.0, 1.0) * (max_y - min_y);
        let rect = |a: f32, b: f32, w: f32, h: f32| [x + a * s, y + b * s, x + (a + w) * s, y + (b + h) * s];
        self.editor_dock_rect = Some(rect(0.0, 0.0, 360.0, 712.0)); window.rect = self.editor_dock_rect;
        self.editor_dock_header = Some(rect(0.0, 0.0, 360.0, 28.0));
        self.editor_dock_bounds = [min_x, min_y, max_x, max_y];
        self.editor_help_rect = None; self.editor_dock_reset = None; self.editor_hint_rect = None;
        self.catalog_open_rect = None; self.tile_open_rect = None; self.terrain_open_rect = None; self.junction_open_rect = None; self.roundabout_open_rect = None;
        self.editor_tools_rect = None; self.editor_connection_rect = None; self.spline_connect_rect = None; self.spline_transition_rect = None; self.spline_cancel_rect = None;
        self.spline_tool_rects.clear(); self.terrain_tool_rects.clear();
        let flat = self.text.flat; self.text.flat = true;
        self.editor_surface(r, scene, self.editor_dock_rect.unwrap(), 5.0 * s, PANEL);
        self.editor_surface(r, scene, self.editor_dock_header.unwrap(), 5.0 * s, PANEL_ALT);
        self.put(r, scene, "Sidewalk along road · drag here", ((13.0 * s) as u32).max(1) | BOLD,
            SOFT, x + 10.0 * s, y + 14.0 * s);
        let mut buttons = vec![(rect(10.0, 36.0, 340.0, 28.0), "Choose sidewalk profile in catalogue …".to_string(), Command::Catalog, true, false)];
        let name = window.file.rsplit(['/', '\\']).next().unwrap_or(&window.file);
        self.catalog_text(r, scene, name, (x + 12.0 * s, y + 70.0 * s), 332.0 * s, 12.0 * s, MUTED);
        for (i, (sides, title)) in [(1, "Left"), (2, "Right"), (3, "Both sides")].into_iter().enumerate() {
            buttons.push((rect(10.0 + i as f32 * 115.0, 94.0, 110.0, 28.0), title.into(), Command::Sides(sides), true, window.settings.sides == sides));
        }
        buttons.extend([
            (rect(10.0, 130.0, 165.0, 28.0), "Selected spline".into(), Command::Connected(false), true, !window.settings.connected),
            (rect(185.0, 130.0, 165.0, 28.0), "Connected road".into(), Command::Connected(true), true, window.settings.connected),
        ]);
        for (i, field) in [Field::Start, Field::Length, Field::Margin, Field::Height].into_iter().enumerate() {
            let row = 166.0 + i as f32 * 34.0;
            self.catalog_text(r, scene, field.title(), (x + 12.0 * s, y + (row + 8.0) * s), 196.0 * s, 12.0 * s, SOFT);
            let step = match field { Field::Start | Field::Length => 5.0, Field::Height => 0.1, _ => 0.5 };
            buttons.push((rect(210.0, row, 26.0, 28.0), "−".into(), Command::Adjust(field, -step), true, false));
            buttons.push((rect(324.0, row, 26.0, 28.0), "+".into(), Command::Adjust(field, step), true, false));
            let input = window.input.as_ref().filter(|input| input.field == field);
            let title = input.map(|input| format!("{}│", input.text)).unwrap_or_else(|| format!("{:.2}", window.value(field)));
            buttons.push((rect(242.0, row, 76.0, 28.0), title, Command::Edit(field), true, input.is_some()));
        }
        buttons.extend([
            (rect(10.0,310.0,340.0,28.0),"Mirror profile".into(),Command::Mirror,true,window.settings.mirror),
            (rect(10.0,348.0,165.0,28.0),"Choose road".into(),Command::PickRoad,true,window.picking==1),
            (rect(185.0,348.0,165.0,28.0),"Select sidewalk".into(),Command::Existing,true,window.picking==2),
            (rect(10.0,388.0,340.0,28.0),"Create as new sidewalk".into(),Command::New,true,window.existing.is_none()),
            (rect(10.0,422.0,340.0,28.0),"Disconnect old sidewalk links".into(),Command::Detach,window.existing.is_some(),window.settings.detach),
            (rect(10.0,532.0,222.0,32.0),if window.existing.is_some(){"Align sidewalk"}else{"Place preview"}.into(),Command::Apply,
                window.error.is_none()&&!window.preview.pieces.is_empty()&&window.input.is_none()&&window.picking==0,false),
            (rect(242.0,532.0,108.0,32.0),"Refresh".into(),Command::Preview,true,false),
            (rect(10.0,572.0,340.0,28.0),"Undo (Ctrl+Z)".into(),Command::Undo,window.can_undo,false),
            (rect(10.0,608.0,165.0,28.0),"Save (Ctrl+S)".into(),Command::Save,true,false),
            (rect(185.0,608.0,165.0,28.0),"Close (Esc)".into(),Command::Close,true,false),
        ]);
        let summary=format!("{} sections · Total sidewalk length {:.1} m",window.preview.pieces.len(),window.preview.actual);
        self.catalog_text(r,scene,&summary,(x+12.0*s,y+460.0*s),336.0*s,12.0*s,[142,142,142,255]);
        self.catalog_text(r,scene,&format!("Reference road: {:.1} m · Textures from sidewalk profile",window.preview.total),(x+12.0*s,y+478.0*s),336.0*s,11.0*s,MUTED);
        self.catalog_text(r,scene,"Green = start · Orange = end · Drag to adjust",(x+12.0*s,y+498.0*s),336.0*s,11.0*s,MUTED);
        self.catalog_text(r,scene,if window.existing.is_some(){"Existing sidewalk will be replaced when confirmed"}else{"New sidewalks · Road unchanged"},(x+12.0*s,y+514.0*s),336.0*s,11.0*s,MUTED);
        let message = if window.input.is_some() { "Enter a number · Enter applies · Esc cancels input" }
            else if window.picking!=0 {&window.message} else { window.error.as_deref().unwrap_or(&window.message) };
        for (i, line) in wrap(&self.text, message, 12.0 * s, 336.0 * s).iter().take(4).enumerate() {
            self.catalog_text(r, scene, line, (x + 12.0 * s, y + (650.0 + i as f32 * 15.0) * s), 336.0 * s, 12.0 * s, SOFT);
        }
        for (area, title, command, enabled, selected) in buttons {
            let color = if !enabled { PANEL_ALT } else if selected { SELECTED }
                else if editor_contains(area, cursor) { LIT } else { CHIP };
            self.editor_surface(r, scene, area, 3.0 * s, color);
            self.catalog_text(r, scene, &title, (area[0] + 6.0 * s, area[1] + 7.0 * s), area[2] - area[0] - 12.0 * s, 12.0 * s, WHITE);
            if enabled { window.rects.push((area, command)); }
        }
        self.text.flat = flat;
    }

    pub fn draw_object_text(&mut self, r: &Renderer, scene: &mut Scene,
        window: Option<&mut crate::object_text::Window>, scale: f32, viewport: [f32;4]) {
        use crate::object_text::Command;
        let Some(window) = window else { return; }; window.rects.clear();
        let s = scale.max(0.5).min((viewport[2] / 728.0).max(0.1)).min((viewport[3] / 572.0).max(0.1));
        let (x,y)=self.editor_window_origin("object-text",viewport,[704.0*s,548.0*s],[viewport[0]+(viewport[2]-704.0*s)*0.5,viewport[1]+(viewport[3]-548.0*s)*0.5],[704.0*s,48.0*s]);
        let rect = |a:f32,b:f32,w:f32,h:f32| [x+a*s,y+b*s,x+(a+w)*s,y+(b+h)*s];
        let flat = self.text.flat; self.text.flat = true;
        self.catalog_plate(r,scene,[0, 0, 0, 150],[viewport[0],viewport[1],viewport[0]+viewport[2],viewport[1]+viewport[3]]);
        self.editor_surface(r,scene,rect(0.0,0.0,704.0,548.0),6.0*s,PANEL);
        self.catalog_text(r,scene,"Edit object text",(x+24.0*s,y+18.0*s),550.0*s,23.0*s,WHITE);
        self.catalog_text(r,scene,&window.name,(x+24.0*s,y+53.0*s),650.0*s,14.0*s,MUTED);
        let page = window.page();
        for (row, index) in (page*6..window.fields.len().min(page*6+6)).enumerate() {
            let (slot,title) = &window.fields[index]; let top = 83.0 + row as f32*52.0;
            self.catalog_text(r,scene,title,(x+24.0*s,y+top*s),648.0*s,12.0*s,[142,142,142,255]);
            let area = rect(24.0,top+17.0,656.0,29.0); let active = window.active == index;
            self.editor_surface(r,scene,area,3.0*s,if active && window.replace {LIT} else if active {SELECTED} else {PANEL_ALT});
            let value = &window.values[*slot]; let text = if value.is_empty() { "(empty)" } else { value.as_str() };
            self.catalog_text(r,scene,text,(area[0]+9.0*s,area[1]+6.0*s),638.0*s,15.0*s,WHITE);
            window.rects.push((area,Command::Field(index)));
        }
        for (area,title,command,enabled) in [
            (rect(24.0,400.0,105.0,28.0),"Back",Command::Page(-1),page>0),
            (rect(575.0,400.0,105.0,28.0),"Next",Command::Page(1),(page+1)*6<window.fields.len()),
            (rect(24.0,501.0,190.0,30.0),"Undo text",Command::Undo,true),
            (rect(225.0,501.0,170.0,30.0),"Close (Esc)",Command::Close,true),
            (rect(407.0,501.0,273.0,30.0),"Apply (Enter)",Command::Apply,true),
        ] {
            self.editor_surface(r,scene,area,3.0*s,if enabled {CHIP} else {TRACK_OFF});
            self.catalog_text(r,scene,title,(area[0]+9.0*s,area[1]+6.0*s),area[2]-area[0]-18.0*s,13.0*s,WHITE);
            if enabled { window.rects.push((area,command)); }
        }
        self.catalog_text(r,scene,&format!("Page {} / {} · Tab: next field",page+1,window.fields.len().div_ceil(6)),
            (x+149.0*s,y+406.0*s),410.0*s,13.0*s,MUTED);
        for (row,line) in wrap(&self.text,&window.message,13.0*s,650.0*s).iter().take(3).enumerate() {
            self.catalog_text(r,scene,line,(x+24.0*s,y+(443.0+row as f32*16.0)*s),650.0*s,13.0*s,SOFT);
        }
        self.text.flat = flat;
    }

    pub fn draw_tile_window(&mut self, r: &Renderer, scene: &mut Scene,
        window: Option<&mut crate::tile_editor::Window>, scale: f32, viewport: [f32; 4], cursor: (f32, f32)) {
        use crate::tile_editor::{adjacent, Command};
        let Some(window) = window else { return; };
        window.rects.clear();
        let s = scale.max(0.5).min((viewport[2] / 800.0).max(0.1)).min((viewport[3] / 540.0).max(0.1));
        let (x,y)=self.editor_window_origin("tiles",viewport,[776.0*s,516.0*s],[viewport[0]+(viewport[2]-776.0*s)*0.5,viewport[1]+(viewport[3]-516.0*s)*0.5],[650.0*s,48.0*s]);
        let rect = |a: f32,b: f32,w: f32,h: f32| [x+a*s,y+b*s,x+(a+w)*s,y+(b+h)*s];
        let flat = self.text.flat; self.text.flat = true;
        self.catalog_plate(r,scene,[0, 0, 0, 150],[viewport[0],viewport[1],viewport[0]+viewport[2],viewport[1]+viewport[3]]);
        self.editor_surface(r,scene,rect(0.0,0.0,776.0,516.0),6.0*s,PANEL);
        self.catalog_text(r,scene,"Extend map",(x+24.0*s,y+18.0*s),510.0*s,23.0*s,WHITE);
        self.catalog_text(r,scene,&format!("View direction: {} · Grid: {:.0} × {:.0} m",window.bearing,
            omsi_map::tile_size(),omsi_map::tile_size()),(x+24.0*s,y+53.0*s),710.0*s,14.0*s,MUTED);
        self.catalog_text(r,scene,"North ↑  ·  West ←  ·  East →  ·  South ↓",(x+24.0*s,y+80.0*s),350.0*s,12.0*s,SOFT);
        for row in 0..5 { for col in 0..5 {
            let key = (window.center.0+col-2, window.center.1+2-row);
            let known = window.known.contains(&key);
            let selected = window.selected == Some(key);
            let available = !known && adjacent(&window.known,key);
            let cell = rect(24.0+col as f32*68.0,110.0+row as f32*58.0,64.0,54.0);
            let color = if selected { SELECTED } else if known { [39,100,70,255] }
                else if available { TRACK_OFF } else { PANEL_ALT };
            self.editor_surface(r,scene,cell,3.0*s,color);
            if key == window.camera {
                self.editor_surface(r,scene,[cell[0]+3.0*s,cell[1]+3.0*s,cell[2]-3.0*s,cell[1]+17.0*s],2.0*s,[142,142,142,255]);
                self.put(r,scene,"Camera",((10.0*s) as u32).max(1),[0, 0, 0, 150],cell[0]+7.0*s,cell[1]+10.0*s);
            }
            self.catalog_text(r,scene,&format!("{}, {}",key.0,key.1),(cell[0]+3.0*s,cell[1]+21.0*s),58.0*s,11.0*s,WHITE);
            self.catalog_text(r,scene,if known { "existing" } else if available { "free" } else { "—" },
                (cell[0]+3.0*s,cell[1]+37.0*s),58.0*s,10.0*s,SOFT);
            window.rects.push((cell,Command::Select(key)));
        } }
        let detail = window.selected.map(|k| format!("New tile ({}, {})",k.0,k.1)).unwrap_or_else(|| "Click a free neighbouring cell".into());
        self.catalog_text(r,scene,&detail,(x+390.0*s,y+118.0*s),360.0*s,18.0*s,WHITE);
        for (area,title,own) in [(rect(390.0,154.0,180.0,28.0),"Use edge heights",false),
            (rect(576.0,154.0,178.0,28.0),"Custom height",true)] {
            self.editor_surface(r,scene,area,3.0*s,if window.own_height==own {SELECTED} else {CHIP});
            self.put(r,scene,title,((12.0*s) as u32).max(1),WHITE,area[0]+8.0*s,(area[1]+area[3])*0.5);
            window.rects.push((area,Command::HeightMode(own)));
        }
        if window.own_height {
            let value=window.height_edit.as_ref().map(|v|format!("{v}│")).unwrap_or_else(||format!("Height: {:.2} m",window.height));
            for (area,title,command) in [(rect(390.0,192.0,45.0,30.0),"− 1".into(),Command::HeightAdjust(-1.0)),
                (rect(441.0,192.0,262.0,30.0),value,Command::EditHeight),
                (rect(709.0,192.0,45.0,30.0),"+ 1".into(),Command::HeightAdjust(1.0))] {
                self.editor_surface(r,scene,area,3.0*s,CHIP);
                self.catalog_text(r,scene,&title,(area[0]+7.0*s,area[1]+7.0*s),area[2]-area[0]-14.0*s,13.0*s,WHITE);
                window.rects.push((area,command));
            }
        }
        for (i,line) in [if window.own_height {"Flat terrain at custom height · Click number field"} else {"Terrain uses neighbouring edge heights"},
            if window.own_height {"May create height steps at map edges"} else {"Extend edge profile smoothly across the new tile"}].iter().enumerate() {
            self.catalog_text(r,scene,line,(x+390.0*s,y+(234.0+i as f32*22.0)*s),362.0*s,13.0*s,SOFT);
        }
        let lines = wrap(&self.text,&window.message,13.0*s,360.0*s);
        for (i,line) in lines.iter().take(6).enumerate() {
            self.catalog_text(r,scene,line,(x+390.0*s,y+(282.0+i as f32*20.0)*s),360.0*s,13.0*s,WHITE);
        }
        for (area,title,command,enabled) in [
            (rect(664.0,16.0,90.0,30.0),"Close",Command::Close,true),
            (rect(24.0,413.0,79.0,30.0),"← West",Command::Pan((-1,0)),true),
            (rect(109.0,413.0,79.0,30.0),"North ↑",Command::Pan((0,1)),true),
            (rect(194.0,413.0,79.0,30.0),"South ↓",Command::Pan((0,-1)),true),
            (rect(279.0,413.0,79.0,30.0),"East →",Command::Pan((1,0)),true),
            (rect(390.0,420.0,364.0,40.0),"Create tile",Command::Create,window.can_create()),
        ] {
            self.editor_surface(r,scene,area,3.0*s,if !enabled { PANEL_ALT }
                else if editor_contains(area,cursor) { LIT } else { CHIP });
            self.put(r,scene,title,((13.0*s) as u32).max(1),WHITE,area[0]+9.0*s,(area[1]+area[3])*0.5);
            if enabled { window.rects.push((area,command)); }
        }
        self.catalog_text(r,scene,"Green: existing · Blue: selected · Grey: free",(x+24.0*s,y+464.0*s),720.0*s,12.0*s,[142,142,142,255]);
        self.catalog_text(r,scene,"Arrow keys: map view · Enter: create · Esc: close",(x+24.0*s,y+487.0*s),720.0*s,12.0*s,[142,142,142,255]);
        self.text.flat = flat;
    }

    /// A separate native window inside the game HUD, with absolute-pixel hitboxes.
    pub fn draw_asset_catalog(&mut self, r: &Renderer, scene: &mut Scene, catalog: Option<&mut crate::asset_catalog::Catalog>, editor_visible: bool, scale: f32, viewport: [f32; 4], cursor: (f32, f32)) {
        use crate::asset_catalog::{Command,Section,Sort};
        let Some(catalog) = catalog.filter(|_| editor_visible) else {
            for (_, tex) in self.catalog_images.drain() { r.free_texture(scene, tex); }
            return;
        };
        self.catalog_open_rect = None;
        catalog.poll(); catalog.rects.clear();
        let visible = catalog.visible();
        let mut keep: hashbrown::HashSet<_> = visible.iter().copied().chain(catalog.selected).map(|i| catalog.image_key(i)).collect();
        let info=catalog.selected.and_then(|i|catalog.details.get(&i));
        let detail=info.and_then(|items|items.get(catalog.info_slot));
        let detail_key=format!("spline-info:{}:{}",catalog.selected.and_then(|i|catalog.entries.get(i)).map(|a|a.path.display().to_string()).unwrap_or_default(),catalog.info_slot);
        if catalog.info_open {if let Some(Ok(image))=detail.map(|d|&d.image) {
            keep.insert(detail_key.clone());
            if !self.catalog_images.contains_key(&detail_key) {self.catalog_images.insert(detail_key.clone(),r.add_texture(scene,image,false));}
        }}
        let expired: Vec<_> = self.catalog_images.keys().filter(|key| !keep.contains(*key)).cloned().collect();
        for key in expired { if let Some(tex) = self.catalog_images.remove(&key) { r.free_texture(scene, tex); } }
        for &index in &visible.iter().copied().chain(catalog.selected).collect::<Vec<_>>() {
            if let Some(Ok(image)) = catalog.previews.get(&index) {
                let key = catalog.image_key(index);
                if !self.catalog_images.contains_key(&key) { self.catalog_images.insert(key, r.add_texture(scene, image, false)); }
            }
        }
        let s = scale.max(0.5).min((viewport[2] / 1024.0).max(0.1)).min((viewport[3] / 744.0).max(0.1));
        let (x,y)=self.editor_window_origin("catalog",viewport,[1000.0*s,720.0*s],[viewport[0]+(viewport[2]-1000.0*s)*0.5,viewport[1]+(viewport[3]-720.0*s)*0.5],[680.0*s,48.0*s]);
        let rect = |a: f32, b: f32, w: f32, h: f32| [x + a * s, y + b * s, x + (a + w) * s, y + (b + h) * s];
        let dim = self.text.solid(r, scene, [0, 0, 0, 150]);
        scene.overlays.push((dim, [viewport[0], viewport[1], viewport[0] + viewport[2], viewport[1] + viewport[3]]));
        self.catalog_plate(r, scene, PANEL, rect(0.0, 0.0, 1000.0, 720.0));
        if catalog.info_open {
            self.catalog_text(r,scene,"Spline textures",(x+24.0*s,y+20.0*s),740.0*s,24.0*s,WHITE);
            let file=catalog.chosen().map(|a|a.path.display().to_string()).unwrap_or_default();
            let mut lines=vec![file];
            if let Some(d)=detail {
                lines.extend([format!("Texture {} / {}: {}",catalog.info_slot+1,info.map_or(0,Vec::len),d.name),d.usage.clone(),d.path.clone()]);
                if let Err(e)=&d.image {lines.push(e.clone());}
                if let (Some(&tex),Ok(image))=(self.catalog_images.get(&detail_key),&d.image) {
                    let k=360.0/(image.width.max(image.height) as f32).max(1.0);
                    scene.overlays.push((tex,rect(28.0,210.0,image.width as f32*k,image.height as f32*k)));
                }
            } else {lines.push(if info.is_some() {"No textures in profile"} else {"Loading texture information …"}.into());}
            // Hard wrapping also keeps long paths without spaces fully readable.
            let lines:Vec<String>=lines.into_iter().flat_map(|l|l.chars().collect::<Vec<_>>().chunks(66)
                .map(|c|c.iter().collect::<String>()).collect::<Vec<_>>()).collect();
            for (i,line) in lines.iter().enumerate() {
                self.catalog_text(r,scene,line,(x+410.0*s,y+(90.0+i as f32*20.0)*s),564.0*s,12.0*s,WHITE);
            }
            for (area,label,command) in [(rect(28.0,630.0,170.0,36.0),"Previous texture",Command::InfoStep(-1)),
                (rect(218.0,630.0,170.0,36.0),"Next texture",Command::InfoStep(1)),
                (rect(790.0,630.0,180.0,36.0),"Back to catalogue",Command::TextureInfo)] {
                self.catalog_plate(r,scene,CHIP,area);
                self.catalog_text(r,scene,label,(area[0]+8.0*s,area[1]+8.0*s),164.0*s,14.0*s,WHITE);
                catalog.rects.push((area,command));
            }
            return;
        }
        self.catalog_text(r, scene, if catalog.texture_only {"Texture catalogue"} else {"Asset catalogue"}, (x + 22.0 * s, y + 16.0 * s), 650.0 * s, 25.0 * s, WHITE);
        for (section, title, left, width) in [(Section::Roads, "Splines & junctions", 22.0, 220.0), (Section::Objects, "Objects", 254.0, 104.0),(Section::Textures,"Textures",370.0,104.0)] {
            if catalog.texture_only && section!=Section::Textures {continue;}
            let box_rect = rect(left, 61.0, width, 36.0);
            let color = if catalog.section == section { SELECTED } else { TRACK_OFF };
            self.catalog_plate(r, scene, color, box_rect);
            self.catalog_text(r, scene, title, (box_rect[0] + 12.0 * s, box_rect[1] + 7.0 * s), box_rect[2] - box_rect[0] - 20.0 * s, 17.0 * s, WHITE);
            catalog.rects.push((box_rect, Command::Section(section)));
        }
        let categories = catalog.section.categories();
        let category_step = 964.0 / categories.len() as f32;
        for (i,&category) in categories.iter().enumerate() {
            let area=rect(22.0+i as f32*category_step,111.0,category_step-8.0,28.0);
            self.catalog_plate(r,scene,if catalog.category==category {SELECTED} else {TRACK_OFF},area);
            self.catalog_text(r,scene,category.title(),(area[0]+7.0*s,area[1]+6.0*s),(category_step-22.0)*s,13.0*s,WHITE);
            catalog.rects.push((area,Command::Category(category)));
        }
        for (i,sort) in [Sort::Name,Sort::Category,Sort::Path].into_iter().enumerate() {
            let area=rect(486.0+i as f32*124.0,61.0,116.0,36.0);
            self.catalog_plate(r,scene,if catalog.sort==sort {SELECTED} else {TRACK_OFF},area);
            self.catalog_text(r,scene,&format!("Sort: {}",sort.title()),(area[0]+7.0*s,area[1]+8.0*s),102.0*s,13.0*s,WHITE);
            catalog.rects.push((area,Command::Sort(sort)));
        }
        let reverse=rect(862.0,61.0,116.0,36.0);
        self.catalog_plate(r,scene,CHIP,reverse);
        self.catalog_text(r,scene,if catalog.descending {"Z → A ↓"} else {"A → Z ↑"},(reverse[0]+9.0*s,reverse[1]+8.0*s),98.0*s,14.0*s,WHITE);
        catalog.rects.push((reverse,Command::Reverse));
        let search = rect(22.0, 151.0, 630.0, 36.0);
        self.catalog_plate(r, scene, PANEL_ALT, search);
        let query = if catalog.query.is_empty() { "Search: type name or path …".into() } else { format!("Search: {}", catalog.query) };
        self.catalog_text(r, scene, &query, (search[0] + 12.0 * s, search[1] + 6.0 * s), 606.0 * s, 17.0 * s, WHITE);
        let mut buttons = vec![(rect(900.0, 16.0, 78.0, 34.0), "Close", Command::Close, true),
            (rect(696.0,16.0,188.0,34.0),"Build junction",Command::BuildJunction,!catalog.texture_only),
            (rect(666.0, 151.0, 106.0, 36.0), "Clear", Command::Clear, true),
            (rect(700.0, 415.0, 85.0, 30.0), "← 90°", Command::Rotate(-1), catalog.chosen().is_some_and(|a|a.kind!=crate::asset_catalog::Kind::Texture)),
            (rect(793.0, 415.0, 90.0, 30.0), "Automatic", Command::ResetView, catalog.chosen().is_some_and(|a|a.kind!=crate::asset_catalog::Kind::Texture)),
            (rect(790.0, 151.0, 188.0, 36.0), if catalog.audit_running() { "Stop check" } else { "Check all previews" }, Command::Audit, !catalog.scanning),
            (rect(891.0, 415.0, 87.0, 30.0), "90° →", Command::Rotate(1), catalog.chosen().is_some_and(|a|a.kind!=crate::asset_catalog::Kind::Texture)),
            (rect(700.0, 586.0, 278.0, 44.0), "Select asset", Command::Choose, catalog.selected.is_some()),
            (rect(22.0, 672.0, 110.0, 30.0), "Back", Command::Page(-1), catalog.page > 0),
            (rect(542.0, 672.0, 110.0, 30.0), "Next", Command::Page(1), catalog.page + 1 < catalog.pages())];
        if catalog.chosen().is_some_and(|a|a.kind==crate::asset_catalog::Kind::Spline) {
            buttons.push((rect(700.0,632.0,278.0,28.0),"Textures / info …",Command::TextureInfo,true));
        }
        for (cell, &index) in visible.iter().enumerate() {
            let left = 22.0 + (cell % 4) as f32 * 160.0; let top = 204.0 + (cell / 4) as f32 * 152.0;
            let tile = rect(left, top, 150.0, 140.0);
            let selected = catalog.selected == Some(index);
            let color = if selected { SELECTED } else { PANEL_ALT };
            self.catalog_plate(r, scene, color, tile);
            let asset = &catalog.entries[index];
            if let Some(&tex) = self.catalog_images.get(&catalog.image_key(index)) { scene.overlays.push((tex, rect(left + 4.0, top + 4.0, 142.0, 94.67))); }
            else {
                let status = match catalog.previews.get(&index) { Some(Err(_)) => "No preview", _ => "Loading preview …" };
                self.catalog_text(r, scene, status, (tile[0] + 8.0 * s, tile[1] + 42.0 * s), 134.0 * s, 13.0 * s, MUTED);
            }
            self.catalog_text(r, scene, &asset.name, (tile[0] + 7.0 * s, tile[1] + 103.0 * s), 136.0 * s, 14.0 * s, WHITE);
            catalog.rects.push((tile, Command::Select(index)));
        }
        if let Some(index) = catalog.selected {
            let asset = &catalog.entries[index];
            self.catalog_text(r, scene, asset.category.title(), (x + 700.0 * s, y + 204.0 * s), 278.0 * s, 18.0 * s, MUTED);
            if let Some(&tex) = self.catalog_images.get(&catalog.image_key(index)) { scene.overlays.push((tex, rect(711.0, 238.0, 256.0, 170.67))); }
            self.catalog_text(r, scene, &asset.name, (x + 700.0 * s, y + 453.0 * s), 278.0 * s, 18.0 * s, WHITE);
            for (n, line) in wrap(&self.text, &asset.file, 13.0 * s, 278.0 * s).iter().take(3).enumerate() {
                self.catalog_text(r, scene, line, (x + 700.0 * s, y + (481.0 + n as f32 * 18.0) * s), 278.0 * s, 13.0 * s, MUTED);
            }
            let status = match catalog.previews.get(&index) { Some(Err(error)) => error.as_str(),
                Some(Ok(_)) if asset.category == crate::asset_catalog::Category::Traffic => "Path preview · No road model\nBlue: road · Green: footpath\nPurple: railway · Yellow: flight path",
                Some(Ok(_)) if asset.kind==crate::asset_catalog::Kind::Texture=>"Texture preview · Checkerboard = transparency",
                Some(Ok(_)) => "Static preview from model", None => "Preview is loading …" };
            let status = omsi_ui::tr(status);
            let status_lines: Vec<_> = status.lines().flat_map(|line| wrap(&self.text, line, 13.0 * s, 278.0 * s)).collect();
            for (row, line) in status_lines.iter().take(3).enumerate() {
                self.catalog_text(r, scene, line, (x + 700.0 * s, y + (539.0 + row as f32 * 14.0) * s), 278.0 * s, 13.0 * s, MUTED);
            }
        }
        if visible.is_empty() {
            self.catalog_text(r, scene, if catalog.scanning { "Searching installed assets …" } else { "No matching assets found" }, (x + 30.0 * s, y + 270.0 * s), 620.0 * s, 19.0 * s, WHITE);
        }
        let footer = format!("{} results · {} · Page {} / {}",catalog.filtered.len(),catalog.category.title(),catalog.page+1,catalog.pages());
        self.catalog_text(r, scene, &footer, (x + 153.0 * s, y + 678.0 * s), 375.0 * s, 15.0 * s, SOFT);
        self.catalog_text(r, scene, if catalog.audit_message.is_empty() { "Enter selects · Esc closes" } else { &catalog.audit_message },
            (x + 22.0 * s, y + 648.0 * s), 650.0 * s, 13.0 * s, MUTED);
        for (box_rect, text, command, enabled) in buttons.drain(..) {
            let hovered = cursor.0 >= box_rect[0] && cursor.0 <= box_rect[2] && cursor.1 >= box_rect[1] && cursor.1 <= box_rect[3];
            let color = if !enabled { PANEL_ALT } else if hovered { LIT } else { CHIP };
            self.catalog_plate(r, scene, color, box_rect);
            self.catalog_text(r, scene, text, (box_rect[0] + 10.0 * s, box_rect[1] + 6.0 * s), box_rect[2] - box_rect[0] - 18.0 * s, 15.0 * s, WHITE);
            if enabled { catalog.rects.push((box_rect, command)); }
        }
    }

    pub fn draw_junction_window(&mut self,r:&Renderer,scene:&mut Scene,window:Option<&mut crate::junction_builder::Window>,
        root:&std::path::Path,scale:f32,viewport:[f32;4],cursor:(f32,f32)) {
        use crate::junction_builder::{Command,Field};
        let Some(window)=window else {if let Some((_,tex))=self.junction_image.take() {r.free_texture(scene,tex);}return;};
        window.refresh(root);window.rects.clear();
        if self.junction_image.as_ref().is_some_and(|(id,_)|*id!=window.preview_id) {
            if let Some((_,tex))=self.junction_image.take() {r.free_texture(scene,tex);}
        }
        if self.junction_image.is_none() {if let Some(img)=&window.preview {self.junction_image=Some((window.preview_id,r.add_texture(scene,img,false)));}}
        let s=scale.max(0.5).min((viewport[2]/1044.0).max(0.1)).min((viewport[3]/764.0).max(0.1));
        let kind=if window.project.roundabout.is_some(){"roundabout"}else{"junction"};
        let (x,y)=self.editor_window_origin(kind,viewport,[1020.0*s,740.0*s],[viewport[0]+(viewport[2]-1020.0*s)*0.5,viewport[1]+(viewport[3]-740.0*s)*0.5],[896.0*s,52.0*s]);
        let rect=|a:f32,b:f32,w:f32,h:f32|[x+a*s,y+b*s,x+(a+w)*s,y+(b+h)*s];
        let flat=self.text.flat;self.text.flat=true;
        self.catalog_plate(r,scene,[0, 0, 0, 150],[viewport[0],viewport[1],viewport[0]+viewport[2],viewport[1]+viewport[3]]);
        self.editor_surface(r,scene,rect(0.0,0.0,1020.0,740.0),7.0*s,PANEL);
        let roundabout=window.project.roundabout.is_some();
        self.catalog_text(r,scene,if roundabout {"Roundabout builder"} else {"Junction builder"},(x+22.0*s,y+18.0*s),680.0*s,24.0*s,WHITE);
        self.catalog_text(r,scene,&window.source_label,(x+22.0*s,y+46.0*s),970.0*s,10.0*s,SOFT);
        self.editor_surface(r,scene,rect(20.0,62.0,358.0,576.0),5.0*s,PANEL);
        let cross=window.project.arms[3].enabled;
        let mut buttons=vec![(rect(908.0,16.0,90.0,32.0),"Close".to_string(),Command::Close,true,false),
            (rect(32.0,78.0,150.0,30.0),if roundabout {"3 entrances"}else{"T-junction"}.into(),Command::Shape(false),true,!cross),
            (rect(194.0,78.0,164.0,30.0),if roundabout {"4 entrances"}else{"Four-way junction"}.into(),Command::Shape(true),true,cross)];
        for (i,label) in ["Arm A","Arm B","Arm C","Arm D"].into_iter().enumerate() {
            buttons.push((rect(32.0+i as f32*83.0,120.0,77.0,30.0),label.into(),Command::Arm(i),window.project.arms[i].enabled,window.arm==i));
            let (status,color)=if window.pending.is_some_and(|(_,arm)|arm==i){("prepared".to_string(),[60,180,255,0])}else if let Some(id)=window.arm_links[i]{(format!("ID {id}"),[75,230,110,0])}else{("free".to_string(),MUTED)};
            self.catalog_text(r,scene,&status,(x+(32.0+i as f32*83.0)*s,y+153.0*s),77.0*s,8.0*s,color);
        }
        let name=window.input.as_ref().filter(|i|i.field==Field::Name).map(|i|format!("{}│",i.text)).unwrap_or_else(||window.project.name.clone());
        buttons.push((rect(32.0,166.0,326.0,32.0),name,Command::Edit(Field::Name),true,window.input.as_ref().is_some_and(|i|i.field==Field::Name)));
        let fields=if roundabout {[Field::Width,Field::Length,Field::Angle,Field::IslandRadius,Field::RingWidth,Field::IslandHeight,Field::TextureMetres]}
            else {[Field::Width,Field::Length,Field::Angle,Field::Bend,Field::Sidewalk,Field::Corner,Field::TextureMetres]};
        for (i,field) in fields.into_iter().enumerate() {
            let row=208.0+i as f32*36.0;
            self.catalog_text(r,scene,field.title(),(x+32.0*s,y+(row+8.0)*s),168.0*s,12.0*s,SOFT);
            let step=match field {Field::Angle|Field::Bend=>5.0,Field::IslandHeight=>0.05,_=>0.5};
            buttons.push((rect(208.0,row,26.0,30.0),"−".into(),Command::Adjust(field,-step),true,false));
            buttons.push((rect(332.0,row,26.0,30.0),"+".into(),Command::Adjust(field,step),true,false));
            let input=window.input.as_ref().filter(|input|input.field==field);
            let text=input.map(|i|format!("{}│",i.text)).unwrap_or_else(||format!("{:.2}",window.value(field)));
            buttons.push((rect(240.0,row,86.0,30.0),text,Command::Edit(field),true,input.is_some()));
        }
        buttons.extend([
            (rect(32.0,468.0,326.0,30.0),"Width / texture from selected spline".into(),Command::UseSpline,true,false),
            (rect(32.0,511.0,158.0,30.0),"Choose asphalt …".into(),Command::RoadTexture,true,!window.project.road_texture.is_empty()),
            (rect(200.0,511.0,158.0,30.0),if roundabout {"Choose island texture …"}else{"Choose sidewalk …"}.into(),Command::WalkTexture,true,!window.project.walk_texture.is_empty()),
            (rect(32.0,551.0,326.0,28.0),if roundabout {if window.project.markings {"Island edge line: ON"}else{"Island edge line: OFF"}}else if window.project.markings {"Centre lines: ON"} else {"Centre lines: OFF"}.into(),Command::Markings,true,window.project.markings),
            (rect(32.0,591.0,158.0,30.0),"Undo".into(),Command::Undo,!window.undo.is_empty(),false),
            (rect(200.0,591.0,158.0,30.0),"Redo".into(),Command::Redo,!window.redo.is_empty(),false),
            (rect(410.0,551.0,184.0,32.0),"Show AI paths".into(),Command::ShowPaths,true,window.show_paths),
            (rect(602.0,551.0,190.0,32.0),"Rotate preview".into(),Command::Rotate,true,false),
            (rect(800.0,551.0,184.0,32.0),"Load project …".into(),Command::Load,true,false),
            (rect(726.0,680.0,272.0,42.0),if window.target.is_some(){if roundabout {"Update placed roundabout"}else{"Update placed junction"}}else{"Save & place"}.into(),Command::Export,window.error.is_none(),false),
        ]);
        if window.target.is_some(){buttons.push((rect(410.0,680.0,302.0,42.0),"Connect road to selected arm".into(),Command::ConnectRoad,window.error.is_none(),window.pending.is_some()));}
        self.editor_surface(r,scene,rect(402.0,76.0,596.0,464.0),5.0*s,PANEL_ALT);
        self.catalog_text(r,scene,"3D preview · Dimensions in metres",(x+416.0*s,y+86.0*s),556.0*s,17.0*s,SOFT);
        let texture=if window.project.road_texture.is_empty() {"No road surface texture"} else {&window.project.road_texture};
        self.catalog_text(r,scene,texture,(x+416.0*s,y+108.0*s),556.0*s,10.0*s,SOFT);
        if let Some((_,tex))=self.junction_image {
            scene.overlays.push((tex,rect(408.0,124.0,584.0,389.33)));
            for &(i,p) in &window.arm_labels {
                let left=408.0+584.0*p.x-17.0;let top=124.0+389.33*p.y-15.0;
                buttons.push((rect(left,top,34.0,30.0),['A','B','C','D'][i].to_string(),Command::Arm(i),true,window.arm==i));
            }
            self.catalog_text(r,scene,&format!("Arm {}",['A','B','C','D'][window.arm]),(x+416.0*s,y+516.0*s),200.0*s,13.0*s,[255,192,55,0]);
        }
        else if let Some(error)=&window.error {
            for (i,line) in wrap(&self.text,error,15.0*s,530.0*s).iter().take(5).enumerate() {
                self.catalog_text(r,scene,line,(x+424.0*s,y+(236.0+i as f32*23.0)*s),530.0*s,15.0*s,[255,203,173,0]);
            }
        }
        let mut digit_hits=Vec::new();
        for (i,field) in [Field::UStart,Field::UEnd,Field::TextureWidth].into_iter().enumerate() {
            let left=414.0+i as f32*194.0;
            self.catalog_text(r,scene,field.title(),(x+left*s,y+592.0*s),180.0*s,11.0*s,SOFT);
            let input=window.input.as_ref().filter(|p|p.field==field);
            let text=input.map(|p|p.text.clone()).unwrap_or_else(||window.number_text(field));
            buttons.push((rect(left,607.0,24.0,28.0),"−".into(),Command::Step(field,-1.0),true,false));
            buttons.push((rect(left+154.0,607.0,24.0,28.0),"+".into(),Command::Step(field,1.0),true,false));
            buttons.push((rect(left+28.0,607.0,122.0,28.0),text.clone(),Command::Edit(field),true,input.is_some()));
            // Hit boxes use the same font size as the value, including small viewports.
            let font_size=((13.0*s) as u32).max(1) as f32;
            let decimal=text.find(['.',',']).unwrap_or(text.len());
            for (offset,ch) in text.char_indices().filter(|(_,c)|c.is_ascii_digit()) {
                let power=if offset<decimal {(decimal-offset-1) as i32} else {decimal as i32-offset as i32};
                let start=self.text.width(&text[..offset],font_size);
                let width=self.text.width(&ch.to_string(),font_size);
                if start+width>108.0*s {break;}
                let area=[x+(left+35.0)*s+start,y+610.0*s,x+(left+35.0)*s+start+width,y+632.0*s];
                digit_hits.push((area,Command::Digit(field,power),window.texture_focus==Some(field)&&window.texture_steps[i]==power));
            }
            let step=if input.is_some_and(|p|!p.replace) {"Step after input / Enter".to_string()}
                else {format!("Step: {} · Click a digit",window.texture_step(field))};
            self.catalog_text(r,scene,&step,(x+left*s,y+637.0*s),180.0*s,10.0*s,AMBER);
            let help=match field {
                Field::UStart=>["Start of the texture crop.","U: 0 to 1 = one image width."],
                Field::UEnd=>["End of the texture crop.","Less than start = mirrored."],
                _=>["Width of the crop in metres.","Larger = displayed wider."],
            };
            for (line,help) in help.into_iter().enumerate() {
                self.catalog_text(r,scene,help,(x+left*s,y+(650.0+line as f32*12.0)*s),180.0*s,10.0*s,SOFT);
            }
        }
        let message=if window.input.is_some() {"Enter value · Enter applies · Esc cancels input"} else {&window.message};
        for (i,line) in wrap(&self.text,message,11.0*s,374.0*s).iter().take(6).enumerate() {
            self.catalog_text(r,scene,line,(x+22.0*s,y+(648.0+i as f32*13.0)*s),374.0*s,11.0*s,SOFT);
        }
        for (area,title,command,enabled,selected) in buttons {
            let primary = matches!(&command, Command::Export);
            self.editor_surface(r,scene,area,3.0*s,if !enabled {PANEL_ALT}
                else if primary {if editor_contains(area,cursor) {ACCENT_HOT} else {ACCENT}}
                else if selected {SELECTED} else if editor_contains(area,cursor) {LIT} else {CHIP});
            self.catalog_text(r,scene,&title,(area[0]+7.0*s,area[1]+7.0*s),area[2]-area[0]-14.0*s,13.0*s,if !enabled {OFF_INK} else if primary {ON_ACCENT} else {WHITE});
            if enabled {window.rects.push((area,command));}
        }
        // Digits take priority over the enclosing editable field.
        for (area,command,selected) in digit_hits {
            if selected {self.catalog_plate(r,scene,ACCENT,[area[0],area[3]-2.0*s,area[2],area[3]]);}
            window.rects.push((area,command));
        }
        self.text.flat=flat;
    }

    fn draw_spline_connection(&mut self, r: &Renderer, scene: &mut Scene,
        status: Option<&(String, bool)>, replace: bool, transition: bool, s: f32, origin: [f32; 2], cursor: (f32, f32)) {
        let Some((message, ready)) = status else { return; };
        let [x, y] = origin;
        let width = 284.0 * s;
        let panel = [x, y, x + width, y + 198.0 * s];
        self.editor_connection_rect = Some(panel);
        self.editor_surface(r, scene, panel, 3.0 * s, PANEL);
        self.catalog_text(r, scene, "Connect splines", (x + 8.0 * s, y + 5.0 * s),
            width - 16.0 * s, 13.0 * s, MUTED);
        let message = omsi_ui::tr(message);
        let lines: Vec<String> = message.lines().flat_map(|line|
            wrap(&self.text, line, 11.0 * s, width - 16.0 * s)).collect();
        for (row, line) in lines.iter().take(7).enumerate() {
            self.catalog_text(r, scene, line, (x + 8.0 * s, y + (25.0 + row as f32 * 14.0) * s),
                width - 16.0 * s, 11.0 * s, WHITE);
        }
        let toggle = [x + 8.0 * s, y + 130.0 * s, x + width - 8.0 * s, y + 156.0 * s];
        self.editor_surface(r, scene, toggle, 3.0 * s, if transition { [35, 115, 100, 255] } else { TRACK_OFF });
        self.put(r, scene, if transition { "[x] Profile / curve transition (openOMSI)" } else { "[ ] Profile / curve transition (openOMSI)" },
            ((11.0 * s) as u32).max(1), WHITE, toggle[0] + 6.0 * s, y + 143.0 * s);
        self.spline_transition_rect = Some(toggle);
        let button_y = y + 164.0 * s;
        let button_w = (width - 24.0 * s) * 0.5;
        for (index, title) in [if replace { "Replace" } else { "Connect" }, "Cancel"].into_iter().enumerate() {
            let bx = x + 8.0 * s + index as f32 * (button_w + 8.0 * s);
            let rect = [bx, button_y, bx + button_w, button_y + 26.0 * s];
            let hovered = editor_contains(rect, cursor);
            let color = if index == 0 && !ready && !replace { TRACK_OFF }
                else if index == 0 && replace { if hovered { [175, 105, 35, 255] } else { [140, 85, 30, 255] } }
                else if hovered { LIT } else { CHIP };
            self.editor_surface(r, scene, rect, 3.0 * s, color);
            self.put(r, scene, title, ((12.0 * s) as u32).max(1), WHITE, bx + 8.0 * s, button_y + 13.0 * s);
            if index == 0 { self.spline_connect_rect = Some(rect); }
            else { self.spline_cancel_rect = Some(rect); }
        }
    }


    /// Draw the frame's interface: its overlays go after the HUD's in `scene.overlays`.
    pub fn draw(&mut self, r: &Renderer, scene: &mut Scene, f: &Frame, dt: f32) {
        let s = f.scale.max(0.5) * f.ui_scale;
        self.text.backdrop = f.opacity;
        // --- name tags above the other players' buses
        for ((x, y), name, sub, alpha) in &f.tags {
            let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
            let l = self.text.label(r, scene, name, (19.0 * s) as u32, [255, 255, 255, 220]);
            let x0 = x - l.w as f32 * 0.5;
            let y0 = y - l.h as f32;
            if a > 0 {
                scene.overlays.push((l.tex, [x0, y0, x0 + l.w as f32, y0 + l.h as f32]));
                if !sub.is_empty() {
                    let m = self.text.label(r, scene, sub, (12.0 * s) as u32, [210, 225, 255, 200]);
                    let mx = x - m.w as f32 * 0.5;
                    scene.overlays.push((m.tex, [mx, y0 + l.h as f32 - 3.0 * s, mx + m.w as f32, y0 + l.h as f32 - 3.0 * s + m.h as f32]));
                }
            }
        }
        // the top of the windows in the corners: the notes on the left start on the line of
        // the timetable and a tutorial page on the right (14 px down, they stood higher)
        let corner_top = (60.0 * s).max(self.editor_notes_top);
        // --- the notes, top left: only the editors' keys while one is on (what the driver
        // has to do comes as a notification): one flat card, the lines in it
        let notes_bottom = {
            let px = (13.0 * s).round() as u32;
            let mut x0 = 16.0 * s;
            let mut room = f.width * 0.6;
            // (below the on-screen buttons of a phone, which keep their size when the
            // interface is made smaller)
            let mut y0 = if crate::platform::touch_controls() { (80.0 * f.scale.max(0.5) * f.ui_scale.max(1.0)).max(corner_top) } else { corner_top };
            if let Some(dock) = self.editor_dock_rect.filter(|rect| y0 + 24.0 * s >= rect[1] && y0 < rect[3]) {
                let right = dock[2] - self.origin_x + 12.0 * s;
                let left = dock[0] - self.origin_x - 28.0 * s;
                if f.width - right >= 180.0 * s { x0 = right; room = room.min(f.width - right - 24.0 * s); }
                else if left >= 180.0 * s { room = room.min(left); }
                else { y0 = dock[3] + 12.0 * s; }
            }
            let lines: Vec<String> = f.notes.iter().filter(|n| !n.trim().is_empty()).take(8).map(|n| clip_to(&self.text, &omsi_ui::tr(n), px as f32, room)).collect();
            if lines.is_empty() {
                y0
            } else {
                let lh = (px as f32 * 1.5).round();
                let (pad_x, pad_y) = (12.0 * s, 8.0 * s);
                let w = lines.iter().map(|t| self.text.width_raw(t, px as f32)).fold(0.0f32, f32::max) + pad_x * 2.0;
                let h = pad_y * 2.0 + lh * lines.len() as f32;
                self.text.rounded(r, scene, [x0, y0, x0 + w, y0 + h], 8.0 * s, [28, 28, 28, (235.0 * self.text.backdrop.min(1.0)) as u8]);
                for (i, t) in lines.iter().enumerate() {
                    let l = self.text.label_op(r, scene, t, px, [184, 184, 184, 0], 1.0);
                    let ly = (y0 + pad_y + lh * (i as f32 + 0.5) - l.h as f32 * 0.5).round();
                    scene.overlays.push((l.tex, [x0 + pad_x, ly, x0 + pad_x + l.w as f32, ly + l.h as f32]));
                }
                y0 + h
            }
        };
        self.notes_bottom = notes_bottom;
        // --- the chat, top left under the notes, as Roblox has it (from the fourth note
        // on they ran into it)
        if let Some(c) = f.chat.as_ref().filter(|_| !self.chat.hidden) {
            // (its own size on top of the interface's: Ctrl + the wheel over it)
            let s = s * f.chat_size.clamp(0.5, 3.0);
            let px = (17.0 * s) as u32;
            let lh = px as f32 * 1.35;
            let x0 = 14.0 * s;
            let y0 = (96.0 * s).max(notes_bottom + 10.0 * s);
            let width = (460.0 * s).min(f.width * 0.6);
            let open = c.typing.is_some();
            let n = c.lines.len();
            let end = n.saturating_sub(if open || self.chat.hovered { self.chat.scroll } else { 0 });
            let box_h = lh * CHAT_SHOWN as f32 + lh * 1.6;
            self.chat.rect = [x0 - 6.0 * s, y0 - 6.0 * s, x0 + width, y0 + box_h];
            self.chat.hovered = self.chat.contains(f.cursor.0, f.cursor.1);
            let show_box = open || self.chat.hovered;
            // the lines, newest at the bottom of the history area; a line too long for the box
            // goes on in rows under it, indented (the oldest line shown may lose its first rows)
            let indent = 14.0 * s;
            let mut rows: Vec<(f32, String, [u8; 4])> = Vec::new();
            {
                let tc = &self.text;
                let measure = |t: &str| tc.width_raw(t, px as f32);
                let mut i = end;
                while i > 0 && rows.len() < CHAT_SHOWN {
                    i -= 1;
                    let line = &c.lines[i];
                    let color = if line.starts_with("* ") { [255, 226, 140, 230] } else { [255, 255, 255, 230] };
                    for (k, t) in wrap_rows(line, width, indent, &measure).into_iter().enumerate().rev() {
                        if rows.len() == CHAT_SHOWN {
                            break;
                        }
                        rows.push((if k == 0 { 0.0 } else { indent }, t, color));
                    }
                }
                rows.reverse();
            }
            let mut y = y0 + lh * (CHAT_SHOWN - rows.len()) as f32;
            for (dx, text, color) in rows {
                let l = self.text.label(r, scene, &text, px, color);
                scene.overlays.push((l.tex, [x0 + dx, y, x0 + dx + l.w as f32, y + l.h as f32]));
                y += lh;
            }
            if show_box {
                // the input box: shown when the cursor is over the chat or it is typed into
                let by = y0 + lh * CHAT_SHOWN as f32 + lh * 0.2;
                let bh = lh * 1.25;
                let plate = self.text.plate(r, scene, 0);
                scene.overlays.push((plate, [x0 - 4.0 * s, by, x0 + width, by + bh]));
                self.chat.caret_t += dt;
                let caret = if open && (self.chat.caret_t % 1.0) < 0.55 { "|" } else { "" };
                let (text, color) = match c.typing {
                    Some(t) => (format!("{t}{caret}"), [255, 255, 255, 240]),
                    None => ("Click here or press / to chat".to_string(), [190, 190, 190, 200]),
                };
                let text = clip_left(&self.text, &text, px as f32, width - 10.0 * s);
                let l = self.text.label(r, scene, &text, px, color);
                let ty = by + (bh - l.h as f32) * 0.5;
                scene.overlays.push((l.tex, [x0 + 2.0 * s, ty, x0 + 2.0 * s + l.w as f32, ty + l.h as f32]));
                if self.chat.scroll > 0 && (open || self.chat.hovered) {
                    // (the words translated apart from the number: whole, the text was no key)
                    let m = self.text.label(r, scene, &format!("{} {}", self.chat.scroll, omsi_ui::tr("newer below")), (11.0 * s) as u32, [200, 200, 200, 200]);
                    scene.overlays.push((m.tex, [x0 + width - m.w as f32, by - m.h as f32, x0 + width, by]));
                }
            }
            if let Some(e) = c.error {
                let l = self.text.label(r, scene, &format!("{}: {}", omsi_ui::tr("Not sent"), omsi_ui::tr(e)), (12.0 * s) as u32, [255, 150, 150, 220]);
                let ey = y0 + box_h;
                scene.overlays.push((l.tex, [x0, ey, x0 + l.w as f32, ey + l.h as f32]));
            }
        } else {
            self.chat.hovered = false;
            self.chat.rect = [0.0; 4];
        }
        // --- the notifications: cards over the navigator (under it when it is in a top
        // corner; at the top middle without it)
        self.draw_notices(r, scene, f, s);
        if let Some(fps) = f.fps {
            // (on a small flat card, as the other plates: over a bright sky bare text was hard to read)
            let l = self.text.label_op(r, scene, &format!("{fps:.0} fps"), (12.0 * s).round() as u32, [184, 184, 184, 0], 1.0);
            let x = (f.width - l.w as f32 - 14.0 * s).round();
            let y = (10.0 * s).round().max(self.editor_notes_top);
            self.text.rounded(r, scene, [x - 6.0 * s, y, x + l.w as f32 + 6.0 * s, y + l.h as f32], 6.0 * s, [28, 28, 28, (220.0 * self.text.backdrop.min(1.0)) as u8]);
            scene.overlays.push((l.tex, [x, y, x + l.w as f32, y + l.h as f32]));
        }
        self.draw_perf(r, scene, f, s, dt);
        // --- the information bar, along the top in the middle: in as many rows as the room
        // it has needs
        self.info_rect = None;
        // (not under the pause menu's rail, from under which it stood out)
        let rail = f.menu.is_some() && f.menu_kind == MenuKind::Game && f.menu_head.is_none() && !f.vr;
        if let Some(info) = f.info.as_ref().filter(|_| !rail) {
            let px = (15.0 * s) as u32;
            let pad = 10.0 * s;
            let [x0, x1, y] = f.info_room.unwrap_or([12.0 * s, f.width - 12.0 * s, 8.0 * s]);
            let y = if self.editor_hud_rect.is_some() { y.max(self.notes_bottom + 8.0 * s) } else { y };
            let rows = info_rows(info, (x1 - x0 - pad * 2.0).max(1.0), |t| self.text.width(t, px as f32));
            let labels: Vec<Label> = rows.iter().map(|t| self.text.label_op(r, scene, &omsi_ui::tr(t), px, [237, 237, 237, 0], 1.0)).collect();
            let lw = labels.iter().map(|l| l.w).max().unwrap_or(0) as f32;
            let lh = labels.iter().map(|l| l.h).max().unwrap_or(0) as f32;
            let (w, h) = (lw + pad * 2.0, lh * labels.len() as f32 + pad * 0.8);
            let x = (x0 + x1 - w) * 0.5;
            self.text.rounded(r, scene, [x, y, x + w, y + h], 8.0 * s, [28, 28, 28, (255.0 * self.text.backdrop.min(1.0)) as u8]);
            for (k, l) in labels.iter().enumerate() {
                let (lx, ly) = (x + (w - l.w as f32) * 0.5, y + pad * 0.4 + k as f32 * lh);
                scene.overlays.push((l.tex, [lx, ly, lx + l.w as f32, ly + l.h as f32]));
            }
            self.info_rect = Some([x, y, x + w, y + h]);
            if self.editor_hud_rect.is_some() {
                self.notes_bottom = self.notes_bottom.max(y + h);
            }
        }
        let tutorial_w = (420.0 * s).min(f.width * 0.42);
        // --- the timetable window, on the right (left of a tutorial page, which has the
        // same corner)
        if let Some(t) = f.timetable.as_ref() {
            let beside = if f.tutorial.is_some() { tutorial_w + 12.0 * s } else { 0.0 };
            self.draw_timetable(r, scene, f, t, corner_top, beside, s);
        }
        // --- a tutorial page, on the right
        if let Some((title, text, image, at, count)) = f.tutorial {
            let w = tutorial_w;
            let x = f.width - w - 16.0 * s;
            let mut y = corner_top;
            let top = y;
            let pad = 14.0 * s;
            let mut items: Vec<(TextureId, [f32; 4])> = Vec::new();
            if let Some(p) = image {
                let entry = self.images.entry(p.to_path_buf()).or_insert_with(|| {
                    omsi_texture::decode_file(p).ok().map(|img| {
                        let (iw, ih) = (img.width, img.height);
                        (r.add_texture(scene, &img, false), iw, ih)
                    })
                });
                if let Some((tex, iw, ih)) = *entry {
                    let dw = w - pad * 2.0;
                    let dh = dw * ih as f32 / iw.max(1) as f32;
                    items.push((tex, [x + pad, y + pad, x + pad + dw, y + pad + dh]));
                    y += dh + pad;
                }
            }
            y += pad * 0.6;
            let tp = (18.0 * s) as u32;
            if !title.is_empty() {
                for line in wrap(&self.text, title, tp as f32, w - pad * 2.0) {
                    let l = self.text.label(r, scene, &line, tp, [255, 200, 110, 0]);
                    items.push((l.tex, [x + pad, y, x + pad + l.w as f32, y + l.h as f32]));
                    y += l.h as f32;
                }
                y += 6.0 * s;
            }
            let bp = (14.0 * s) as u32;
            let max_y = f.height - 60.0 * s;
            'text: for para in text.lines() {
                for line in wrap(&self.text, para, bp as f32, w - pad * 2.0) {
                    if y > max_y {
                        break 'text;
                    }
                    let l = self.text.label(r, scene, &line, bp, [235, 235, 235, 0]);
                    items.push((l.tex, [x + pad, y, x + pad + l.w as f32, y + l.h as f32]));
                    y += l.h as f32 * 0.95;
                }
                y += 5.0 * s;
            }
            let tr = |t: &str| omsi_ui::tr(t).into_owned();
            let foot = format!("{} {}/{}   |   {}   |   {}   |   {}", tr("Page"), at + 1, count, tr("Enter next"), tr("Page Up back"), tr("Ctrl+T hide"));
            let l = self.text.label(r, scene, &foot, (12.0 * s) as u32, [150, 150, 150, 0]);
            y += 4.0 * s;
            items.push((l.tex, [x + pad, y, x + pad + l.w as f32, y + l.h as f32]));
            y += l.h as f32 + pad;
            let plate = self.text.plate(r, scene, 3);
            scene.overlays.push((plate, [x, top, x + w, y]));
            scene.overlays.extend(items);
        }
        // --- the game menu and its lists, in the middle over a dimmed picture
        self.anim_dt = dt.clamp(0.0, 0.1);
        self.text.flat = true;
        self.draw_menu(r, scene, f);
        self.text.flat = false;
        // --- the mouse-over name, right of the cursor
        self.vr_tooltip_overlay = None;
        if let Some(t) = f.tooltip.as_ref().filter(|t| !t.is_empty()) {
            let l = self.text.label_op(r, scene, &omsi_ui::tr(t), (13.0 * s).round() as u32, [237, 237, 237, 0], 1.0);
            let mut x = f.cursor.0.clamp(0.0, f.width) + 16.0 * s;
            let mut y = f.cursor.1 + 2.0 * s;
            if f.tooltip_above {
                x = (f.cursor.0 - l.w as f32 * 0.5).clamp(5.0 * s, (f.width - l.w as f32 - 5.0 * s).max(5.0 * s));
                y = (f.cursor.1 - 56.0 * s - l.h as f32).max(0.0);
            } else if x + l.w as f32 > f.width {
                x = (f.cursor.0.clamp(0.0, f.width) - 8.0 * s - l.w as f32).max(5.0 * s);
            }
            if y + l.h as f32 > f.height {
                y = f.height - l.h as f32;
            }
            if f.vr {
                self.vr_tooltip_overlay = Some(scene.overlays.len());
            } else {
                // (in a headset the text alone is placed in front of the eye)
                self.text.rounded(r, scene, [x - 7.0 * s, y, x + l.w as f32 + 7.0 * s, y + l.h as f32], 6.0 * s, [28, 28, 28, 230]);
            }
            scene.overlays.push((l.tex, [x, y, x + l.w as f32, y + l.h as f32]));
        }
        if let (Some((x, y)), false) = (f.steer_cross, f.vr) {
            // (the size of the system's crosshair cursor: 17 logical pixels)
            let tex = self.text.steer_cross(r, scene);
            let half = 8.5 * f.scale;
            scene.overlays.push((tex, [x - half, y - half, x + half, y + half]));
        }
        if f.vr {
            let pointer = self.text.vr_pointer(r, scene);
            self.vr_cursor_overlay = Some(scene.overlays.len());
            scene.overlays.push((pointer, [0.0, 0.0, 7.0 * s, 7.0 * s]));
        } else {
            self.vr_cursor_overlay = None;
        }
        self.text.end_frame(r, scene);
    }
}

/// The pixels (RGBA, `n`×`n`) of the steering cross: arms two pixels wide across the whole
/// picture, white, with a one-pixel dark edge so it shows on a bright sky and a dark road
/// alike, and a small gap in the middle that leaves the point itself visible.
pub(crate) fn steer_cross_pixels(n: usize) -> Vec<u8> {
    let mut rgba = vec![0u8; n * n * 4];
    let c = n as f32 * 0.5;
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = ((x as f32 + 0.5 - c).abs(), (y as f32 + 0.5 - c).abs());
            let (along, across) = if dx <= dy { (dy, dx) } else { (dx, dy) };
            let gap = along < 3.0;
            let px = if gap || along > c - 1.0 {
                [0, 0, 0, 0]
            } else if across <= 1.0 {
                [255, 255, 255, 255]
            } else if across <= 2.0 {
                [0, 0, 0, 200]
            } else {
                [0, 0, 0, 0]
            };
            rgba[(y * n + x) * 4..(y * n + x + 1) * 4].copy_from_slice(&px);
        }
    }
    rgba
}

impl Ui {
    /// The timetable window as the launcher's card: a header with the line on an amber sign,
    /// the terminus and how late, the tour and trip under it; then the stops as the
    /// launcher's trips table (arrives, stop, departs), the stops passed muted, the next one
    /// on a soft amber band with an amber bar at its left; the next trip at the end.
    #[allow(clippy::too_many_arguments)]
    fn draw_timetable(&mut self, r: &Renderer, scene: &mut Scene, f: &Frame, t: &TimetableView, top: f32, beside: f32, s: f32) {
        const INK: [u8; 4] = [237, 237, 237, 0];
        const INK_2: [u8; 4] = [184, 184, 184, 0];
        const MUTED: [u8; 4] = [138, 138, 138, 0];
        let w = (400.0 * s).min(f.width * 0.42).round();
        let x = (f.width - w - 16.0 * s - beside).max(16.0 * s).round();
        let pad = (16.0 * s).round();
        let px = (13.0 * s).round() as u32;
        let lh = (30.0 * s).round();
        let head_h = 64.0 * s;
        let th_h = 26.0 * s;
        let max_rows = (((f.height * 0.72 - head_h - th_h) / lh).floor() as usize).max(3);
        let n = t.rows.len() + t.next.is_some() as usize;
        let shown = n.min(max_rows);
        // (from the stop before the next one on)
        let next = t.rows.iter().position(|r| r.state == 1).unwrap_or(0);
        let first = next.saturating_sub(1).min(n.saturating_sub(shown));
        let h = (head_h + th_h + lh * shown as f32 + pad * 0.6).round();
        let a = (255.0 * self.text.backdrop.min(1.0)) as u8;
        self.text.shadow(r, scene, [x, top, x + w, top + h], 8.0 * s, 18.0 * s, 4.0 * s, 60);
        self.text.rounded(r, scene, [x, top, x + w, top + h], 8.0 * s, [28, 28, 28, a]);
        // the header: the line's sign, the terminus, how late
        let cy = top + pad + 11.0 * s;
        let sign = self.text.label_op(r, scene, &t.line, (13.0 * s).round() as u32 | BOLD, [26, 18, 6, 0], 1.0);
        let sw = (sign.w as f32 + 10.0 * s).max(30.0 * s).round();
        let sh = (22.0 * s).round();
        self.text.rounded(r, scene, [x + pad, cy - sh * 0.5, x + pad + sw, cy + sh * 0.5], 4.0 * s, [232, 160, 48, 255]);
        let (sx, sy) = ((x + pad + (sw - sign.w as f32) * 0.5).round(), (cy - sign.h as f32 * 0.5).round());
        scene.overlays.push((sign.tex, [sx, sy, sx + sign.w as f32, sy + sign.h as f32]));
        let d = t.delay;
        let (dtext, dcol) = {
            let m = (d.abs() / 60.0) as i64;
            let sec = (d.abs() % 60.0) as i64;
            let sign = if d < 0.0 { "-" } else { "+" };
            let col = if d > 59.0 { [229, 83, 75, 0] } else if d < -59.0 { [96, 160, 232, 0] } else { [76, 183, 130, 0] };
            (format!("{sign}{m}:{sec:02}"), col)
        };
        let dl = self.text.label_op(r, scene, &dtext, (13.0 * s).round() as u32 | BOLD, dcol, 1.0);
        let dx = (x + w - pad - dl.w as f32).round();
        scene.overlays.push((dl.tex, [dx, (cy - dl.h as f32 * 0.5).round(), dx + dl.w as f32, (cy - dl.h as f32 * 0.5).round() + dl.h as f32]));
        let tx = x + pad + sw + 10.0 * s;
        let term = clip_to(&self.text, &t.terminus, 15.0 * s, dx - 10.0 * s - tx);
        let tl = self.text.label_op(r, scene, &term, (15.0 * s).round() as u32 | BOLD, INK, 1.0);
        scene.overlays.push((tl.tex, [tx.round(), (cy - tl.h as f32 * 0.5).round(), tx.round() + tl.w as f32, (cy - tl.h as f32 * 0.5).round() + tl.h as f32]));
        let my = top + pad + 34.0 * s;
        let icon_px = (14.0 * s).round() as u32;
        if let Some(ic) = self.text.icon(r, scene, "departure_board", icon_px, [138, 138, 138, 255]) {
            let iy = (my - icon_px as f32 * 0.5).round();
            scene.overlays.push((ic, [x + pad, iy, x + pad + icon_px as f32, iy + icon_px as f32]));
        }
        let ml = self.text.label_op(r, scene, &t.meta, (12.0 * s).round() as u32, MUTED, 1.0);
        let mx = (x + pad + icon_px as f32 + 6.0 * s).round();
        scene.overlays.push((ml.tex, [mx, (my - ml.h as f32 * 0.5).round(), mx + ml.w as f32, (my - ml.h as f32 * 0.5).round() + ml.h as f32]));
        // the table's head
        let col_arr = x + pad;
        let col_stop = x + pad + 58.0 * s;
        let col_dep = x + w - pad;
        let hy = top + head_h + th_h * 0.5;
        let hpx = (11.5 * s).round() as u32 | BOLD;
        for (txt, at, right) in [(omsi_ui::tr("Arrives"), col_arr, false), (omsi_ui::tr("Stop"), col_stop, false), (omsi_ui::tr("Departs"), col_dep, true)] {
            let l = self.text.label_op(r, scene, &txt, hpx, MUTED, 1.0);
            let lx = if right { at - l.w as f32 } else { at };
            let ly = (hy - l.h as f32 * 0.5).round();
            scene.overlays.push((l.tex, [lx.round(), ly, lx.round() + l.w as f32, ly + l.h as f32]));
        }
        // the rows
        let time_w = 44.0 * s;
        for k in first..first + shown {
            let ry = top + head_h + th_h + lh * (k - first) as f32;
            let band = [x + 6.0 * s, ry, x + w - 6.0 * s, ry + lh - 2.0 * s];
            let (name, arr, dep, state) = match t.rows.get(k) {
                Some(row) => (row.name.clone(), row.arr.clone(), row.dep.clone(), row.state),
                None => match t.next.as_ref() {
                    Some((what, when)) => (format!("› {what}"), String::new(), when.clone(), 3),
                    None => continue,
                },
            };
            if state == 1 {
                self.text.rounded(r, scene, band, 4.0 * s, [232, 160, 48, 26]);
                self.text.rounded(r, scene, [band[0], band[1], band[0] + 3.0 * s, band[3]], 1.5 * s, [232, 160, 48, 255]);
            } else if (k - first) % 2 == 1 {
                self.text.rounded(r, scene, band, 4.0 * s, [33, 33, 33, a]);
            }
            let (ink, time_ink) = match state {
                0 | 3 => (MUTED, MUTED),
                1 => (INK, [255, 200, 110, 0]),
                _ => (INK_2, INK),
            };
            let size = if state == 1 { px | BOLD } else { px };
            let cy = ry + (lh - 2.0 * s) * 0.5;
            for (txt, at, right) in [(&arr, col_arr, false), (&dep, col_dep, true)] {
                if txt.is_empty() {
                    continue;
                }
                let l = self.text.label_op(r, scene, txt, size, time_ink, 1.0);
                let lx = if right { at - l.w as f32 } else { at };
                let ly = (cy - l.h as f32 * 0.5).round();
                scene.overlays.push((l.tex, [lx.round(), ly, lx.round() + l.w as f32, ly + l.h as f32]));
            }
            let name = clip_to(&self.text, &name, px as f32, col_dep - time_w - 8.0 * s - col_stop);
            let l = self.text.label_op(r, scene, &name, size, ink, 1.0);
            let ly = (cy - l.h as f32 * 0.5).round();
            scene.overlays.push((l.tex, [col_stop.round(), ly, col_stop.round() + l.w as f32, ly + l.h as f32]));
        }
    }

    /// The notifications as the launcher's cards: flat graphite, no edge, the kind's icon in
    /// its colour, the text wrapped to as many rows as it needs. A card comes in rising a
    /// little and fading, and goes fading while the room it took closes; the newest stands
    /// on top of the stack.
    fn draw_notices(&mut self, r: &Renderer, scene: &mut Scene, f: &Frame, s: f32) {
        if f.notices.is_empty() {
            return;
        }
        let gap = 8.0 * s;
        let px = (14.0 * s).round() as u32;
        let lh = (px as f32 * 1.45).round();
        let pad_y = 11.0 * s;
        let pad_l = 14.0 * s;
        let icon_px = (18.0 * s).round() as u32;
        let text_x = pad_l + icon_px as f32 + 10.0 * s;
        let pad_r = 16.0 * s;
        let max_text = (340.0 * s).min(f.width - 32.0 * s - text_x - pad_r).max(80.0 * s);
        // where the stack stands: over a navigator in the lower half (growing up), under one
        // in the upper half, or at the top middle; on the navigator's side of the window
        let (up, mut y, place) = match f.notice_anchor {
            Some(a) => {
                let left = (a[0] + a[2]) * 0.5 < f.width * 0.5;
                let place = if left { (a[0].max(8.0 * s), 0) } else { (a[2].min(f.width - 8.0 * s), 2) };
                if a[1] > f.height * 0.5 { (true, a[1] - gap, place) } else { (false, a[3] + gap, place) }
            }
            None => (false, 70.0 * s, (f.width * 0.5, 1)),
        };
        // (growing up, the oldest stands by the navigator and the newest on top; growing
        // down, the newest first)
        let order: Vec<Notice> = if up { f.notices.to_vec() } else { f.notices.iter().rev().cloned().collect() };
        for n in &order {
            let p = n.alpha();
            if p <= 0.0 {
                continue;
            }
            let text = omsi_ui::tr(&n.text).into_owned();
            let mut rows = {
                let tc = &self.text;
                wrap_rows(&text, max_text, 0.0, &|t: &str| tc.width_raw(t, px as f32))
            };
            // (six rows at the most; a longer text says it goes on)
            if rows.len() > 6 {
                rows.truncate(6);
                rows[5].push('…');
            }
            let tw = rows.iter().map(|t| self.text.width_raw(t, px as f32)).fold(0.0f32, f32::max);
            let w = (text_x + tw + pad_r).max(220.0 * s).min(f.width - 16.0 * s).round();
            let h = (pad_y * 2.0 + lh * rows.len() as f32).round();
            let top = if up { y - h } else { y };
            if top < 0.0 || top + h > f.height {
                break;
            }
            let x0 = match place.1 {
                0 => place.0,
                2 => place.0 - w,
                _ => place.0 - w * 0.5,
            }
            .clamp(8.0 * s, (f.width - w - 8.0 * s).max(8.0 * s))
            .round();
            // (it rises into its place as it comes, sinks as it goes)
            let ty = top + (1.0 - p) * 10.0 * s;
            let k = (p * 16.0).round() / 16.0;
            let (rgb, icon) = n.kind.look();
            self.text.shadow(r, scene, [x0, ty, x0 + w, ty + h], 8.0 * s, 18.0 * s, 4.0 * s, (70.0 * k) as u8);
            self.text.rounded(r, scene, [x0, ty, x0 + w, ty + h], 8.0 * s, [28, 28, 28, (246.0 * k) as u8]);
            if let Some(t) = self.text.icon(r, scene, icon, icon_px, [rgb[0], rgb[1], rgb[2], (255.0 * k) as u8]) {
                let iy = (ty + pad_y + lh * 0.5 - icon_px as f32 * 0.5).round();
                let ix = (x0 + pad_l).round();
                scene.overlays.push((t, [ix, iy, ix + icon_px as f32, iy + icon_px as f32]));
            }
            for (i, row) in rows.iter().enumerate() {
                let l = self.text.label_op(r, scene, row, px, [237, 237, 237, 0], k);
                let cy = ty + pad_y + lh * (i as f32 + 0.5);
                let ly = (cy - l.h as f32 * 0.5).round();
                let lx = (x0 + text_x).round();
                scene.overlays.push((l.tex, [lx, ly, lx + l.w as f32, ly + l.h as f32]));
            }
            let slot = (h + gap) * p;
            y = if up { y - slot } else { y + slot };
        }
    }
}

/// How long the loading screen stands full once its bar is at the end, and how long it then
/// takes to fade over the first pictures of the session (seconds).
const LOADING_HOLD: f32 = 0.12;
const LOADING_FADE: f32 = 0.5;

/// The loading screen's own state. The start area's tiles are counted only as each is done,
/// and most of a load the count stands at none, so the bar is driven by the time the load is
/// expected to take (`LoadTimes`): it goes along that time to `LOADING_EXPECTED_AT` and on,
/// slower, towards `LOADING_CREEP` if the load takes longer - never standing still, never going
/// back - and takes the real count whenever that is ahead. Once the world is there it runs to
/// its end in `LOADING_RUN` seconds, stands a moment, and the screen fades over the session's
/// first pictures instead of being cut away.
#[derive(Default)]
pub struct Loading {
    /// The map's name.
    pub title: String,
    /// How long this load is expected to take (seconds; 0: not known yet).
    pub expected: f32,
    /// The bar as shown (0 to 1).
    shown: f32,
    /// Seconds the loading screen has stood.
    elapsed: f32,
    /// Seconds the screen has been shown, the run and the fade too (the stripes, the tips).
    clock: f32,
    /// The tip shown first (a different one each start).
    tip0: usize,
    /// The world is there: how fast the bar runs to its end (a share a second).
    run: Option<f32>,
    /// The run has not had a frame yet: the time before it (the world being started, no
    /// frame drawn) is not run through unseen.
    run_fresh: bool,
    /// Seconds since the bar came to its end.
    out: f32,
    /// The screen was shown at all (a map without streaming starts at once: nothing fades).
    drawn: bool,
    last: Option<std::time::Instant>,
}

/// The tips under the loading screen's "Tips", one at a time.
pub const LOADING_TIPS: [&str; 8] = [
    "Shift+U puts the bus into service: the electrics, the engine and the lights in one go",
    "F1 is the driver's seat, F2 the passengers' view, F3 outside the bus and F4 the free camera",
    "Esc opens the menu: the options, the line and tour, saving and the photo mode",
    "Shift+N switches the navigator between the map, the map with the next stops, and off",
    "Insert shows the timetable of your trip",
    "Shift+M opens the city map",
    "P pauses the game; Alt+S saves the situation",
    "Shift+Y shows the information bar: the time, the speed, the temperatures and the next stop",
];
/// How long a tip stands (seconds).
const TIP_SECS: f32 = 6.0;

/// Where the bar stands when the expected time is up, and how far it creeps on after it.
const LOADING_EXPECTED_AT: f32 = 0.92;
const LOADING_CREEP: f32 = 0.98;
/// How long the bar takes from wherever it is to its end once the world is there (seconds).
const LOADING_RUN: f32 = 0.35;
/// The expected time of a map loaded for the first time (seconds).
pub const LOADING_FIRST_GUESS: f32 = 8.0;

/// The bar for `u` of the expected time gone by: nearly even along the time (a little quicker
/// at first, a little slower towards the end) to `LOADING_EXPECTED_AT`, then on more and more
/// slowly towards `LOADING_CREEP`.
pub fn time_progress(u: f32) -> f32 {
    let u = u.max(0.0);
    if u <= 1.0 {
        LOADING_EXPECTED_AT * u * (1.3 - 0.3 * u)
    } else {
        LOADING_EXPECTED_AT + (LOADING_CREEP - LOADING_EXPECTED_AT) * (1.0 - 1.0 / (1.0 + (u - 1.0) * 3.0))
    }
}

/// The bar shown after `dt` seconds on its way to `target`: an exponential approach (it
/// slows as it gets near) held between a floor speed, so that it never stands just short of
/// where it is going, and a ceiling, so that a sudden jump of the count is spread out (`ready`:
/// a quicker approach).
pub fn ease_progress(shown: f32, target: f32, ready: bool, dt: f32) -> f32 {
    let target = target.clamp(0.0, 1.0);
    let gap = target - shown;
    if gap <= 0.0 {
        return shown;
    }
    let (tau, floor, ceil) = if ready { (0.1, 1.0, 3.5) } else { (0.35, 0.12, 1.2) };
    (shown + (gap / tau).clamp(floor, ceil) * dt.clamp(0.0, 0.1)).min(target)
}

impl Loading {
    /// Seconds since the last frame of the loading screen (none before the first).
    pub fn tick(&mut self) -> f32 {
        let now = std::time::Instant::now();
        let dt = self.last.map(|t| (now - t).as_secs_f32()).unwrap_or(0.0);
        self.last = Some(now);
        dt
    }

    /// A frame of the loading screen with the start area `real` of the way loaded.
    pub fn step(&mut self, real: f32, dt: f32) {
        if !self.drawn {
            self.tip0 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as usize).unwrap_or(0);
        }
        self.drawn = true;
        let dt = dt.clamp(0.0, 1.0);
        self.clock += dt.min(0.1);
        match self.run {
            None => {
                // (by the clock: the time is what the bar goes along, and what the next load
                // of the map expects)
                self.elapsed += dt;
                let by_time = time_progress(self.elapsed / self.expected_secs());
                // (the count, when it is ahead, eased: it comes in jumps)
                let by_count = ease_progress(self.shown, real.clamp(0.0, 1.0) * LOADING_CREEP, false, dt);
                self.shown = self.shown.max(by_time).max(by_count);
            }
            Some(_) if self.run_fresh => self.run_fresh = false,
            Some(rate) => {
                self.shown = (self.shown + rate * dt).min(1.0);
                if self.shown >= 1.0 {
                    // (a fade of a few steps at the least, however slow the frames are)
                    self.out += dt.min(0.1);
                }
            }
        }
    }

    fn expected_secs(&self) -> f32 {
        if self.expected > 0.5 { self.expected } else { LOADING_FIRST_GUESS }
    }

    /// The world is there: the rest of the bar in `LOADING_RUN` seconds.
    pub fn finish(&mut self) {
        if self.run.is_none() {
            self.run = Some(((1.0 - self.shown) / LOADING_RUN).max(0.5));
            self.run_fresh = true;
        }
    }

    /// Seconds the loading screen stood (the load's time, for `LoadTimes`).
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// The bar shown.
    pub fn shown(&self) -> f32 {
        self.shown
    }

    /// Seconds the screen has been shown (its stripes and tips go by it).
    pub fn clock(&self) -> f32 {
        self.clock
    }

    /// The tip shown now and how far it is faded in: a new one every `TIP_SECS`, fading
    /// out and the next in.
    pub fn tip(&self) -> (&'static str, f32) {
        let n = (self.clock / TIP_SECS) as usize;
        let t = self.clock - n as f32 * TIP_SECS;
        let k = (t / 0.4).min((TIP_SECS - t) / 0.4).clamp(0.0, 1.0);
        (LOADING_TIPS[(self.tip0 + n) % LOADING_TIPS.len()], k)
    }

    /// The screen still stands over the session's pictures: how opaque it is (none: it is
    /// gone, or it never showed).
    pub fn over_session(&self) -> Option<f32> {
        if !(self.drawn && self.run.is_some()) {
            return None;
        }
        let t = ((self.out - LOADING_HOLD) / LOADING_FADE).clamp(0.0, 1.0);
        (t < 1.0).then(|| 1.0 - t * t * (3.0 - 2.0 * t))
    }
}

/// How long the last loads of each map took (`~/.openomsi/cache/load-times.json`: the map's
/// file → the last three loads in seconds), so that the loading bar can go along the time the
/// next one will likely take.
pub struct LoadTimes;

impl LoadTimes {
    fn path() -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(std::path::PathBuf::from(home).join(".openomsi").join("cache").join("load-times.json"))
    }

    fn read() -> serde_json::Map<String, serde_json::Value> {
        Self::path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    /// The expected time of loading `map`: the average of its last loads (none: never loaded).
    pub fn expected(map: &str) -> Option<f32> {
        expected_of(Self::read().get(map)?)
    }

    /// `map` took `secs` to load this time.
    pub fn record(map: &str, secs: f32) {
        let Some(p) = Self::path() else { return };
        let mut all = Self::read();
        let mut last: Vec<f32> = all.get(map).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect()).unwrap_or_default();
        last.push(secs);
        let keep = last.len().saturating_sub(3);
        last.drain(..keep);
        let last: Vec<f64> = last.iter().map(|x| (*x as f64 * 100.0).round() / 100.0).collect();
        all.insert(map.to_string(), serde_json::json!(last));
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&p, serde_json::Value::Object(all).to_string()) {
            log::warn!("load times: {} not written: {e}", p.display());
        }
    }
}

/// The average of the loads kept for a map.
fn expected_of(v: &serde_json::Value) -> Option<f32> {
    let t: Vec<f32> = v.as_array()?.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).filter(|x| x.is_finite() && *x > 0.0).collect();
    (!t.is_empty()).then(|| t.iter().sum::<f32>() / t.len() as f32)
}

/// The openOMSI wordmark (white letters, the orange "OMSI"), for the loading screen.
const WORDMARK: &str = include_str!("../../../assets/logos/openomsi-wordmark-light.svg");

/// The pixels of a stretch of the loading bar `w` x `h` with its stripes: 45 degree bands of
/// the accent and a lighter one, `period` pixels apart along the bar, moved `phase` pixels on.
fn stripe_pixels(w: u32, h: u32, period: u32, phase: u32, alpha: u8) -> Vec<u8> {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let light = (x + period * 4 + phase - y % period) % period < period / 2;
            let c = if light { [244, 182, 82] } else { [232, 160, 48] };
            px.extend_from_slice(&[c[0], c[1], c[2], alpha]);
        }
    }
    px
}

impl Ui {
    /// The loading screen: the openOMSI wordmark large in the middle and the map's name under
    /// it; along the bottom a square amber bar whose stripes slide on (it is working), over
    /// it a rotating tip on the left and how far on the right. All of it `opacity` (fading
    /// over the first pictures of the session); `clock` moves the stripes and the tips.
    #[allow(clippy::too_many_arguments)]
    pub fn loading_screen(&mut self, r: &Renderer, scene: &mut Scene, width: f32, height: f32, scale: f32, title: &str, progress: f32, opacity: f32, clock: f32, tip: (&str, f32)) {
        let s = scale.max(0.5);
        let k = (opacity.clamp(0.0, 1.0) * 16.0).round() / 16.0;
        if k <= 0.0 {
            return;
        }
        let a = |v: f32| (v * k).round().clamp(0.0, 255.0) as u8;
        let bg = self.text.solid(r, scene, [18, 18, 18, a(255.0)]);
        scene.overlays.push((bg, [0.0, 0.0, width, height]));
        // the wordmark, large, a little above the middle
        let lw = (560.0 * s).min(width * 0.6).round() as u32;
        let lh = (lw as f32 * 303.0 / 1142.0).round() as u32;
        let ka = a(255.0);
        let logo = self.text.cached(r, scene, ("\u{0}wordmark".to_string(), lw, [255, 255, 255, ka]), (lw, lh), || {
            let mut px = omsi_ui::icons::rasterize_rgba(WORDMARK, lw, lh).unwrap_or_else(|| vec![0; (lw * lh * 4) as usize]);
            for p in px.chunks_mut(4) {
                p[3] = (p[3] as u32 * ka as u32 / 255) as u8;
            }
            px
        });
        let lx = ((width - lw as f32) * 0.5).round();
        let ly = (height * 0.42 - lh as f32 * 0.5).round();
        scene.overlays.push((logo, [lx, ly, lx + lw as f32, ly + lh as f32]));
        // the map's name under it
        let tpx = (24.0 * s).round() as u32;
        let title = clip_to(&self.text, title, tpx as f32, width - 64.0 * s);
        let t = self.text.label_op(r, scene, &title, tpx, [184, 184, 184, 0], k);
        let tx = ((width - t.w as f32) * 0.5).round();
        let ty = (ly + lh as f32 + 18.0 * s).round();
        scene.overlays.push((t.tex, [tx, ty, tx + t.w as f32, ty + t.h as f32]));
        // the bar along the bottom: square, the track and the striped fill
        let margin = (48.0 * s).round();
        let bw = (width - margin * 2.0).min(1100.0 * s).round();
        let bx = ((width - bw) * 0.5).round();
        let bh = (14.0 * s).round().max(6.0);
        let by = (height - margin - bh).round();
        let track = self.text.solid(r, scene, [42, 42, 42, a(255.0)]);
        scene.overlays.push((track, [bx, by, bx + bw, by + bh]));
        let p = progress.clamp(0.0, 1.0);
        let fill_w = (bw * p).round() as u32;
        if fill_w > 0 {
            // (tiles of one period with the stripes moved on, a cut one at the end: the
            // overlays cannot move a texture's pixels, so the moving is in which tile is drawn)
            let period = (bh as u32 * 2).max(8);
            let phase = ((clock * 40.0 * s) as u32) % period;
            let (h, fa) = (bh as u32, a(255.0));
            let mut x = 0u32;
            while x < fill_w {
                let w = period.min(fill_w - x);
                let key = (format!("\u{0}stripes{period}x{h}p{phase}"), w, [232, 160, 48, fa]);
                let tex = self.text.cached(r, scene, key, (w, h), || stripe_pixels(w, h, period, phase, fa));
                scene.overlays.push((tex, [bx + x as f32, by, bx + (x + w) as f32, by + bh]));
                x += w;
            }
        }
        // over the bar: a tip on the left, how far on the right
        let small = (11.0 * s).round() as u32 | BOLD;
        let tip_px = (14.0 * s).round() as u32;
        let pct = self.text.label_op(r, scene, &format!("{:.0} %", p * 100.0), (15.0 * s).round() as u32 | BOLD, [237, 237, 237, 0], k);
        let row_y = by - 12.0 * s;
        scene.overlays.push((pct.tex, [bx + bw - pct.w as f32, row_y - pct.h as f32, bx + bw, row_y]));
        let (tip, tk) = tip;
        let tip = clip_to(&self.text, &omsi_ui::tr(tip), tip_px as f32, bw - pct.w as f32 - 40.0 * s);
        let tl = self.text.label_op(r, scene, &tip, tip_px, [184, 184, 184, 0], k * tk);
        scene.overlays.push((tl.tex, [bx, row_y - tl.h as f32, bx + tl.w as f32, row_y]));
        let head = self.text.label_op(r, scene, &omsi_ui::tr("Tips").to_uppercase(), small, [232, 160, 48, 0], k);
        let hy = row_y - tl.h as f32 - head.h as f32 + 2.0 * s;
        scene.overlays.push((head.tex, [bx, hy, bx + head.w as f32, hy + head.h as f32]));
    }

    /// The loading screen alone, before the session's first picture.
    #[allow(clippy::too_many_arguments)]
    pub fn loading(&mut self, r: &Renderer, scene: &mut Scene, width: f32, height: f32, scale: f32, title: &str, progress: f32, clock: f32, tip: (&str, f32)) {
        self.loading_screen(r, scene, width, height, scale, title, progress, 1.0, clock, tip);
        self.text.end_frame(r, scene);
    }
}

/// `text` broken into lines of at most `width` pixels, between words.
fn wrap(tc: &TextCache, text: &str, px: f32, width: f32) -> Vec<String> {
    let translated = omsi_ui::tr(text);
    let text = translated.as_ref();
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let try_line = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if tc.width(&try_line, px) > width && !line.is_empty() {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = try_line;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// `text` cut at the end to fit `width` pixels ("…").
fn clip_to(tc: &TextCache, text: &str, px: f32, width: f32) -> String {
    // Translate the complete key before truncation, otherwise the ellipsis breaks lookup.
    let translated = omsi_ui::tr(text);
    let text = translated.as_ref();
    if tc.width_raw(text, px) <= width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let head = |n: usize| {
        let mut t: String = chars[..n].iter().collect();
        t.push('…');
        t
    };
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if tc.width_raw(&head(mid), px) <= width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    head(lo)
}

/// `text` in rows that fit `width` (as `measure` measures): broken between words, and inside a
/// word too long for a row. The rows after the first fit `width - indent` (they are drawn
/// indented, under the first).
fn wrap_rows(text: &str, width: f32, indent: f32, measure: &impl Fn(&str) -> f32) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut cur = String::new();
    let room = |rows: &Vec<String>| if rows.is_empty() { width } else { width - indent };
    for word in text.split(' ') {
        let joined = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if measure(&joined) <= room(&rows) {
            cur = joined;
            continue;
        }
        if !cur.is_empty() {
            rows.push(std::mem::take(&mut cur));
        }
        // the word alone, cut where a row is full (a long link, a run of letters)
        let mut w: Vec<char> = word.chars().collect();
        while w.len() > 1 && measure(&w.iter().collect::<String>()) > room(&rows) {
            let mut k = w.len() - 1;
            while k > 1 && measure(&w[..k].iter().collect::<String>()) > room(&rows) {
                k -= 1;
            }
            rows.push(w[..k].iter().collect());
            w.drain(..k);
        }
        cur = w.into_iter().collect();
    }
    if !cur.is_empty() || rows.is_empty() {
        rows.push(cur);
    }
    rows
}

/// `text` cut at the start to fit (the end of what is being typed stays visible).
fn clip_left(tc: &TextCache, text: &str, px: f32, width: f32) -> String {
    let mut t: Vec<char> = text.chars().collect();
    while t.len() > 1 && tc.width(&t.iter().collect::<String>(), px) > width {
        t.remove(0);
    }
    t.into_iter().collect()
}

/// A chat line with its bad words starred out (rustrict: profanity, slurs and the usual
/// ways of writing around a filter, without a word list to keep).
pub fn filter_chat(text: &str) -> String {
    use rustrict::CensorStr;
    text.censor()
}

/// Fit all VR page buttons above the separately reserved Back button.
fn vr_settings_sidebar_step(available: f32, pages: usize, scale: f32) -> f32 {
    (available.max(0.0) / pages.max(1) as f32).min(42.0 * scale)
}

#[cfg(test)]
mod tests {
    /// The steering cross: white arms with a dark edge, open in the middle, nothing in the
    /// corners - the same in all four directions.
    #[test]
    fn the_steering_cross_is_a_white_cross_with_a_dark_edge() {
        let n = 34;
        let px = super::steer_cross_pixels(n);
        let at = |x: usize, y: usize| &px[(y * n + x) * 4..(y * n + x) * 4 + 4];
        let c = n / 2;
        assert_eq!(at(c, c)[3], 0, "the middle is open");
        assert_eq!(at(0, 0)[3], 0, "the corners are empty");
        for (x, y) in [(c, 6), (c, n - 7), (6, c), (n - 7, c)] {
            assert_eq!(at(x, y), [255, 255, 255, 255], "arm at {x},{y}");
        }
        assert_eq!(at(c + 2, 6)[..3], [0, 0, 0], "the arm's edge is dark");
    }

    use super::*;

    #[test]
    fn reload_button_fits_between_tools_and_help(){
        for (width,height) in [(1280.0,720.0),(1920.0,1080.0)] {
            let l=editor_dock_layout([0.0,0.0,width,height],1.0,80.0,20,192.0,None);
            assert!(l.reload[1]>=l.tiles[3]);assert!(l.reload[3]<=l.toggle[1]);
            assert!(l.reload[0]>=l.panel[0]&&l.reload[2]<=l.panel[2]);
            assert!(l.hints[1]>=l.toggle[3]);
            assert!(l.audit[1]>=l.reload[3]&&l.audit[3]<=l.toggle[1]);
            assert!(l.audit[0]>=l.panel[0]&&l.audit[2]<=l.panel[2]);
        }
    }
    #[test]
    fn editor_dock_keeps_catalogue_and_footer_inside_small_or_shifted_viewports() {
        for viewport in [[0.0, 0.0, 872.0, 558.0], [0.0, 0.0, 544.0, 480.0],
            [0.0, 0.0, 384.0, 320.0], [1920.0, 0.0, 1920.0, 1080.0]] {
            for scale in [0.5, 1.0, 2.0] {
                let s = editor_scale(scale, viewport);
                let hud = editor_hud_layout(viewport, s);
                assert!(hud.panel[3] - hud.panel[1] <= 66.0 * s + 0.01);
                for spline in [false, true] {
                    for expanded in [false, true] {
                        let count = editor_shortcuts(spline, expanded).len();
                        for footer in [0.0, 128.0, 160.0, 164.0, 192.0, 198.0, 224.0, 256.0, 288.0, 320.0] {
                            for position in [None, Some([1.0, 1.0]), Some([-5.0, 8.0])] {
                                let dock = editor_dock_layout(viewport, s, hud.panel[3], count, footer, position);
                                assert!(dock.panel[0] >= viewport[0]);
                                assert!(dock.panel[1] > hud.panel[3]);
                                assert!(dock.panel[2] <= viewport[0] + viewport[2]);
                                assert!(dock.panel[3] <= viewport[1] + viewport[3]);
                                assert!(dock.catalog[3] < dock.hints[1]);
                                assert!(dock.builders[1]>dock.catalog[3]);
                                assert!(dock.builders[3]<dock.tiles[1]);
                                assert!(dock.tiles[1] > dock.catalog[3]);
                                assert!(dock.tiles[3] < dock.toggle[1]);
                                assert!(dock.hints[3] <= dock.panel[3]);
                                assert_eq!(dock.rows + dock.max_scroll, count);
                                if footer > 0.0 {
                                    assert!(dock.footer_top + footer * s < dock.panel[3]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn editor_dialogs_drag_independently_and_stay_inside_resized_viewports() {
        let mut ui=Ui::new().unwrap();
        let viewport=[1920.0,0.0,1280.0,720.0];
        for kind in ["audit","paths","path-manager","path-connector","construction",
            "object-text","tiles","catalog","junction","roundabout"] {
            let (x,y)=ui.editor_window_origin(kind,viewport,[400.0,500.0],[1932.0,80.0],[280.0,32.0]);
            assert!(!ui.editor_window_drag_press((x+350.0,y+16.0)),"close button area must remain clickable");
            assert!(ui.editor_window_drag_press((x+8.0,y+8.0)));
            assert!(ui.editor_dock_move((10000.0,10000.0)));
            assert!(ui.end_editor_dock_drag());
            let (x,y)=ui.editor_window_origin(kind,viewport,[400.0,500.0],[1932.0,80.0],[280.0,32.0]);
            assert_eq!((x,y),(2800.0,220.0));
            let (x,y)=ui.editor_window_origin(kind,[0.0,0.0,800.0,600.0],[400.0,500.0],[12.0,80.0],[280.0,32.0]);
            assert_eq!((x,y),(400.0,100.0));
        }
        assert!(ui.editor_dock_position.is_none());
        assert_eq!(ui.editor_window_positions.len(),10);
        let (x,y)=ui.editor_window_origin("construction",viewport,[400.0,500.0],[1932.0,80.0],[280.0,32.0]);
        ui.editor_window_drag_press((x+8.0,y+8.0));
        ui.editor_dock_move((-100.0,-100.0));ui.end_editor_dock_drag();
        assert_eq!(ui.editor_window_positions["construction"],[0.0,0.0]);
        assert_eq!(ui.editor_window_positions["catalog"],[1.0,1.0]);
    }

    #[test]
    fn moving_editor_dock_moves_click_targets_and_clamps_without_moving_selection() {
        let mut ui = Ui::new().unwrap();
        let hud = editor_hud_layout([1920.0, 0.0, 872.0, 558.0], 1.0);
        let dock = editor_dock_layout([1920.0, 0.0, 872.0, 558.0], 1.0, hud.panel[3], 7, 128.0, None);
        ui.editor_hud_rect = Some(hud.panel);
        ui.editor_dock_rect = Some(dock.panel);
        ui.editor_dock_header = Some(dock.header);
        ui.tile_open_rect = Some(dock.tiles);
        ui.roundabout_open_rect = Some(dock.builders);
        ui.terrain_open_rect = Some(dock.terrain);
        ui.editor_dock_bounds = dock.bounds;
        ui.catalog_open_rect = Some(dock.catalog);
        let button = [dock.panel[0] + 16.0, dock.footer_top + 27.0,
            dock.panel[0] + 288.0, dock.footer_top + 54.0];
        ui.spline_tool_rects.push((button, crate::editor::Action::FitTerrain));
        ui.terrain_tool_rects.push((button, crate::terrain_editor::Command::Undo));
        assert!(ui.editor_hud_press((dock.header[0] + 5.0, dock.header[1] + 5.0)));
        assert!(ui.editor_dock_move((10_000.0, -100.0)));
        let moved = ui.editor_dock_rect.unwrap();
        let tile = ui.tile_open_rect.unwrap();
        let roundabout=ui.roundabout_open_rect.unwrap();
        assert!((roundabout[1]-moved[1]-64.0).abs()<0.01);
        assert!((tile[0] - moved[0] - 10.0).abs() < 0.01);
        assert!((tile[1] - moved[1] - 96.0).abs() < 0.01);
        assert_eq!((moved[0], moved[1]), (dock.bounds[2], dock.bounds[1]));
        assert_eq!(ui.editor_hud_rect, Some(hud.panel));
        let dx = moved[0] - dock.panel[0];
        assert_eq!(ui.catalog_open_rect.unwrap()[0], dock.catalog[0] + dx);
        assert_eq!(ui.spline_tool_rects[0].0[0], button[0] + dx);
        assert_eq!(ui.terrain_tool_rects[0].0[0],button[0]+dx);
        assert_eq!(ui.terrain_open_rect.unwrap()[0],dock.terrain[0]+dx);
        assert!(ui.end_editor_dock_drag());
        assert!(!ui.end_editor_dock_drag());
        assert!(!ui.editor_dock_move((50.0, 50.0)));
    }

    #[test]
    fn terrain_dock_remains_below_selection_and_inside_resized_viewport() {
        for viewport in [[0.0,0.0,384.0,320.0],[0.0,0.0,872.0,558.0],[1920.0,0.0,1920.0,1080.0]] {
            for scale in [0.5,1.0,2.0] {
                let hud=editor_hud_layout(viewport,editor_scale(scale,viewport));
                for position in [None,Some([1.0,1.0]),Some([-1.0,2.0])] {
                    let layout=terrain_dock_layout(viewport,scale,hud.panel[3],position);
                    assert!(layout.panel[0]>=viewport[0]);
                    assert!(layout.panel[1]>hud.panel[3]);
                    assert!(layout.panel[2]<=viewport[0]+viewport[2]);
                    assert!(layout.panel[3]<=viewport[1]+viewport[3]);
                }
            }
        }
        let mut ui=Ui::new().unwrap();ui.editor_hud_rect=Some([12.0,8.0,850.0,74.0]);
        assert!(ui.editor_panel_contains((100.0,50.0)));
    }

    #[test]
    fn editor_dock_captures_wheel_and_reset_without_world_actions() {
        let mut ui = Ui::new().unwrap();
        ui.editor_dock_rect = Some([100.0, 90.0, 404.0, 490.0]);
        ui.editor_hint_rect = Some([110.0, 184.0, 394.0, 350.0]);
        ui.editor_help_rect = Some([110.0, 154.0, 342.0, 176.0]);
        ui.editor_dock_reset = Some([348.0, 154.0, 394.0, 176.0]);
        ui.editor_help_scroll_max = 20;
        assert!(ui.editor_dock_scroll((200.0, 220.0), -100.0));
        assert_eq!(ui.editor_help_scroll, 20);
        assert!(ui.editor_dock_scroll((200.0, 125.0), 3.0));
        assert_eq!(ui.editor_help_scroll, 20);
        assert!(!ui.editor_dock_scroll((600.0, 220.0), -1.0));
        assert!(ui.editor_hud_press((200.0, 165.0)));
        assert!(ui.editor_help_expanded);
        assert_eq!(ui.editor_help_scroll, 0);
        ui.editor_dock_position = Some([1.0, 1.0]);
        assert!(ui.editor_hud_press((370.0, 165.0)));
        assert!(ui.editor_dock_position.is_none());
        assert!(ui.editor_panel_contains((105.0, 485.0)));
        assert!(!ui.editor_panel_contains((450.0, 485.0)));
    }

    /// The information bar is broken between its parts into rows that fit the room it has,
    /// as many parts to a row as go in; a part wider than the room alone is a row of its own
    /// (#1164).
    #[test]
    fn the_information_bar_breaks_into_rows_that_fit() {
        let line = ["09:00:07", "0 km/h", "EXT 15 °C / INT 15 °C", "tank 80 %", "0 Passengers", "76 › Krankenhaus", "next: Bauernhof", "-6:52"].join(INFO_SEP);
        let w = |t: &str| t.chars().count() as f32 * 10.0;
        assert_eq!(info_rows(&line, 5000.0, w), vec![line.clone()]);
        let rows = info_rows(&line, 500.0, w);
        assert_eq!(rows, vec![
            format!("09:00:07{INFO_SEP}0 km/h{INFO_SEP}EXT 15 °C / INT 15 °C"),
            format!("tank 80 %{INFO_SEP}0 Passengers"),
            format!("76 › Krankenhaus{INFO_SEP}next: Bauernhof{INFO_SEP}-6:52"),
        ]);
        assert!(rows.iter().all(|r| w(r) <= 500.0));
        assert_eq!(rows.join(INFO_SEP), line);
        assert_eq!(info_rows(&line, 50.0, w).len(), 8);
    }

    #[test]
    fn a_server_notice_is_read_and_fades() {
        let (id, n) = Notice::parse("a1b2 5 warn Service 15/5: departure at 06:07 | depot").unwrap();
        assert_eq!(id, "a1b2");
        assert_eq!((n.kind, n.text.as_str(), n.total, n.left), (NoticeKind::Warn, "Service 15/5: departure at 06:07 | depot", 5.0, 5.0));
        // seconds held to 2 .. 30, an unknown kind is information
        assert_eq!(Notice::parse("x 0.5 info hi").unwrap().1.total, 2.0);
        assert_eq!(Notice::parse("x 600 whatever hi").unwrap().1, Notice { kind: NoticeKind::Info, text: "hi".into(), left: 30.0, total: 30.0 });
        // no text, no seconds, nothing: not a notice
        for bad in ["", "x", "x 5", "x 5 info", "x 5 info   ", "x five info hi", "x NaN info hi"] {
            assert!(Notice::parse(bad).is_none(), "{bad}");
        }
        // fades in, stays, fades out (eased)
        let mut n = Notice::parse("x 5 alert hi").unwrap().1;
        assert_eq!(n.alpha(), 0.0);
        n.left = 4.0;
        assert_eq!(n.alpha(), 1.0);
        n.left = 0.2;
        assert!((n.alpha() - 0.5).abs() < 1e-6);
        assert_eq!(Notice::parse("x 5 action hi").unwrap().1.kind, NoticeKind::Action);
        // four at most: the oldest fade out to make room
        let mut list = Vec::new();
        for k in 0..6 {
            push_notice(&mut list, Notice::parse(&format!("{k} 5 info n{k}")).unwrap().1);
        }
        let live: Vec<&str> = list.iter().filter(|n| n.left > NOTICE_OUT).map(|n| n.text.as_str()).collect();
        assert_eq!(live, ["n2", "n3", "n4", "n5"]);
        // the same text again stands once, for its time again, without fading in anew
        let mut list = vec![Notice::new(NoticeKind::Action, "Doors are open")];
        list[0].left = 2.0;
        push_notice(&mut list, Notice::new(NoticeKind::Action, "Doors are open"));
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].left, list[0].alpha()), (7.0, 1.0));
    }

    /// What the game tells the driver becomes a card when it starts, not every frame, and
    /// again once it was gone for a while.
    #[test]
    fn a_note_becomes_a_notification_once_when_it_starts() {
        let mut w = NoteWatch::default();
        let mut list = Vec::new();
        let brake = |v: f32| vec![(note_key(&format!("Air pressure is low ({v:.1} bar)")), format!("Air pressure is low ({v:.1} bar)"), NoticeKind::Action)];
        assert_eq!(note_key("Parking brake is on  (. releases it)"), "Parking brake is on");
        for k in 0..100 {
            // (the pressure in the text changes, the condition is the same)
            w.frame(0.016, &brake(4.0 + k as f32 * 0.01), &mut list);
        }
        assert_eq!(list.len(), 1);
        // gone a moment (the throttle let go) and back: the same reason
        w.frame(1.0, &[], &mut list);
        w.frame(0.016, &brake(5.0), &mut list);
        assert_eq!(list.len(), 1);
        // gone for longer, and back: shown again
        w.frame(NoteWatch::REARM + 0.5, &[], &mut list);
        list.clear();
        w.frame(0.016, &brake(5.0), &mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, NoticeKind::Action);
    }

    /// The loading bar goes along the expected time evenly, never stands still or goes back,
    /// takes the real count when that is ahead, runs to its end in about a third of a second
    /// once the world is there, and then fades.
    #[test]
    fn the_loading_bar_follows_the_expected_time_and_fades() {
        assert_eq!(time_progress(0.0), 0.0);
        assert!((time_progress(1.0) - 0.92).abs() < 1e-6);
        let mut prev = 0.0;
        for k in 1..=60 {
            let p = time_progress(k as f32 * 0.05);
            assert!(p > prev && p < 0.98, "{k}: {p}");
            prev = p;
        }
        // half the time: about half the bar
        assert!((time_progress(0.5) - 0.53).abs() < 0.03);
        // a load of 4 s expected, the count standing at none: the bar goes on evenly
        let mut l = Loading { expected: 4.0, ..Default::default() };
        let mut shown = Vec::new();
        for _ in 0..240 {
            l.step(0.0, 1.0 / 60.0);
            shown.push(l.shown());
        }
        assert!(shown.windows(2).all(|w| w[1] >= w[0]));
        assert!(shown[119] > 0.4 && shown[119] < 0.65, "{}", shown[119]);
        assert!(shown[239] > 0.8 && shown[239] <= 0.92, "{}", shown[239]);
        // the count ahead of the time: the bar follows the count
        l.step(0.97, 1.0);
        assert!(l.shown() > 0.92);
        // the world is there: the rest in about 0.35 s, then a moment, then the fade
        l.finish();
        let mut t = 0.0;
        while l.shown() < 1.0 {
            l.step(1.0, 1.0 / 60.0);
            t += 1.0 / 60.0;
        }
        assert!(t < 0.4 + 1.0 / 60.0, "{t}");
        assert_eq!(l.over_session(), Some(1.0));
        for _ in 0..60 {
            l.step(1.0, 1.0 / 60.0);
        }
        assert_eq!(l.over_session(), None);
        // a map that needed no loading screen: nothing fades
        let mut quiet = Loading::default();
        quiet.finish();
        assert_eq!(quiet.over_session(), None);
        // the last three loads, averaged
        assert_eq!(expected_of(&serde_json::json!([2.0, 4.0, 6.0])), Some(4.0));
        assert_eq!(expected_of(&serde_json::json!([])), None);
    }

    #[test]
    fn centre_hud_moves_menu_controls_and_chat_hitboxes_together() {
        let mut ui = Ui::new().unwrap();
        ui.menu_rects.push([20.0, 100.0, 900.0, 140.0]);
        ui.menu_ctl.push(Some([600.0, 110.0, 860.0, 130.0]));
        ui.menu_side.push([30.0, 100.0, 200.0, 140.0]);
        ui.dd_rects.push([650.0, 150.0, 850.0, 180.0]);
        ui.menu_arrows.push(Some([600.0, 640.0, 820.0]));
        ui.chat.rect = [10.0, 800.0, 400.0, 1000.0];
        ui.dd_scroll = Some(([850.0, 150.0, 860.0, 300.0], [844.0, 160.0, 864.0, 200.0]));
        ui.menu_pane_scroll = Some((
            [880.0, 150.0, 890.0, 700.0],
            [874.0, 160.0, 894.0, 220.0],
            40,
            10,
        ));
        ui.shift_hitboxes(1920.0);
        let track = ui.menu_ctl[0].unwrap();
        assert_eq!((2650.0 - track[0]) / (track[2] - track[0]), 0.5);
        assert!(ui.chat.contains(2020.0, 900.0));
        assert!(!ui.chat.contains(100.0, 900.0));
        assert_eq!(ui.dd_rects[0], [2570.0, 150.0, 2770.0, 180.0]);
        assert_eq!(ui.menu_arrows[0], Some([2520.0, 2560.0, 2740.0]));
        assert_eq!(ui.dd_scroll.unwrap().0[0], 2770.0);
        assert_eq!(ui.menu_pane_scroll.unwrap().1[0], 2794.0);
        ui.shift_hitboxes(-1920.0);
        assert_eq!(ui.menu_ctl[0], Some([600.0, 110.0, 860.0, 130.0]));
    }

    #[test]
    fn a_long_chat_line_goes_on_in_rows() {
        // (10 px a character)
        let m = |t: &str| t.chars().count() as f32 * 10.0;
        // fits: one row, as it was
        assert_eq!(wrap_rows("Anton: hello", 200.0, 20.0, &m), ["Anton: hello"]);
        assert_eq!(wrap_rows("", 200.0, 20.0, &m), [""]);
        // between words; the rows after the first are 20 px narrower
        let rows = wrap_rows("Admin (private): take tour 13/1 at 04:47 from the depot", 200.0, 20.0, &m);
        assert_eq!(rows, ["Admin (private):", "take tour 13/1 at", "04:47 from the", "depot"]);
        assert!(rows[0].chars().count() <= 20 && rows[1..].iter().all(|r| r.chars().count() <= 18));
        // nothing lost: the words come back in order
        assert_eq!(rows.join(" "), "Admin (private): take tour 13/1 at 04:47 from the depot");
        // a word longer than a row is cut inside
        let rows = wrap_rows("x: https://example.org/a/very/long/link", 100.0, 20.0, &m);
        assert_eq!(rows, ["x:", "https://", "example.", "org/a/ve", "ry/long/", "link"]);
        assert_eq!(rows.concat(), "x:https://example.org/a/very/long/link");
        // a box narrower than a character still ends (a character a row)
        assert_eq!(wrap_rows("abc", 5.0, 0.0, &m), ["a", "b", "c"]);
    }

    #[test]
    fn vr_settings_sidebar_keeps_back_clear() {
        for scale in [0.5, 1.0, 2.0] {
            for height in [220.0, 440.0] {
                let pages_top = (72.0 + 6.0) * scale;
                let back_top = (height - PAD - 6.0 - 38.0) * scale;
                for pages in [7, 8, 10] {
                    let step = vr_settings_sidebar_step(back_top - 6.0 * scale - pages_top, pages, scale);
                    let last_bottom = pages_top + (pages - 1) as f32 * step + (step - 4.0 * scale).max(1.0);
                    assert!(last_bottom < back_top);
                }
            }
        }
        assert_eq!(vr_settings_sidebar_step(1000.0, 8, 1.0), 42.0);
    }

    #[test]
    fn text_renders_with_an_outline() {
        let f = FontVec::try_from_vec(INTER.to_vec()).unwrap();
        let img = render_text(&f, "Savva: hi", 20.0, [255, 255, 255, 220]);
        assert!(img.width > 40 && img.height > 14);
        // white text and dark outline pixels are both there
        let px: Vec<&[u8]> = img.rgba.chunks(4).collect();
        assert!(px.iter().any(|p| p[3] > 200 && p[0] > 200));
        assert!(px.iter().any(|p| p[3] > 100 && p[0] < 40));
    }

    /// The Esc menu in Chinese, Korean and Thai: the system's fonts, not boxes.
    #[test]
    fn scripts_inter_lacks_come_from_the_system() {
        let f = FontVec::try_from_vec(INTER.to_vec()).unwrap();
        for t in ["继续", "繼續", "계속", "ดำเนินการต่อ"] {
            if t.chars().next().and_then(omsi_ui::text::fallback_font).is_none() {
                continue;
            }
            for c in t.chars() {
                let g = font_for(&f, c);
                assert!(!std::ptr::eq(g, &f) && g.glyph_id(c).0 != 0, "{t}: {c}");
            }
            let img = render_text(&f, t, 20.0, [255, 255, 255, 220]);
            let ink = img.rgba.chunks(4).filter(|p| p[3] > 128 && p[0] > 128).count();
            assert!(ink > 30, "{t}: {ink}");
        }
    }

    /// Laid out for 1080p: a lower window as it always was, a taller one in proportion.
    #[test]
    fn the_interface_grows_with_tall_windows() {
        assert_eq!(size_factor(900.0, 1.0, 1.0, true), 1.0);
        assert_eq!(size_factor(1080.0, 1.0, 1.0, true), 1.0);
        assert_eq!(size_factor(2160.0, 1.0, 1.0, true), 2.0);
        assert_eq!(size_factor(4320.0, 1.0, 1.0, true), 2.0);
        // (4K at 150 % display scaling is 1440 logical lines: with the screen's scale the
        // same size as at 100 %)
        assert!((1.5 * size_factor(2160.0, 1.5, 1.0, true) - 2.0).abs() < 1e-5);
        assert_eq!(size_factor(1080.0, 1.0, 1.5, true), 1.5);
        assert_eq!(size_factor(2160.0, 1.0, 0.5, true), 1.0);
        // (switched off: the size alone)
        assert_eq!(size_factor(2160.0, 1.0, 1.0, false), 1.0);
        assert_eq!(size_factor(2160.0, 1.0, 1.25, false), 1.25);
    }

    /// A thinned panel's light texts get an outline; dark ones - the highlighted menu line's,
    /// on the solid accent - do not (it came out as a smear round them, like a shadow).
    #[test]
    fn panel_texts_get_an_outline_only_where_it_helps() {
        // (full panels: as asked)
        assert_eq!(outline_for([235, 235, 235, 0], 1.0), 0);
        // (thinned: the light text gets one, the dark text none)
        assert!(outline_for([235, 235, 235, 0], 0.47) > 100);
        assert!(outline_for([140, 140, 140, 0], 0.47) > 100);
        assert_eq!(outline_for([20, 20, 20, 0], 0.47), 0);
        // (texts with an outline of their own keep it)
        assert_eq!(outline_for([255, 255, 255, 235], 0.47), 235);
    }

    /// The opacity's default is the design; below it the backgrounds fade, never quite away.
    #[test]
    fn backgrounds_follow_the_opacity_setting() {
        assert_eq!(backdrop(0.85), 1.0);
        assert!((backdrop(0.425) - 0.5).abs() < 1e-6);
        assert_eq!(backdrop(0.2), 0.3);
        assert!(backdrop(1.0) > 1.0);
    }

    #[test]
    fn the_game_menu_grows_with_the_interface_size_as_far_as_it_fits() {
        // (it ignored the setting: 1.5 drew the menu as 1.0 while the HUD grew)
        assert_eq!(super::menu_scale_for(1.0, 1.0, 1280.0, 720.0, false), 1.0);
        let s = super::menu_scale_for(1.0, 1.5, 1280.0, 720.0, false);
        assert!(s > 1.1 && s < 1.5 && 880.0 * s + 24.0 <= 1280.0 && 600.0 * s <= 720.0 * 0.94 + 0.01, "{s}");
        assert_eq!(super::menu_scale_for(1.0, 1.5, 3840.0, 2160.0, false), 1.5);
        // never smaller than it always was, and the headset keeps its own size
        assert_eq!(super::menu_scale_for(1.0, 1.5, 800.0, 500.0, false), 1.0);
        assert_eq!(super::menu_scale_for(1.0, 1.5, 1920.0, 1080.0, true), 1.0);
    }

    #[test]
    fn destinations_split_into_code_and_name() {
        assert_eq!(super::code_and_name(" 13  BETRIEBSFAHRT"), Some(("13", "BETRIEBSFAHRT")));
        assert_eq!(super::code_and_name("1001  Blanko.tga"), Some(("1001", "Blanko.tga")));
        assert_eq!(super::code_and_name("Route number: 5..."), None);
        assert_eq!(super::code_and_name("Line 76  (1 tours)"), None);
    }

    #[test]
    fn chat_filter_stars_out_swearing() {
        assert_ne!(filter_chat("you are a fucking idiot"), "you are a fucking idiot");
        assert_eq!(filter_chat("next stop Rathaus Spandau"), "next stop Rathaus Spandau");
    }
}

impl Ui {
    fn draw_path_manager(&mut self,r:&Renderer,scene:&mut Scene,w:&mut crate::traffic_editor::Window,s:f32,x:f32,y:f32,cursor:(f32,f32)) {
        use crate::traffic_editor::Command;
        let m=w.manager.as_ref().unwrap();
        let rect=|a:f32,b:f32,ww:f32,h:f32|[x+a*s,y+b*s,x+(a+ww)*s,y+(b+h)*s];
        self.put(r,scene,"Select / edit AI paths",((15.0*s)as u32).max(1)|BOLD,WHITE,x+12.0*s,y+18.0*s);
        let selected=m.selected.as_ref().is_some();
        let title=if let Some(l)=m.selected.as_ref(){let k=l.key.unwrap();
            format!("{} {} / {} · {}",omsi_ui::tr(if l.source==1 {"Road"}else{"Object"}),k.id,k.path+1,omsi_ui::tr(if l.reversed{"Backward"}else{"Forward"}))
        }else{omsi_ui::tr("Click a blue AI path to select it").into_owned()};
        self.catalog_text(r,scene,&title,(x+12.0*s,y+42.0*s),386.0*s,12.0*s,WHITE);
        self.catalog_text(r,scene,"Green: bend. Purple: move start. Yellow: move end.",(x+12.0*s,y+78.0*s),386.0*s,12.0*s,MUTED);
        for (area,title,cmd,enabled) in [
            (rect(10.0,120.0,390.0,30.0),"Next overlapping path (Tab)",Command::PathNext,m.candidates.len()>1),
            (rect(10.0,158.0,190.0,32.0),if m.deleted{"Restore AI path"}else{"Delete AI path (Del)"},Command::PathDelete,selected),
            (rect(210.0,158.0,190.0,32.0),if m.editing{"Finish curve editing"}else{"Edit curve"},Command::PathEdit,selected&&!m.deleted&&m.selected.as_ref().is_some_and(|l|l.points.len()>=2)),
            (rect(10.0,198.0,390.0,30.0),"Restore original curve",Command::PathReset,selected&&!m.deleted),
            (rect(10.0,236.0,190.0,30.0),"Undo (Ctrl+Z)",Command::PathUndo,m.can_undo),
            (rect(210.0,236.0,190.0,30.0),"Redo (Ctrl+Y)",Command::PathRedo,m.can_redo),
            (rect(10.0,274.0,390.0,32.0),"Save map (Ctrl+S)",Command::PathSave,true),
            (rect(10.0,314.0,390.0,30.0),"AI paths along the selected road",Command::Generate,m.selected.as_ref().is_some_and(|l|l.source==1)),
            (rect(10.0,352.0,390.0,30.0),"Close (Esc)",Command::Close,true),
        ] {
            self.editor_surface(r,scene,area,3.0*s,if !enabled{PANEL_ALT}else if editor_contains(area,cursor){LIT}else{CHIP});
            self.catalog_text(r,scene,title,(area[0]+6.0*s,area[1]+8.0*s),area[2]-area[0]-12.0*s,12.0*s,WHITE);
            if enabled{w.rects.push((area,cmd));}
        }
        self.catalog_text(r,scene,"Save, then reload the map to update AI traffic.",(x+12.0*s,y+402.0*s),386.0*s,12.0*s,MUTED);
        if let Some(error)=w.error.as_ref(){self.catalog_text(r,scene,error,(x+12.0*s,y+440.0*s),386.0*s,12.0*s,[255,102,76,0]);}
    }
}
