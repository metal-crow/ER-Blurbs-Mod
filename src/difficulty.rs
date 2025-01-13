use crate::{
    player::GameDataMan,
    util::{display_message, get_game_base, get_section, FullscreenMsgIndex},
};
use broadsword::scanner;
use std::sync::LazyLock;

pub fn set_scaling() {
    let base = get_game_base().expect("Could not acquire game base");
    let apply_speffect_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u32, u8)>(base + 0x3e8cf0) };

    //Apply the NG+ speffects to all active enemies
    //This is run as a task, so it will apply to any newly loaded enemies as well
    let world_chr_man = get_world_chr_man().expect("Could not acquire world_chr_man");

    unsafe {
        //get list of all enemies around the current player
        //This code is taken from inuNorii's Kill All Mobs script in TGA table
        let mut chr_set = *((world_chr_man + 0x1CC60) as *mut u64); //legacy dungeon
        let open_field_chr_set = *((world_chr_man + 0x1E270) as *mut u64); //open world

        let mut use_legacy = false;
        let mut chr_count = *((open_field_chr_set + 0x20) as *mut u32);
        if chr_count == 0xffffffff {
            chr_count = *((chr_set + 0x10) as *mut u32);
            use_legacy = true
        }

        if use_legacy {
            chr_set = *((chr_set + 0x18) as *mut u64);
        } else {
            chr_set = *((open_field_chr_set + 0x18) as *mut u64);
        }

        for i in 1..chr_count {
            let chrins_enemy = *((chr_set + (i * 0x10) as u64) as *mut u64);
            if chrins_enemy != 0 {
                //for this enemy, get the speffect for NG+1 scaling speffect
                let npcparam = *((chrins_enemy + 0x5f0) as *mut u64);
                let npcparam_st = *((npcparam + 0) as *mut u64);
                let gameclear_speffect = *((npcparam_st + 0x6c) as *mut u32);

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

pub fn get_game_data_man<'a>() -> Option<&'a mut GameDataMan> {
    let gdm = &*GAME_DATA_MAN;
    let gdm_ptr_ptr = *gdm as *const *mut GameDataMan;

    unsafe { gdm_ptr_ptr.as_ref().and_then(|gdm_ptr| gdm_ptr.as_mut()) }
}

static GAME_DATA_MAN: LazyLock<usize> = LazyLock::new(|| {
    const GAME_DATA_MAN_PATTERN: &str = "48 8B 05 ? ? ? ? 48 85 C0 74 05 48 8B 40 58 C3 C3";
    const INSTRUCTION_SIZE: usize = 7;

    let (text_range, text_slice) = get_section(".text").expect("Could not get game text section.");

    let pattern = scanner::Pattern::from_byte_pattern(GAME_DATA_MAN_PATTERN)
        .expect("Could not parse pattern");

    let result = scanner::simple::scan(text_slice, &pattern).expect("Could not find GameDataMan");

    let mut buff = [0; 4];
    buff.copy_from_slice(&text_slice[result.location + 3..result.location + 3 + 4]);
    let gameman =
        text_range.start + result.location + INSTRUCTION_SIZE + u32::from_le_bytes(buff) as usize;
    log::info!("GameDataMan ptr {result:?} + {text_range:?} = {gameman:?}");
    return gameman;
});

pub fn get_world_chr_man() -> Option<u64> {
    let wcm = &*WORLD_CHR_MAN;
    let wcm_ptr_ptr = *wcm as *mut u64;
    unsafe { Some(*wcm_ptr_ptr as u64) }
}

static WORLD_CHR_MAN: LazyLock<usize> = LazyLock::new(|| {
    const WORLD_CHR_MAN_PATTERN: &str = "48 8B 05 ?? ?? ?? ?? 48 85 C0 74 0F 48 39 88";
    const INSTRUCTION_SIZE: usize = 7;

    let (text_range, text_slice) = get_section(".text").expect("Could not get game text section.");

    let pattern = scanner::Pattern::from_byte_pattern(WORLD_CHR_MAN_PATTERN)
        .expect("Could not parse pattern");

    let result = scanner::simple::scan(text_slice, &pattern).expect("Could not find WorldChrMan");

    let mut buff = [0; 4];
    buff.copy_from_slice(&text_slice[result.location + 3..result.location + 3 + 4]);
    let worldman =
        text_range.start + result.location + INSTRUCTION_SIZE + u32::from_le_bytes(buff) as usize;
    log::info!("WorldChrMan ptr {result:?} + {text_range:?} = {worldman:?}");
    return worldman;
});
