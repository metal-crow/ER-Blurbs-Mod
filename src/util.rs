use crate::player::{get_camera, GameDataMan};
use crate::reflection::SectionLookupError;
use crate::spiritash;
use broadsword::runtime;
use broadsword::scanner;
use lazy_static::lazy_static;
use serde::Serialize;
use std::sync::mpsc::Sender;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::{ops, slice};
use widestring::U16CString;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    BloodMessageEvent {
        text: String,
    },
    PositionEvent {
        player: CameraInfo,
        spirit: Vec<Position>,
    },
    SpiritSummonEvent,
    SpiritLeaveEvent,
    SpiritDeathEvent,
}

lazy_static! {
    pub(crate) static ref GAMEPUSH_SEND: Mutex<Option<Sender<tungstenite::Message>>> =
        Mutex::new(None);
}

#[derive(Debug, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Serialize)]
pub struct CameraInfo {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

pub fn report_position() {
    if let Some(cam) = get_camera() {
        if let Some(spirits) = spiritash::get_position() {
            if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
                sender
                    .send(tungstenite::Message::Text(
                        serde_json::to_string(&OutgoingMessage::PositionEvent {
                            player: cam,
                            spirit: spirits,
                        })
                        .unwrap(),
                    ))
                    .expect("Send failed");
            }
        }
    }
}

/// Attempts to figure out what people called the exe
fn get_game_module() -> Option<&'static str> {
    const MODULE_NAMES: [&str; 2] = ["eldenring.exe", "start_protected_game.exe"];

    for name in MODULE_NAMES.iter() {
        if runtime::get_module_handle(name).is_ok() {
            return Some(name);
        }
    }
    None
}

pub fn get_section(section: &str) -> Result<(ops::Range<usize>, &[u8]), SectionLookupError> {
    let module = get_game_module().ok_or(SectionLookupError::NoGameBase)?;

    let section_range = runtime::get_module_section_range(module, section)
        .map_err(|_| SectionLookupError::SectionNotFound)?;

    let section_slice = unsafe {
        slice::from_raw_parts(
            section_range.start as *const u8,
            section_range.end - section_range.start,
        )
    };

    Ok((section_range, section_slice))
}

pub fn get_game_base() -> Option<usize> {
    const MODULE_NAMES: [&str; 2] = ["eldenring.exe", "start_protected_game.exe"];

    for name in MODULE_NAMES.iter() {
        let handle = runtime::get_module_handle(name);
        if handle.is_ok() {
            return handle.ok();
        }
    }
    None
}

#[allow(dead_code)]
pub fn display_custom_text_message(text: String) {
    let base = get_game_base().expect("Could not acquire game base");

    let displaymsg_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, *mut u16)>(base + 0x841c50) };
    unsafe {
        let csmenu_man_imp = *((base + 0x3d6b7b0) as *mut u64);
        let fe_system_announce_view_model = *((csmenu_man_imp + 0x860) as *mut u64);
        let message = U16CString::from_str_unchecked(&text);
        displaymsg_fn(fe_system_announce_view_model, message.into_raw()); //this will cause a memory leak. i don't care, it's fine probably
    }
}

#[repr(u32)]
#[allow(non_camel_case_types, dead_code)]
pub enum FullscreenMsgIndex {
    DemigodFelled = 1,
    LegendFelled = 2,
    GreatEnemyFelled = 3,
    EnemyFelled = 4,
    YouDied = 5,
    HostVanquished = 7,
    BloodFingerVanquished = 8,
    DutyFullFilled = 9,
    LostGraceDiscovered = 11,
    MapFound = 17,
    GreatRuneRestored = 21,
    GodSlain = 22,
    DuelistVanquished = 23,
    Defeat = 16,
    InvaderVanquished = 25,
    Down6 = 13,
    Down5 = 14,
    Down4 = 15,
    Down3 = 24,
    Down2 = 30,
    Down1 = 31,
    Down0 = 32,
    Up7 = 33,
    Up6 = 34,
    Up5 = 35,
    Up4 = 36,
    Up3 = 37,
    Up2 = 38,
    Up1 = 39,
    HeartStolen = 40,
}

pub fn display_message(msg_id: FullscreenMsgIndex) {
    let base = get_game_base().expect("Could not acquire game base");

    let displaymsg_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u32)>(base + 0x766460) };
    unsafe {
        let csmenu_man_imp = *((base + 0x3d6b7b0) as *mut u64);
        displaymsg_fn(csmenu_man_imp, msg_id as u32);
    }
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

pub fn get_field_area() -> Option<u64> {
    let wcm = &*FIELD_AREA;
    let wcm_ptr_ptr = *wcm as *mut u64;
    unsafe { Some(*wcm_ptr_ptr as u64) }
}

static FIELD_AREA: LazyLock<usize> = LazyLock::new(|| {
    const FAPATTERN: &str = "48 8B 3D ?? ?? ?? ?? 49 8B D8 48 8B F2 4C 8B F1 48 85 FF";
    const INSTRUCTION_SIZE: usize = 7;

    let (text_range, text_slice) = get_section(".text").expect("Could not get game text section.");

    let pattern = scanner::Pattern::from_byte_pattern(FAPATTERN).expect("Could not parse pattern");

    let result = scanner::simple::scan(text_slice, &pattern).expect("Could not find Field Area");

    let mut buff = [0; 4];
    buff.copy_from_slice(&text_slice[result.location + 3..result.location + 3 + 4]);
    let field =
        text_range.start + result.location + INSTRUCTION_SIZE + u32::from_le_bytes(buff) as usize;
    log::info!("Field ptr {result:?} + {text_range:?} = {field:?}");
    return field;
});
