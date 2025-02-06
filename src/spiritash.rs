use crate::{
    player::ChrIns,
    util::{get_game_base, get_world_chr_man, OutgoingMessage, Position, GAMEPUSH_SEND},
};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn report_position() {
    if let Some(pos) = get_position() {
        if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
            sender
                .send(tungstenite::Message::Text(
                    serde_json::to_string(&OutgoingMessage::SpiritPositionEvent { pos: pos })
                        .unwrap(),
                ))
                .expect("Send failed");
        }
    }
}

fn get_position() -> Option<Vec<Position>> {
    log::info!("Getting Spirit coords");

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

                positions.push(Position {
                    x: coords.0,
                    z: coords.1,
                    y: coords.2,
                });
            }
        }
    }

    return Some(positions);
}

static LAST_SPIRIT_EXIST_N_ALIVE: AtomicBool = AtomicBool::new(false);

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

    let mut cur_spirit_count = 0;
    let mut cur_spirit_hp = 0;

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
                let chrins = chrins_ptr as *const ChrIns;
                if (*chrins).vftable == 0 {
                    continue;
                }

                //check the teamtype is spiritash
                if (*chrins).team_type != 0x2f {
                    continue;
                }

                //buddy system seems to not actually set it's count, only sets capacity so we have to manually count
                cur_spirit_count += 1;
                cur_spirit_hp += (*chrins).module_container.data.hp;
            }
        }
    }

    //only send a creation/leave/death with all summons leave
    //if it's a multi-summon, wait til they all go away
    let last_check = LAST_SPIRIT_EXIST_N_ALIVE.load(Ordering::SeqCst);

    //last we saw they didn't exist, now they do and are alive
    if !last_check && cur_spirit_count > 0 && cur_spirit_hp > 0 {
        LAST_SPIRIT_EXIST_N_ALIVE.store(true, Ordering::SeqCst);

        if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
            sender
                .send(tungstenite::Message::Text(
                    serde_json::to_string(&OutgoingMessage::SpiritSummonEvent).unwrap(),
                ))
                .expect("Send failed");
        }
    }
    //last we saw they existed and were alive, now they don't exist
    else if last_check && cur_spirit_count == 0 {
        LAST_SPIRIT_EXIST_N_ALIVE.store(false, Ordering::SeqCst);

        if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
            sender
                .send(tungstenite::Message::Text(
                    serde_json::to_string(&OutgoingMessage::SpiritLeaveEvent).unwrap(),
                ))
                .expect("Send failed");
        }
    }
    //last we saw they existed and were alive, now they aren't alive
    else if last_check && cur_spirit_hp == 0 {
        LAST_SPIRIT_EXIST_N_ALIVE.store(false, Ordering::SeqCst);

        if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
            sender
                .send(tungstenite::Message::Text(
                    serde_json::to_string(&OutgoingMessage::SpiritDeathEvent).unwrap(),
                ))
                .expect("Send failed");
        }
    }
}
