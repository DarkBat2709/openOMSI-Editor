//! Object-mode angle keys. Spline curvature/gradient keys remain mode-specific.
use winit::keyboard::KeyCode;
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Step {Turn(f64),Tilt(usize,f64)}
pub fn key(code:KeyCode,shift:bool,ctrl:bool)->Option<Step> {
    if ctrl{return None;}
    let turn=if shift{0.5}else{5.0};let tilt=if shift{0.05}else{0.5};
    Some(match code {
        KeyCode::KeyN=>Step::Turn(-turn),KeyCode::KeyM=>Step::Turn(turn),
        KeyCode::Home=>Step::Tilt(0,tilt),KeyCode::End=>Step::Tilt(0,-tilt),
        KeyCode::PageUp=>Step::Tilt(1,tilt),KeyCode::PageDown=>Step::Tilt(1,-tilt),
        _=>return None,
    })
}
pub fn adjusted(mut angles:[f64;2],axis:usize,delta:f64)->Result<[f64;2],String> {
    if axis>=2 || !delta.is_finite() || angles.iter().any(|v|!v.is_finite()||v.abs()>89.0) {
        return Err("Invalid object tilt".into());
    }
    angles[axis]=(angles[axis]+delta).clamp(-89.0,89.0);
    // Avoid accumulated binary rounding in repeated 0.05° steps.
    angles[axis]=(angles[axis]*100.0).round()/100.0;
    Ok(angles)
}
pub fn shape(angles:[f64;2])->glam::Mat4 {
    omsi_geometry::object_rotation(omsi_geometry::map_rotation([0.0,angles[0],angles[1]]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn object_keys_are_distinct_and_fine_steps_preserve_other_axis(){
        assert_eq!(key(KeyCode::KeyN,false,false),Some(Step::Turn(-5.0)));
        assert_eq!(key(KeyCode::KeyM,true,false),Some(Step::Turn(0.5)));
        for (k,axis,d) in [(KeyCode::Home,0,0.5),(KeyCode::End,0,-0.5),(KeyCode::PageUp,1,0.5),(KeyCode::PageDown,1,-0.5)] {
            assert_eq!(key(k,false,false),Some(Step::Tilt(axis,d)));
            assert_eq!(key(k,true,false),Some(Step::Tilt(axis,d/10.0)));
            assert!(key(k,false,true).is_none());
        }
        assert!(key(KeyCode::Delete,false,false).is_none());
        let mut angles=[0.0,2.75];for _ in 0..100{angles=adjusted(angles,0,0.05).unwrap();}
        assert_eq!(angles,[5.0,2.75]);
        assert_eq!(adjusted([88.95,-89.0],0,0.5).unwrap(),[89.0,-89.0]);
        assert!(adjusted([0.0;2],2,0.5).is_err());assert!(adjusted([f64::NAN,0.0],0,0.5).is_err());
    }
    #[test]fn live_shape_matches_native_map_rotation_after_reload(){
        for angles in [[0.0,0.0],[5.0,-2.5],[-0.05,0.05],[20.0,15.0]] {
            for heading in [0.0,35.0,270.0] {
                let live=glam::Mat4::from_rotation_z((-heading as f64).to_radians()as f32)*shape(angles);
                let saved=omsi_geometry::object_rotation(omsi_geometry::map_rotation([heading,angles[0],angles[1]]));
                assert!(live.abs_diff_eq(saved,1e-5));
            }
        }
    }
}
