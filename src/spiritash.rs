use crate::{
    player::ChrIns,
    util::{get_game_base, get_world_chr_man, OutgoingMessage, Position, GAMEPUSH_SEND},
};

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
        //get list of all entities around the current player
        let mut chr_set = *((world_chr_man + 0x1CC60) as *mut u64); //legacy dungeon
        if chr_set == 0 {
            return None;
        }
        let open_field_chr_set = *((world_chr_man + 0x1E270) as *mut u64); //open world
        if open_field_chr_set == 0 {
            return None;
        }

        let mut use_legacy = false;
        let mut chr_count = *((open_field_chr_set + 0x20) as *mut u32);
        if chr_count == 0xffffffff {
            chr_count = *((chr_set + 0x10) as *mut u32);
            use_legacy = true;
        }

        if use_legacy {
            chr_set = *((chr_set + 0x18) as *mut u64);
        } else {
            chr_set = *((open_field_chr_set + 0x18) as *mut u64);
        }
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
