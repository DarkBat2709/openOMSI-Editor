use super::*;
impl SplineEditor {
    pub fn apply_dimensions(&mut self,world:&World,values:[f64;4])->Result<(),String> {
        if self.multiple_selected(){return Err("Select one spline for numeric dimensions".into());}
        if self.connection_active() || self.generation_started(){return Err("Finish or cancel spline placement first".into());}
        let key=self.selected.ok_or("Select a road first")?;
        let s=world.spline_edits.lock().current(key).ok_or("Road no longer exists")?;
        let changed=crate::spline_dimensions::reshape(&s,values)?;
        if !inside_tile(&changed){return Err("Road crosses a tile boundary; split it first".into());}
        self.finish_drag();self.set(world,key,changed).ok_or("Could not update road")?;
        Ok(())
    }
    pub(super) fn add_dimensions(&mut self,world:&World,file:&str,start:DVec3,direction:DVec3,values:[f64;4])->String {
        let result=(||->Result<_,String>{
            if omsi_map::world_coordinates(){return Err("Spline editing on world-coordinate maps is not supported yet".into());}
            let ty=world.spline_type(file).ok_or("Spline type not found")?;
            if ty.def.profiles.is_empty() && ty.def.paths.is_empty(){return Err("Choose a road or traffic path type".into());}
            if (direction-start).truncate().length()<0.01{return Err("Click away from the start to choose a direction".into());}
            let tile=tile_at(start);
            let path=world.tile_source(tile.0,tile.1).ok_or("The road leaves the map; nothing created")?;
            let base=Tile::load(&path).map_err(|_|"Cannot read tile")?;
            if base.version!=0 && base.version<14{return Err("Spline editing needs a version 14 tile".into());}
            let id=world.allocate_editor_id().ok_or("No free map ID")?;
            let s=between(file,id,tile,start,direction,0.0);
            let s=crate::spline_dimensions::reshape(&s,values)?;
            if !inside_tile(&s){return Err("Road crosses a tile boundary; reduce length or move the start".into());}
            Ok((tile,s))
        })();
        let (tile,s)=match result{Ok(v)=>v,Err(e)=>return e};
        let key=(tile,s.id);let mut deleted=s.clone();deleted.deleted=true;
        self.remember(vec![(key,deleted)]);
        let mut edits=world.spline_edits.lock();edits.originals.insert(key,s.clone());edits.added.insert(key,s);
        edits.dirty_tiles.insert(tile);edits.dirty=true;self.selected=Some(key);
        "New spline: exact dimensions · Ctrl+S to save".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_creation_edit_save_reload_and_undo() {
        let root=std::env::temp_dir().join(format!("spline-numeric-{}",std::process::id())).join("original");let map=root.join("maps/Test");
        std::fs::create_dir_all(&map).unwrap();std::fs::create_dir_all(root.join("Splines")).unwrap();
        std::fs::write(root.join("Splines/road.sli"),"[profile]\n0\n[profilepnt]\n-3\n0\n0\n1\n[profilepnt]\n3\n0\n1\n1\n").unwrap();
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n").unwrap();std::fs::write(map.join("tile_0_0.map"),"[version]\n14\n").unwrap();
        let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();let mut ed=SplineEditor::default();
        ed.choose_file("Splines/road.sli".into());ed.dimensions=Some([5.0,20.0,4.0,12.0]);
        ed.generate_on_plane(&world,Some(DVec3::new(100.0,100.0,0.0)));let msg=ed.generate_on_plane(&world,Some(DVec3::new(100.0,180.0,0.0)));assert!(msg.starts_with("New spline"),"{msg}");
        let key=ed.selected.unwrap();let first=world.spline_edits.lock().current(key).unwrap();assert_eq!(first.length,5.0);assert_eq!(first.pos[2],12.0);
        ed.apply_dimensions(&world,[8.0,-30.0,-2.0,15.0]).unwrap();let changed=world.spline_edits.lock().current(key).unwrap();
        let content=root.parent().unwrap().join("saved");crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&root).unwrap();
        let tile=Tile::load(&content.join("maps/Test/tile_0_0.map")).unwrap();let saved=tile.splines.iter().find(|s|s.id==key.1).unwrap();
        assert_eq!((saved.length,saved.radius,saved.grad_start,saved.pos[2]),(8.0,-30.0,-2.0,15.0));assert_eq!(saved.delta_h,changed.delta_h);
        ed.undo(&world);assert_eq!(world.spline_edits.lock().current(key).unwrap(),first);
        assert!(ed.apply_dimensions(&world,[500.0,0.0,0.0,12.0]).is_err());assert_eq!(world.spline_edits.lock().current(key).unwrap(),first);
        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
