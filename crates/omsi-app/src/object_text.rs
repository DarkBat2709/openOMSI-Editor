//! Modal editing of a scenery object's map strings; camera shortcuts never see typing.
use crate::scene::ObjectType;

#[derive(Clone, Copy)]
pub enum Target { Map(i64), Added(usize) }
#[derive(Clone, Copy)]
pub enum Command { Field(usize), Page(i32), Apply, Undo, Close }

pub struct Window {
    pub target: Target,
    pub name: String,
    pub fields: Vec<(usize, String)>,
    pub values: Vec<String>,
    pub active: usize,
    pub replace: bool,
    pub message: String,
    pub rects: Vec<([f32; 4], Command)>,
}
pub fn fields(ot: &ObjectType) -> Vec<(usize, String)> {
    if ot.sco.tree.is_some() { return Vec::new(); }
    let mut fields = std::collections::BTreeMap::new();
    if let Some(program) = ot.program.as_ref().filter(|_| !ot.model.text_textures.is_empty()) {
        for (slot, name) in program.str_var_names.iter().enumerate().take(4096) {
            fields.insert(slot, format!("{} · Field {}", name, slot + 1));
        }
    }
    for tt in &ot.model.text_textures {
        if let Ok(slot) = tt.variable.trim().parse::<usize>() {
            if slot < 4096 { fields.insert(slot, format!("Text field {} · {}", slot + 1, tt.font)); }
        }
    }
    fields.into_iter().collect()
}
impl Window {
    pub fn new(target: Target, name: String, fields: Vec<(usize, String)>, mut values: Vec<String>) -> Result<Self, String> {
        let count = fields.iter().map(|(slot, _)| slot + 1).max().unwrap_or(0);
        if count == 0 { return Err("This object has no editable text fields. Text painted onto the object belongs to its texture.".into()); }
        if count > 4096 || values.len() > 4096 { return Err("Too many text fields in object".into()); }
        values.resize(values.len().max(count), String::new());
        Ok(Self { target, name, fields, values, active: 0, replace: true, rects: Vec::new(),
            message: "Click field and type · Apply shows the text · Ctrl+S saves the map".into() })
    }
    pub fn page(&self) -> usize { self.active / 6 }
    pub fn select(&mut self, index: usize) { if index < self.fields.len() { self.active = index; self.replace = true; } }
    pub fn next(&mut self, delta: i32) {
        self.select((self.active as i64 + delta as i64).rem_euclid(self.fields.len() as i64) as usize);
    }
    pub fn type_text(&mut self, text: &str) {
        let typed: String = text.chars().filter(|c| !c.is_control()).collect();
        if typed.is_empty() { return; }
        let slot = self.fields[self.active].0; let value = &mut self.values[slot];
        if self.replace { value.clear(); self.replace = false; }
        let remaining = 512usize.saturating_sub(value.chars().count());
        value.extend(typed.chars().take(remaining));
    }
    pub fn erase(&mut self, all: bool) {
        let value = &mut self.values[self.fields[self.active].0];
        if all || self.replace { value.clear(); } else { value.pop(); }
        self.replace = false;
    }
    pub fn hit(&self, p: (f32, f32)) -> Option<Command> {
        self.rects.iter().rev().find(|(r, _)| p.0 >= r[0] && p.0 <= r[2] && p.1 >= r[1] && p.1 <= r[3]).map(|(_, c)| *c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_slots_unicode_replacement_and_control_characters() {
        let mut w = Window::new(Target::Map(5), "Schild".into(), vec![(2,"Ort".into()),(4,"Kreis".into())], vec!["intern".into()]).unwrap();
        w.type_text("Röthenbach\n[object]\t");
        assert_eq!(w.values[0], "intern"); assert_eq!(w.values[2], "Röthenbach[object]");
        w.next(1); w.type_text("Nürnberg"); w.erase(false);
        assert_eq!(w.values[4], "Nürnber"); w.next(-1); w.type_text("München");
        assert_eq!(w.values[2], "München");
    }
}
