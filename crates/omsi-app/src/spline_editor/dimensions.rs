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
    pub(crate) fn add_dimensions(&mut self,world:&World,file:&str,start:DVec3,direction:DVec3,values:[f64;4])->String {
        let result=(||->Result<_,String>{
            if omsi_map::world_coordinates(){return Err("Spline editing on world-coordinate maps is not supported yet".into());}
            let ty=world.spline_type(file).ok_or("Spline type not found")?;
            if ty.def.profiles.is_empty() && ty.def.paths.is_empty(){return Err("Choose a road or traffic path type".into());}
            if (direction-start).truncate().length()<0.01{return Err("Click away from the start to choose a direction".into());}
            let tile=tile_at(start);
            let s=between(file,0,tile,start,direction,0.0);
            let s=crate::spline_dimensions::reshape(&s,values)?;
            let mut sections=omsi_geometry::spline_dimensions::tile_sections(tile,&s);
            for (tile,part) in &mut sections {
                let path=world.tile_source(tile.0,tile.1).ok_or("The road leaves the map; nothing created")?;
                let base=Tile::load(&path).map_err(|_|"Cannot read tile")?;
                if base.version!=0 && base.version<14{return Err("Spline editing needs a version 14 tile".into());}
                part.id=world.allocate_editor_id().ok_or("No free map ID")?;
            }
            for i in 0..sections.len() {
                sections[i].1.prev_id=if i==0 {0}else{sections[i-1].1.id};
                sections[i].1.next_id=if i+1==sections.len(){0}else{sections[i+1].1.id};
            }
            Ok(sections)
        })();
        let sections=match result{Ok(v)=>v,Err(e)=>return e};
        log::info!("Numeric spline: length={} m, radius={} m, angle={} deg, height={} m, sections={}",values[0],values[1],omsi_geometry::spline_dimensions::turn_degrees(values),values[3],sections.len());
        self.remember(sections.iter().map(|(tile,s)|{let mut absent=s.clone();absent.deleted=true;((*tile,s.id),absent)}).collect());
        let mut edits=world.spline_edits.lock();
        for (tile,s) in sections {let key=(tile,s.id);edits.originals.insert(key,s.clone());edits.added.insert(key,s);edits.dirty_tiles.insert(tile);self.selected=Some(key);}
        edits.dirty=true;
        "New spline: exact dimensions · Click next start · Esc to finish · Ctrl+S to save".into()
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
        assert!(!ed.generation_started());assert!(!ed.placement_finished());
        ed.generate_on_plane(&world,Some(DVec3::new(120.0,100.0,0.0)));assert!(ed.generation_started());
        assert!(ed.generate_on_plane(&world,Some(DVec3::new(120.0,180.0,0.0))).starts_with("New spline"));assert_eq!(world.spline_edits.lock().added.len(),2);
        ed.dimensions=None;assert!(ed.placement_finished());ed.dimensions=Some([5.0,20.0,4.0,12.0]);
        let key=ed.selected.unwrap();let first=world.spline_edits.lock().current(key).unwrap();assert_eq!(first.length,5.0);assert_eq!(first.pos[2],12.0);
        ed.apply_dimensions(&world,[8.0,-30.0,-2.0,15.0]).unwrap();let changed=world.spline_edits.lock().current(key).unwrap();
        let content=root.parent().unwrap().join("saved");crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&root).unwrap();
        let tile=Tile::load(&content.join("maps/Test/tile_0_0.map")).unwrap();let saved=tile.splines.iter().find(|s|s.id==key.1).unwrap();
        assert_eq!((saved.length,saved.radius,saved.grad_start,saved.pos[2]),(8.0,-30.0,-2.0,15.0));assert_eq!(saved.delta_h,changed.delta_h);
        ed.undo(&world);assert_eq!(world.spline_edits.lock().current(key).unwrap(),first);
        assert!(ed.apply_dimensions(&world,[500.0,0.0,0.0,12.0]).is_err());assert_eq!(world.spline_edits.lock().current(key).unwrap(),first);
        ed.cancel_generation();let before=world.spline_edits.lock().added.len();
        assert!(ed.add_dimensions(&world,"Splines/road.sli",DVec3::new(100.0,290.0,12.0),DVec3::new(100.0,400.0,12.0),[20.0,0.0,4.0,12.0]).contains("leaves the map"));
        assert_eq!(world.spline_edits.lock().added.len(),before);
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n0\n1\ntile_0_1.map\n").unwrap();std::fs::write(map.join("tile_0_1.map"),"[version]\n14\n").unwrap();
        let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();let mut ed=SplineEditor::default();
        assert!(ed.add_dimensions(&world,"Splines/road.sli",DVec3::new(100.0,290.0,12.0),DVec3::new(100.0,400.0,12.0),[20.0,0.0,4.0,12.0]).starts_with("New spline"));
        crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&root).unwrap();
        let a=Tile::load(&content.join("maps/Test/tile_0_0.map")).unwrap();let b=Tile::load(&content.join("maps/Test/tile_0_1.map")).unwrap();
        let a=a.splines.iter().find(|s|s.next_id!=0).unwrap();let b=b.splines.iter().find(|s|s.prev_id==a.id).unwrap();
        assert_eq!(a.next_id,b.id);assert!((a.length+b.length-20.0).abs()<1e-8);assert!(curve((0,0),a).end_point().distance(curve((0,1),b).start)<1e-8);
        ed.undo(&world);let edits=world.spline_edits.lock();assert!(edits.added.keys().all(|k|edits.current(*k).is_none_or(|s|s.deleted)));drop(edits);
        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
