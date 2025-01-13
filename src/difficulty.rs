use crate::{
    player::GameDataMan,
    util::{display_message, get_game_base, get_section, FullscreenMsgIndex},
};
use broadsword::scanner;
use std::sync::LazyLock;

#[allow(non_camel_case_types, dead_code)]
struct MultiPlayerCorrectionParamData {
    param_id: u32,
    padding: u32,
    param_data: u64,
}

fn set_scaling() {
    let base = get_game_base().expect("Could not acquire game base");
    let set_mpscaling_for_chr_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u64)>(base + 0x3fada0) };

    //set the new value for the mpScaling
    //the code we inject into WHERE will pull this value for us whenever it is run by the game
    //and the game reruns it automatically for newly loaded enemies
    //So we just have to take care of the currently active enemies

    let const_correction_param = MultiPlayerCorrectionParamData {
        param_id: 0,
        padding: 0,
        param_data: 0,
    };

    let world_chr_man = get_world_chr_man().expect("Could not acquire world_chr_man");
    unsafe {
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
                set_mpscaling_for_chr_fn(chrins_enemy, &const_correction_param as *const _ as u64);
            }
        }
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
    if game_data_man.clear_count < 8 {
        game_data_man.clear_count += 1;
    }
    set_scaling();

    display_message(FullscreenMsgIndex::RecusantRankAdvanced);
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
    set_scaling();

    display_message(FullscreenMsgIndex::HunterRankAdvanced);
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
    let wcm_ptr_ptr = *wcm;
    Some(wcm_ptr_ptr as u64)
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
