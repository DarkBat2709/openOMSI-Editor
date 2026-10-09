//! Native editor asset catalogue. Disk scanning, model loading and thumbnail rendering
//! run on one cancellable worker, without touching the live scene or its type caches.
use glam::{DVec2, Vec2, Vec3};
use hashbrown::{HashMap, HashSet};
use omsi_geometry::{MeshData, SplineCurve};
use omsi_texture::Image;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc};

pub const PAGE_SIZE: usize = 12;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Spline, Object, Texture }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section { Roads, Objects, Textures }
impl Section {
    pub fn categories(self) -> &'static [Category] {
        match self {
            Self::Roads => &[Category::All, Category::Profiles, Category::Junctions, Category::Traffic, Category::Water],
            Self::Objects => &[Category::All, Category::Vegetation, Category::Buildings,
                Category::Street, Category::Water, Category::Other],
            Self::Textures => &[Category::All,Category::Grass,Category::Soil,Category::Gravel,Category::Stone,Category::Asphalt,Category::Other],
        }
    }
    fn contains(self, asset: &Asset) -> bool {
        let road = asset.kind == Kind::Spline || matches!(asset.category, Category::Junctions | Category::Traffic);
        match self { Self::Roads => road, Self::Objects => !road && asset.kind!=Kind::Texture, Self::Textures=>asset.kind==Kind::Texture }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category { All, Profiles, Vegetation, Buildings, Junctions, Traffic, Street, Water, Other, Grass, Soil, Gravel, Stone, Asphalt }
impl Category {
    pub fn title(self)->&'static str {match self {Self::All=>"Alle",Self::Profiles=>"Spline-Profile",Self::Vegetation=>"Vegetation",Self::Buildings=>"Gebäude",
        Self::Junctions=>"Kreuzungen",Self::Traffic=>"Verkehrslogik",Self::Street=>"Straßenzubehör",Self::Water=>"Gewässer",Self::Other=>"Sonstiges",
        Self::Grass=>"Wiesen",Self::Soil=>"Erde",Self::Gravel=>"Schotter",Self::Stone=>"Steine",Self::Asphalt=>"Asphalt"}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort { Name, Category, Path }
impl Sort {pub fn title(self)->&'static str {match self {Self::Name=>"Name",Self::Category=>"Kategorie",Self::Path=>"Pfad"}}}
#[derive(Clone)]
pub struct Asset { pub kind: Kind, pub file: String, pub name: String, pub path: PathBuf, pub category:Category, pub groups:String }
#[derive(Clone, Copy)]
pub enum Command { Section(Section), Category(Category), Sort(Sort), Reverse, Select(usize), Page(i32), Rotate(i32), ResetView, Audit, Choose, Close, Clear, BuildJunction, TextureInfo, InfoStep(i32) }
pub struct SplineTextureInfo {pub name:String,pub path:String,pub usage:String,pub image:Result<Image,String>}
enum Reply { Details(u64,usize,Vec<SplineTextureInfo>), Scanned(Vec<Asset>), Preview(u64, usize, Result<Image, String>) }
struct Request { epoch: u64, index: usize, asset: Asset, view: u8 }

pub struct Catalog {
    pub texture_only:bool,
    pub info_open:bool,pub info_slot:usize,pub details:HashMap<usize,Vec<SplineTextureInfo>>,
    pub section: Section,
    pub category: Category,
    pub sort: Sort,
    pub descending: bool,
    pub query: String,
    pub entries: Vec<Asset>,
    pub filtered: Vec<usize>,
    pub selected: Option<usize>,
    pub page: usize,
    pub scanning: bool,
    pub audit_message: String,
    pub previews: HashMap<usize, Result<Image, String>>,
    pub rects: Vec<([f32; 4], Command)>,
    pending: HashSet<usize>,
    rx: mpsc::Receiver<Reply>,
    tx: mpsc::Sender<Request>,
    cancelled: Arc<AtomicBool>,
    epoch: Arc<AtomicU64>,
    view: u8,
    root: PathBuf,
    audit: Option<Audit>,
}

struct Audit { stop: Arc<AtomicBool>, rx: mpsc::Receiver<String>, running: bool }

impl Drop for Catalog {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(audit) = &self.audit { audit.stop.store(true, Ordering::Relaxed); }
    }
}
impl Catalog {
    pub fn with_map(root:PathBuf,kind:Kind,map_dir:Option<PathBuf>) -> Self {
        let (tx, requests) = mpsc::channel::<Request>();
        let (results, rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let epoch = Arc::new(AtomicU64::new(0));
        let stop = cancelled.clone(); let generation = epoch.clone();
        let worker_root = root.clone();
        std::thread::spawn(move || {
            let root = worker_root;
            let mut entries = if kind==Kind::Texture {Vec::new()} else {scan(&root, &stop)};
            entries.extend(scan_textures(&root,map_dir.as_deref(),&stop));
            if stop.load(Ordering::Relaxed) || results.send(Reply::Scanned(entries)).is_err() { return; }
            while let Ok(req) = requests.recv() {
                if stop.load(Ordering::Relaxed) { break; }
                if req.epoch != generation.load(Ordering::Relaxed) { continue; }
                if req.asset.kind==Kind::Spline {
                    let info=spline_texture_info(&root,&req.asset.path,&stop);
                    if results.send(Reply::Details(req.epoch,req.index,info)).is_err() {break;}
                }
                let image = preview(&root, &req.asset, req.view, &stop);
                if stop.load(Ordering::Relaxed) { break; }
                if results.send(Reply::Preview(req.epoch, req.index, image)).is_err() { break; }
            }
        });
        Self { info_open:false,info_slot:0,details:HashMap::new(),texture_only:kind==Kind::Texture,section: match kind {Kind::Spline=>Section::Roads,Kind::Object=>Section::Objects,Kind::Texture=>Section::Textures}, category:Category::All,sort:Sort::Name,descending:false,query: String::new(), entries: Vec::new(), filtered: Vec::new(), selected: None,
            page: 0, scanning: true, audit_message: String::new(), previews: HashMap::new(), rects: Vec::new(), pending: HashSet::new(),
            rx, tx, cancelled, epoch, view: 0, root, audit: None }
    }
    pub fn poll(&mut self) {
        if let Some(audit) = &mut self.audit {
            loop { match audit.rx.try_recv() {
                Ok(status) => self.audit_message = status,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if audit.running && self.audit_message.starts_with("Prüfe ") {
                        self.audit_message = "Prüfung unerwartet beendet · bisherige Tabellen und game.log prüfen".into();
                    }
                    audit.running = false; break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            } }
            if !self.audit_message.starts_with("Prüfe ") { audit.running = false; }
        }
        while let Ok(reply) = self.rx.try_recv() {
            match reply {
                Reply::Details(epoch,index,info) if epoch==self.epoch.load(Ordering::Relaxed)=>{self.details.insert(index,info);}
                Reply::Scanned(entries) => { self.entries = entries; self.scanning = false; self.filter(); }
                Reply::Preview(epoch, index, image) if epoch == self.epoch.load(Ordering::Relaxed) => {
                    if let Err(error) = &image { log::warn!("Katalog-Vorschau {}: {error}", self.entries[index].file); }
                    self.pending.remove(&index); self.previews.insert(index, image);
                }
                _ => {}
            }
        }
        if self.previews.len() > 48 {
            let keep: HashSet<_> = self.visible().into_iter().chain(self.selected).collect();
            self.previews.retain(|k, _| keep.contains(k));self.details.retain(|k,_|keep.contains(k));
        }
        for index in self.selected.into_iter().chain(self.visible()) {
            if self.previews.contains_key(&index) || !self.pending.insert(index) { continue; }
            let req = Request { epoch: self.epoch.load(Ordering::Relaxed), index, asset: self.entries[index].clone(), view: self.view };
            if self.tx.send(req).is_err() { self.pending.remove(&index); }
        }
    }
    pub fn visible(&self) -> Vec<usize> { self.filtered.iter().skip(self.page * PAGE_SIZE).take(PAGE_SIZE).copied().collect() }
    pub fn audit_running(&self) -> bool { self.audit.as_ref().is_some_and(|a| a.running) }
    fn start_audit(&mut self) {
        if self.audit_running() {
            self.audit.as_ref().unwrap().stop.store(true, Ordering::Relaxed);
            self.audit_message = "Prüfe … wird beendet; bisherige Ergebnisse bleiben gespeichert".into();
            return;
        }
        if self.scanning { return; }
        let folder = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(".")) .join("logs")
            .join(format!("vorschau-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        let root = self.root.clone(); let entries = self.entries.clone();
        let stop = Arc::new(AtomicBool::new(false)); let worker_stop = stop.clone();
        let (tx, rx) = mpsc::channel();
        self.audit_message = format!("Prüfe 0 / {} Bauteile · Katalog geöffnet lassen", entries.len());
        self.audit = Some(Audit { stop, rx, running: true });
        std::thread::spawn(move || {
            match audit_previews(&root, &entries, &folder, &worker_stop, &tx) {
                Ok((done, failed)) => {
                    let status = if worker_stop.load(Ordering::Relaxed) { "Abgebrochen" } else { "Fertig" };
                    let message = format!("{status}: {done} geprüft, {failed} ohne Vorschau · {}", folder.display());
                    log::info!("Katalog-Diagnose: {message}"); let _ = tx.send(message);
                }
                Err(error) => { let _ = tx.send(format!("Diagnose nicht gespeichert: {error}")); }
            }
        });
    }
    pub fn pages(&self) -> usize { self.filtered.len().div_ceil(PAGE_SIZE).max(1) }
    pub fn image_key(&self, index: usize) -> String { format!("{}:{}", self.view, self.entries[index].file) }
    fn invalidate(&mut self) { self.epoch.fetch_add(1, Ordering::Relaxed); self.pending.clear(); }
    fn set_view(&mut self, view: u8) {
        if self.view == view { return; }
        self.view = view; self.previews.clear(); self.invalidate();
    }
    fn filter(&mut self) {
        let query = self.query.to_lowercase().replace('\\', "/");
        self.filtered = filter_assets(&self.entries,self.section,self.category,&query,self.sort,self.descending);
        self.page = 0; self.selected = self.filtered.first().copied(); self.invalidate();
    }
    pub fn type_text(&mut self, text: &str) {
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        if text.is_empty() { return; }
        let remaining = 100usize.saturating_sub(self.query.chars().count());
        self.query.extend(text.chars().take(remaining)); self.filter();
    }
    pub fn backspace(&mut self) { self.query.pop(); self.filter(); }
    pub fn command(&mut self, command: Command) {
        match command {
            Command::TextureInfo=>{self.info_open=!self.info_open;self.info_slot=0;},
            Command::InfoStep(delta)=>{let n=self.selected.and_then(|i|self.details.get(&i)).map_or(0,Vec::len);
                if n>0 {self.info_slot=(self.info_slot as i64+delta as i64).rem_euclid(n as i64) as usize;}},
            Command::Section(section) => {
                if self.texture_only && section!=Section::Textures {return;}
                self.section = section;
                if !section.categories().contains(&self.category) { self.category = Category::All; }
                self.filter();
            }
            Command::Category(category) if self.section.categories().contains(&category)=>{self.category=category;self.filter();}
            Command::Rotate(delta) => self.set_view((self.view as i64 + delta as i64).rem_euclid(4) as u8),
            Command::ResetView => self.set_view(0),
            Command::Audit => self.start_audit(),
            Command::Sort(sort)=>{self.sort=sort;self.filter();}
            Command::Reverse=>{self.descending=!self.descending;self.filter();}
            Command::Clear => { self.query.clear(); self.filter(); }
            Command::Select(index) if self.filtered.contains(&index) => self.selected = Some(index),
            Command::Page(delta) => {
                self.page = (self.page as i64 + delta as i64).clamp(0, self.pages() as i64 - 1) as usize;
                self.selected = self.visible().first().copied(); self.invalidate();
            }
            _ => {}
        }
    }
    pub fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() { return; }
        let cur = self.selected.and_then(|s| self.filtered.iter().position(|i| *i == s)).unwrap_or(0);
        let next = (cur as i64 + delta as i64).clamp(0, self.filtered.len() as i64 - 1) as usize;
        self.selected = Some(self.filtered[next]);
        if self.page != next / PAGE_SIZE { self.page = next / PAGE_SIZE; self.invalidate(); }
    }
    pub fn chosen(&self) -> Option<Asset> { self.selected.and_then(|i| self.entries.get(i)).cloned() }
    pub fn hit(&self, p: (f32, f32)) -> Option<Command> {
        self.rects.iter().rev().find(|(r, _)| p.0 >= r[0] && p.0 <= r[2] && p.1 >= r[1] && p.1 <= r[3]).map(|(_, c)| *c)
    }
}

fn report_cell(text: &str) -> String { text.replace(['\t', '\r', '\n'], " ") }
/// Stream results to disk; keep only one thumbnail in memory and never reload the map.
fn spline_texture_info(root:&Path,path:&Path,stop:&AtomicBool)->Vec<SplineTextureInfo> {
    let def=match omsi_scenery::Spline::load(path) {Ok(def)=>def,Err(e)=>return vec![SplineTextureInfo {
        name:"Profil nicht lesbar".into(),path:path.display().to_string(),usage:String::new(),image:Err(e.to_string())}]};
    let dirs=crate::scene::texture_dirs(root,path.parent().unwrap_or(root));
    let refs:Vec<&Path>=dirs.iter().map(|p|p.as_path()).collect();
    def.textures.iter().enumerate().take_while(|_|!stop.load(Ordering::Relaxed)).map(|(slot,t)| {
        let resolved=omsi_texture::find_texture(&t.file,&refs);
        let image=resolved.as_ref().ok_or_else(||"Textur fehlt".to_string()).and_then(|p|
            omsi_texture::decode_file(p).map(thumbnail_texture).map_err(|e|e.to_string()));
        let profiles:Vec<_>=def.profiles.iter().enumerate().filter(|(_,p)|p.texture==slot).map(|(i,_)|(i+1).to_string()).collect();
        SplineTextureInfo {name:t.file.clone(),path:resolved.map(|p|p.display().to_string()).unwrap_or_else(||format!("Nicht gefunden: {}",t.file)),
            usage:format!("Slot {} · Profile: {}{}{}",slot,profiles.join(", "),if t.patchwork.is_some() {" · Patchwork"} else {""},
                if t.scale_by_length {" · längenabhängig"} else {""}),image}
    }).collect()
}

fn audit_previews(root: &Path, entries: &[Asset], folder: &Path, stop: &AtomicBool,
    progress: &mpsc::Sender<String>) -> Result<(usize, usize), String> {
    use std::io::Write;
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut all = std::io::BufWriter::new(std::fs::File::create(folder.join("alle.tsv")).map_err(|e| e.to_string())?);
    let mut errors = std::io::BufWriter::new(std::fs::File::create(folder.join("fehler.tsv")).map_err(|e| e.to_string())?);
    let header = "Status\tTyp\tKategorie\tName\tDateipfad\tOriginalpfad\tFehler\n";
    all.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
    errors.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
    let mut done = 0; let mut failed = 0;
    for asset in entries {
        if stop.load(Ordering::Relaxed) { break; }
        let result = preview(root, asset, 0, stop);
        if stop.load(Ordering::Relaxed) { break; }
        let (status, error) = match result { Ok(_) => ("OK", String::new()), Err(error) => ("KEINE_VORSCHAU", error) };
        let row = format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\n", status,
            match asset.kind {Kind::Spline=>"Spline",Kind::Object=>"Objekt",Kind::Texture=>"Textur"}, asset.category.title(),
            report_cell(&asset.name), report_cell(&asset.file), report_cell(&asset.path.to_string_lossy()), report_cell(&error));
        all.write_all(row.as_bytes()).map_err(|e| e.to_string())?;
        if !error.is_empty() { failed += 1; errors.write_all(row.as_bytes()).map_err(|e| e.to_string())?; }
        done += 1;
        if done % 25 == 0 || done == entries.len() {
            all.flush().map_err(|e| e.to_string())?; errors.flush().map_err(|e| e.to_string())?;
            let _ = progress.send(format!("Prüfe {done} / {} Bauteile · {failed} ohne Vorschau", entries.len()));
        }
    }
    all.flush().map_err(|e| e.to_string())?; errors.flush().map_err(|e| e.to_string())?;
    Ok((done, failed))
}

fn filter_assets(entries:&[Asset],section:Section,category:Category,query:&str,sort:Sort,descending:bool)->Vec<usize> {
    if !section.categories().contains(&category) { return Vec::new(); }
    let words:Vec<_>=query.split_whitespace().collect();
    let mut indices:Vec<_>=entries.iter().enumerate().filter(|(_,a)|section.contains(a)
        && (category==Category::All||a.category==category)
        && words.iter().all(|word|format!("{} {} {}",a.file,a.name,a.groups).to_lowercase().replace('\\',"/").contains(*word)))
        .map(|(i,_)|i).collect();
    indices.sort_by_cached_key(|&i|{let a=&entries[i];let main=match sort {Sort::Name=>a.name.as_str(),Sort::Category=>a.category.title(),Sort::Path=>a.file.as_str()};
        (main.to_lowercase(),a.name.to_lowercase(),a.file.to_lowercase())});
    if descending {indices.reverse();}indices
}

fn category_words(text: &str) -> Vec<String> {
    // Match whole words, including underscore/digit separated asset names. Author
    // names such as Waldheini and Busdrivers must never match "wald" or "river".
    text.to_lowercase().replace('ä', "ae").replace('ö', "oe")
        .replace('ü', "ue").replace('ß', "ss")
        .split(|c: char| !c.is_alphabetic()).filter(|s| !s.is_empty())
        .map(str::to_owned).collect()
}
fn has_category_word(words: &[String], keys: &[&str]) -> bool {
    words.iter().any(|word| keys.contains(&word.as_str()))
}
fn street_component(words: &[String]) -> bool {
    has_category_word(words, &["wegweiser", "wegweisers", "sign", "signs", "signpost", "signposts",
        "schild", "schilder", "autobahnschild", "autobahnschilder", "bundesstrassenschild",
        "bundesstrassenschilder", "verkehrsschild", "verkehrsschilder", "verkehrszeichen", "beschilderung",
        "ortstafel", "ortstafeln", "ortshinweis", "flusshinweis", "ampel", "ampeln", "ampelobjekt",
        "ampelobjekte", "ampelobjects", "ampelmast", "ampelmasten", "ampelanlage", "ampelanlagen",
        "trafficlight", "trafficlights", "pfeil", "pfeile", "arrow", "arrows", "markierung",
        "markierungen", "roadmarking", "roadmarkings", "zone", "tonnen", "achtung", "fahne",
        "fahnen", "flag", "flags", "logo", "logos", "dummy", "dummyobjekt", "hilfsobjekt", "hilfsobjekte"])
        || (has_category_word(words, &["abfahrt", "auffahrt"])
            && has_category_word(words, &["ab", "autobahn", "bundesstrasse", "spurig"]))
}
fn label_category(words: &[String], object_name: bool) -> Option<Category> {
    if street_component(words) { return Some(Category::Street); }
    if has_category_word(words, &["vegetation", "baum", "baeume", "baeumen", "laubbaum", "laubbaeume",
        "nadelbaum", "nadelbaeume", "strassenbaum", "baumreihe", "tree", "trees", "treeobjects",
        "bush", "bushes", "busch", "buesche", "shrub", "shrubs", "pflanze", "pflanzen", "plant",
        "plants", "hecke", "hecken", "hedge", "hedges", "grass", "gras", "forest", "wald"]) {
        return Some(Category::Vegetation);
    }
    if has_category_word(words, &["gewaesser", "water", "waterobjects", "wasser", "wasserobjekte",
        "wasserflaeche", "river", "rivers", "fluss", "fluesse", "lake", "lakes", "see", "seen",
        "teich", "teiche", "pond", "ponds", "bach", "baeche", "stream", "streams", "canal", "kanal",
        "ufer", "shore"]) { return Some(Category::Water); }
    if has_category_word(words, &["kreuzung", "kreuzungen", "crossing", "crossings", "junction",
        "junctions", "intersection", "intersections", "einmuendung", "einmuendungen", "roundabout",
        "roundabouts", "kreisverkehr"]) { return Some(Category::Junctions); }
    // Location names like "Kreuz Rathaus" are not building categories. Singular
    // building types are useful in the asset's own name, but not in folder hints.
    if has_category_word(words, &["gebaeude", "building", "buildings", "buildingobjects", "houses",
        "haeuser", "wohngebaeude", "gewerbegebaeude"])
        || (object_name && has_category_word(words, &["house", "haus", "rathaus", "baumhaus", "huette",
            "waldhuette", "shed", "kirche", "church", "garage", "factory", "fabrik", "bahnhof"])) {
        return Some(Category::Buildings);
    }
    if has_category_word(words, &["streetobjects", "streetfurniture", "strassenobjekte", "strassenzubehoer",
        "strassen", "strasse", "street", "road", "verkehr", "lamp", "lamps", "light", "lights",
        "laterne", "laternen", "fence", "fences", "zaun", "zaeune", "bench", "benches", "bank",
        "haltestelle", "haltestellen", "busstop", "busstops", "shelter", "bruecke", "bridge"]) {
        return Some(Category::Street);
    }
    if object_name && has_category_word(words, &["ahorn", "birke", "birken", "eiche", "eichen", "buche",
        "buchen", "kiefer", "kiefern", "tanne", "tannen", "fichte", "fichten", "maple", "birch",
        "oak", "pine", "spruce"]) { return Some(Category::Vegetation); }
    None
}
fn classify(file: &str, name: &str, groups: &str, tree: bool) -> Category {
    if tree { return Category::Vegetation; }
    let parts: Vec<_> = file.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let stem = parts.last().copied().unwrap_or("").rsplit_once('.')
        .map(|(stem, _)| stem).unwrap_or_else(|| parts.last().copied().unwrap_or(""));
    let name_words = category_words(name);
    let file_words = category_words(stem);
    let group_words: Vec<_> = groups.split('/').map(category_words).collect();
    // Ignore Sceneryobjects/Splines and the add-on's top-level author/package
    // folder. Remaining hints are considered separately, closest folder first.
    let folder_words: Vec<_> = parts.iter().skip(2).take(parts.len().saturating_sub(3))
        .rev().map(|s| category_words(s)).collect();
    // A sign about a river, roundabout or town hall is still street furniture.
    // This also identifies numeric signal assets inside AMPELOBJEKTE folders.
    if street_component(&name_words) || street_component(&file_words) {
        return Category::Street;
    }
    label_category(&name_words, true)
        .or_else(|| label_category(&file_words, true))
        .or_else(|| folder_words.iter().chain(group_words.iter())
            .any(|w| street_component(w)).then_some(Category::Street))
        .or_else(|| group_words.iter().rev().find_map(|w| label_category(w, false)))
        .or_else(|| folder_words.iter().find_map(|w| label_category(w, false)))
        .unwrap_or(Category::Other)
}

fn asset_metadata(path:&Path,kind:Kind,file:String,fallback:String)->Asset {
    // Parse only catalogue metadata, not full models, on the scanning worker.
    let mut name=fallback;let mut groups=Vec::new();let mut tree=false;
    let mut light=false;let mut geometry=false;let mut paths=false;let mut editor_only=false;
    if let Ok(cfg)=omsi_cfg::CfgFile::read(path) {
        let mut r=cfg.reader().disabled_blocks();
        while let Some(k)=r.next_keyword() {match k.as_str() {
            "friendlyname"=>{let text=r.str().trim();if !text.is_empty() {name=text.to_string();}},
            "groups"=>{let count=r.usize().min(64);for _ in 0..count {groups.push(r.str().to_string());}},
            "tree"=>tree=true,"maplight"=>light=true,"mesh"|"model"=>geometry=true,
            "path"|"path_2"=>paths=true,"onlyeditor"=>editor_only=true,_=>{},
        }}
    }
    let groups=groups.join(" / ");let category=classify_asset(kind,&file,&name,&groups,tree,light,geometry,paths,editor_only);
    Asset {kind,file,name,path:path.to_path_buf(),category,groups}
}
fn classify_asset(kind: Kind, file: &str, name: &str, groups: &str, tree: bool, light: bool, geometry: bool, paths: bool, editor_only: bool) -> Category {
    let category = classify(file, name, groups, tree);
    match kind {
        Kind::Spline => if category == Category::Water { Category::Water } else { Category::Profiles },
        Kind::Object if paths && (editor_only || !geometry) && !tree => Category::Traffic,
        Kind::Object if light && !geometry && !tree => Category::Other,
        Kind::Object => category,
        Kind::Texture=>texture_category(file),
    }
}

fn scan(root: &Path, stop: &AtomicBool) -> Vec<Asset> {
    let mut out = Vec::new(); let mut seen = HashSet::new();
    let mut roots = omsi_cfg::content_roots();
    if !roots.iter().any(|r| r == root) { roots.push(root.to_path_buf()); }
    for (folder, extension, kind) in [("Splines", "sli", Kind::Spline), ("Sceneryobjects", "sco", Kind::Object)] {
        for root in &roots {
            let dir = omsi_cfg::resolve_path(root, folder);
            let mut stack = vec![(dir, folder.to_string(), 0usize)]; let mut visited = HashSet::new();
            while let Some((dir, relative, depth)) = stack.pop() {
                if stop.load(Ordering::Relaxed) { return out; }
                if depth > 32 || !visited.insert(dir.canonicalize().unwrap_or_else(|_| dir.clone())) { continue; }
                let Some(entries) = omsi_cfg::vfs::list_dir(&dir) else { continue; };
                for (name, is_dir) in entries {
                    let path = dir.join(&name); let file = format!("{relative}/{}", name.to_string_lossy());
                    if is_dir { stack.push((path, file, depth + 1)); }
                    else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(extension)) && seen.insert(file.to_lowercase()) {
                        out.push(asset_metadata(&path,kind,file,path.file_stem().unwrap_or(&name).to_string_lossy().into_owned()));
                    }
                }
            }
        }
    }
    out.sort_by_cached_key(|a| a.file.to_lowercase()); out
}

fn texture_category(file:&str)->Category {
    let name=Path::new(file).file_stem().unwrap_or_default().to_string_lossy();let words=category_words(&name);
    if has_category_word(&words,&["grass","gras","wiese","wiesen","meadow","lawn","rasen"]) {Category::Grass}
    else if has_category_word(&words,&["gravel","schotter","kies","kiesel","ballast"]) {Category::Gravel}
    else if has_category_word(&words,&["soil","earth","erde","dirt","sand","boden","acker","mud"]) {Category::Soil}
    else if has_category_word(&words,&["stone","stones","stein","steine","rock","rocks","pflaster","cobble","cobblestone"]) {Category::Stone}
    else if has_category_word(&words,&["asphalt","asphaltierung","road","strasse","street","beton","concrete"]) {Category::Asphalt}
    else {Category::Other}
}

fn texture_key(path:&Path)->String {path.to_string_lossy().replace('\\',"/").to_lowercase()}
fn remember_surface_texture(name:&str,dirs:&[PathBuf],references:&mut HashSet<String>) {
    if name.trim().is_empty() {return;}
    let dirs:Vec<&Path>=dirs.iter().map(|p|p.as_path()).collect();
    if let Some(path)=omsi_texture::find_texture(name,&dirs) {references.insert(texture_key(&path));}
}

/// Keep explicitly used map/spline surfaces even when their names are numeric. Sign
/// graphics from object texture folders are otherwise unsuitable for the ground brush.
fn texture_is_decal(file:&str,used_surface:bool)->bool {
    if used_surface {return false;}
    let normal=file.replace('\\',"/");let words=category_words(&normal);
    if has_category_word(&words,&["verkehrszeichen","verkehrsschild","verkehrsschilder",
        "schild","schilder","schildertexturen","wegweiser","wegweisers","beschilderung",
        "ortstafel","ortstafeln","zusatzschild","zusatzschilder","zusatzzeichen",
        "sign","signs","signpost","signposts","trafficsigns","roadsigns",
        "textfeld","beschriftung","beschriftungen","font","fonts","schrift","schriften",
        "logo","logos","icon","icons"]) {return true;}
    // Preserve descriptive names such as "01_asphalt", "grass_70" and "7_gravel".
    if texture_category(&normal)!=Category::Other {return false;}
    let stem=Path::new(&normal).file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
    let head=stem.split(['_','-',' ','.']).next().unwrap_or("");
    let numeric=head.as_bytes().first().is_some_and(|byte|byte.is_ascii_digit());
    let suffix=head.trim_start_matches(|c:char|c.is_ascii_digit());
    numeric && matches!(suffix,""|"t"|"m"|"km"|"kg"|"kmh"|"gen")
}

fn scan_textures(root:&Path,map_dir:Option<&Path>,stop:&AtomicBool)->Vec<Asset> {
    let mut roots=omsi_cfg::content_roots();if !roots.iter().any(|r|r==root) {roots.push(root.to_path_buf());}
    let map_rel=map_dir.and_then(|map|roots.iter().find_map(|r|map.strip_prefix(r).ok())).map(Path::to_path_buf);
    let mut out=Vec::new();let mut seen=HashSet::new();let mut references=HashSet::new();let mut profiles=HashSet::new();
    if let Some(map)=map_dir {
        if let Ok(global)=omsi_map::GlobalCfg::load(&omsi_cfg::resolve_path(map,"global.cfg")) {
            let dirs=vec![root.to_path_buf()];
            for ground in global.ground_textures {
                remember_surface_texture(&ground.texture,&dirs,&mut references);
                remember_surface_texture(&ground.detail_texture,&dirs,&mut references);
            }
        }
    }
    for root in &roots {
        let mut folders=vec!["Texture".to_string(),"Splines".into(),"Sceneryobjects".into()];
        if let Some(map)=&map_rel {folders.push(map.join("texture").to_string_lossy().replace('\\',"/"));}
        for relative in folders {
            let mut stack=vec![(omsi_cfg::resolve_path(root,&relative),relative,0usize)];let mut visited=HashSet::new();
            while let Some((dir,relative,depth))=stack.pop() {
                if stop.load(Ordering::Relaxed) {return out;}
                if depth>32 || !visited.insert(dir.canonicalize().unwrap_or_else(|_|dir.clone())) {continue;}
                let Some(entries)=omsi_cfg::vfs::list_dir(&dir) else {continue;};
                for (name,is_dir) in entries {
                    let path=dir.join(&name);let file=format!("{relative}/{}",name.to_string_lossy());
                    if is_dir {
                        // Alpha masks and baked night maps are not paintable ground textures.
                        if relative.to_lowercase().ends_with("/texture") && name.to_string_lossy().eq_ignore_ascii_case("map") {continue;}
                        stack.push((path,file,depth+1));
                    } else if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("sli")) && profiles.insert(file.to_lowercase()) {
                        if let Ok(spline)=omsi_scenery::Spline::load(&path) {
                            let dirs=crate::scene::texture_dirs(root,&dir);
                            for texture in spline.textures {remember_surface_texture(&texture.file,&dirs,&mut references);}
                        }
                    } else if path.extension().and_then(|e|e.to_str()).is_some_and(|e|
                        ["dds","bmp","png","tga","jpg","jpeg"].iter().any(|ext|e.eq_ignore_ascii_case(ext)))
                        && seen.insert(file.to_lowercase()) {
                        let title=path.file_stem().unwrap_or(&name).to_string_lossy().into_owned();
                        out.push(Asset {kind:Kind::Texture,category:texture_category(&file),groups:String::new(),name:title,file,path});
                    }
                }
            }
        }
    }
    let found=out.len();out.retain(|asset|!texture_is_decal(&asset.file,references.contains(&texture_key(&asset.path))));
    log::info!("Texturkatalog: {} Texturen · {} Schild-/Beschriftungsbilder ausgeblendet",out.len(),found-out.len());
    out.sort_by_cached_key(|a|a.file.to_lowercase());out
}

struct Paint { diffuse: [f32; 4], texture: Option<Image>, alpha: i32, front_hint: bool }
struct Part { mesh: MeshData, paints: Vec<Paint> }
fn paint(name: &str, diffuse: [f32; 4], alpha: i32, dirs: &[PathBuf]) -> Paint {
    let refs: Vec<&Path> = dirs.iter().map(|p| p.as_path()).collect();
    let texture = if name.is_empty() { None } else {
        omsi_texture::find_texture(name, &refs).and_then(|p| omsi_texture::decode_file(&p).ok()).map(thumbnail_texture)
    };
    Paint { diffuse, texture, alpha, front_hint: false }
}
fn thumbnail_texture(image: Image) -> Image {
    let largest=image.width.max(image.height);
    if largest<=1024 || image.width==0 || image.height==0{return image;}
    let width=((image.width as u64*1024)/largest as u64).max(1)as u32;
    let height=((image.height as u64*1024)/largest as u64).max(1)as u32;
    let Some(buffer)=image::RgbaImage::from_raw(image.width,image.height,image.rgba)else{return Image{width:0,height:0,rgba:Vec::new(),has_alpha:false};};
    let rgba=image::imageops::resize(&buffer,width,height,image::imageops::FilterType::Lanczos3).into_raw();
    Image{width,height,rgba,has_alpha:image.has_alpha}
}
/// Bilinear filtering avoids visible square texels in enlarged previews.
fn preview_sample(tex:&Image,uv:Vec2)->[u8;4]{
    if tex.width==0||tex.height==0||!uv.is_finite(){return [255;4];}
    let x=uv.x.rem_euclid(1.0)*tex.width as f32-0.5;let y=uv.y.rem_euclid(1.0)*tex.height as f32-0.5;
    let ix=x.floor()as i64;let iy=y.floor()as i64;let fx=x-x.floor();let fy=y-y.floor();
    let sample=|dx:i64,dy:i64,c:usize|{let x=(ix+dx).rem_euclid(tex.width as i64)as usize;let y=(iy+dy).rem_euclid(tex.height as i64)as usize;tex.rgba.get((y*tex.width as usize+x)*4+c).copied().unwrap_or(255)as f32};
    std::array::from_fn(|c|((sample(0,0,c)*(1.0-fx)+sample(1,0,c)*fx)*(1.0-fy)+(sample(0,1,c)*(1.0-fx)+sample(1,1,c)*fx)*fy).round().clamp(0.0,255.0)as u8)
}
fn preview(root: &Path, asset: &Asset, view: u8, stop: &AtomicBool) -> Result<Image, String> {
    let dir = asset.path.parent().ok_or("Ordner fehlt")?;
    let mut parts = Vec::new();
    let yaw = preview_yaw(asset.kind, view);
    match asset.kind {
        Kind::Texture => {
            let img=thumbnail_texture(omsi_texture::decode_file(&asset.path).map_err(|e|e.to_string())?);
            if img.width==0 || img.height==0 || img.rgba.len()!=img.width as usize*img.height as usize*4 {
                return Err("Ungültiges Texturbild".into());
            }
            let (w,h)=(960usize,640usize);let mut rgba=vec![0u8;w*h*4];
            for y in 0..h {for x in 0..w {
                let sx=x*img.width as usize/w;let sy=y*img.height as usize/h;
                let p=&img.rgba[(sy*img.width as usize+sx)*4..][..4];let at=(y*w+x)*4;
                let alpha=if img.has_alpha {p[3] as f32/255.0} else {1.0};
                let grey=if (x/48+y/48)%2==0 {74.0} else {104.0};
                for c in 0..3 {rgba[at+c]=(p[c] as f32*alpha+grey*(1.0-alpha)) as u8;}rgba[at+3]=255;
            }}
            return Ok(Image {width:w as u32,height:h as u32,rgba,has_alpha:false});
        }
        Kind::Spline => {
            let def = omsi_scenery::Spline::load(&asset.path).map_err(|_| "Spline kann nicht gelesen werden")?;
            if def.only_editor || def.profiles.is_empty() { return Err("Kein sichtbares Straßenprofil".into()); }
            let s = omsi_map::MapSpline { length: 20.0, ..Default::default() };
            let curve = SplineCurve::from_map(&s, DVec2::ZERO);
            let mesh = omsi_geometry::build_spline_mesh(&def, &curve, false, glam::DVec3::ZERO);
            let dirs = crate::scene::texture_dirs(root, dir);
            let paints = def.textures.iter().map(|t| paint(&t.file, [1.0; 4], 1, &dirs)).collect();
            parts.push(Part { mesh, paints });
        }
        Kind::Object => {
            // Path-only traffic objects and editor-only controllers often have no
            // mesh, or just Dummy.x/block.x. Show their actual paths, not that marker.
            if asset.category == Category::Traffic {
                let sco = omsi_scenery::SceneryObject::load(&asset.path)
                    .map_err(|e| format!("SCO-Datei {}: {e:#}", asset.path.display()))?;
                let part = traffic_preview_part(&sco, stop).ok_or_else(||
                    "Verkehrslogik ohne darstellbare Fahrwege".to_string())?;
                return rasterize_camera(&[&part], view as f32 * std::f32::consts::FRAC_PI_2, true, stop)
                    .ok_or_else(|| "Keine darstellbare Verkehrsweg-Vorschau".into());
            }
            let (sco, _, model, model_dir) = crate::scene::scenery_definition(root, &asset.path.to_string_lossy())?;
            if let Some((texture,_,_,_,_))=&sco.tree {
                let mut mesh=crate::scene::tree_quad_mesh();
                let shape=crate::scene::editor_helper_shape(&sco,&[],[0.0,0.0]);
                for p in &mut mesh.positions {*p=shape.transform_point3(*p);}
                let dirs=crate::scene::texture_dirs(root,dir);
                parts.push(Part {mesh,paints:vec![paint(texture,[1.0;4],1,&dirs)]});
                return rasterize_auto(&parts.iter().collect::<Vec<_>>(),yaw,stop).ok_or_else(||"Keine sichtbare Baumvorschau".into());
            }
            if model.meshes.is_empty() && !sco.map_lights.is_empty() {
                return Err("Beleuchtungsobjekt ohne sichtbares Modell".into());
            }
            let dirs = crate::scene::scenery_texture_dirs(root, &sco, &model_dir);
            return object_preview(&model, &model_dir, &dirs, yaw, stop);
        }
    }
    rasterize(&parts, yaw, stop).ok_or_else(|| "Keine statische Modellvorschau verfügbar".into())
}

pub fn junction_preview(root:&Path,mesh:&omsi_o3d::Mesh,view:u8,surface:Option<&crate::junction_builder::RoadSurface>)->Result<Image,String> {
    let dirs=[root.to_path_buf()];
    let mut paints:Vec<_>=mesh.materials.iter().map(|m|paint(&m.texture,m.diffuse,0,&dirs)).collect();
    if let Some(surface)=surface {if !mesh.materials[0].texture.is_empty() {
        let path=omsi_texture::find_texture(&mesh.materials[0].texture,&[root]).ok_or("Fahrbahntextur fehlt")?;
        let image=omsi_texture::decode_file(&path).map_err(|e|e.to_string())?;
        paints[0].texture=Some(thumbnail_texture(crate::junction_builder::crop_surface(&image,surface)?));
    }}
    let part=Part {mesh:omsi_geometry::mesh_from_o3d(mesh),paints};
    rasterize_parts(&[&part],preview_yaw(Kind::Object,view),&AtomicBool::new(false))
        .ok_or_else(||"Keine Kreuzungsvorschau erzeugt".into())
}
fn preview_yaw(kind: Kind, view: u8) -> f32 {
    let base = if kind == Kind::Object { std::f32::consts::PI } else { 0.0 };
    base + view as f32 * std::f32::consts::FRAC_PI_2
}

/// Schematic plan of the object's real traffic paths. It is a catalogue image,
/// never an asphalt mesh added to the map. Sample arcs with the same builder as AI.
fn traffic_preview_part(sco: &omsi_scenery::SceneryObject, stop: &AtomicBool) -> Option<Part> {
    use omsi_sim::traffic::{LaneBuilder, LaneKind};
    let mut mesh = MeshData::default();
    let colors = [[0.35, 0.65, 1.0, 1.0], [0.22, 0.9, 0.65, 1.0],
        [0.8, 0.55, 1.0, 1.0], [1.0, 0.75, 0.3, 1.0]];
    let paints = colors.into_iter().map(|diffuse| Paint { diffuse, texture: None, alpha: 0, front_hint: false }).collect();
    for path in sco.paths.iter().take(1024) {
        if stop.load(Ordering::Relaxed) { return None; }
        let v = &path.params;
        if v.len() < 11 || !path.width.is_finite() || !v.iter().take(11).all(|n| n.is_finite())
            || v[5] <= 0.01 || v[5] > 10000.0 { continue; }
        let lane = LaneBuilder::arc(glam::DVec3::new(v[0] as f64, v[1] as f64, 0.0),
            v[3] as f64, v[5] as f64, v[4] as f64, 0.0, LaneKind::from_code(path.kind), path.width);
        let half = path.width.clamp(0.25, 20.0) * 0.5;
        let slot = match path.kind { 1 => 1, 2 => 2, 3 => 3, _ => 0 };
        let first = mesh.indices.len() as u32;
        for pair in lane.points.windows(2) {
            let a = pair[0].as_vec3(); let b = pair[1].as_vec3();
            if !a.is_finite() || !b.is_finite() { continue; }
            let dir = (b - a).normalize_or_zero();
            if dir.length_squared() < 0.5 { continue; }
            let side = Vec3::new(dir.y, -dir.x, 0.0) * half;
            let base = mesh.positions.len() as u32;
            mesh.positions.extend_from_slice(&[a - side, a + side, b + side, b - side]);
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let count = mesh.indices.len() as u32 - first;
        if count > 0 { mesh.ranges.push((first, count, slot)); }
        if mesh.indices.len() >= 180000 { break; }
    }
    if mesh.indices.is_empty() { None } else { Some(Part { mesh, paints }) }
}

fn preview_part(md: &omsi_model::MeshDef, model_dir: &Path, dirs: &[PathBuf]) -> Result<Part, String> {
    let path = crate::scene::scenery_mesh_path(model_dir, &md.file);
    let mesh = omsi_o3d::load_mesh(&path).map_err(|error| {
        log::debug!("Katalog-Vorschau {}: {error}", path.display());
        format!("Modelldatei {}: {error}", md.file)
    })?;
    let paints = mesh.materials.iter().enumerate().map(|(slot, material)| {
        let alpha = match crate::scene::material_alpha(&mesh.materials, slot, &md.materials) {
            omsi_render::AlphaMode::Opaque => 0,
            omsi_render::AlphaMode::Test => 1,
            omsi_render::AlphaMode::Blend => 2,
        };
        let allcolor = md.materials.iter().filter(|d| !d.item
            && omsi_sim::vehicle::override_slot(&mesh.materials, d) == Some(slot))
            .find_map(|d| d.allcolor);
        let diffuse = crate::scene::d3d_material(material, allcolor, !material.texture.is_empty()).0;
        let mut paint = paint(&material.texture, diffuse, alpha, dirs);
        paint.front_hint = md.materials.iter().any(|d| !d.item && d.use_text_texture.is_some()
            && omsi_sim::vehicle::override_slot(&mesh.materials, d) == Some(slot));
        paint
    }).collect();
    Ok(Part { mesh: omsi_geometry::mesh_from_o3d(&mesh), paints })
}

fn object_preview(model: &omsi_model::Model, model_dir: &Path, dirs: &[PathBuf], yaw: f32, stop: &AtomicBool) -> Result<Image, String> {
    let mut first_error = None;
    let mut has_geometry = false;
    // Use the first renderable level rather than combining overlapping LODs.
    for lod in 0..model.lods.len().max(1) {
        let defs = if model.lods.is_empty() { model.meshes.as_slice() } else { model.lod_meshes(lod) };
        let mut preferred = HashMap::new();
        for md in defs.iter().filter(|md| !md.is_shadow) {
            if let Some((variable, value)) = &md.visible {
                if value.is_finite() {
                    preferred.entry(variable.as_str()).and_modify(|selected| {
                        if *value == 0.0 { *selected = 0.0; }
                    }).or_insert(*value);
                }
            }
        }
        let mut loaded = Vec::new();
        for md in defs.iter().filter(|md| !md.is_shadow) {
            if stop.load(Ordering::Relaxed) { return Err("Abgebrochen".into()); }
            has_geometry = true;
            // Prefer state zero; when a variable has no zero variant, show one
            // declared state alongside the object's unconditional geometry.
            if md.visible.as_ref().is_some_and(|(variable, value)| !value.is_finite()
                || preferred.get(variable.as_str()) != Some(value)) { continue; }
            match preview_part(md, model_dir, dirs) {
                Ok(part) => loaded.push((md, part)),
                Err(error) => { if first_error.is_none() { first_error = Some(error); } }
            }
        }
        let normal: Vec<_> = loaded.iter().map(|(_, part)| part).collect();
        if let Some(image) = rasterize_auto(&normal, yaw, stop) { return Ok(image); }
        // No preferred-state image: choose a visible representative state per
        // variable. Scripts are not run, and alternative versions are not stacked.
        let mut parts: Vec<_> = loaded.into_iter().filter(|(md, _)| md.visible.is_none())
            .map(|(_, part)| part).collect();
        let mut states = HashMap::new();
        for md in defs.iter().filter(|md| !md.is_shadow) {
            if stop.load(Ordering::Relaxed) { return Err("Abgebrochen".into()); }
            let Some((variable, value)) = &md.visible else { continue; };
            if *value == 0.0 || !value.is_finite()
                || states.get(variable.as_str()).is_some_and(|selected| selected != value) { continue; }
            match preview_part(md, model_dir, dirs) {
                Ok(part) => {
                    if rasterize_parts(&[&part], yaw, stop).is_some() {
                        states.insert(variable.as_str(), *value);
                        parts.push(part);
                    }
                }
                Err(error) => { if first_error.is_none() { first_error = Some(error); } }
            }
        }
        if let Some(image) = rasterize_auto(&parts.iter().collect::<Vec<_>>(), yaw, stop) { return Ok(image); }
    }
    if stop.load(Ordering::Relaxed) { return Err("Abgebrochen".into()); }
    Err(first_error.unwrap_or_else(|| if has_geometry { "Modell geladen, aber keine darstellbare Fläche".into() }
        else { "Keine sichtbare Modellgeometrie definiert".into() }))
}

// Orthographic three-quarter view with a depth buffer. This is deliberately static:
// scripts, dynamic displays and animations never run in an asset thumbnail.
fn rasterize_auto(parts: &[&Part], offset: f32, stop: &AtomicBool) -> Option<Image> {
    let yaw = automatic_yaw(parts, stop) + offset - std::f32::consts::PI;
    rasterize_parts(parts, yaw, stop)
}

/// Prefer the winding's front, especially text slots and detailed/coloured upright
/// faces. Sample each triangle's own UV region so a shared atlas's grey back does
/// not receive the same score as its printed front. Manual rotation is an offset.
fn automatic_yaw(parts: &[&Part], stop: &AtomicBool) -> f32 {
    const VIEWS: usize = 32;
    let base = std::f32::consts::PI;
    let directions: [Vec3; VIEWS] = std::array::from_fn(|i| glam::Quat::from_rotation_z(
        base + i as f32 * std::f32::consts::TAU / VIEWS as f32) * Vec3::new(-0.6, -0.8, 0.0));
    let mut scores = [0.0f32; VIEWS];
    for part in parts {
        for &(first, count, material) in &part.mesh.ranges {
            let end = (first as usize).saturating_add(count as usize).min(part.mesh.indices.len());
            if first as usize > end { continue; }
            let Some(paint) = part.paints.get(material as usize) else { continue; };
            let triangles = part.mesh.indices[first as usize..end].chunks_exact(3);
            let stride = triangles.len().div_ceil(50000).max(1);
            for ids in triangles.step_by(stride) {
                if stop.load(Ordering::Relaxed) { return base; }
                let (Some(&a), Some(&b), Some(&c)) = (part.mesh.positions.get(ids[0] as usize),
                    part.mesh.positions.get(ids[1] as usize), part.mesh.positions.get(ids[2] as usize)) else { continue; };
                // Converted OMSI meshes retain Direct3D's clockwise front winding.
                let cross = (c - a).cross(b - a);
                let area = cross.length(); let n = cross.normalize_or_zero();
                if !area.is_finite() || area <= 1e-7 || n.z.abs() > 0.75 { continue; }
                let uv = [ids[0], ids[1], ids[2]].map(|i| part.mesh.uvs.get(i as usize).copied().unwrap_or(Vec2::ZERO));
                let mut weight = 0.05;
                if paint.front_hint { weight += 20.0; }
                if let Some(tex) = &paint.texture {
                    if tex.width > 0 && tex.height > 0 {
                        let mut mean = 0.0; let mut square = 0.0; let mut saturation = 0.0; let mut samples = 0.0;
                        for u in 0..5 { for v in 0..5-u {
                            let a = (u as f32 + 0.2) / 5.0; let b = (v as f32 + 0.2) / 5.0;
                            let coord = uv[0] * a + uv[1] * b + uv[2] * (1.0 - a - b);
                            if !coord.is_finite() { continue; }
                            let x = ((coord.x.rem_euclid(1.0) * tex.width as f32) as usize).min(tex.width as usize - 1);
                            let y = ((coord.y.rem_euclid(1.0) * tex.height as f32) as usize).min(tex.height as usize - 1);
                            let at = (y * tex.width as usize + x) * 4;
                            let Some(pixel) = tex.rgba.get(at..at + 4) else { continue; };
                            if paint.alpha != 0 && pixel[3] < 128 { continue; }
                            let rgb = [pixel[0], pixel[1], pixel[2]].map(|c| c as f32 / 255.0);
                            let brightness = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
                            mean += brightness; square += brightness * brightness;
                            saturation += rgb.iter().copied().fold(0.0f32, f32::max) - rgb.iter().copied().fold(1.0f32, f32::min);
                            samples += 1.0;
                        } }
                        if samples > 0.0 {
                            weight += (square / samples - (mean / samples).powi(2)).max(0.0) * 15.0 + saturation / samples * 3.0;
                        } else { continue; }
                    }
                }
                for (score, dir) in scores.iter_mut().zip(&directions) {
                    *score += area * stride as f32 * weight * n.dot(*dir).max(0.0).powi(4);
                }
            }
        }
    }
    let mut best = 0;
    for i in 1..VIEWS { if scores[i] > scores[best] * 1.001 + 1e-6 { best = i; } }
    base + best as f32 * std::f32::consts::TAU / VIEWS as f32
}

fn rasterize(parts: &[Part], yaw: f32, stop: &AtomicBool) -> Option<Image> {
    rasterize_parts(&parts.iter().collect::<Vec<_>>(), yaw, stop)
}
fn rasterize_parts(parts: &[&Part], yaw: f32, stop: &AtomicBool) -> Option<Image> {
    rasterize_camera(parts, yaw, false, stop)
}
fn rasterize_camera(parts: &[&Part], yaw: f32, top_down: bool, stop: &AtomicBool) -> Option<Image> {
    const W: usize = 960; const H: usize = 640;
    let rotation = glam::Quat::from_rotation_z(yaw);
    let right = rotation * if top_down { Vec3::X } else { Vec3::new(0.8, -0.6, 0.0) };
    let up = rotation * if top_down { Vec3::Y } else { Vec3::new(0.3, 0.4, 0.8660254) };
    let toward = right.cross(up);
    let project = |p: Vec3| Vec3::new(p.dot(right), -p.dot(up), p.dot(toward));
    let mut min = Vec3::splat(f32::INFINITY); let mut max = Vec3::splat(f32::NEG_INFINITY);
    for p in parts.iter().flat_map(|p| &p.mesh.positions).filter(|p| p.is_finite()) {
        let p = project(*p); min = min.min(p); max = max.max(p);
    }
    if !min.is_finite() || !max.is_finite() { return None; }
    let span = max - min; let center = (min + max) * 0.5;
    let scale = ((W - 96) as f32 / span.x.max(0.01)).min((H - 96) as f32 / span.y.max(0.01));
    let screen = |p: Vec3| { let p = (project(p) - center) * scale; Vec3::new(p.x + W as f32 * 0.5, p.y + H as f32 * 0.5, p.z) };
    let mut rgba = vec![0u8; W * H * 4];
    for y in 0..H { for x in 0..W { let c = if (x / 64 + y / 64) % 2 == 0 { [38, 48, 61, 255] } else { [42, 53, 67, 255] }; rgba[(y * W + x) * 4..(y * W + x) * 4 + 4].copy_from_slice(&c); } }
    let mut depth = vec![f32::NEG_INFINITY; W * H]; let mut drawn = false;
    for part in parts {
        if stop.load(Ordering::Relaxed) { return None; }
        let mesh = &part.mesh;
        let fallback = Paint { diffuse: [0.72, 0.75, 0.8, 1.0], texture: None, alpha: 0, front_hint: false };
        for &(first, count, material) in &mesh.ranges {
            let paint = part.paints.get(material as usize).unwrap_or(&fallback);
            let end = (first as usize).saturating_add(count as usize).min(mesh.indices.len());
            if first as usize > end { continue; }
            for (n, ids) in mesh.indices[first as usize..end].chunks_exact(3).enumerate() {
                if n % 1024 == 0 && stop.load(Ordering::Relaxed) { return None; }
                let [i, j, k] = [ids[0] as usize, ids[1] as usize, ids[2] as usize];
                let (Some(&a), Some(&b), Some(&c)) = (mesh.positions.get(i), mesh.positions.get(j), mesh.positions.get(k)) else { continue; };
                if !a.is_finite() || !b.is_finite() || !c.is_finite() { continue; }
                let normal = (b - a).cross(c - a).normalize_or_zero();
                let shade = 0.55 + 0.45 * normal.dot(Vec3::new(-0.3, -0.4, 0.8660254)).abs();
                let [a, b, c] = [screen(a), screen(b), screen(c)];
                let area = (b.truncate() - a.truncate()).perp_dot(c.truncate() - a.truncate());
                if area.abs() < 1e-6 { continue; }
                let lo = a.min(b).min(c); let hi = a.max(b).max(c);
                let x0 = (lo.x.floor() as i32).clamp(0, W as i32 - 1); let x1 = (hi.x.ceil() as i32).clamp(0, W as i32 - 1);
                let y0 = (lo.y.floor() as i32).clamp(0, H as i32 - 1); let y1 = (hi.y.ceil() as i32).clamp(0, H as i32 - 1);
                let uv = [i, j, k].map(|v| mesh.uvs.get(v).copied().unwrap_or(Vec2::ZERO));
                for y in y0..=y1 { for x in x0..=x1 {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let w1 = (p - a.truncate()).perp_dot(c.truncate() - a.truncate()) / area;
                    let w2 = (b.truncate() - a.truncate()).perp_dot(p - a.truncate()) / area;
                    let w0 = 1.0 - w1 - w2;
                    if w0 < -1e-5 || w1 < -1e-5 || w2 < -1e-5 { continue; }
                    let z = a.z * w0 + b.z * w1 + c.z * w2; let pixel = y as usize * W + x as usize;
                    if z <= depth[pixel] { continue; }
                    let uv = uv[0] * w0 + uv[1] * w1 + uv[2] * w2;
                    let mut color = [255u8; 4];
                    if let Some(tex) = &paint.texture {
                        if tex.width > 0 && tex.height > 0 && uv.is_finite() {
                            color=preview_sample(tex,uv);
                        }
                    }
                    // Opaque slots ignore alpha. With a texture, OMSI takes alpha
                    // from that texture alone, not its product with diffuse alpha.
                    let alpha = if paint.alpha == 0 { 1.0 } else if paint.texture.is_some() {
                        color[3] as f32 / 255.0
                    } else { paint.diffuse[3] }.clamp(0.0, 1.0);
                    if alpha < if paint.alpha == 1 { 0.5 } else { 0.02 } { continue; }
                    for ch in 0..3 {
                        let value = color[ch] as f32 * paint.diffuse[ch].clamp(0.0, 1.0) * shade;
                        rgba[pixel * 4 + ch] = (value * alpha + rgba[pixel * 4 + ch] as f32 * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
                    }
                    depth[pixel] = z; drawn = true;
                } }
            }
        }
    }
    drawn.then_some(Image { width: W as u32, height: H as u32, rgba, has_alpha: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_filter_excludes_reported_signs_and_preserves_surface_names() {
        for file in ["Sceneryobjects/Rumpelhans/Verkehrszeichen/texture/1024-14.bmp",
            "Sceneryobjects/Rumpelhans/Verkehrszeichen/texture/1026-39.bmp",
            "Sceneryobjects/DavidM-Objekte/texture/70.jpg","Sceneryobjects/DavidM-Objekte/texture/75T.bmp",
            "Sceneryobjects/DavidM-Objekte/texture/8.bmp","Sceneryobjects/DavidM-Objekte/texture/800m.bmp",
            "Sceneryobjects/DavidM-Objekte/texture/810gen_1.bmp","Texture/textfeld.bmp"] {
            assert!(texture_is_decal(file,false),"{file}");
        }
        for file in ["Texture/01_asphalt.bmp","Texture/grass_70.bmp","Texture/7_gravel.bmp",
            "Sceneryobjects/Waldheini12/texture/wiese.bmp","Splines/Signmaster/texture/pflaster.bmp"] {
            assert!(!texture_is_decal(file,false),"{file}");
        }
        assert!(!texture_is_decal("Texture/8.bmp",true));
    }
    #[test]
    fn texture_scan_keeps_exact_ground_and_spline_references_without_admitting_same_named_signs() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root=std::env::temp_dir().join(format!("omsi-surface-catalogue-{stamp}"));let map=root.join("maps/TextureTest");
        std::fs::create_dir_all(&map).unwrap();
        let mut mask=crate::ground_paint::Mask::empty(8).unwrap();mask.alpha.fill(1.0);let image=mask.dds();
        for file in ["Texture/70.bmp","Texture/7_gravel.bmp","Splines/Paket/texture/8.bmp",
            "Sceneryobjects/DavidM-Objekte/texture/70.bmp","Sceneryobjects/DavidM-Objekte/texture/8.bmp",
            "Sceneryobjects/DavidM-Objekte/texture/810gen_1.bmp",
            "Sceneryobjects/Rumpelhans/Verkehrszeichen/texture/1024-14.bmp","maps/TextureTest/texture/map/tile_0_0.map.1.dds"] {
            let path=root.join(file);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,&image).unwrap();
        }
        std::fs::write(map.join("global.cfg"),"[groundtex]\nTexture/70.bmp\n\n6\n25\n25\n").unwrap();
        std::fs::write(root.join("Splines/Paket/strasse.sli"),"[texture]\n8.bmp\n[profile]\n0\n[profilepnt]\n-3.5\n0\n0\n0.25\n[profilepnt]\n3.5\n0\n1\n0.25\n").unwrap();
        omsi_cfg::content_changed();let entries=scan_textures(&root,Some(&map),&AtomicBool::new(false));
        for keep in ["Texture/70.bmp","Texture/7_gravel.bmp","Splines/Paket/texture/8.bmp"] {
            assert!(entries.iter().any(|a|a.file==keep),"{keep}");
        }
        for hide in ["Sceneryobjects/DavidM-Objekte/texture/70.bmp","Sceneryobjects/DavidM-Objekte/texture/8.bmp",
            "Sceneryobjects/DavidM-Objekte/texture/810gen_1.bmp","Sceneryobjects/Rumpelhans/Verkehrszeichen/texture/1024-14.bmp",
            "maps/TextureTest/texture/map/tile_0_0.map.1.dds"] {
            assert!(!entries.iter().any(|a|a.file==hide),"{hide}");
        }
        let _=std::fs::remove_dir_all(root);
    }
    fn item(name:&str,file:&str,kind:Kind,category:Category,groups:&str)->Asset {
        Asset {name:name.into(),file:file.into(),kind,category,groups:groups.into(),path:PathBuf::from(file)}
    }
    #[test]
    fn catalogue_filters_names_groups_and_types_and_sorts_in_both_directions() {
        let entries=vec![item("Zeder","Sceneryobjects/a.sco",Kind::Object,Category::Vegetation,"Bäume / Nadelbäume"),
            item("Ahorn","Sceneryobjects/z.sco",Kind::Object,Category::Vegetation,"Bäume / Laubbäume"),
            item("Fluss","Splines/river.sli",Kind::Spline,Category::Water,"Wasser"),
            item("Haus","Sceneryobjects/h.sco",Kind::Object,Category::Buildings,"Gebäude")];
        assert_eq!(filter_assets(&entries,Section::Objects,Category::Vegetation,"",Sort::Name,false),vec![1,0]);
        assert_eq!(filter_assets(&entries,Section::Objects,Category::Vegetation,"",Sort::Name,true),vec![0,1]);
        assert_eq!(filter_assets(&entries,Section::Objects,Category::Vegetation,"laub ahorn",Sort::Path,false),vec![1]);
        assert_eq!(filter_assets(&entries,Section::Roads,Category::Water,"wasser",Sort::Name,false),vec![2]);
        assert_eq!(filter_assets(&entries,Section::Objects,Category::All,"",Sort::Path,false),vec![0,3,1]);
        assert_eq!(filter_assets(&entries,Section::Objects,Category::All,"",Sort::Category,false),vec![3,1,0]);
        assert_eq!(classify("tree.sco","", "",true),Category::Vegetation);
        assert_eq!(classify("lake.sco","See","Gewässer",false),Category::Water);
        assert_eq!(classify("x.sco","","Kreuzungen",false),Category::Junctions);
    }
    #[test]
    fn roads_section_includes_sco_junctions_and_sli_profiles_but_excludes_buildings_and_signs() {
        let entries = vec![
            item("Neues Rathaus1", "Splines/Oberpfalz 3D/Krummenaab/Neues Rathaus1.sli",
                Kind::Spline, Category::Profiles, ""),
            item("Kreuzung", "Sceneryobjects/Lemmental/Kreuzungen/tee.sco",
                Kind::Object, Category::Junctions, ""),
            item("Rathaus", "Sceneryobjects/paket/Rathaus.sco", Kind::Object, Category::Buildings, ""),
            item("Wegweiser", "Sceneryobjects/paket/sign.sco", Kind::Object, Category::Street, ""),
            item("Ampellogik", "Sceneryobjects/paket/controller.sco", Kind::Object, Category::Traffic, ""),
        ];
        assert_eq!(filter_assets(&entries, Section::Roads, Category::All, "", Sort::Name, false), vec![4, 1, 0]);
        assert_eq!(filter_assets(&entries, Section::Roads, Category::Traffic, "", Sort::Name, false), vec![4]);
        assert_eq!(filter_assets(&entries, Section::Roads, Category::Junctions, "", Sort::Name, false), vec![1]);
        assert_eq!(filter_assets(&entries, Section::Roads, Category::Profiles, "", Sort::Name, false), vec![0]);
        assert_eq!(filter_assets(&entries, Section::Objects, Category::All, "", Sort::Name, false), vec![2, 3]);
        assert!(filter_assets(&entries, Section::Roads, Category::Buildings, "", Sort::Name, false).is_empty());
        // Selecting a crossing must still dispatch to object placement.
        assert_eq!(entries[1].kind, Kind::Object);
    }
    #[test]
    fn switching_sections_resets_invalid_category_and_rotating_discards_old_worker_images() {
        let (results, rx) = mpsc::channel();
        let (tx, _requests) = mpsc::channel();
        let mut catalog = Catalog {
            texture_only:false,info_open:false,info_slot:0,details:HashMap::new(),
            section: Section::Objects, category: Category::Buildings, sort: Sort::Name, descending: false,
            query: String::new(), entries: vec![item("Kreuzung", "Sceneryobjects/tee.sco",
                Kind::Object, Category::Junctions, "")], filtered: Vec::new(), selected: None,
            page: 0, scanning: false, audit_message: String::new(), previews: HashMap::new(), rects: Vec::new(), pending: HashSet::new(),
            rx, tx, cancelled: Arc::new(AtomicBool::new(false)), epoch: Arc::new(AtomicU64::new(0)), view: 0, root: PathBuf::new(), audit: None,
        };
        catalog.command(Command::Section(Section::Roads));
        assert_eq!(catalog.category, Category::All);
        assert_eq!(catalog.chosen().unwrap().kind, Kind::Object);
        catalog.command(Command::Category(Category::Buildings));
        assert_eq!(catalog.category, Category::All);
        let old_key = catalog.image_key(0);
        let old_epoch = catalog.epoch.load(Ordering::Relaxed);
        catalog.previews.insert(0, Err("old cached result".into()));
        catalog.command(Command::Rotate(-1));
        assert_eq!(catalog.view, 3);
        assert!(catalog.previews.is_empty());
        assert_ne!(catalog.image_key(0), old_key);
        assert!(results.send(Reply::Preview(old_epoch, 0, Err("old worker result".into()))).is_ok());
        catalog.poll();
        assert!(catalog.previews.is_empty());
        catalog.command(Command::ResetView);
        assert_eq!(catalog.image_key(0), old_key);
        catalog.command(Command::Category(Category::Junctions));
        catalog.command(Command::Section(Section::Objects));
        assert_eq!(catalog.category, Category::All);
        assert!(catalog.filtered.is_empty());
    }
    #[test]
    fn catalogue_does_not_classify_author_names_or_place_names_as_asset_types() {
        for (file, name, groups, expected) in [
            ("Sceneryobjects/Waldheini12/Ettbruck/Skigebiet/Schwankbrettl.sco",
                "\"Schwankbrettl\" Apres Ski", "Waldheini12 / Ettbruck / Gebäude", Category::Buildings),
            ("Sceneryobjects/Waldheini12/Ettbruck/Skigebiet/Schwankbrettl.sco",
                "\"Schwankbrettl\" Apres Ski", "Waldheini12 / Ettbruck / Skigebiet", Category::Other),
            ("Sceneryobjects/BusdriversObjekte/AB Abfahrt.sco", "AB Abfahrt", "BusdriversObjekte", Category::Street),
            ("Sceneryobjects/BusdriversObjekte/unbekannt.sco", "Unbekannt", "BusdriversObjekte", Category::Other),
            ("Sceneryobjects/DavidM2412 - Objekte/AMPELOBJEKTE/Kreuz Rathaus/1.sco",
                "1", "DavidM2412 / Kreuz Rathaus", Category::Street),
            ("Sceneryobjects/kosta_Objekte/Wegweiser_Narrow/N/2gerade_links.sco",
                "2gerade_links", "Kreuzungen", Category::Street),
            ("Sceneryobjects/Waldheini12/385_Flusshinweis.sco", "385 als Flusshinweis", "Waldheini12", Category::Street),
            ("Sceneryobjects/paket/Achtung_Kreisverkehr.sco", "Achtung_Kreisverkehr", "Kreuzungen", Category::Street),
            ("Sceneryobjects/DavidM2412/AirbusLogo.sco", "Airbus Logo", "Rathaus", Category::Street),
            ("Sceneryobjects/paket/Kreuz Rathaus/1.sco", "1", "Kreuz Rathaus", Category::Other),
            ("Sceneryobjects/Wald/unbekannt.sco", "Unbekannt", "", Category::Other),
            ("Sceneryobjects/Water/unbekannt.sco", "Unbekannt", "", Category::Other),
        ] {
            assert_eq!(classify(file, name, groups, false), expected, "{file}: {name}");
        }
    }
    #[test]
    fn catalogue_keeps_genuine_asset_types_and_uses_nearest_category_hint() {
        for (file, name, groups, expected) in [
            ("Sceneryobjects/BusdriversObjekte/See_01.sco", "See 01", "", Category::Water),
            ("Splines/Waldheini12/Fluss_01.sli", "Fluss 01", "", Category::Water),
            ("Splines/paket/river_2.sli", "river_2", "", Category::Water),
            ("Sceneryobjects/paket/Laubbaum_01.sco", "Laubbaum 01", "", Category::Vegetation),
            ("Sceneryobjects/paket/unbekannt.sco", "Unbekannt", "Natur / Bäume", Category::Vegetation),
            ("Sceneryobjects/paket/a.sco", "a", "Gewässer / Gebäude", Category::Buildings),
            ("Sceneryobjects/paket/a.sco", "a", "Gebäude / Gewässer", Category::Water),
            ("Sceneryobjects/paket/a.sco", "a", "Gebäude / Kreuzungen", Category::Junctions),
            ("Sceneryobjects/paket/Rathaus.sco", "Rathaus", "", Category::Buildings),
            ("Sceneryobjects/paket/Kreuzung.sco", "Kreuzung", "StreetObjects", Category::Junctions),
            ("Sceneryobjects/paket/Gebäude/Vegetation/1.sco", "1", "", Category::Vegetation),
            ("Sceneryobjects/paket/Vegetation/Gebäude/1.sco", "1", "", Category::Buildings),
            ("Sceneryobjects/paket/Baumhaus.sco", "Baumhaus", "", Category::Buildings),
        ] {
            assert_eq!(classify(file, name, groups, false), expected, "{file}: {groups}");
        }
        assert_eq!(classify("Sceneryobjects/BusdriversObjekte/1.sco", "1", "Gewässer", true), Category::Vegetation);
    }
    #[test]
    fn catalogue_category_filter_excludes_the_reported_false_positives() {
        let mut entries = vec![
            item("Haus 1", "Sceneryobjects/Waldheini12/Haus1.sco", Kind::Object, Category::Other, ""),
            item("AB Abfahrt", "Sceneryobjects/BusdriversObjekte/AB Abfahrt.sco", Kind::Object, Category::Other, ""),
            item("2gerade_links", "Sceneryobjects/kosta_Objekte/Wegweiser_Narrow/N/2gerade_links.sco",
                Kind::Object, Category::Other, "Kreuzungen"),
            item("Ahorn", "Sceneryobjects/paket/Ahorn.sco", Kind::Object, Category::Other, ""),
            item("Fluss 1", "Splines/paket/Fluss1.sli", Kind::Spline, Category::Other, ""),
            item("Kreuzung", "Sceneryobjects/paket/Kreuzung.sco", Kind::Object, Category::Other, ""),
        ];
        for asset in &mut entries { asset.category = classify(&asset.file, &asset.name, &asset.groups, false); }
        assert_eq!(filter_assets(&entries, Section::Objects, Category::Vegetation, "", Sort::Category, false), vec![3]);
        assert_eq!(filter_assets(&entries, Section::Objects, Category::Buildings, "", Sort::Category, false), vec![0]);
        assert_eq!(filter_assets(&entries, Section::Objects, Category::Water, "", Sort::Category, false), Vec::<usize>::new());
        assert_eq!(filter_assets(&entries, Section::Roads, Category::Water, "", Sort::Name, false), vec![4]);
        assert_eq!(filter_assets(&entries, Section::Roads, Category::Junctions, "", Sort::Category, false), vec![5]);
        assert_eq!(filter_assets(&entries, Section::Objects, Category::Street, "", Sort::Name, false), vec![2, 1]);
        assert_eq!(filter_assets(&entries, Section::Objects, Category::Street, "", Sort::Name, true), vec![1, 2]);
        assert_eq!(filter_assets(&entries, Section::Objects, Category::Street, "kosta", Sort::Name, false), vec![2]);
    }
    #[test]
    fn catalogue_reads_friendly_names_and_groups_with_native_encoding() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(format!("omsi-catalogue-{}-{stamp}.sco",std::process::id()));
        let text="[friendlyname]\r\nGroßer Ahorn\r\n[groups]\r\n2\r\nNatur\r\nVegetation\r\n[tree]\r\nleaf.bmp\r\n8\r\n10\r\n0.4\r\n0.6\r\n";
        std::fs::write(&path,crate::editor::encode(text,crate::editor::Encoding::Utf16Le)).unwrap();
        let asset=asset_metadata(&path,Kind::Object,"Sceneryobjects/generic.sco".into(),"generic".into());
        assert_eq!(asset.name,"Großer Ahorn");assert_eq!(asset.category,Category::Vegetation);
        assert_eq!(asset.groups,"Natur / Vegetation");
        std::fs::remove_file(path).unwrap();
    }
    struct PreviewFixture(PathBuf);
    impl PreviewFixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!("omsi-preview-{}-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir_all(&path).unwrap(); Self(path)
        }
        fn write(&self, relative: &str, data: &[u8]) -> PathBuf {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, data).unwrap(); path
        }
        fn mesh(&self, relative: &str, diffuse: [f32; 4]) {
            self.mesh_at(relative, diffuse, 0.0);
        }
        fn mesh_at(&self, relative: &str, diffuse: [f32; 4], offset_x: f32) {
            // Minimal real O3D v2 surface with one material and one triangle.
            let mut data = vec![0x84, 0x19, 2, 0x17];
            data.extend_from_slice(&3u16.to_le_bytes());
            for mut position in [[-2.0f32, -2.0, 0.0], [2.0, -2.0, 0.0], [0.0, 2.0, 0.0]] {
                position[0] += offset_x;
                for value in position.into_iter().chain([0.0, 0.0, 1.0, 0.0, 0.0]) {
                    data.extend_from_slice(&value.to_le_bytes());
                }
            }
            data.push(0x49); data.extend_from_slice(&1u16.to_le_bytes());
            for value in [0u16, 1, 2, 0] { data.extend_from_slice(&value.to_le_bytes()); }
            data.push(0x26); data.extend_from_slice(&1u16.to_le_bytes());
            for value in diffuse.into_iter().chain([0.0; 7]) { data.extend_from_slice(&value.to_le_bytes()); }
            data.push(0); // Empty texture filename; opaque material alpha is zero.
            self.write(relative, &data);
        }
        fn image(&self, definition: &str) -> Result<Image, String> {
            let path = self.write("Sceneryobjects/Lemmental/Kreuzungen/tee.sco", definition.as_bytes());
            let asset = item("tee", &path.to_string_lossy(), Kind::Object, Category::Junctions, "");
            preview(&self.0, &asset, 0, &AtomicBool::new(false))
        }
    }
    impl Drop for PreviewFixture {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    #[test]
    fn traffic_metadata_and_preview_cover_path_only_lt_objects_and_dummy_mesh_controllers() {
        let fixture = PreviewFixture::new();
        let path = "[path]\n0\n0\n0\n0\n0\n20\n0\n0\n0\n2.5\n0\n1\n";
        for (definition, file) in [
            (format!("[groups]\n2\nLemmental\nKreuzungen\n{path}"), "LT_Bahn.sco"),
            (format!("[groups]\n1\nKreuzungen\n[onlyeditor]\n{path}[mesh]\nblock.x\n"), "FriedrichEbert1.sco"),
            (format!("[groups]\n1\nAmpelobjekte\n[onlyeditor]\n{path}[mesh]\nDummy.x\n"), "Ampellogik.sco"),
        ] {
            let relative = format!("Sceneryobjects/Lemmental/Kreuzungen/{file}");
            let path = fixture.write(&relative, definition.as_bytes());
            let asset = asset_metadata(&path, Kind::Object, relative, file.into());
            assert_eq!(asset.category, Category::Traffic);
            // A traffic schematic works without needing a dummy editor mesh.
            let image = preview(&fixture.0, &asset, 0, &AtomicBool::new(false)).unwrap();
            assert!(image.rgba.chunks_exact(4).any(|pixel| pixel[2] > 180));
            assert_eq!(asset.kind, Kind::Object);
        }
        let relative = "Sceneryobjects/Lemmental/Kreuzungen/Plate.sco";
        let path = fixture.write(relative, format!("{path}[mesh]\nroad.o3d\n").as_bytes());
        assert_eq!(asset_metadata(&path, Kind::Object, relative.into(), "Plate".into()).category, Category::Junctions);
    }
    #[test]
    fn traffic_preview_samples_signed_arcs_in_the_same_units_as_the_ai_and_flattens_height() {
        for radius in [10.0f32, -10.0] {
            let mut sco = omsi_scenery::SceneryObject::default();
            sco.paths.push(omsi_scenery::PathDef { kind: 0, width: 2.5,
                params: vec![0.0, 0.0, 100.0, 0.0, radius, 10.0 * std::f32::consts::FRAC_PI_2,
                    0.0, 20.0, 0.0, 2.5, 0.0, 1.0], ..Default::default() });
            let part = traffic_preview_part(&sco, &AtomicBool::new(false)).unwrap();
            assert!(part.mesh.positions.iter().all(|point| point.z == 0.0));
            let points = &part.mesh.positions;
            let end = (points[points.len() - 2] + points[points.len() - 1]) * 0.5;
            assert!((end.x - radius).abs() < 0.01 && (end.y - 10.0).abs() < 0.01, "{end:?}");
        }
    }
    #[test]
    fn traffic_preview_skips_invalid_paths_and_respects_cancellation() {
        let mut sco = omsi_scenery::SceneryObject::default();
        sco.paths.push(omsi_scenery::PathDef { width: 2.5, params: vec![0.0; 12], ..Default::default() });
        assert!(traffic_preview_part(&sco, &AtomicBool::new(false)).is_none());
        sco.paths[0].params[5] = 20.0;
        assert!(traffic_preview_part(&sco, &AtomicBool::new(true)).is_none());
        sco.paths[0].params[4] = f32::NAN;
        assert!(traffic_preview_part(&sco, &AtomicBool::new(false)).is_none());
    }
    #[test]
    fn spline_metadata_uses_profile_category_despite_building_place_name() {
        let fixture = PreviewFixture::new();
        let path = fixture.write("Splines/Oberpfalz 3D/Krummenaab/Neues Rathaus1.sli",
            b"[friendlyname]\nNeues Rathaus1\n[groups]\n1\nBuildings\n");
        let asset = asset_metadata(&path, Kind::Spline,
            "Splines/Oberpfalz 3D/Krummenaab/Neues Rathaus1.sli".into(), "Neues Rathaus1".into());
        assert_eq!(asset.category, Category::Profiles);
    }
    #[test]
    fn light_only_sco_is_not_a_junction_and_explains_why_no_model_preview_exists() {
        let fixture = PreviewFixture::new();
        let definition = "[friendlyname]\nLT_Bahn\n[maplight]\n0\n0\n5\n1\n1\n1\n20\n";
        let path = fixture.write("Sceneryobjects/Lemmental/Kreuzungen/LT_Bahn.sco", definition.as_bytes());
        let asset = asset_metadata(&path, Kind::Object,
            "Sceneryobjects/Lemmental/Kreuzungen/LT_Bahn.sco".into(), "LT_Bahn".into());
        assert_eq!(asset.category, Category::Other);
        assert_eq!(fixture.image(definition).err().unwrap(), "Beleuchtungsobjekt ohne sichtbares Modell");
    }
    #[test]
    fn object_preview_default_and_opposite_views_show_opposite_sides_of_a_sign() {
        let face = |y: f32, color: [f32; 4]| Part { mesh: MeshData {
            positions: vec![Vec3::new(-2.0, y, 0.0), Vec3::new(2.0, y, 0.0),
                Vec3::new(2.0, y, 4.0), Vec3::new(-2.0, y, 4.0)],
            indices: vec![0, 1, 2, 0, 2, 3], ranges: vec![(0, 6, 0)], ..Default::default()
        }, paints: vec![Paint { diffuse: color, texture: None, alpha: 0, front_hint: false }] };
        let parts = [face(0.02, [1.0, 0.0, 0.0, 1.0]), face(-0.02, [0.0, 0.0, 1.0, 1.0])];
        let stop = AtomicBool::new(false);
        let front = rasterize(&parts, preview_yaw(Kind::Object, 0), &stop).unwrap();
        let back = rasterize(&parts, preview_yaw(Kind::Object, 2), &stop).unwrap();
        let center = ((front.height / 2 * front.width + front.width / 2) * 4) as usize;
        assert!(front.rgba[center] > 150 && front.rgba[center + 2] == 0);
        assert!(back.rgba[center + 2] > 150 && back.rgba[center] == 0);
    }
    #[test]
    fn junction_preview_loads_inline_and_external_models_with_opaque_zero_alpha() {
        let fixture = PreviewFixture::new();
        fixture.mesh("Sceneryobjects/Lemmental/Kreuzungen/Model/road.o3d", [0.8, 0.65, 0.4, 0.0]);
        let inline = fixture.image("[mesh]\nroad.o3d\n").unwrap();
        assert!(inline.rgba.chunks_exact(4).any(|pixel| pixel[0] > 100));
        fixture.write("Sceneryobjects/Lemmental/Kreuzungen/Model/road.cfg", b"[mesh]\nroad.o3d\n");
        let external = fixture.image("[model]\nModel\\road.cfg\n").unwrap();
        assert_eq!(inline.rgba, external.rgba);
    }
    #[test]
    fn junction_preview_chooses_one_visible_variant_when_zero_state_is_empty() {
        let fixture = PreviewFixture::new();
        fixture.mesh("Sceneryobjects/Lemmental/Kreuzungen/Model/red.o3d", [1.0, 0.0, 0.0, 0.0]);
        fixture.mesh("Sceneryobjects/Lemmental/Kreuzungen/Model/green.o3d", [0.0, 1.0, 0.0, 0.0]);
        let plain = fixture.image("[mesh]\nred.o3d\n").unwrap();
        let variant = fixture.image("[mesh]\nred.o3d\n[visible]\nvariant\n1\n[mesh]\ngreen.o3d\n[visible]\nvariant\n2\n").unwrap();
        assert_eq!(plain.rgba, variant.rgba);
        let zero = fixture.image("[mesh]\ngreen.o3d\n[visible]\nvariant\n0\n[mesh]\nred.o3d\n[visible]\nvariant\n1\n").unwrap();
        let plain_green = fixture.image("[mesh]\ngreen.o3d\n").unwrap();
        assert_eq!(zero.rgba, plain_green.rgba);
        let recovered = fixture.image("[mesh]\nmissing.o3d\n[visible]\nvariant\n0\n[mesh]\nred.o3d\n[visible]\nvariant\n1\n").unwrap();
        assert_eq!(recovered.rgba, plain.rgba);
        // Unconditional geometry does not hide a model's only declared variant.
        fixture.mesh_at("Sceneryobjects/Lemmental/Kreuzungen/Model/green.o3d", [0.0, 1.0, 0.0, 0.0], 6.0);
        let mixed = fixture.image("[mesh]\nred.o3d\n[mesh]\ngreen.o3d\n[visible]\nvariant\n1\n").unwrap();
        let both = fixture.image("[mesh]\nred.o3d\n[mesh]\ngreen.o3d\n").unwrap();
        assert_eq!(mixed.rgba, both.rgba);
        assert_ne!(mixed.rgba, plain.rgba);
    }
    #[test]
    fn junction_preview_uses_later_renderable_lod_and_reports_missing_mesh_names() {
        let fixture = PreviewFixture::new();
        fixture.mesh("Sceneryobjects/Lemmental/Kreuzungen/Model/road.o3d", [0.8, 0.65, 0.4, 0.0]);
        let image = fixture.image("[LOD]\n0.5\n[mesh]\nmissing.o3d\n[LOD]\n0\n[mesh]\nroad.o3d\n").unwrap();
        assert!(image.rgba.chunks_exact(4).any(|pixel| pixel[0] > 100));
        let error = fixture.image("[mesh]\nmissing.o3d\n").err().expect("missing mesh produced a preview");
        assert!(error.contains("missing.o3d"), "{error}");
        let empty = fixture.image("[friendlyname]\nUnsichtbarer Trigger\n").err().unwrap();
        assert!(empty.contains("Keine sichtbare Modellgeometrie"), "{empty}");
    }
    #[test]
    fn referenced_model_preview_prioritizes_the_objects_own_texture_folder() {
        let fixture = PreviewFixture::new();
        let path = fixture.write("Sceneryobjects/Retexture/tee.sco", b"[model]\n..\\Original\\Model\\road.cfg\n");
        fixture.write("Sceneryobjects/Original/Model/road.cfg", b"[mesh]\nroad.o3d\n");
        let (sco, _, _, model_dir) = crate::scene::scenery_definition(&fixture.0, &path.to_string_lossy()).unwrap();
        let dirs = crate::scene::scenery_texture_dirs(&fixture.0, &sco, &model_dir);
        assert_eq!(dirs.first(), Some(&fixture.0.join("Sceneryobjects/Retexture/texture")));
    }
    fn textured_triangle(alpha: i32, texture_alpha: u8) -> Part {
        Part { mesh: MeshData {
            positions: vec![Vec3::new(-2.0, -2.0, 0.0), Vec3::new(2.0, -2.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            indices: vec![0, 1, 2], ranges: vec![(0, 3, 0)], ..Default::default()
        }, paints: vec![Paint { diffuse: [1.0, 1.0, 1.0, 0.0], alpha, front_hint: false,
            texture: Some(Image { width: 1, height: 1, rgba: vec![0, 0, 255, texture_alpha], has_alpha: true }) }] }
    }
    #[test]
    fn thumbnail_material_alpha_matches_opaque_cutout_and_blended_surface_rules() {
        let stop = AtomicBool::new(false);
        let opaque = rasterize(&[textured_triangle(0, 0)], 0.0, &stop).unwrap();
        assert!(opaque.rgba.chunks_exact(4).any(|pixel| pixel[2] > 150));
        let cutout = rasterize(&[textured_triangle(1, 255)], 0.0, &stop).unwrap();
        assert!(cutout.rgba.chunks_exact(4).any(|pixel| pixel[2] > 150));
        assert!(rasterize(&[textured_triangle(1, 0)], 0.0, &stop).is_none());
        let blend = rasterize(&[textured_triangle(2, 128)], 0.0, &stop).unwrap();
        assert!(blend.rgba.chunks_exact(4).any(|pixel| pixel[2] > 80));
    }
    #[test]
    fn thumbnail_depth_keeps_front_triangle_independent_of_mesh_order() {
        let make = |z: f32, color: [f32; 4]| Part { mesh: MeshData {
            positions: vec![Vec3::new(-2.0, -2.0, z), Vec3::new(2.0, -2.0, z), Vec3::new(0.0, 2.0, z)],
            indices: vec![0, 1, 2], ranges: vec![(0, 3, 0)], ..Default::default()
        }, paints: vec![Paint { diffuse: color, texture: None, alpha: 0, front_hint: false }] };
        let stop = AtomicBool::new(false);
        let a = rasterize(&[make(0.0, [1.0, 0.0, 0.0, 1.0]), make(0.02, [0.0, 1.0, 0.0, 1.0])], 0.0, &stop).unwrap();
        let b = rasterize(&[make(0.02, [0.0, 1.0, 0.0, 1.0]), make(0.0, [1.0, 0.0, 0.0, 1.0])], 0.0, &stop).unwrap();
        assert_eq!(a.rgba, b.rgba);
        assert!(a.rgba.chunks_exact(4).any(|c| c[1] > 150 && c[0] == 0));
    }
    #[test]
    fn cancelled_preview_never_produces_an_image() {
        assert!(rasterize(&[], 0.0, &AtomicBool::new(true)).is_none());
    }
    #[test]
    fn automatic_view_follows_each_signs_printed_face_and_manual_offset() {
        let stop = AtomicBool::new(false);
        for angle in [0.0f32, 0.7, 1.57, 3.14] {
            let turn = glam::Quat::from_rotation_z(angle);
            let face = |y: f32, printed: bool| Part { mesh: MeshData {
                positions: vec![Vec3::new(-2.0,y,0.0),Vec3::new(2.0,y,0.0),Vec3::new(2.0,y,4.0),Vec3::new(-2.0,y,4.0)]
                    .into_iter().map(|p|turn*p).collect(),
                indices: if printed {vec![0,1,2,0,2,3]} else {vec![0,2,1,0,3,2]},
                ranges:vec![(0,6,0)], ..Default::default()
            }, paints:vec![Paint { diffuse: if printed {[1.0,0.0,0.0,1.0]} else {[0.0,0.0,1.0,1.0]},
                texture:None, alpha:0, front_hint:printed }] };
            let front = face(0.02,true); let back = face(-0.02,false); let parts = [&front,&back];
            let yaw = automatic_yaw(&parts,&stop);
            let toward = glam::Quat::from_rotation_z(yaw)*Vec3::new(-0.6,-0.8,0.0);
            assert!(toward.dot(turn*Vec3::Y)>0.98);
            let image = rasterize_auto(&parts,std::f32::consts::PI,&stop).unwrap();
            let opposite = rasterize_auto(&parts,std::f32::consts::TAU,&stop).unwrap();
            let at = ((image.height/2*image.width+image.width/2)*4) as usize;
            assert!(image.rgba[at]>150 && image.rgba[at+2]==0);
            assert!(opposite.rgba[at+2]>150 && opposite.rgba[at]==0);
        }
    }
    #[test]
    fn audit_reports_every_asset_and_keeps_only_failed_assets_in_error_list() {
        let fixture = PreviewFixture::new();
        let path = fixture.write("Sceneryobjects/logic.sco", b"[path]\n0\n0\n0\n0\n0\n10\n0\n0\n0\n2.5\n0\n1\n");
        let mut good = item("Logik","Sceneryobjects/logic.sco",Kind::Object,Category::Traffic,""); good.path=path;
        let bad = item("Fehlt\tName","Sceneryobjects/missing.sco",Kind::Object,Category::Street,"");
        let folder = fixture.0.join("report"); let (tx,_rx) = mpsc::channel();
        let (done,failed) = audit_previews(&fixture.0,&[good,bad],&folder,&AtomicBool::new(false),&tx).unwrap();
        assert_eq!((done,failed),(2,1));
        let all = std::fs::read_to_string(folder.join("alle.tsv")).unwrap();
        let errors = std::fs::read_to_string(folder.join("fehler.tsv")).unwrap();
        assert_eq!(all.lines().count(),3); assert_eq!(errors.lines().count(),2);
        assert!(errors.contains("missing.sco") && !errors.contains("logic.sco"));
        assert!(all.lines().all(|line|line.split('\t').count()==7));
    }
    #[test]
    fn spline_details_include_all_slots_resolved_paths_and_missing_images() {
        let fixture=PreviewFixture::new();
        let path=fixture.write("Splines/Test/road.sli",b"[texture]\nroad.png\n[texture]\nmissing.png\n[profile]\n0\n[profilepnt]\n-3\n0\n0\n0.25\n[profilepnt]\n3\n0\n1\n0.25\n");
        let texture=fixture.0.join("Splines/Test/texture/road.png");std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(2,2,image::Rgba([80,90,100,255])).save(&texture).unwrap();omsi_cfg::content_changed();
        let details=spline_texture_info(&fixture.0,&path,&AtomicBool::new(false));assert_eq!(details.len(),2);
        assert_eq!(std::path::Path::new(&details[0].path),texture.as_path());assert!(details[0].image.is_ok());
        assert!(details[0].usage.contains("Profile: 1"));assert!(details[1].image.is_err());assert_eq!(details[1].name,"missing.png");
    }

    #[test]fn high_resolution_texture_keeps_aspect_and_filters_samples(){
        let texture=Image{width:2048,height:1024,rgba:vec![127;2048*1024*4],has_alpha:false};
        let resized=thumbnail_texture(texture);assert_eq!((resized.width,resized.height),(1024,512));
        let small=Image{width:2,height:1,rgba:vec![0,0,0,255,255,255,255,255],has_alpha:false};
        assert_eq!(preview_sample(&small,Vec2::new(0.5,0.5)),[128,128,128,255]);
        assert_eq!(preview_sample(&small,Vec2::new(0.25,0.5)),[0,0,0,255]);
        assert_eq!(preview_sample(&small,Vec2::new(1.25,0.5)),[0,0,0,255]);
    }
    #[test]fn mesh_preview_renders_at_960_by_640_and_remains_cancellable(){
        let mesh=MeshData{positions:vec![Vec3::ZERO,Vec3::X,Vec3::Y],indices:vec![0,1,2],ranges:vec![(0,3,0)],..Default::default()};
        let part=Part{mesh,paints:vec![Paint{diffuse:[1.0;4],texture:None,alpha:0,front_hint:false}]};
        let stop=AtomicBool::new(false);let img=rasterize_camera(&[&part],0.0,true,&stop).unwrap();
        assert_eq!((img.width,img.height),(960,640));assert_eq!(img.rgba.len(),960*640*4);
        assert!(img.rgba.chunks_exact(4).any(|p|p[0]>180&&p[1]>180&&p[2]>180));
        stop.store(true,Ordering::Relaxed);assert!(rasterize_camera(&[&part],0.0,true,&stop).is_none());
    }

}
