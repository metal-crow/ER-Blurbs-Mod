use crate::reflection::SectionLookupError;
use broadsword::runtime;
use lazy_static::lazy_static;
use std::{ops, slice};
use widestring::{u16cstr, U16CString};

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

pub fn display_message(text: String) {
    let base = get_game_base().expect("Could not acquire game base");
    let (data_range, data_slice) = get_section(".data").expect("Could not get game data section.");

    let displaymsg_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, *mut u16)>(base + 0x841c50) };
    unsafe {
        let CSMenuManImp = *((base + 0x3d6b7b0) as *mut u64);
        let FeSystemAnnounceViewModel = *((CSMenuManImp + 0x860) as *mut u64);
        let message = U16CString::from_str_unchecked(&text);
        displaymsg_fn(FeSystemAnnounceViewModel, message.into_raw()); //this will cause a memory leak. i don't care, it's fine probably
    }
}
