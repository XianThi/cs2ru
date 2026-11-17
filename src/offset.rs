use crate::hash::{json_get_by_hash, rt_hash};
use obfstr::obfstr;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

pub fn default_offsets_url() -> String {
    obfstr!("https://raw.githubusercontent.com/a2x/cs2-dumper/master/output/offsets.json")
        .to_string()
}

pub fn default_client_dll_url() -> String {
    obfstr!("https://raw.githubusercontent.com/a2x/cs2-dumper/master/output/client_dll.json")
        .to_string()
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct ControllerOffsets {
    pub m_iPing: usize,
    pub m_hPawn: usize,
    pub m_steamID: usize,
    pub m_iszPlayerName: usize,
    pub m_bIsLocalPlayerController: usize,
    pub m_pInGameMoneyServices: usize,
    pub m_iAccount: usize,
}

impl Default for ControllerOffsets {
    fn default() -> Self {
        Self {
            m_iPing: crate::obf_usize!(0x828),
            m_hPawn: crate::obf_usize!(0x6C4),
            m_steamID: crate::obf_usize!(0x780),
            m_iszPlayerName: crate::obf_usize!(0x6F8),
            m_bIsLocalPlayerController: crate::obf_usize!(0x788),
            m_pInGameMoneyServices: crate::obf_usize!(0x808),
            m_iAccount: crate::obf_usize!(0x40),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct PawnOffsets {
    pub m_vOldOrigin: usize,
    pub m_iHealth: usize,
    pub m_iTeamNum: usize,
    pub m_bIsScoped: usize,
    pub m_ArmorValue: usize,
    pub m_bIsDefusing: usize,
    pub m_pGameSceneNode: usize,
    pub m_pClippingWeapon: usize,
    pub m_entitySpottedState: usize,
    pub m_bSpottedByMask: usize,
    pub m_flFlashOverlayAlpha: usize,
}

impl Default for PawnOffsets {
    fn default() -> Self {
        Self {
            m_vOldOrigin: crate::obf_usize!(0x1588),
            m_iHealth: crate::obf_usize!(0x354),
            m_iTeamNum: crate::obf_usize!(0x3F3),
            m_bIsScoped: crate::obf_usize!(0x26F8),
            m_ArmorValue: crate::obf_usize!(0x272C),
            m_bIsDefusing: crate::obf_usize!(0x26FA),
            m_pGameSceneNode: crate::obf_usize!(0x338),
            m_pClippingWeapon: crate::obf_usize!(0x3DC0),
            m_entitySpottedState: crate::obf_usize!(0x26E0),
            m_bSpottedByMask: crate::obf_usize!(0xC),
            m_flFlashOverlayAlpha: crate::obf_usize!(0x15EC),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct BombOffsets {
    pub m_isPlanted: usize,
    pub m_bC4Activated: usize,
    pub m_nBombSite: usize,
    pub m_vecAbsOrigin: usize,
}

impl Default for BombOffsets {
    fn default() -> Self {
        Self {
            m_isPlanted: crate::obf_usize!(0x8),
            m_bC4Activated: crate::obf_usize!(0x11B8),
            m_nBombSite: crate::obf_usize!(0x1174),
            m_vecAbsOrigin: crate::obf_usize!(0xD0),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct BoneOffsets {
    pub m_modelState: usize,
}

impl Default for BoneOffsets {
    fn default() -> Self {
        Self {
            m_modelState: crate::obf_usize!(0x160),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct GlobalVarsOffsets {
    pub maxClients: usize,
    pub currentMapName: usize,
    pub currentTime: usize,
}

impl Default for GlobalVarsOffsets {
    fn default() -> Self {
        Self {
            maxClients: crate::obf_usize!(0x10),
            currentMapName: crate::obf_usize!(0x180),
            currentTime: crate::obf_usize!(0x2C),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct Signatures {
    pub viewMatrix: String,
    pub globalVars: String,
    pub entityList: String,
    pub localPlayerController: String,
    pub plantedC4: String,
    pub weaponC4: String,
    pub buildNumber: String,
}

impl Default for Signatures {
    fn default() -> Self {
        Self {
            viewMatrix: obfstr!("48 8D 0D ?? ?? ?? ?? 48 C1 E0 06").to_string(),
            globalVars: obfstr!("48 89 15 ?? ?? ?? ?? 48 89 42").to_string(),
            entityList: obfstr!("48 8B 0D ?? ?? ?? ?? 48 89 7C 24 ?? 8B FA C1 EB").to_string(),
            localPlayerController: obfstr!("48 8B 05 ?? ?? ?? ?? 41 89 BE").to_string(),
            plantedC4: obfstr!("48 8b 15 ?? ?? ?? ?? 41 FF C0 48 8D 4C 24 ?? 44 89 05 ?? ?? ?? ??").to_string(),
            weaponC4: obfstr!("48 89 05 ?? ?? ?? ?? F7 C1 ?? ?? ?? ?? 74 ?? 81 E1 ?? ?? ?? ?? 89 0D ?? ?? ?? ?? 8B 05 ?? ?? ?? ?? 89 1D ?? ?? ?? ?? EB ?? 48 8B 15 ?? ?? ?? ?? 48 8B 5C 24 ?? FF C0 89 05 ?? ?? ?? ?? 48 8B C6 48 89 34 EA 80 BE").to_string(),
            buildNumber: obfstr!("89 05 ?? ?? ?? ?? 48 8d 0d ?? ?? ?? ?? ff 15 ?? ?? ?? ?? 48 8b 0d").to_string(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct GameOptions {
    pub glowESPEnabled: bool,
    pub triggerBotEnabled: bool,
    pub autoBHOPEnabled: bool,
    pub soundESPEnabled: bool,
    pub rcsEnabled: bool,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            glowESPEnabled: true,
            triggerBotEnabled: true,
            autoBHOPEnabled: true,
            soundESPEnabled: true,
            rcsEnabled: true,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct OffsetsConfig {
    pub entityList: u64,
    pub viewMatrix: u64,
    pub localPlayerController: u64,
    pub globalVars: u64,
    pub plantedC4: u64,
    pub buildNumber: u64,
    pub controller: ControllerOffsets,
    pub pawn: PawnOffsets,
    pub bomb: BombOffsets,
    pub bone: BoneOffsets,
    pub global_vars: GlobalVarsOffsets,
    pub signatures: Signatures,
}

impl Default for OffsetsConfig {
    fn default() -> Self {
        Self {
            entityList: crate::obf_u64!(0x24B0258),
            viewMatrix: crate::obf_u64!(0x2310F10),
            localPlayerController: crate::obf_u64!(0x22F5028),
            globalVars: crate::obf_u64!(0x205F540),
            plantedC4: crate::obf_u64!(0x2318A60),
            buildNumber: crate::obf_u64!(0x60E514),
            controller: ControllerOffsets::default(),
            pawn: PawnOffsets::default(),
            bomb: BombOffsets::default(),
            bone: BoneOffsets::default(),
            global_vars: GlobalVarsOffsets::default(),
            signatures: Signatures::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub offsets: OffsetsConfig,
    pub options: GameOptions,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            offsets: OffsetsConfig::default(),
            options: GameOptions::default(),
        }
    }
}

/// Sınıf ve alan adlarını düz metin olarak saklamak yerine 64-bit derleme zamanı hash'i ile arar.
fn find_class_field_offset_by_hash(
    classes: &Value,
    field_hash: u64,
    preferred_class_hash: Option<u64>,
) -> Option<usize> {
    const H_FIELDS: u64 = crate::kh!("fields");

    if let Some(pref_hash) = preferred_class_hash {
        if let Some(val) = json_get_by_hash(classes, pref_hash)
            .and_then(|c| json_get_by_hash(c, H_FIELDS))
            .and_then(|f| json_get_by_hash(f, field_hash))
            .and_then(|v| v.as_u64())
        {
            return Some(val as usize);
        }
    }

    if let Some(map) = classes.as_object() {
        for (_class_name, class_val) in map {
            if let Some(fields) = json_get_by_hash(class_val, H_FIELDS) {
                if let Some(val) = json_get_by_hash(fields, field_hash).and_then(|v| v.as_u64()) {
                    return Some(val as usize);
                }
            }
        }
    }

    None
}

pub fn fetch_json_from_url(url: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .user_agent(obfstr!("Mozilla/5.0"))
        .build();
    let body = agent.get(url)
        .call()?
        .into_string()?;
    let value: Value = serde_json::from_str(&body)?;
    Ok(value)
}

#[allow(dead_code)]
pub fn update_offsets_from_web(config: &mut GameConfig) -> Result<(), Box<dyn std::error::Error>> {
    config.update_from_web()
}

#[allow(dead_code)]
pub fn update_offsets_from_urls(
    config: &mut GameConfig,
    offsets_url: &str,
    client_dll_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    config.update_from_network(offsets_url, client_dll_url)
}

fn toml_get_by_hash<'a>(table: &'a toml::Value, target_hash: u64) -> Option<&'a toml::Value> {
    let map = table.as_table()?;
    for (k, v) in map {
        if rt_hash(k) == target_hash {
            return Some(v);
        }
    }
    None
}

impl GameConfig {
    /// `offsets.toml` dosyasını derleme zamanı alan adı string'i barındırmadan,
    /// `rt_hash(key) == kh!(...)` eşleşmesiyle yükler.
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let root: toml::Value = toml::from_str(&content)?;
        let mut cfg = Self::default();

        if let Some(off) = toml_get_by_hash(&root, crate::kh!("offsets")) {
            let u64_of = |t: &toml::Value, h: u64| -> Option<u64> {
                toml_get_by_hash(t, h).and_then(|v| v.as_integer()).map(|i| i as u64)
            };
            let usize_of = |t: &toml::Value, h: u64| -> Option<usize> {
                toml_get_by_hash(t, h).and_then(|v| v.as_integer()).map(|i| i as usize)
            };
            let str_of = |t: &toml::Value, h: u64| -> Option<String> {
                toml_get_by_hash(t, h).and_then(|v| v.as_str()).map(|s| s.to_string())
            };

            if let Some(v) = u64_of(off, crate::kh!("entityList")) { cfg.offsets.entityList = v; }
            if let Some(v) = u64_of(off, crate::kh!("viewMatrix")) { cfg.offsets.viewMatrix = v; }
            if let Some(v) = u64_of(off, crate::kh!("localPlayerController")) { cfg.offsets.localPlayerController = v; }
            if let Some(v) = u64_of(off, crate::kh!("globalVars")) { cfg.offsets.globalVars = v; }
            if let Some(v) = u64_of(off, crate::kh!("plantedC4")) { cfg.offsets.plantedC4 = v; }
            if let Some(v) = u64_of(off, crate::kh!("buildNumber")) { cfg.offsets.buildNumber = v; }

            if let Some(c) = toml_get_by_hash(off, crate::kh!("controller")) {
                if let Some(v) = usize_of(c, crate::kh!("m_iPing")) { cfg.offsets.controller.m_iPing = v; }
                if let Some(v) = usize_of(c, crate::kh!("m_hPawn")) { cfg.offsets.controller.m_hPawn = v; }
                if let Some(v) = usize_of(c, crate::kh!("m_steamID")) { cfg.offsets.controller.m_steamID = v; }
                if let Some(v) = usize_of(c, crate::kh!("m_iszPlayerName")) { cfg.offsets.controller.m_iszPlayerName = v; }
                if let Some(v) = usize_of(c, crate::kh!("m_bIsLocalPlayerController")) { cfg.offsets.controller.m_bIsLocalPlayerController = v; }
                if let Some(v) = usize_of(c, crate::kh!("m_pInGameMoneyServices")) { cfg.offsets.controller.m_pInGameMoneyServices = v; }
                if let Some(v) = usize_of(c, crate::kh!("m_iAccount")) { cfg.offsets.controller.m_iAccount = v; }
            }

            if let Some(p) = toml_get_by_hash(off, crate::kh!("pawn")) {
                if let Some(v) = usize_of(p, crate::kh!("m_vOldOrigin")) { cfg.offsets.pawn.m_vOldOrigin = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_iHealth")) { cfg.offsets.pawn.m_iHealth = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_iTeamNum")) { cfg.offsets.pawn.m_iTeamNum = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_bIsScoped")) { cfg.offsets.pawn.m_bIsScoped = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_ArmorValue")) { cfg.offsets.pawn.m_ArmorValue = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_bIsDefusing")) { cfg.offsets.pawn.m_bIsDefusing = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_pGameSceneNode")) { cfg.offsets.pawn.m_pGameSceneNode = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_pClippingWeapon")) { cfg.offsets.pawn.m_pClippingWeapon = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_entitySpottedState")) { cfg.offsets.pawn.m_entitySpottedState = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_bSpottedByMask")) { cfg.offsets.pawn.m_bSpottedByMask = v; }
                if let Some(v) = usize_of(p, crate::kh!("m_flFlashOverlayAlpha")) { cfg.offsets.pawn.m_flFlashOverlayAlpha = v; }
            }

            if let Some(b) = toml_get_by_hash(off, crate::kh!("bomb")) {
                if let Some(v) = usize_of(b, crate::kh!("m_isPlanted")) { cfg.offsets.bomb.m_isPlanted = v; }
                if let Some(v) = usize_of(b, crate::kh!("m_bC4Activated")) { cfg.offsets.bomb.m_bC4Activated = v; }
                if let Some(v) = usize_of(b, crate::kh!("m_nBombSite")) { cfg.offsets.bomb.m_nBombSite = v; }
                if let Some(v) = usize_of(b, crate::kh!("m_vecAbsOrigin")) { cfg.offsets.bomb.m_vecAbsOrigin = v; }
            }

            if let Some(bn) = toml_get_by_hash(off, crate::kh!("bone")) {
                if let Some(v) = usize_of(bn, crate::kh!("m_modelState")) { cfg.offsets.bone.m_modelState = v; }
            }

            if let Some(gv) = toml_get_by_hash(off, crate::kh!("global_vars")) {
                if let Some(v) = usize_of(gv, crate::kh!("maxClients")) { cfg.offsets.global_vars.maxClients = v; }
                if let Some(v) = usize_of(gv, crate::kh!("currentMapName")) { cfg.offsets.global_vars.currentMapName = v; }
                if let Some(v) = usize_of(gv, crate::kh!("currentTime")) { cfg.offsets.global_vars.currentTime = v; }
            }

            if let Some(sg) = toml_get_by_hash(off, crate::kh!("signatures")) {
                if let Some(v) = str_of(sg, crate::kh!("viewMatrix")) { cfg.offsets.signatures.viewMatrix = v; }
                if let Some(v) = str_of(sg, crate::kh!("globalVars")) { cfg.offsets.signatures.globalVars = v; }
                if let Some(v) = str_of(sg, crate::kh!("entityList")) { cfg.offsets.signatures.entityList = v; }
                if let Some(v) = str_of(sg, crate::kh!("localPlayerController")) { cfg.offsets.signatures.localPlayerController = v; }
                if let Some(v) = str_of(sg, crate::kh!("plantedC4")) { cfg.offsets.signatures.plantedC4 = v; }
                if let Some(v) = str_of(sg, crate::kh!("weaponC4")) { cfg.offsets.signatures.weaponC4 = v; }
                if let Some(v) = str_of(sg, crate::kh!("buildNumber")) { cfg.offsets.signatures.buildNumber = v; }
            }
        }

        if let Some(opt) = toml_get_by_hash(&root, crate::kh!("options")) {
            let bool_of = |t: &toml::Value, h: u64| -> Option<bool> {
                toml_get_by_hash(t, h).and_then(|v| v.as_bool())
            };
            if let Some(v) = bool_of(opt, crate::kh!("glowESPEnabled")) { cfg.options.glowESPEnabled = v; }
            if let Some(v) = bool_of(opt, crate::kh!("triggerBotEnabled")) { cfg.options.triggerBotEnabled = v; }
            if let Some(v) = bool_of(opt, crate::kh!("autoBHOPEnabled")) { cfg.options.autoBHOPEnabled = v; }
            if let Some(v) = bool_of(opt, crate::kh!("soundESPEnabled")) { cfg.options.soundESPEnabled = v; }
            if let Some(v) = bool_of(opt, crate::kh!("rcsEnabled")) { cfg.options.rcsEnabled = v; }
        }

        Ok(cfg)
    }

    /// Alan adlarını `.rdata`'ya gömmeden (`obfstr!` ile çalışma zamanında çözerek) TOML olarak kaydeder.
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use toml::map::Map;
        use toml::Value as Tv;

        let mut root = Map::new();
        let mut off = Map::new();
        off.insert(obfstr!("entityList").to_string(), Tv::Integer(self.offsets.entityList as i64));
        off.insert(obfstr!("viewMatrix").to_string(), Tv::Integer(self.offsets.viewMatrix as i64));
        off.insert(obfstr!("localPlayerController").to_string(), Tv::Integer(self.offsets.localPlayerController as i64));
        off.insert(obfstr!("globalVars").to_string(), Tv::Integer(self.offsets.globalVars as i64));
        off.insert(obfstr!("plantedC4").to_string(), Tv::Integer(self.offsets.plantedC4 as i64));
        off.insert(obfstr!("buildNumber").to_string(), Tv::Integer(self.offsets.buildNumber as i64));

        let mut ctrl = Map::new();
        ctrl.insert(obfstr!("m_iPing").to_string(), Tv::Integer(self.offsets.controller.m_iPing as i64));
        ctrl.insert(obfstr!("m_hPawn").to_string(), Tv::Integer(self.offsets.controller.m_hPawn as i64));
        ctrl.insert(obfstr!("m_steamID").to_string(), Tv::Integer(self.offsets.controller.m_steamID as i64));
        ctrl.insert(obfstr!("m_iszPlayerName").to_string(), Tv::Integer(self.offsets.controller.m_iszPlayerName as i64));
        ctrl.insert(obfstr!("m_bIsLocalPlayerController").to_string(), Tv::Integer(self.offsets.controller.m_bIsLocalPlayerController as i64));
        ctrl.insert(obfstr!("m_pInGameMoneyServices").to_string(), Tv::Integer(self.offsets.controller.m_pInGameMoneyServices as i64));
        ctrl.insert(obfstr!("m_iAccount").to_string(), Tv::Integer(self.offsets.controller.m_iAccount as i64));
        off.insert(obfstr!("controller").to_string(), Tv::Table(ctrl));

        let mut pawn = Map::new();
        pawn.insert(obfstr!("m_vOldOrigin").to_string(), Tv::Integer(self.offsets.pawn.m_vOldOrigin as i64));
        pawn.insert(obfstr!("m_iHealth").to_string(), Tv::Integer(self.offsets.pawn.m_iHealth as i64));
        pawn.insert(obfstr!("m_iTeamNum").to_string(), Tv::Integer(self.offsets.pawn.m_iTeamNum as i64));
        pawn.insert(obfstr!("m_bIsScoped").to_string(), Tv::Integer(self.offsets.pawn.m_bIsScoped as i64));
        pawn.insert(obfstr!("m_ArmorValue").to_string(), Tv::Integer(self.offsets.pawn.m_ArmorValue as i64));
        pawn.insert(obfstr!("m_bIsDefusing").to_string(), Tv::Integer(self.offsets.pawn.m_bIsDefusing as i64));
        pawn.insert(obfstr!("m_pGameSceneNode").to_string(), Tv::Integer(self.offsets.pawn.m_pGameSceneNode as i64));
        pawn.insert(obfstr!("m_pClippingWeapon").to_string(), Tv::Integer(self.offsets.pawn.m_pClippingWeapon as i64));
        pawn.insert(obfstr!("m_entitySpottedState").to_string(), Tv::Integer(self.offsets.pawn.m_entitySpottedState as i64));
        pawn.insert(obfstr!("m_bSpottedByMask").to_string(), Tv::Integer(self.offsets.pawn.m_bSpottedByMask as i64));
        pawn.insert(obfstr!("m_flFlashOverlayAlpha").to_string(), Tv::Integer(self.offsets.pawn.m_flFlashOverlayAlpha as i64));
        off.insert(obfstr!("pawn").to_string(), Tv::Table(pawn));

        let mut bomb = Map::new();
        bomb.insert(obfstr!("m_isPlanted").to_string(), Tv::Integer(self.offsets.bomb.m_isPlanted as i64));
        bomb.insert(obfstr!("m_bC4Activated").to_string(), Tv::Integer(self.offsets.bomb.m_bC4Activated as i64));
        bomb.insert(obfstr!("m_nBombSite").to_string(), Tv::Integer(self.offsets.bomb.m_nBombSite as i64));
        bomb.insert(obfstr!("m_vecAbsOrigin").to_string(), Tv::Integer(self.offsets.bomb.m_vecAbsOrigin as i64));
        off.insert(obfstr!("bomb").to_string(), Tv::Table(bomb));

        let mut bone = Map::new();
        bone.insert(obfstr!("m_modelState").to_string(), Tv::Integer(self.offsets.bone.m_modelState as i64));
        off.insert(obfstr!("bone").to_string(), Tv::Table(bone));

        let mut gv = Map::new();
        gv.insert(obfstr!("maxClients").to_string(), Tv::Integer(self.offsets.global_vars.maxClients as i64));
        gv.insert(obfstr!("currentMapName").to_string(), Tv::Integer(self.offsets.global_vars.currentMapName as i64));
        gv.insert(obfstr!("currentTime").to_string(), Tv::Integer(self.offsets.global_vars.currentTime as i64));
        off.insert(obfstr!("global_vars").to_string(), Tv::Table(gv));

        let mut sig = Map::new();
        sig.insert(obfstr!("viewMatrix").to_string(), Tv::String(self.offsets.signatures.viewMatrix.clone()));
        sig.insert(obfstr!("globalVars").to_string(), Tv::String(self.offsets.signatures.globalVars.clone()));
        sig.insert(obfstr!("entityList").to_string(), Tv::String(self.offsets.signatures.entityList.clone()));
        sig.insert(obfstr!("localPlayerController").to_string(), Tv::String(self.offsets.signatures.localPlayerController.clone()));
        sig.insert(obfstr!("plantedC4").to_string(), Tv::String(self.offsets.signatures.plantedC4.clone()));
        sig.insert(obfstr!("weaponC4").to_string(), Tv::String(self.offsets.signatures.weaponC4.clone()));
        sig.insert(obfstr!("buildNumber").to_string(), Tv::String(self.offsets.signatures.buildNumber.clone()));
        off.insert(obfstr!("signatures").to_string(), Tv::Table(sig));

        root.insert(obfstr!("offsets").to_string(), Tv::Table(off));

        let mut opt = Map::new();
        opt.insert(obfstr!("glowESPEnabled").to_string(), Tv::Boolean(self.options.glowESPEnabled));
        opt.insert(obfstr!("triggerBotEnabled").to_string(), Tv::Boolean(self.options.triggerBotEnabled));
        opt.insert(obfstr!("autoBHOPEnabled").to_string(), Tv::Boolean(self.options.autoBHOPEnabled));
        opt.insert(obfstr!("soundESPEnabled").to_string(), Tv::Boolean(self.options.soundESPEnabled));
        opt.insert(obfstr!("rcsEnabled").to_string(), Tv::Boolean(self.options.rcsEnabled));
        root.insert(obfstr!("options").to_string(), Tv::Table(opt));

        let content = toml::to_string_pretty(&Tv::Table(root))?;
        fs::write(path, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn from_dict(data: HashMap<String, HashMap<String, u64>>) -> Self {
        let binding: HashMap<String, u64> = HashMap::new();
        let client = data
            .iter()
            .find(|(k, _)| crate::hash::rt_hash(k) == crate::kh!("client_dll"))
            .map(|(_, v)| v)
            .unwrap_or(&binding);
        let engine = data
            .iter()
            .find(|(k, _)| crate::hash::rt_hash(k) == crate::kh!("engine2_dll"))
            .map(|(_, v)| v)
            .unwrap_or(&binding);

        let get_h = |m: &HashMap<String, u64>, h: u64, def: u64| -> u64 {
            m.iter()
                .find(|(k, _)| crate::hash::rt_hash(k) == h)
                .map(|(_, &v)| v)
                .unwrap_or(def)
        };

        Self {
            offsets: OffsetsConfig {
                entityList: get_h(client, crate::kh!("entitylist"), crate::obf_u64!(0x24B0258)),
                viewMatrix: get_h(client, crate::kh!("viewmatrix"), crate::obf_u64!(0x2310F10)),
                localPlayerController: get_h(client, crate::kh!("localplayercontroller"), crate::obf_u64!(0x22F5028)),
                globalVars: get_h(client, crate::kh!("globalvars"), crate::obf_u64!(0x205F540)),
                plantedC4: get_h(client, crate::kh!("plantedc4"), crate::obf_u64!(0x2318A60)),
                buildNumber: get_h(engine, crate::kh!("buildnumber"), crate::obf_u64!(0x60E514)),
                ..Default::default()
            },
            options: GameOptions::default(),
        }
    }

    pub fn update_from_network(
        &mut self,
        offsets_url: &str,
        client_dll_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let offsets_json = fetch_json_from_url(offsets_url)?;
        let client_json = fetch_json_from_url(client_dll_url)?;
        self.apply_remote_offsets(&offsets_json, &client_json);
        Ok(())
    }

    pub fn update_from_web(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let off_url = default_offsets_url();
        let cli_url = default_client_dll_url();
        self.update_from_network(&off_url, &cli_url)
    }

    pub fn apply_remote_offsets(&mut self, offsets_json: &Value, client_json: &Value) {
        if let Some(client_dll) = json_get_by_hash(offsets_json, crate::kh!("client.dll")) {
            if let Some(val) = json_get_by_hash(client_dll, crate::kh!("dwEntityList")).and_then(|v| v.as_u64()) {
                self.offsets.entityList = val;
            }
            if let Some(val) = json_get_by_hash(client_dll, crate::kh!("dwViewMatrix")).and_then(|v| v.as_u64()) {
                self.offsets.viewMatrix = val;
            }
            if let Some(val) = json_get_by_hash(client_dll, crate::kh!("dwLocalPlayerController")).and_then(|v| v.as_u64()) {
                self.offsets.localPlayerController = val;
            }
            if let Some(val) = json_get_by_hash(client_dll, crate::kh!("dwGlobalVars")).and_then(|v| v.as_u64()) {
                self.offsets.globalVars = val;
            }
            if let Some(val) = json_get_by_hash(client_dll, crate::kh!("dwPlantedC4")).and_then(|v| v.as_u64()) {
                self.offsets.plantedC4 = val;
            }
        }

        if let Some(engine_dll) = json_get_by_hash(offsets_json, crate::kh!("engine2.dll")) {
            if let Some(val) = json_get_by_hash(engine_dll, crate::kh!("dwBuildNumber")).and_then(|v| v.as_u64()) {
                self.offsets.buildNumber = val;
            }
        }

        if let Some(classes) = json_get_by_hash(client_json, crate::kh!("client.dll"))
            .and_then(|c| json_get_by_hash(c, crate::kh!("classes")))
        {
            const C_CS_CTRL: Option<u64> = Some(crate::kh!("CCSPlayerController"));
            const C_BASE_CTRL: Option<u64> = Some(crate::kh!("CBasePlayerController"));
            const C_MONEY: Option<u64> = Some(crate::kh!("CCSPlayerController_InGameMoneyServices"));
            const C_BASE_PAWN: Option<u64> = Some(crate::kh!("C_BasePlayerPawn"));
            const C_BASE_ENT: Option<u64> = Some(crate::kh!("C_BaseEntity"));
            const C_CS_PAWN: Option<u64> = Some(crate::kh!("C_CSPlayerPawn"));
            const C_CS_PAWN_BASE: Option<u64> = Some(crate::kh!("C_CSPlayerPawnBase"));
            const C_SPOTTED: Option<u64> = Some(crate::kh!("EntitySpottedState_t"));
            const C_PLANTED_C4: Option<u64> = Some(crate::kh!("C_PlantedC4"));
            const C_SCENE_NODE: Option<u64> = Some(crate::kh!("CGameSceneNode"));
            const C_SKELETON: Option<u64> = Some(crate::kh!("CSkeletonInstance"));

            // Controller
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_iPing"), C_CS_CTRL) {
                self.offsets.controller.m_iPing = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_hPawn"), C_BASE_CTRL) {
                self.offsets.controller.m_hPawn = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_steamID"), C_BASE_CTRL) {
                self.offsets.controller.m_steamID = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_iszPlayerName"), C_BASE_CTRL) {
                self.offsets.controller.m_iszPlayerName = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_bIsLocalPlayerController"), C_BASE_CTRL) {
                self.offsets.controller.m_bIsLocalPlayerController = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_pInGameMoneyServices"), C_CS_CTRL) {
                self.offsets.controller.m_pInGameMoneyServices = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_iAccount"), C_MONEY) {
                self.offsets.controller.m_iAccount = v;
            }

            // Pawn
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_vOldOrigin"), C_BASE_PAWN) {
                self.offsets.pawn.m_vOldOrigin = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_iHealth"), C_BASE_ENT) {
                self.offsets.pawn.m_iHealth = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_iTeamNum"), C_BASE_ENT) {
                self.offsets.pawn.m_iTeamNum = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_bIsScoped"), C_CS_PAWN) {
                self.offsets.pawn.m_bIsScoped = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_ArmorValue"), C_CS_PAWN) {
                self.offsets.pawn.m_ArmorValue = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_bIsDefusing"), C_CS_PAWN) {
                self.offsets.pawn.m_bIsDefusing = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_pGameSceneNode"), C_BASE_ENT) {
                self.offsets.pawn.m_pGameSceneNode = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_pClippingWeapon"), C_CS_PAWN_BASE) {
                self.offsets.pawn.m_pClippingWeapon = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_entitySpottedState"), C_CS_PAWN) {
                self.offsets.pawn.m_entitySpottedState = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_bSpottedByMask"), C_SPOTTED) {
                self.offsets.pawn.m_bSpottedByMask = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_flFlashOverlayAlpha"), C_CS_PAWN_BASE) {
                self.offsets.pawn.m_flFlashOverlayAlpha = v;
            }

            // Bomb
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_isPlanted"), C_PLANTED_C4) {
                self.offsets.bomb.m_isPlanted = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_bC4Activated"), C_PLANTED_C4) {
                self.offsets.bomb.m_bC4Activated = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_nBombSite"), C_PLANTED_C4) {
                self.offsets.bomb.m_nBombSite = v;
            }
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_vecAbsOrigin"), C_SCENE_NODE) {
                self.offsets.bomb.m_vecAbsOrigin = v;
            }

            // Bone
            if let Some(v) = find_class_field_offset_by_hash(classes, crate::kh!("m_modelState"), C_SKELETON) {
                self.offsets.bone.m_modelState = v;
            }
        }
    }

    /// RIP-bağıl (RIP-relative) komut adresinden modül RVA ofsetini hesaplar:
    /// `target_va = instr_va + instr_len + disp32`, `rva = target_va - module_base`.
    pub fn compute_rip_rva(
        module_base: usize,
        instr_va: usize,
        instr_len: usize,
        disp32: i32,
    ) -> Option<u64> {
        let next_rip = instr_va.checked_add(instr_len)?;
        let target_va = (next_rip as isize).checked_add(disp32 as isize)? as usize;
        target_va.checked_sub(module_base).map(|rva| rva as u64)
    }

    /// İnternet erişimi olmadığında veya dumper geciktiğinde `offsets.toml` içindeki
    /// `[offsets.signatures]` imzalarını kullanarak ana tablo ofsetlerini bellekten çözer.
    pub unsafe fn update_from_signatures(
        &mut self,
        kit: &sysmemkit::SysMemKit,
        client_base: usize,
        engine_base: usize,
    ) -> usize {
        const CLIENT_SCAN_SIZE: usize = 0x0280_0000; // ~40 MB
        const ENGINE_SCAN_SIZE: usize = 0x00A0_0000; // ~10 MB
        let mut resolved = 0usize;

        let resolve_one = |base: usize, scan_size: usize, sig: &str, disp_off: usize, ins_len: usize| -> Option<u64> {
            if base == 0 || sig.trim().is_empty() {
                return None;
            }
            let addr = unsafe { kit.scan(sig, base, scan_size) }?;
            let disp: i32 = unsafe { kit.read(addr + disp_off) };
            if disp == 0 {
                return None;
            }
            Self::compute_rip_rva(base, addr, ins_len, disp)
        };

        if let Some(rva) = resolve_one(client_base, CLIENT_SCAN_SIZE, &self.offsets.signatures.viewMatrix, 3, 7) {
            self.offsets.viewMatrix = rva;
            resolved += 1;
        }
        if let Some(rva) = resolve_one(client_base, CLIENT_SCAN_SIZE, &self.offsets.signatures.entityList, 3, 7) {
            self.offsets.entityList = rva;
            resolved += 1;
        }
        if let Some(rva) = resolve_one(client_base, CLIENT_SCAN_SIZE, &self.offsets.signatures.localPlayerController, 3, 7) {
            self.offsets.localPlayerController = rva;
            resolved += 1;
        }
        if let Some(rva) = resolve_one(client_base, CLIENT_SCAN_SIZE, &self.offsets.signatures.globalVars, 3, 7) {
            self.offsets.globalVars = rva;
            resolved += 1;
        }
        if let Some(rva) = resolve_one(client_base, CLIENT_SCAN_SIZE, &self.offsets.signatures.plantedC4, 3, 7) {
            self.offsets.plantedC4 = rva;
            resolved += 1;
        }
        if let Some(rva) = resolve_one(engine_base, ENGINE_SCAN_SIZE, &self.offsets.signatures.buildNumber, 2, 6) {
            self.offsets.buildNumber = rva;
            resolved += 1;
        }

        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_rip_rva() {
        let base = 0x7FF8_0000_0000usize;
        let instr = base + 0x0010_0000;
        // 7-byte instruction: target at base + 0x0231_0F10 -> disp = 0x0231_0F10 - 0x0010_0007
        let disp: i32 = 0x0231_0F10 - 0x0010_0007;
        assert_eq!(
            GameConfig::compute_rip_rva(base, instr, 7, disp),
            Some(0x0231_0F10)
        );
    }

    #[test]
    fn test_update_offsets_from_web() {
        let mut config = GameConfig::default();
        let result = config.update_from_web();
        assert!(result.is_ok(), "Offset güncellemesi başarısız oldu: {:?}", result.err());
        assert_ne!(config.offsets.entityList, 0);
        assert_ne!(config.offsets.viewMatrix, 0);
        assert_ne!(config.offsets.controller.m_iPing, 0);
        assert_ne!(config.offsets.pawn.m_vOldOrigin, 0);
        assert_ne!(config.offsets.pawn.m_iHealth, 0);
        println!("Updated entityList: 0x{:X}", config.offsets.entityList);
        println!("Updated viewMatrix: 0x{:X}", config.offsets.viewMatrix);
        println!("Updated m_iPing: 0x{:X}", config.offsets.controller.m_iPing);
    }
}
