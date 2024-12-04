use crate::reflection::SectionLookupError;
use broadsword::runtime;
use std::{ops, slice};
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
