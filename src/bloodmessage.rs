use lazy_static::lazy_static;
use retour::static_detour;
use std::ptr;
use std::sync::atomic::AtomicU64;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering},
        OnceLock, RwLock,
    },
};
use tungstenite::Message;
use widestring::{U16CStr, U16CString};

use crate::util::{get_game_base, OutgoingMessage, GAMEPUSH_SEND};
use crate::{
    player::{MapId, WorldChrMan},
    reflection::{get_instance, DLRFLocatable},
};

// Despawn the message and remove the message text entry
pub fn delete_message(message: &str) {
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

    log::info!("Removing message {message:?}");

    let base = get_game_base().expect("Could not acquire game base");
    let netman = {
        let instance = get_instance::<CSNetMan>().expect("Could not find CSNetMan static");

        if instance.is_none() {
            log::info!("CSNetMan does not have an instance");
            return;
        }

        instance.unwrap()
    };
    let destruct_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u32)>(base + 0x1b73f0) };
    let dealloc_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u64)>(base + 0xe1d990) };

    //remove the entry(s) from the BloodMessageInsMan list
    unsafe {
        let mut current_ptr = netman
            .blood_message_db
            .blood_message_ins_man_1
            .blood_message_list_head as *mut BloodMessageIns;
        let mut prev_ptr: *mut BloodMessageIns = ptr::null_mut();
        while !current_ptr.is_null() {
            let current = &mut *current_ptr;

            let current_txt = get_message(current.template);
            if current_txt.is_some() && normalized_is_equal(current_txt.unwrap(), message) {
                // Remove the current entry
                if !prev_ptr.is_null() {
                    (*prev_ptr).next = current.next;
                } else {
                    // Update the head pointer in the manager if the first node is being removed
                    (*(netman.blood_message_db.blood_message_ins_man_1)).blood_message_list_head =
                        current.next;
                }

                // Move to the next node
                let next_ptr = current.next as *mut BloodMessageIns;

                log::info!("Removing {current_ptr:?}");
                remove_message(current.template); //remove the template entry
                                                  // Free and destruct the BloodMessageIns object
                destruct_fn(current_ptr as u64, 0); //this cleans up the sfx but doesn't free the memory
                dealloc_fn(0, current_ptr as u64); //this frees the memory

                current_ptr = next_ptr;
            } else {
                // Move to the next node, keeping the current as the previous
                prev_ptr = current_ptr;
                current_ptr = current.next as *mut BloodMessageIns;
            }
        }
    }
}

// Spawns a message on the floor at the players location
pub fn spawn_message(message: &str, msg_visual: i32) {
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

    log::info!("Spawning message {message:?}");

    let netman = {
        let instance = get_instance::<CSNetMan>().expect("Could not find CSNetMan static");

        if instance.is_none() {
            log::info!("CSNetMan does not have an instance");
            return;
        }

        instance.unwrap()
    };

    let world_chr_man = {
        let instance = get_instance::<WorldChrMan>().expect("Could not find WorldChrMan static");

        if instance.is_none() {
            log::info!("WorldChrMan does not have an instance");
            return;
        }

        instance.unwrap()
    };

    let map_id = world_chr_man.main_player.map_id_1;

    let ride_info = &world_chr_man.main_player.module_container.ride;
    let player_info = &world_chr_man.main_player.module_container.physics;
    let map_coordinates = match ride_info.is_mounted {
        0 => &player_info.unk70_position,
        1_u8..=u8::MAX => &ride_info.position,
    };

    let base = get_game_base().expect("Could not acquire game base");

    let params = SpawnMessageParams {
        blood_message_db_item: 0x0,
        map_id,
        position_x: map_coordinates.0,
        position_y: map_coordinates.1,
        position_z: map_coordinates.2,
        angle: -3.13653,
        template_id: add_message(message), //the only thing we need here is a unique id for lookup later in our BLOOD_MESSAGE_LOOKUP_HOOK
        unk1e: -1,
        unk1f: 66,
        unk20: 30001,
        unk24: 0,
        unk28: 0,
        unk2c: -1,
        magic_value: u32::MAX, //this is a functional magic value, don't touch
        unk34: -1,
        message_sign_visual: msg_visual,
        unk3c: 0,
        unk40: -1,
        unk44: -1,
        unk48: -1,
        unk4c: -1,
    };

    let spawn_fn = unsafe {
        std::mem::transmute::<
            usize,
            extern "C" fn(&BloodMessageInsMan, &SpawnMessageParams, u32, &u32, &u64),
        >(base + 0x1b9720)
    };

    spawn_fn(
        netman.blood_message_db.blood_message_ins_man_1,
        &params,
        4,
        &0u32,
        &0u64,
    );

    log::info!("Spawned message at {map_id:?} - {map_coordinates:?} template num {0} with text \"{message}\"", params.template_id);
}

#[repr(C)]
struct CSNetMan<'a> {
    pub vftable: usize,
    unk8: [u8; 0x60],
    pub sos_db: usize,
    pub wandering_ghost_db: usize,
    pub blood_message_db: &'a mut CSNetBloodMessageDb<'a>,
    pub bloodstain_db: usize,
    pub bonfire_db: usize,
    pub spiritual_statue_db: usize,
    // the rest....
}

impl DLRFLocatable for CSNetMan<'_> {
    const DLRF_NAME: &'static str = "CSNetMan";
}

#[repr(C)]
struct CSNetBloodMessageDb<'a> {
    pub vftable: usize,
    unk8: [u8; 0x58],
    blood_message_ins_man_1: &'a mut BloodMessageInsMan,
}

#[repr(C)]
struct BloodMessageInsMan {
    pub vftable: usize,
    unk8: [u8; 0x8],
    blood_message_list_head: u64,
}

#[repr(C)]
struct BloodMessageIns {
    unk1: [u8; 0x2C],
    template: u16,
    unk2: [u8; 0x89A],
    next: u64,
}
const _: () = assert!(std::mem::size_of::<BloodMessageIns>() == 0x8d0);
const _: () = assert!(std::mem::offset_of!(BloodMessageIns, template) == 0x2C);
const _: () = assert!(std::mem::offset_of!(BloodMessageIns, next) == 0x8c8);

#[repr(C)]
struct SpawnMessageParams {
    // Houses a pointer to a blood message db item ordinarily, rendering
    // assumes dev message if it's a null ptr.
    pub blood_message_db_item: usize,
    pub map_id: MapId,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub angle: f32,
    pub template_id: u16,
    pub unk1e: i8,
    pub unk1f: u8,
    pub unk20: u32,
    pub unk24: u32,
    pub unk28: u32,
    pub unk2c: i32,
    pub magic_value: u32,
    pub unk34: i32,
    pub message_sign_visual: i32,
    pub unk3c: u32,
    pub unk40: i32,
    pub unk44: i32,
    pub unk48: i32,
    pub unk4c: i32,
}

static MESSAGE_COUNTER: AtomicU16 = AtomicU16::new(1);
static MESSAGE_TABLE: OnceLock<RwLock<HashMap<u16, U16CString>>> = OnceLock::new();

fn add_message(message: &str) -> u16 {
    let index = MESSAGE_COUNTER.fetch_add(1, Ordering::Relaxed);

    MESSAGE_TABLE
        .get_or_init(Default::default)
        .write()
        .expect("Could not acquire message table write lock")
        .insert(index, U16CString::from_str(message).unwrap());

    index
}

fn normalized_is_equal(msg1: *const u16, msg2: &str) -> bool {
    let mut basic_message = String::from(msg2);
    basic_message.retain(|c| !c.is_whitespace());
    let mut basic_text = unsafe { U16CString::from_ptr_str(msg1) }.to_string_lossy();
    basic_text.retain(|c| !c.is_whitespace());
    return basic_message == basic_text;
}

fn get_message(index: u16) -> Option<*const u16> {
    MESSAGE_TABLE
        .get_or_init(Default::default)
        .read()
        .expect("Could not acquire message table read lock")
        .get(&index)
        .map(|f| f.as_ptr())
}

fn remove_message(id: u16) {
    let mut map = MESSAGE_TABLE
        .get_or_init(Default::default)
        .write()
        .expect("Could not acquire message table read/write lock");

    map.remove(&id);
}

static_detour! {
    static BLOOD_MESSAGE_LOOKUP_HOOK: unsafe extern "system" fn(u64, u32) -> *const u16;
}

pub fn init_hooks() {
    let base = get_game_base().expect("Could not acquire game base");

    //this hooks MsgRepositoryImpCategory::GetEntry, which is called by MsgRepositoryImp::LookupEntry
    let msg_hook_location = base + 0x266dc20;
    unsafe {
        BLOOD_MESSAGE_LOOKUP_HOOK
            .initialize(std::mem::transmute(msg_hook_location), blood_message_lookup)
            .expect("Could not initialize blood message hook");

        BLOOD_MESSAGE_LOOKUP_HOOK
            .enable()
            .expect("Could not enable blood message hook");
    }
}

lazy_static! {
    static ref msg_last_read: AtomicU64 = AtomicU64::new(0);
}

fn blood_message_lookup(param_1: u64, template_id: u32) -> *const u16 {
    if let Ok(message_index) = u16::try_from(template_id) {
        if let Some(message) = get_message(message_index) {
            //i can't just call SEND here, this is hit every frame. Check the last time we read it
            let now = SystemTime::now();
            let cur_read_time = now
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            if cur_read_time - msg_last_read.load(Ordering::Relaxed) > 2 {
                if let Some(sender) = GAMEPUSH_SEND.lock().unwrap().as_ref() {
                    sender
                        .send(Message::Text(
                            serde_json::to_string(&OutgoingMessage::BloodMessageEvent {
                                text: unsafe { U16CStr::from_ptr_str(message) }
                                    .to_string()
                                    .unwrap(),
                            })
                            .unwrap(),
                        ))
                        .expect("Send failed");
                }
            }
            msg_last_read.store(cur_read_time, Ordering::Relaxed);
            return message;
        }
    }

    unsafe { BLOOD_MESSAGE_LOOKUP_HOOK.call(param_1, template_id) }
}
