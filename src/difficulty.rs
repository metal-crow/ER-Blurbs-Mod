use crate::{
    player::{GameDataMan, WorldChrMan},
    reflection::{self, get_instance},
    task::CSTaskGroupIndex,
    util::{display_message, get_section},
};
use broadsword::scanner;
use std::{slice::SliceIndex, sync::LazyLock};

pub fn increase_difficulty() {
    let game_data_man = {
        let game_data_man = get_game_data_man();
        if game_data_man.is_none() {
            log::info!("GameDataMan does not have an instance");
            return;
        }

        game_data_man.unwrap()
    };
    if (game_data_man.clear_count < 8) {
        game_data_man.clear_count += 1;
    }
    display_message(format!("DIFFICULTY UP: {}", game_data_man.clear_count));
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

    if (game_data_man.clear_count > 1) {
        game_data_man.clear_count -= 1;
    }
    display_message(format!("DIFFICULTY DOWN: {}", game_data_man.clear_count));
}

pub fn get_game_data_man<'a>() -> Option<&'a mut GameDataMan> {
    let gdm = &*GAME_DATA_MAN;
    let gdm_ptr_ptr = *gdm as *const *mut GameDataMan;

    unsafe { gdm_ptr_ptr.as_ref().and_then(|gdm_ptr| gdm_ptr.as_mut()) }
}

static GAME_DATA_MAN: LazyLock<usize> = LazyLock::new(|| {
    const GAME_DATA_MAN_PATTERN: &str = "48 8B 05 ? ? ? ? 48 85 C0 74 05 48 8B 40 58 C3 C3";
    const OFFSET_SIZE: usize = std::mem::size_of::<u32>();
    const INSTRUCTION_SIZE: usize = 7;

    let (text_range, text_slice) = get_section(".text").expect("Could not get game text section.");

    let pattern = scanner::Pattern::from_byte_pattern(GAME_DATA_MAN_PATTERN)
        .expect("Could not parse pattern");

    let result = scanner::simple::scan(text_slice, &pattern).expect("Could not find GameDataMan");

    let mut buff = [0; 4];
    buff.copy_from_slice(&text_slice[result.location + 3..result.location + 3 + 4]);
    let gameman = unsafe {
        text_range.start + result.location + INSTRUCTION_SIZE + u32::from_le_bytes(buff) as usize
    };
    log::info!("GameDataMan ptr {result:?} + {text_range:?} = {gameman:?}");
    return gameman;
});
