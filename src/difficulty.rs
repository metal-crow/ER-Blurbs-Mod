use crate::util::{
    display_message, get_game_base, get_game_data_man, get_world_chr_man, FullscreenMsgIndex,
};

pub fn set_scaling() {
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

    let apply_speffect_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u32, u8)>(base + 0x3e8cf0) };

    //Apply the NG+ speffects to all active enemies
    //This is run as a task, so it will apply to any newly loaded enemies as well
    let world_chr_man = {
        let world_chr_man = get_world_chr_man();
        if world_chr_man.is_none() {
            log::info!("world_chr_man does not have an instance");
            return;
        }

        world_chr_man.unwrap()
    };
    if world_chr_man == 0 {
        return;
    }

    unsafe {
        //get list of all enemies around the current player
        //This code is taken from inuNorii's Kill All Mobs script in TGA table
        let mut chr_set = *((world_chr_man + 0x1CC60) as *mut u64); //legacy dungeon
        if chr_set == 0 {
            return;
        }
        let open_field_chr_set = *((world_chr_man + 0x1E270) as *mut u64); //open world
        if open_field_chr_set == 0 {
            return;
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
            return;
        }

        for i in 1..chr_count {
            let chrins_enemy = *((chr_set + (i * 0x10) as u64) as *mut u64);
            if chrins_enemy != 0 {
                let chrins_enemy_vtable = *((chrins_enemy + 0) as *mut usize);
                if chrins_enemy_vtable == 0 {
                    continue;
                }

                //for this enemy, get the speffect for NG+1 scaling speffect
                //need to check vtable to determine offset
                let param: u64;
                //enemy
                if chrins_enemy_vtable == base + 0x2a44010 {
                    param = *((chrins_enemy + 0x598) as *mut u64);
                }
                //player
                else if chrins_enemy_vtable == base + 0x2a7cb40 {
                    param = *((chrins_enemy + 0x5f0) as *mut u64);
                }
                //unknown
                else {
                    continue;
                }
                if param == 0 {
                    continue;
                }

                let npcparam_st = *((param + 0) as *mut u64);
                if npcparam_st == 0 {
                    continue;
                }
                let gameclear_speffect = *((npcparam_st + 0x6c) as *mut u32);
                if gameclear_speffect == 0 {
                    continue;
                }

                //i don't have to do any extar NG+X X>1 work, since the game seems to magically apply the extra scaling based on the game_data_man.clear_count
                //don't have to clear the value either, since it seems the game also does that

                //get the speffect for NG+ for the enemy, and apply it
                apply_speffect_fn(chrins_enemy, gameclear_speffect, 1);
            }
        }
    }
}

fn ng_val_to_msg(ng: u32, isup: bool) -> FullscreenMsgIndex {
    match (ng, isup) {
        (0, false) => FullscreenMsgIndex::Down0,
        (1, false) => FullscreenMsgIndex::Down1,
        (2, false) => FullscreenMsgIndex::Down2,
        (3, false) => FullscreenMsgIndex::Down3,
        (4, false) => FullscreenMsgIndex::Down4,
        (5, false) => FullscreenMsgIndex::Down5,
        (6, false) => FullscreenMsgIndex::Down6,
        (1, true) => FullscreenMsgIndex::Up1,
        (2, true) => FullscreenMsgIndex::Up2,
        (3, true) => FullscreenMsgIndex::Up3,
        (4, true) => FullscreenMsgIndex::Up4,
        (5, true) => FullscreenMsgIndex::Up5,
        (6, true) => FullscreenMsgIndex::Up6,
        (7, true) => FullscreenMsgIndex::Up7,
        (_, _) => FullscreenMsgIndex::YouDied,
    }
}

pub fn increase_difficulty() {
    let game_data_man = {
        let game_data_man = get_game_data_man();
        if game_data_man.is_none() {
            log::info!("GameDataMan does not have an instance");
            return;
        }

        game_data_man.unwrap()
    };
    if game_data_man.clear_count < 7 {
        game_data_man.clear_count += 1;
    }

    display_message(ng_val_to_msg(game_data_man.clear_count, true));
}

pub fn decrease_difficulty() {
    let game_data_man = {
        let game_data_man = get_game_data_man();
        if game_data_man.is_none() {
            log::info!("GameDataMan does not have an instance");
            return;
        }

        game_data_man.unwrap()
    };

    if game_data_man.clear_count > 0 {
        game_data_man.clear_count -= 1;
    }

    display_message(ng_val_to_msg(game_data_man.clear_count, false));
}
