//! Native map rules for generated roundabouts. Mark only the block owned by the
//! builder so repeated saves and geometry updates preserve unrelated manual rules.
use super::*;
const START:&str="[openomsi_editor_roundabout_rules]";
const END:&str="[openomsi_editor_roundabout_rules_end]";

pub(super) fn asset_rules(asset:&Path)->Result<Option<String>,String> {
    let Some(parent)=asset.parent() else{return Ok(None);};
    let file=parent.join("junction.junction.json");
    if !file.is_file(){return Ok(None);}
    let bytes=std::fs::read(file).map_err(|e|e.to_string())?;
    if bytes.len()>64*1024{return Err("Junction project is too large".into());}
    let project:Project=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    if project.roundabout.is_none(){return Ok(None);}
    let built=build(&project)?;
    Ok(Some(priority_rules(&project,&built)))
}
fn priority_rules(p:&Project,b:&Built)->String {
    let n=p.arms.iter().filter(|a|a.enabled).count();let mut index=0;let mut out=String::new();
    for (group,(points,_,_)) in b.paths.iter().enumerate() {
        let priority=if (2*n..3*n).contains(&group){64}else{192};
        for pair in points.windows(2) {
            if (pair[1]-pair[0]).truncate().length()<0.011 {continue;}
            out.push_str(&format!("[rule]\n{index}\npriority\n{priority}\n0\n[rule]\n{index}\nspeedlimit\n20\n0\n"));index+=1;
        }
    }out
}
/// An entry with None removes only a former generated block (deletion or undo).
pub fn rewrite(text:&str,objects:&[(i64,Option<String>)])->Result<String,String> {
    if objects.is_empty(){return Ok(text.into());}
    let eol=if text.contains("\r\n"){ "\r\n" }else{"\n"};
    let mut clean=String::new();let lines:Vec<_>=text.split_inclusive('\n').collect();let mut i=0;
    while i<lines.len() {
        // Skip actual label payloads: a sign may contain strings equal to our markers.
        if lines[i].trim().eq_ignore_ascii_case("[object]") {
            let end=crate::editor::object_record_end(&lines,i).ok_or("Invalid object record while saving roundabout rules")?;
            clean.extend(lines[i..end].iter().copied());i=end;continue;
        }
        if lines[i].trim()==START {
            let id=lines.get(i+1).and_then(|s|s.trim().parse::<i64>().ok());
            if id.is_some_and(|id|objects.iter().any(|o|o.0==id)) {
                let end=(i+2..lines.len()).find(|&j|lines[j].trim()==END).ok_or("Incomplete roundabout rule block")?;
                // Never consume a placement if a damaged block is missing its end marker.
                if lines[i+2..end].iter().any(|s|matches!(s.trim(),"[object]"|"[spline]"|"[spline_h]")) {return Err("Incomplete roundabout rule block".into());}
                i=end+1;continue;
            }
        }
        clean.push_str(lines[i]);i+=1;
    }
    let lines:Vec<_>=clean.split_inclusive('\n').collect();let mut out=String::new();i=0;
    while i<lines.len() {
        if lines[i].trim().eq_ignore_ascii_case("[object]") {
            let id=lines.get(i+3).and_then(|s|s.trim().parse::<i64>().ok());
            let end=crate::editor::object_record_end(&lines,i).ok_or("Invalid object record while saving roundabout rules")?;
            out.extend(lines[i..end].iter().copied());i=end;
            if let Some((id,Some(rules)))=objects.iter().find(|o|Some(o.0)==id) {
                if !out.ends_with('\n'){out.push_str(eol);}
                out.push_str(&format!("{START}{eol}{id}{eol}"));out.push_str(&rules.replace('\n',eol));out.push_str(END);out.push_str(eol);
            }
        }else{out.push_str(lines[i]);i+=1;}
    }Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_rules_match_exported_paths_and_save_idempotently() {
        for left in [false,true] {for cross in [false,true] {
            let mut w=Window::new_roundabout(left);w.command(Command::Shape(cross));let b=build(&w.project).unwrap();
            let rules=priority_rules(&w.project,&b);
            let source="[version]\n14\n[object]\n0\nring.sco\n99\n0\n0\n0\n0\n0\n0\n1\n[openomsi_editor_roundabout_rules]\n[rule]\n0\ntrafficdensity\n0.4\n0\n";
            let targets=vec![(99,Some(rules))];let saved=rewrite(source,&targets).unwrap();
            assert_eq!(rewrite(&saved,&targets).unwrap(),saved);
            let tile=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("tile.map",&saved));
            let sco=omsi_scenery::SceneryObject::parse(&omsi_cfg::CfgFile::from_str("ring.sco",&sco(&w.project,&b)));
            assert_eq!(tile.objects[0].rules.len(),sco.paths.len()*2+1);
            assert_eq!(tile.objects[0].extra,vec![START]);
            let priorities:Vec<_>=tile.objects[0].rules.iter().filter(|r|r.kind=="priority").collect();
            assert_eq!(priorities.len(),sco.paths.len());
            for (index,r) in priorities.iter().enumerate(){assert_eq!(r.path_index,index as i32);}
            assert!(priorities.iter().any(|r|r.value==64.0));assert_eq!(priorities[0].value,192.0);
            assert_eq!(rewrite(&saved,&[(99,None)]).unwrap(),source);
            let corrupt=saved.replacen(END,"",1);assert!(rewrite(&corrupt,&targets).is_err());
        }}
    }
}
