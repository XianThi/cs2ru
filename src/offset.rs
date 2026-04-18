use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneOffsets {
    pub m_modelState: usize,
}

impl Default for BoneOffsets {
    fn default() -> Self {
        Self { m_modelState: 0x160 }
    }
}


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
        let binding: HashMap<String,u64> = HashMap::new();
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
}