use std::{pin::Pin, sync::LazyLock};
use broadsword::scanner;
use crate::{
    util::get_section
};

//FUN_140eb1750
const REGISTER_TASK_PATTERN: &str = concat!(
// PUSH RDI
"01000... 01010111",
// SUB RSP, 0x30
"01001... 10000011 11101100 00110000",
// MOV [RSP+0x20], -0x2
"01001... 11000111 01000100 ..100100 00100000 11111110 11111111 11111111 11111111",
// MOV [RSP+0x40], RBX
"01001... 10001001 01011100 ..100100 01000000",
// MOV EBX, EDX
"10001011 11011010",
// MOV RDI, RCX
"01001... 10001011 11111001",
// CMP EDX,???
"10000001 11111010 ........ ........ ........ ........",
// JA ???
"00001111 10000111 ........ ........ ........ ........",
);

static REGISTER_TASK: LazyLock<extern "C" fn(&CSEzTask, CSTaskGroupIndex)> = LazyLock::new(|| {
    let (text_range, text_slice) = get_section(".text")
        .expect( "Could not get game text section.");

    let pattern = scanner::Pattern::from_bit_pattern(REGISTER_TASK_PATTERN)
        .expect("Could not parse pattern");

    let result = scanner::simple::scan(text_slice, &pattern).expect("Could not find CSTask::RegisterTask");

    log::info!("CSTask::RegisterTask at {text_range:#?}+{result:#?}");
    unsafe { std::mem::transmute(text_range.start+result.location) }
});

#[repr(C)]
struct CSEzTaskVftable {
    // DLRF reflection metadata.
    pub get_runtime_class: fn(),
    // Bare execute call (gets called by the dispatcher).
    pub execute: fn(&FD4TaskData),
    // Called by execute() in the case of CSEzTask.
    pub eztask_execute: fn(),
    // Called to register the task to the appropriate runtime.
    pub register_task: fn(),
    // Called to free up the task.
    pub free_task: fn(),
    // Getter for the task group.
    pub get_task_group: fn(),
}

#[repr(C)]
struct CSEzTask {
    vftable: *const CSEzTaskVftable,
    unk8: u32,
    _padc: u32,
    task_proxy: usize,
}

#[repr(C)]
pub struct FD4Time {
    pub vftable: usize,
    pub time: f32,
    _padc: u32,
}

#[repr(C)]
struct FD4TaskData {
    delta_time: FD4Time,
    task_group_id: u32,
    seed: i32,
}

pub struct TaskProxy {
    _vftable: Pin<Box<CSEzTaskVftable>>,
    task: Pin<Box<CSEzTask>>,
}

impl Drop for TaskProxy {
    fn drop(&mut self) {
        let free = unsafe { (*self.task.vftable).free_task };
        free();
    }
}

pub fn run_task(execute_fn: fn(), task_group: CSTaskGroupIndex) -> TaskProxy {
    log::info!("run_task Address {:?}", execute_fn);

    let vftable = Box::pin(CSEzTaskVftable {
        get_runtime_class: || tracing::error!("TASK::get_runtime_class called"),
        execute: |_| tracing::error!("TASK::execute called"),
        eztask_execute: execute_fn,
        register_task: || tracing::error!("TASK::register_task called"),
        free_task: || tracing::error!("TASK::free_task called"),
        get_task_group: || tracing::error!("TASK::get_task_group called"),
    });

    let task = Box::pin(CSEzTask {
        vftable: vftable.as_ref().get_ref() as *const CSEzTaskVftable,
        task_proxy: 0,
        unk8: 0,
        _padc: 0,
    });

    REGISTER_TASK(&task, task_group);

    TaskProxy { _vftable: vftable, task }
}

#[repr(u32)]
#[allow(non_camel_case_types, dead_code)]
pub enum CSTaskGroupIndex {
    FrameBegin,
    SteamThread0,
    SteamThread1,
    SteamThread2,
    SteamThread3,
    SteamThread4,
    SteamThread5,
    SystemStep,
    ResStep,
    PadStep,
    GameFlowStep,
    EndShiftWorldPosition,
    GameMan,
    TaskLineIdx_Sys,
    TaskLineIdx_Test,
    TaskLineIdx_NetworkFlowStep,
    TaskLineIdx_InGame_InGameStep,
    TaskLineIdx_InGame_InGameStayStep,
    MovieStep,
    RemoStep,
    TaskLineIdx_InGame_MoveMapStep,
    FieldArea_EndWorldAiManager,
    EmkSystem_Pre,
    EmkSystem_ConditionStatus,
    EmkSystem_Post,
    EventMan,
    FlverResDelayDelectiionBegin,
    TaskLineIdx_InGame_FieldAreaStep,
    TaskLineIdx_InGame_TestNetStep,
    TaskLineIdx_InGame_InGameMenuStep,
    TaskLineIdx_InGame_TitleMenuStep,
    TaskLineIdx_InGame_CommonMenuStep,
    TaskLineIdx_FrpgNet_Sys,
    TaskLineIdx_FrpgNet_Lobby,
    TaskLineIdx_FrpgNet_ConnectMan,
    TaskLineIdx_FrpgNet_Connect,
    TaskLineIdx_FrpgNet_Other,
    SfxMan,
    FaceGenMan,
    FrpgNetMan,
    NetworkUserManager,
    SessionManager,
    BlockList,
    LuaConsoleServer,
    RmiMan,
    ResMan,
    SfxDebugger,
    REMOTEMAN,
    Geom_WaitActivateFade,
    Geom_UpdateDraw,
    Grass_BatchUpdate,
    Grass_ResourceLoadKick,
    Grass_ResourceLoad,
    Grass_ResourceCleanup,
    WorldChrMan_Respawn,
    WorldChrMan_Prepare,
    ChrIns_CalcUpdateInfo_PerfBegin,
    ChrIns_CalcUpdateInfo,
    ChrIns_CalcUpdateInfo_PerfEnd,
    WorldChrMan_PrePhysics,
    WorldChrMan_CalcOmissionLevel_Begin,
    WorldChrMan_CalcOmissionLevel,
    WorldChrMan_CalcOmissionLevel_End,
    WorldChrMan_ConstructUpdateList,
    WorldChrMan_ChrNetwork,
    ChrIns_Prepare,
    ChrIns_NaviCache,
    ChrIns_AILogic_PerfBegin,
    ChrIns_AILogic,
    ChrIns_AILogic_PerfEnd,
    AI_SimulationStep,
    ChrIns_PreBehavior,
    ChrIns_PreBehaviorSafe,
    GeomModelInsCreatePartway_Begin,
    HavokBehavior,
    GeomModelInsCreatePartway_End,
    ChrIns_BehaviorSafe,
    ChrIns_PrePhysics_Begin,
    ChrIns_PrePhysics,
    ChrIns_PrePhysics_End,
    NetFlushSendData,
    ChrIns_PrePhysicsSafe,
    ChrIns_RagdollSafe,
    ChrIns_GarbageCollection,
    GeomModelInsCreate,
    AiBeginCollectGabage,
    WorldChrMan_Update_RideCheck,
    InGameDebugViewer,
    LocationStep,
    LocationUpdate_PrePhysics,
    LocationUpdate_PrePhysics_Parallel,
    LocationUpdate_PrePhysics_Post,
    LocationUpdate_PostCloth,
    LocationUpdate_PostCloth_Parallel,
    LocationUpdate_PostCloth_Post,
    LocationUpdate_DebugDraw,
    EventCondition_BonfireNearEnemyCheck,
    HavokWorldUpdate_Pre,
    RenderingSystemUpdate,
    HavokWorldUpdate_Post,
    ChrIns_PreCloth,
    ChrIns_PreClothSafe,
    HavokClothUpdate_Pre_AddRemoveRigidBody,
    HavokClothUpdate_Pre_ClothModelInsSafe,
    HavokClothUpdate_Pre_ClothModelIns,
    HavokClothUpdate_Pre_ClothManager,
    CameraStep,
    DrawParamUpdate,
    GetNPAuthCode,
    SoundStep,
    HavokClothUpdate_Post_ClothManager,
    HavokClothUpdate_Post_ClothModelIns,
    HavokClothVertexUpdateFinishWait,
    ChrIns_PostPhysics,
    ChrIns_PostPhysicsSafe,
    CSDistViewManager_Update,
    HavokAi_SilhouetteGeneratorHelper_Begin,
    WorldChrMan_PostPhysics,
    GameFlowInGame_MoveMap_PostPhysics_0,
    HavokAi_SilhouetteGeneratorHelper_End,
    DmgMan_Pre,
    DmgMan_ShapeCast,
    DmgMan_Post,
    GameFlowInGame_MoveMap_PostPhysics_1_Core0,
    GameFlowInGame_MoveMap_PostPhysics_1_Core1,
    GameFlowInGame_MoveMap_PostPhysics_1_Core2,
    MenuMan,
    WorldChrMan_Update_BackreadRequestPre,
    ChrIns_Update_BackreadRequest,
    WorldChrMan_Update_BackreadRequestPost,
    HavokAi_World,
    WorldAiManager_BeginUpdateFormation,
    WorldAiManager_EndUpdateFormation,
    GameFlowInGame_TestNet,
    GameFlowInGame_InGameMenu,
    GameFlowInGame_TitleMenu,
    GameFlowInGame_CommonMenu,
    GameFlowFrpgNet_Sys,
    GameFlowFrpgNet_Lobby,
    GameFlowFrpgNet_ConnectMan,
    GameFlowFrpgNet_Connect,
    GameFlowStep_Post,
    ScaleformStep,
    FlverResDelayDelectiionEnd,
    Draw_Pre,
    GraphicsStep,
    DebugDrawMemoryBar,
    DbgMenuStep,
    DbgRemoteStep,
    PlaylogSystemStep,
    ReviewMan,
    ReportSystemStep,
    DbgDispStep,
    DrawStep,
    DrawBegin,
    GameSceneDraw,
    AdhocDraw,
    DrawEnd,
    Draw_Post,
    SoundPlayLimitterUpdate,
    BeginShiftWorldPosition,
    FileStep,
    FileStepUpdate_Begin,
    FileStepUpdate_End,
    Flip,
    DelayDeleteStep,
    AiEndCollectGabage,
    RecordHeapStats,
    FrameEnd,
}