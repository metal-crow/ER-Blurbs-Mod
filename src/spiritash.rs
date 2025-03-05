use crate::{
    player::{get_camera, ChrIns, WorldChrMan},
    reflection::get_instance,
    util::{get_game_base, get_world_chr_man, OutgoingMessage, Position, GAMEPUSH_SEND},
};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

pub fn get_position() -> Option<Vec<Position>> {
    let mut positions = Vec::new();

    let base = get_game_base().expect("Could not acquire game base");
    unsafe {
        //check if we're loading
        let loading_helper = *((base + 0x3d60ec8) as *mut u64);
        if loading_helper == 0 {
            return None;
        }
        let loaded = *((loading_helper + 0xED) as *mut u8);
        if loaded != 1 {
            return None;
        }
    }

    let world_chr_man = {
        let world_chr_man = get_world_chr_man();
        if world_chr_man.is_none() {
            log::info!("world_chr_man does not have an instance");
            return None;
        }

        world_chr_man.unwrap()
    };
    if world_chr_man == 0 {
        return None;
    }

    unsafe {
        let buddy_chr_set = (world_chr_man + 0x10f90) as u64;
        let mut chr_count = *((buddy_chr_set + 0x20) as *mut u32);
        if chr_count == 0xffffffff {
            chr_count = *((buddy_chr_set + 0x10) as *mut u32);
        }

        let chr_set = *((buddy_chr_set + 0x18) as *mut u64);
        if chr_set == 0 {
            return None;
        }

        for i in 1..chr_count {
            let chrins_ptr = *((chr_set + (i * 0x10) as u64) as *mut u64);
            if chrins_ptr != 0 {
                let chrins = chrins_ptr as *const ChrIns;
                if (*chrins).vftable == 0 {
                    continue;
                }

                //check the teamtype is spiritash
                if (*chrins).team_type != 0x2f {
                    continue;
                }

                let coords = &(*chrins).module_container.physics.unk70_position;
                let handle = (*chrins).field_ins_handle.instance_id;

                positions.push(Position {
                    id: handle,
                    x: coords.0,
                    y: coords.1,
                    z: coords.2,
                });
            }
        }
    }

    return Some(positions);
}

lazy_static! {
    static ref LAST_SPIRIT_CHECK: Mutex<HashMap<i32, u32>> = Mutex::new(HashMap::new());
}

pub fn get_status() {
    let base = get_game_base().expect("Could not acquire game base");
    unsafe {
        //check if we're loading
        let loading_helper = *((base + 0x3d60ec8) as *mut u64);
        if loading_helper == 0 {
            return;
        }
        let loaded = *((loading_helper + 0xED) as *mut u8);
        if loaded != 1 {
            return;
        }
    }

    let world_chr_man = {
        let world_chr_man = get_world_chr_man();
        if world_chr_man.is_none() {
            return;
        }

        world_chr_man.unwrap()
    };
    if world_chr_man == 0 {
        return;
    }

    let mut cur_spirit_check: HashMap<i32, (*mut ChrIns, u32)> = HashMap::new();

    unsafe {
        let buddy_chr_set = (world_chr_man + 0x10f90) as u64;
        let mut chr_count = *((buddy_chr_set + 0x20) as *mut u32);
        if chr_count == 0xffffffff {
            chr_count = *((buddy_chr_set + 0x10) as *mut u32);
        }

        let chr_set = *((buddy_chr_set + 0x18) as *mut u64);
        if chr_set == 0 {
            return;
        }

        for i in 1..chr_count {
            let chrins_ptr = *((chr_set + (i * 0x10) as u64) as *mut u64);
            if chrins_ptr != 0 {
                let chrins = chrins_ptr as *mut ChrIns;
                if (*chrins).vftable == 0 {
                    continue;
                }

                //check the teamtype is spiritash
                if (*chrins).team_type != 0x2f {
                    continue;
                }

                //buddy system seems to not actually set it's count, only sets capacity so we have to manually count
                cur_spirit_check.insert(
                    (*chrins).field_ins_handle.instance_id,
                    (chrins, (*chrins).module_container.data.hp),
                );
            }
        }
    }

    let mut last_check = LAST_SPIRIT_CHECK.lock().unwrap();

    //if the player is dead, send a leave event and clear the last_check
    let instance = get_instance::<WorldChrMan>()
        .expect("Could not find WorldChrMan static")
        .unwrap();
    let hp = instance.main_player.module_container.data.hp;
    if hp == 0 {
        last_check.retain(|id, _| {
            if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
                sender
                    .send(tungstenite::Message::Text(
                        serde_json::to_string(&OutgoingMessage::SpiritLeaveEvent { id: *id })
                            .unwrap(),
                    ))
                    .expect("Send failed");
            }
            return false;
        });
    } else {
        last_check.retain(|id, hp| {
            //newly desummoned. existed and had hp before, but doesn't now
            if *hp > 0 && !cur_spirit_check.contains_key(&id) {
                if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
                    sender
                        .send(tungstenite::Message::Text(
                            serde_json::to_string(&OutgoingMessage::SpiritLeaveEvent { id: *id })
                                .unwrap(),
                        ))
                        .expect("Send failed");
                }
                return false;
            }
            return true;
        });

        let apply_speffect_fn =
            unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u32, u8)>(base + 0x3e8cf0) };

        for (id, (chrins, hp)) in cur_spirit_check {
            //newly summoned. didn't exist before, does now with hp
            if !last_check.contains_key(&id) && hp > 0 {
                if let Some(cam) = get_camera() {
                    if let Some(spirits) = get_position() {
                        if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
                            sender
                                .send(tungstenite::Message::Text(
                                    serde_json::to_string(&OutgoingMessage::SpiritSummonEvent {
                                        id: id,
                                        player: cam,
                                        spirit: spirits,
                                    })
                                    .unwrap(),
                                ))
                                .expect("Send failed");
                        }
                    }
                }

                //hacks!
                unsafe {
                    //apply the host mirror speffect to the spirit, to remove the blue glow
                    apply_speffect_fn(chrins as u64, 360800, 1);
                    //we need to save off the original hp base, stick it in here
                    (*chrins).module_container.data.recoverable_hp_left1 =
                        (*chrins).module_container.data.hp_base as f32;
                }

                last_check.insert(id, hp);
            }
            //newly dead. existed before, and still does now but with no hp
            else if last_check.contains_key(&id) && last_check[&id] > 0 && hp == 0 {
                if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
                    sender
                        .send(tungstenite::Message::Text(
                            serde_json::to_string(&OutgoingMessage::SpiritDeathEvent { id: id })
                                .unwrap(),
                        ))
                        .expect("Send failed");
                }
                last_check.insert(id, hp);
            }
        }
    }
}

pub fn set_size(size: f32, power: f32) {
    let base = get_game_base().expect("Could not acquire game base");
    unsafe {
        //check if we're loading
        let loading_helper = *((base + 0x3d60ec8) as *mut u64);
        if loading_helper == 0 {
            return;
        }
        let loaded = *((loading_helper + 0xED) as *mut u8);
        if loaded != 1 {
            return;
        }
    }

    let world_chr_man = {
        let world_chr_man = get_world_chr_man();
        if world_chr_man.is_none() {
            return;
        }

        world_chr_man.unwrap()
    };
    if world_chr_man == 0 {
        return;
    }

    unsafe {
        let buddy_chr_set = (world_chr_man + 0x10f90) as u64;
        let mut chr_count = *((buddy_chr_set + 0x20) as *mut u32);
        if chr_count == 0xffffffff {
            chr_count = *((buddy_chr_set + 0x10) as *mut u32);
        }

        let chr_set = *((buddy_chr_set + 0x18) as *mut u64);
        if chr_set == 0 {
            return;
        }

        for i in 1..chr_count {
            let chrins_ptr = *((chr_set + (i * 0x10) as u64) as *mut u64);
            if chrins_ptr != 0 {
                let chrins = chrins_ptr as *mut ChrIns;
                if (*chrins).vftable == 0 {
                    continue;
                }

                //check the teamtype is spiritash
                if (*chrins).team_type != 0x2f {
                    continue;
                }

                (*chrins).chr_ctrl.scale_size[0] = size;
                (*chrins).chr_ctrl.scale_size[1] = size;
                (*chrins).chr_ctrl.scale_size[2] = size;

                (*chrins).module_container.data.hp_base =
                    ((*chrins).module_container.data.recoverable_hp_left1 * power) as u32;
                (*chrins).module_container.behavior.animation_speed = power;
            }
        }
    }
}
