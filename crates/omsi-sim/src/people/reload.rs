//! In-memory continuation of the player's passengers across an editor map reload.
use super::*;

/// Own the old CPU simulation until the replacement bus and cabin are ready.
/// Nearby waiting passengers are reconciled with the newly loaded map.
pub struct PlayerPassengers(PeopleSim, Vec<(Person,DVec3,f64)>);

impl PeopleSim {
    pub fn into_player_passengers(mut self) -> PlayerPassengers {
        let mut waiting=Vec::new();
        for i in (0..self.people.len()).rev() {
            let p=&self.people[i];
            if p.puppet.is_none() && !p.remote && p.place==Place::Ground {
                if let State::Pax(pax)=&p.state {
                    if pax.inside.is_none() && matches!(pax.task,Task::WaitingForBus|Task::WalkingToBusstop) {
                        if let Some(pos)=pax.stop.and_then(|id|self.stops.get(&id)).map(|s|(s.pos,s.heading)) {
                            waiting.push((self.people.swap_remove(i),pos.0,pos.1));
                        }
                    }
                }
            }
        }
        self.people.retain(|p| {
            p.puppet.is_none()
                && !p.remote
                && match &p.state {
                    State::Pax(pax) => pax.inside == Some(BusId::Player),
                    _ => p.inside(BusId::Player),
                }
        });
        // Render bodies belong to the replacement view; only CPU passengers survive.
        self.bodies.ops.clear();
        if let Some(m) = &mut self.money {
            m.meshes.clear();
            m.hidden.clear();
            for (instance, ..) in &mut m.placed {
                *instance = 0;
            }
        }
        PlayerPassengers(self,waiting)
    }

    /// Keep existing queues at unchanged stops; geometry and spot reservations are
    /// rebuilt from the new map. Never keep a reference to a retired AI bus or path.
    pub fn restore_player_passengers_in_world(&mut self, mut saved:PlayerPassengers,
        world:&dyn World) -> (usize,usize) {
        let waiting=std::mem::take(&mut saved.1);
        let riders=self.restore_player_passengers(saved);
        let count=self.restore_waiting_passengers(waiting,world);
        (riders,count)
    }

    fn restore_waiting_passengers(&mut self, waiting:Vec<(Person,DVec3,f64)>, world:&dyn World) -> usize {
        let mut count=0;
        for (mut person,old_pos,old_heading) in waiting {
            let State::Pax(pax)=&mut person.state else{continue;};
            let Some(id)=pax.stop else{continue;};
            let Some((_,pos,heading,name))=world.bus_stops().iter().find(|s|s.0==id && (s.1-old_pos).length()<=30.0).cloned() else{continue;};
            // Small edits keep the existing queue; a distant relocation is a new area.
            if !world.has_ground(pos.x,pos.y) {continue;}
            let angle=(heading-old_heading).to_radians();
            let offset=person.position-old_pos;
            let expected=pos+DVec3::new(angle.cos()*offset.x+angle.sin()*offset.y,
                -angle.sin()*offset.x+angle.cos()*offset.y,offset.z);
            if !self.stops.contains_key(&id) {
                let stop=self.build_pax_stop(world,None,id,pos,heading,&name);
                self.stops.insert(id,stop);
            }
            let stop=self.stops.get_mut(&id).unwrap();
            let spot=stop.spots.iter().enumerate().filter(|(i,_)|!stop.taken[*i])
                .min_by(|(_,a),(_,b)|(a.pos-expected).length_squared().total_cmp(&(b.pos-expected).length_squared()))
                .map(|(i,_)|i);
            // Moving just the stop marker can leave its old shelter/people markers
            // outside the platform bounds. Rebind existing waiters to their relative
            // positions at the new platform instead of silently deleting the queue.
            let spot=match spot {
                Some(spot)=>spot,
                None=>{
                    let Some(z)=world.walk_height_near(expected.x,expected.y,expected.z) else{continue;};
                    if (z-expected.z).abs()>3.0 {continue;}
                    let spot=stop.spots.len();
                    stop.spots.push(WaitSpot{pos:DVec3::new(expected.x,expected.y,z),
                        face:person.heading+heading-old_heading,height:0.0});
                    stop.taken.push(false);spot
                }
            };
            stop.taken[spot]=true;
            let at=&stop.spots[spot];
            pax.task=Task::WaitingForBus;pax.st=0;pax.bus=None;pax.inside=None;
            pax.pt=None;pax.pt_target=None;pax.door_since=None;pax.shunned=None;pax.spot=Some(spot);
            pax.seat_h=at.height;pax.pax_state=if at.height==0.0{0.0}else{2.0};
            pax.pos=at.pos-if at.height==0.0{DVec3::ZERO}else{DVec3::Z*person.ty.def.seat_height as f64};
            pax.target=pax.pos;pax.yaw=at.face.to_radians();
            pax.line=pax.dest.as_ref().and_then(|d|stop.lines.iter().position(|(n,_)|n.trim()==d.trim()));
            person.position=pax.pos;person.heading=at.face;person.place=Place::Ground;
            person.vel=DVec2::ZERO;person.activity=if at.height==0.0{Activity::Stand}else{Activity::Sit};person.t_state=0.0;
            if self.people.iter().any(|p|p.id==person.id) {person.id=self.next_id;self.next_id+=1;}
            self.next_id=self.next_id.max(person.id+1);
            self.bodies.ops.push(BodyOp::Spawn{id:person.id,ty:person.ty.clone(),variant:person.variant,position:person.position});
            self.people.push(person);count+=1;
        }
        count
    }

    /// Restore after ordinary startup, before another simulation tick. The same bus
    /// model is resumed, so cabin path indices and reserved seats remain valid.
    pub fn restore_player_passengers(&mut self, saved: PlayerPassengers) -> usize {
        let mut old = saved.0;
        // Startup may have populated the map; make room for the continuing identities.
        let mut next = old.next_id.max(self.next_id);
        for p in &mut self.people {
            if old.people.iter().any(|kept| kept.id == p.id) {
                let id = next;
                next += 1;
                self.bodies.renamed(p.id, id);
                p.id = id;
            }
        }
        self.next_id = next;
        self.time = old.time;
        for p in &old.people {
            self.bodies.ops.push(BodyOp::Spawn {
                id: p.id,
                ty: p.ty.clone(),
                variant: p.variant,
                position: p.position,
            });
        }
        let count = old.people.len();
        // Only actual riders reserve seats, never people left outside during reload.
        let mut seats = old.seats.remove(&BusId::Player).unwrap_or_default();
        seats.fill(false);
        for p in &old.people {
            if let State::Pax(pax) = &p.state {
                if let Some(seat) = pax.seat {
                    if let Some(taken) = seats.get_mut(seat) {
                        *taken = true;
                    }
                }
            }
        }
        self.seats.insert(BusId::Player, seats);
        for (to, from) in [
            (&mut self.odometer, &mut old.odometer),
            (&mut self.last_door_open, &mut old.last_door_open),
        ] {
            if let Some(value) = from.remove(&BusId::Player) {
                to.insert(BusId::Player, value);
            }
        }
        if let Some(value) = old.bus_motion.remove(&BusId::Player) {
            self.bus_motion.insert(BusId::Player, value);
        }
        // These are recalculated next tick from the retained passengers and doors.
        self.pax_req.remove(&BusId::Player);
        self.pax_busy.remove(&BusId::Player);
        self.pax_places.remove(&BusId::Player);
        self.desk_busy = old
            .desk_busy
            .filter(|id| old.people.iter().any(|p| p.id == *id));
        macro_rules! keep { ($($field:ident),* $(,)?) => { $(self.$field = old.$field;)* }; }
        keep!(
            pardons,
            pardon_max,
            served_stop,
            player_next_stop,
            stop_request,
            tickets_sold,
            ticket_cash,
            sales,
            boarded,
            served,
            stepped_in,
            content,
            ticket_requests,
            ticket_points,
            duty,
            voice_said,
            last_chat
        );
        if self.desk_busy.is_some() {
            keep!(request, paid, change_due);
        }
        self.money = old.money;
        self.people.extend(old.people);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reload_keeps_riders_seats_ticket_and_distance_without_old_scene_handles() {
        let dir = std::env::temp_dir().join(format!("omsi-reload-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("person.hum"), "[model]\nmodel.cfg\n").unwrap();
        std::fs::write(dir.join("model.cfg"), "").unwrap();
        let ty = Arc::new(HumanType::load(&dir.join("person.hum")).unwrap());
        std::fs::remove_dir_all(&dir).unwrap();
        let person = |id, remote, puppet, standing| Person {
            id,
            ty: ty.clone(),
            variant: 0,

            position: DVec3::Y * 1000.0,
            heading: 0.0,
            lheading: 0.0,
            place: Place::Ground,
            vel: DVec2::ZERO,
            pace: 1.1,
            activity: Activity::Stand,
            anim: OmsiAnim::default(),

            state: if standing {
                State::Standing
            } else {
                State::Pax(Box::new(Pax::new(1.1, 0.5)))
            },
            t_state: 0.0,



            interior: 0.0,

            tilt: Mat4::IDENTITY,
            age: 40.0,
            stuck: 0.0,
            ghost: 0.0,
            car_wait: 0.0,
            detour: 0.0,
            detour_side: 0.0,
            why: "",




            puppet,
            remote,
        };
        for task in [Task::SittingInBus, Task::InBusToPlace, Task::InBusToExit] {
            let mut old = PeopleSim::new(Path::new("/nonexistent"), 200);
            let mut rider = person(12, false, None, false);
            rider.place = Place::Bus(BusId::Player, Vec3::new(1.0, -3.0, 0.5));
            if let State::Pax(pax) = &mut rider.state {
                pax.inside = Some(BusId::Player);
                pax.bus = Some(BusId::Player);
                pax.task = task;
                pax.seat = Some(1);
                pax.dest = Some("Hauptbahnhof".into());
                pax.ticket_id = 3;
                pax.km_start = 2.5;
                pax.stop_request_at = Some((123, 4.0));
            }
            let mut ai = person(13, false, None, false);
            ai.place = Place::Bus(BusId::Ai(4), Vec3::ZERO);
            let mut just_exited = person(14, false, None, false);
            // The display pose can lag the authoritative passenger state by one tick.
            just_exited.place = Place::Bus(BusId::Player, Vec3::ZERO);
            old.people = vec![rider, ai, just_exited];
            old.seats.insert(BusId::Player, vec![true, true, true]);
            old.odometer.insert(BusId::Player, 8.5);
            old.next_id = 15;
            old.time = 42.0;
            old.ticket_cash = 7.5;
            old.tickets_sold = 4;
            if task == Task::InBusToPlace {
                old.desk_busy = Some(12);
                old.request = Some(("Fahrschein".into(), 2.5));
                old.paid = Some((5.0, 2.5));
                old.change_due = Some(2.5);
                let mut money = Money::new(Path::new("/nonexistent"), "currency.cfg");
                money.meshes.insert(1, (88, vec![99]));
                money.placed.push((
                    77,
                    Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)),
                    1,
                    false,
                    Some("Kasse".into()),
                ));
                old.money = Some(money);
            }
            let saved = old.into_player_passengers();
            assert_eq!(saved.0.people.len(), 1);
            let mut fresh = PeopleSim::new(Path::new("/nonexistent"), 200);
            fresh.people.push(person(12, false, None, true));
            fresh.bodies.ops.push(BodyOp::Spawn {
                id: 12,
                ty: ty.clone(),
                variant: 0,
                position: DVec3::ZERO,
            });
            assert_eq!(fresh.restore_player_passengers(saved), 1);
            assert_eq!(fresh.people.len(), 2);
            assert_ne!(fresh.people[0].id, 12);
            assert_eq!(fresh.seats[&BusId::Player], vec![false, true, false]);
            assert_eq!(fresh.odometer[&BusId::Player], 8.5);
            assert_eq!(fresh.time, 42.0);
            assert_eq!(fresh.ticket_cash, 7.5);
            assert_eq!(fresh.tickets_sold, 4);
            assert!(fresh.next_id > fresh.people[0].id);
            let State::Pax(pax) = &fresh.people[1].state else {
                panic!("lost passenger");
            };
            assert_eq!(pax.dest.as_deref(), Some("Hauptbahnhof"));
            assert_eq!(pax.task, task);
            if task == Task::InBusToPlace {
                assert_eq!(fresh.desk_busy, Some(12));
                assert_eq!(fresh.request.as_ref().unwrap().0, "Fahrschein");
                assert_eq!(fresh.paid, Some((5.0, 2.5)));
                assert_eq!(fresh.change_due, Some(2.5));
                let money = fresh.money.as_ref().unwrap();
                assert!(money.meshes.is_empty());
                assert_eq!(money.placed[0].0, 0);
                assert_eq!(
                    money.placed[0].1.w_axis.truncate(),
                    Vec3::new(1.0, 2.0, 3.0)
                );
                assert_eq!(money.placed[0].4.as_deref(), Some("Kasse"));
            }
            assert_eq!(pax.ticket_id, 3);
            assert_eq!(pax.km_start, 2.5);
            assert_eq!(pax.stop_request_at, Some((123, 4.0)));
            assert_eq!(fresh.bodies.ops.len(), 2);
            assert!(matches!(&fresh.bodies.ops[1], BodyOp::Spawn { id: 12, .. }));
        }
    }
}
