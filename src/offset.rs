use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

pub const DEFAULT_OFFSETS_URL: &str = "https://raw.githubusercontent.com/a2x/cs2-dumper/master/output/offsets.json";
pub const DEFAULT_CLIENT_DLL_URL: &str = "https://raw.githubusercontent.com/a2x/cs2-dumper/master/output/client_dll.json";

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            m_iPing: 0x828,
            m_hPawn: 0x6C4,
            m_steamID: 0x780,
            m_iszPlayerName: 0x6F8,
            m_bIsLocalPlayerController: 0x788,
            m_pInGameMoneyServices: 0x808,
            m_iAccount: 0x40,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            m_vOldOrigin: 0x1588,
            m_iHealth: 0x354,
            m_iTeamNum: 0x3F3,
            m_bIsScoped: 0x26F8,
            m_ArmorValue: 0x272C,
            m_bIsDefusing: 0x26FA,
            m_pGameSceneNode: 0x338,
            m_pClippingWeapon: 0x3DC0,
            m_entitySpottedState: 0x26E0,
            m_bSpottedByMask: 0xC,
            m_flFlashOverlayAlpha: 0x15EC,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BombOffsets {
    pub m_isPlanted: usize,
    pub m_bC4Activated: usize,
    pub m_nBombSite: usize,
    pub m_vecAbsOrigin: usize,
}

impl Default for BombOffsets {
    fn default() -> Self {
        Self {
            m_isPlanted: 0x8,
            m_bC4Activated: 0x11B8,
            m_nBombSite: 0x1174,
            m_vecAbsOrigin: 0xD0,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneOffsets {
    pub m_modelState: usize,
}

impl Default for BoneOffsets {
    fn default() -> Self {
        Self { m_modelState: 0x160 }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalVarsOffsets {
    pub maxClients: usize,
    pub currentMapName: usize,
    pub currentTime: usize,
}

impl Default for GlobalVarsOffsets {
    fn default() -> Self {
        Self {
            maxClients: 0x10,
            currentMapName: 0x180,
            currentTime: 0x2C,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            viewMatrix: "48 8D 0D ?? ?? ?? ?? 48 C1 E0 06".to_string(),
            globalVars: "48 89 15 ?? ?? ?? ?? 48 89 42".to_string(),
            entityList: "48 8B 0D ?? ?? ?? ?? 48 89 7C 24 ?? 8B FA C1 EB".to_string(),
            localPlayerController: "48 8B 05 ?? ?? ?? ?? 41 89 BE".to_string(),
            plantedC4: "48 8b 15 ?? ?? ?? ?? 41 FF C0 48 8D 4C 24 ?? 44 89 05 ?? ?? ?? ??".to_string(),
            weaponC4: "48 89 05 ?? ?? ?? ?? F7 C1 ?? ?? ?? ?? 74 ?? 81 E1 ?? ?? ?? ?? 89 0D ?? ?? ?? ?? 8B 05 ?? ?? ?? ?? 89 1D ?? ?? ?? ?? EB ?? 48 8B 15 ?? ?? ?? ?? 48 8B 5C 24 ?? FF C0 89 05 ?? ?? ?? ?? 48 8B C6 48 89 34 EA 80 BE".to_string(),
            buildNumber: "89 05 ?? ?? ?? ?? 48 8d 0d ?? ?? ?? ?? ff 15 ?? ?? ?? ?? 48 8b 0d".to_string(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            entityList: 0x24B0258,
            viewMatrix: 0x2310F10,
            localPlayerController: 0x22F5028,
            globalVars: 0x205F540,
            plantedC4: 0x2318A60,
            buildNumber: 0x60E514,
            controller: ControllerOffsets::default(),
            pawn: PawnOffsets::default(),
            bomb: BombOffsets::default(),
            bone: BoneOffsets::default(),
            global_vars: GlobalVarsOffsets::default(),
            signatures: Signatures::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn find_class_field_offset(classes: &Value, field_name: &str, preferred_class: Option<&str>) -> Option<usize> {
    if let Some(pref) = preferred_class {
        if let Some(val) = classes
            .get(pref)
            .and_then(|c| c.get("fields"))
            .and_then(|f| f.get(field_name))
            .and_then(|v| v.as_u64())
        {
            return Some(val as usize);
        }
    }

    if let Some(map) = classes.as_object() {
        for (_class_name, class_val) in map {
            if let Some(fields) = class_val.get("fields").and_then(|f| f.as_object()) {
                if let Some(val) = fields.get(field_name).and_then(|v| v.as_u64()) {
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
        .user_agent("cs2ru/1.0")
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

impl GameConfig {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: GameConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn from_dict(data: HashMap<String, HashMap<String, u64>>) -> Self {
        let binding: HashMap<String, u64> = HashMap::new();
        let client = data.get("client_dll").unwrap_or(&binding);
        let engine = data.get("engine2_dll").unwrap_or(&binding);

        Self {
            offsets: OffsetsConfig {
                entityList: *client.get("entitylist").unwrap_or(&0x24B0258),
                viewMatrix: *client.get("viewmatrix").unwrap_or(&0x2310F10),
                localPlayerController: *client.get("localplayercontroller").unwrap_or(&0x22F5028),
                globalVars: *client.get("globalvars").unwrap_or(&0x205F540),
                plantedC4: *client.get("plantedc4").unwrap_or(&0x2318A60),
                buildNumber: *engine.get("buildnumber").unwrap_or(&0x60E514),
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
        self.update_from_network(DEFAULT_OFFSETS_URL, DEFAULT_CLIENT_DLL_URL)
    }

    pub fn apply_remote_offsets(&mut self, offsets_json: &Value, client_json: &Value) {
        if let Some(client_dll) = offsets_json.get("client.dll") {
            if let Some(val) = client_dll.get("dwEntityList").and_then(|v| v.as_u64()) {
                self.offsets.entityList = val;
            }
            if let Some(val) = client_dll.get("dwViewMatrix").and_then(|v| v.as_u64()) {
                self.offsets.viewMatrix = val;
            }
            if let Some(val) = client_dll.get("dwLocalPlayerController").and_then(|v| v.as_u64()) {
                self.offsets.localPlayerController = val;
            }
            if let Some(val) = client_dll.get("dwGlobalVars").and_then(|v| v.as_u64()) {
                self.offsets.globalVars = val;
            }
            if let Some(val) = client_dll.get("dwPlantedC4").and_then(|v| v.as_u64()) {
                self.offsets.plantedC4 = val;
            }
        }

        if let Some(engine_dll) = offsets_json.get("engine2.dll") {
            if let Some(val) = engine_dll.get("dwBuildNumber").and_then(|v| v.as_u64()) {
                self.offsets.buildNumber = val;
            }
        }

        if let Some(classes) = client_json.get("client.dll").and_then(|c| c.get("classes")) {
            // Controller
            if let Some(v) = find_class_field_offset(classes, "m_iPing", Some("CCSPlayerController")) {
                self.offsets.controller.m_iPing = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_hPawn", Some("CBasePlayerController")) {
                self.offsets.controller.m_hPawn = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_steamID", Some("CBasePlayerController")) {
                self.offsets.controller.m_steamID = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_iszPlayerName", Some("CBasePlayerController")) {
                self.offsets.controller.m_iszPlayerName = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_bIsLocalPlayerController", Some("CBasePlayerController")) {
                self.offsets.controller.m_bIsLocalPlayerController = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_pInGameMoneyServices", Some("CCSPlayerController")) {
                self.offsets.controller.m_pInGameMoneyServices = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_iAccount", Some("CCSPlayerController_InGameMoneyServices")) {
                self.offsets.controller.m_iAccount = v;
            }

            // Pawn
            if let Some(v) = find_class_field_offset(classes, "m_vOldOrigin", Some("C_BasePlayerPawn")) {
                self.offsets.pawn.m_vOldOrigin = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_iHealth", Some("C_BaseEntity")) {
                self.offsets.pawn.m_iHealth = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_iTeamNum", Some("C_BaseEntity")) {
                self.offsets.pawn.m_iTeamNum = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_bIsScoped", Some("C_CSPlayerPawn")) {
                self.offsets.pawn.m_bIsScoped = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_ArmorValue", Some("C_CSPlayerPawn")) {
                self.offsets.pawn.m_ArmorValue = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_bIsDefusing", Some("C_CSPlayerPawn")) {
                self.offsets.pawn.m_bIsDefusing = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_pGameSceneNode", Some("C_BaseEntity")) {
                self.offsets.pawn.m_pGameSceneNode = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_pClippingWeapon", Some("C_CSPlayerPawnBase")) {
                self.offsets.pawn.m_pClippingWeapon = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_entitySpottedState", Some("C_CSPlayerPawn")) {
                self.offsets.pawn.m_entitySpottedState = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_bSpottedByMask", Some("EntitySpottedState_t")) {
                self.offsets.pawn.m_bSpottedByMask = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_flFlashOverlayAlpha", Some("C_CSPlayerPawnBase")) {
                self.offsets.pawn.m_flFlashOverlayAlpha = v;
            }

            // Bomb
            if let Some(v) = find_class_field_offset(classes, "m_isPlanted", Some("C_PlantedC4")) {
                self.offsets.bomb.m_isPlanted = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_bC4Activated", Some("C_PlantedC4")) {
                self.offsets.bomb.m_bC4Activated = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_nBombSite", Some("C_PlantedC4")) {
                self.offsets.bomb.m_nBombSite = v;
            }
            if let Some(v) = find_class_field_offset(classes, "m_vecAbsOrigin", Some("CGameSceneNode")) {
                self.offsets.bomb.m_vecAbsOrigin = v;
            }

            // Bone
            if let Some(v) = find_class_field_offset(classes, "m_modelState", Some("CSkeletonInstance")) {
                self.offsets.bone.m_modelState = v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
