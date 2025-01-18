use lazy_static::lazy_static;
use retour::static_detour;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering},
        OnceLock, RwLock,
    },
};
use widestring::{U16CStr, U16CString};

use crate::util::get_game_base;
use crate::{
    player::{MapId, WorldChrMan},
    reflection::{get_instance, DLRFLocatable},
};

// Despawn the message and remove the message text entry
pub fn delete_message(message: &str) {
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
    let msg_id = match get_message_id(message) {
        None => {
            log::info!("Could not find message");
            return;
        }
        Some(x) => x,
    };

    //remove the entry from the BloodMessageInsMan list
    let mut removed_msg_entry: u64 = 0;

    unsafe {
        let mut msg_list_head = netman
            .blood_message_db
            .blood_message_ins_man_1
            .blood_message_list_head;
        let mut msg_list_head_read = netman
            .blood_message_db
            .blood_message_ins_man_1
            .blood_message_list_head as *const BloodMessageIns;
        //if it's the head entry
        if (*msg_list_head_read).template == msg_id {
            removed_msg_entry = netman
                .blood_message_db
                .blood_message_ins_man_1
                .blood_message_list_head;
            netman
                .blood_message_db
                .blood_message_ins_man_1
                .blood_message_list_head = (*msg_list_head_read).next;
        } else {
            loop {
                if (*((*msg_list_head_read).next as *const BloodMessageIns)).template == msg_id {
                    removed_msg_entry = (*msg_list_head_read).next;
                    (*(msg_list_head as *mut BloodMessageIns)).next =
                        (*((*(msg_list_head as *const BloodMessageIns)).next
                            as *const BloodMessageIns))
                            .next;
                    break;
                }
                msg_list_head = (*msg_list_head_read).next;
                msg_list_head_read = (*msg_list_head_read).next as *const BloodMessageIns;
            }
        }
    }

    if removed_msg_entry == 0 {
        log::info!("Could not find message");
        return;
    }

    //free and destrut the BloodMessageIns object
    let destruct_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u32)>(base + 0x1b73f0) };
    destruct_fn(removed_msg_entry, 0); //this cleans up the sfx but doesn't free the memory
    let dealloc_fn =
        unsafe { std::mem::transmute::<usize, extern "C" fn(u64, u64)>(base + 0xe1d990) };
    dealloc_fn(0, removed_msg_entry); //this frees the memory

    //remove the message text entry
    remove_message(&message);
}

// Spawns a message on the floor at the players location
pub fn spawn_message(message: &str) {
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
    let map_coordinates = &world_chr_man
        .main_player
        .module_container
        .physics
        .unk70_position;

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
        unk38: -1,
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

    log::info!("Spawned message at {map_id:?} - {map_coordinates:?} with text \"{message}\"");
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
    pub unk38: i32,
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

fn remove_message(message: &str) {
    let basic_message = String::from(message).retain(|c| !c.is_whitespace());

    let mut map = MESSAGE_TABLE
        .get_or_init(Default::default)
        .write()
        .expect("Could not acquire message table read/write lock");

    map.retain(|_, text| {
        let basic_text = text.to_string().unwrap().retain(|c| !c.is_whitespace());
        return basic_text != basic_message;
    });
}

fn get_message_id(message: &str) -> Option<u16> {
    let basic_message = String::from(message).retain(|c| !c.is_whitespace());

    let map = MESSAGE_TABLE
        .get_or_init(Default::default)
        .read()
        .expect("Could not acquire message table read lock");

    for (key, val) in map.clone().into_iter() {
        let basic_text = val.to_string().unwrap().retain(|c| !c.is_whitespace());
        if basic_text == basic_message {
            return Some(key);
        }
    }

    return None;
}

fn get_message(index: u16) -> Option<*const u16> {
    MESSAGE_TABLE
        .get_or_init(Default::default)
        .read()
        .expect("Could not acquire message table read lock")
        .get(&index)
        .map(|f| f.as_ptr())
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
    pub(crate) static ref MSGINFO_SEND: Mutex<Option<Sender<String>>> = Mutex::new(None);
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
                if let Some(sender) = MSGINFO_SEND.lock().unwrap().as_ref() {
                    sender
                        .send(
                            unsafe { U16CStr::from_ptr_str(message) }
                                .to_string()
                                .unwrap(),
                        )
                        .expect("Send failed");
                }
            }
            msg_last_read.store(cur_read_time, Ordering::Relaxed);
            return message;
        }
    }

    unsafe { BLOOD_MESSAGE_LOOKUP_HOOK.call(param_1, template_id) }
}
