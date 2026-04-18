use crate::SysMemKit;
use crate::offset::OffsetsConfig;
use crate::vec3::Vec3;
use std::time::{Duration, Instant};
pub struct Player<'a> {
    index: usize,
    list_entry: usize,
    mem: &'a SysMemKit,
    offsets: &'a OffsetsConfig,
    clientbase: usize,
    controller: usize,
    pawn: usize,
    pub pos: Vec3,
    pub ping: u32,
    pub team: i32,
    pub health: i32,
    pub armor: i32,
    pub money: i32,
    pub bot: bool,
    pub alive: bool,
    pub scoped: bool,
    pub flashed: bool,
    pub spotted: bool,
    pub defusing: bool,
    pub localplayer: bool,
    pub name: String,
    pub steam_id: u64,
    pub weapon: String,
    pub clean_weapon: String,
    pub bone_list: Vec<Vec3>,
    last_slow_update: Instant,
    slow_update_interval: Duration,
    pawn_read_size: usize,
}

impl<'a> Player<'a> {
    pub fn new(
        index: usize,
        list_entry: usize,
        mem: &'a SysMemKit,
        offsets: &'a OffsetsConfig,
        clientbase: usize,
    ) -> Self {
        let pawn_read_size = Self::calc_max_pawn_offset(&offsets);

        Self {
            index,
            list_entry,
            mem,
            offsets,
            clientbase,
            controller: 0,
            pawn: 0,
            pos: Vec3::zero(),
            ping: 0,
            team: 0,
            health: 0,
            armor: 0,
            money: 0,
            bot: true,
            alive: false,
            scoped: false,
            flashed: false,
            spotted: false,
            defusing: false,
            localplayer: false,
            name: String::new(),
            steam_id: 0,
            weapon: String::new(),
            clean_weapon: String::new(),
            bone_list: Vec::with_capacity(30),
            last_slow_update: Instant::now(),
            slow_update_interval: Duration::from_millis(500),
            pawn_read_size,
        }
    }

    fn calc_max_pawn_offset(offsets: &OffsetsConfig) -> usize {
        let offsets_list = vec![
            offsets.pawn.m_iHealth,
            offsets.pawn.m_vOldOrigin + 12,
            offsets.pawn.m_pGameSceneNode + 8,
            offsets.pawn.m_iTeamNum,
            offsets.pawn.m_ArmorValue,
            offsets.pawn.m_bIsDefusing,
            offsets.pawn.m_entitySpottedState + offsets.pawn.m_bSpottedByMask,
            offsets.pawn.m_flFlashOverlayAlpha,
            offsets.pawn.m_bIsScoped,
        ];
        *offsets_list.iter().max().unwrap_or(&0) as usize + 32
    }

    pub fn update(&mut self, force_slow: bool) -> bool {
        if self.controller == 0 {
            if !self.get_controller() {
                return false;
            }
        }
        if self.pawn == 0 {
            if !self.get_pawn() {
                return false;
            }
        }
        if !self.update_pawn() {
            return false;
        }
        self.update_controller();
        let now = Instant::now();
        if force_slow || now.duration_since(self.last_slow_update) > self.slow_update_interval {
            self.update_controller();
            self.update_weapon();
            self.last_slow_update = now;
        }
        true
    }

    fn get_controller(&mut self) -> bool {
        unsafe {
            let controller: usize = self
                .mem
                .read::<usize>(self.list_entry + ((self.index + 1) * 0x70));
            //println!("controller : {:?}",controller);
            if controller != 0 {
                self.controller = controller;
                true
            } else {
                false
            }
        }
    }

    fn get_pawn(&mut self) -> bool {
        unsafe {
            let entity_pawn_handle = self
                .mem
                .read::<usize>(self.controller + self.offsets.controller.m_hPawn);
            if entity_pawn_handle == 0 {
                return false;
            }

            let entity_list = self
                .mem
                .read::<usize>(self.clientbase + self.offsets.entityList as usize);
            if entity_list == 0 {
                return false;
            }

            let list_entry = self
                .mem
                .read::<usize>(entity_list + 0x10 + 0x8 * ((entity_pawn_handle & 0x7FFF) >> 9));
            if list_entry == 0 {
                return false;
            }

            let pawn = self
                .mem
                .read::<usize>(list_entry + 0x70 * (entity_pawn_handle & 0x1FF));
            //println!("pawn : {:?}",pawn);
            if pawn != 0 {
                self.pawn = pawn;
                true
            } else {
                false
            }
        }
    }

    fn update_pawn(&mut self) -> bool {
        unsafe {
            //println!("pawn - read size : {:?} - {:?}",self.pawn, self.pawn_read_size);
            //let raw = self.mem.read::<[u8;10060]>(self.pawn);
            let raw = self.mem.read_buffer(self.pawn, self.pawn_read_size);
            //println!("raw pawn : {:?}",raw);
            if raw.is_empty() {
                return false;
            }

            self.health = self.read_i32(&raw, self.offsets.pawn.m_iHealth);
            self.alive = self.health > 0;
            if !self.alive {
                return true;
            }

            self.pos = Vec3 {
                x: self.read_f32(&raw, self.offsets.pawn.m_vOldOrigin),
                y: self.read_f32(&raw, self.offsets.pawn.m_vOldOrigin + 4),
                z: self.read_f32(&raw, self.offsets.pawn.m_vOldOrigin + 8),
            };

            self.team = self.read_i32(&raw, self.offsets.pawn.m_iTeamNum);
            self.armor = self.read_i32(&raw, self.offsets.pawn.m_ArmorValue);
            self.defusing = self.read_bool(&raw, self.offsets.pawn.m_bIsDefusing);
            self.spotted = self.read_bool(
                &raw,
                self.offsets.pawn.m_entitySpottedState + self.offsets.pawn.m_bSpottedByMask,
            );
            self.flashed = self.read_f32(&raw, self.offsets.pawn.m_flFlashOverlayAlpha) > 0.0;
            self.scoped = self.read_bool(&raw, self.offsets.pawn.m_bIsScoped);

            self.update_skeleton(&raw);
            true
        }
    }

    fn update_controller(&mut self) {
        unsafe {
            self.steam_id = self
                .mem
                .read::<u64>(self.controller + self.offsets.controller.m_steamID);
            self.bot = self.steam_id == 0;
            let name_ptr = self.controller + self.offsets.controller.m_iszPlayerName as usize;
            let name_bytes = self.mem.read_buffer(name_ptr, 32);
            let name = name_bytes.split(|&b| b == 0).next().unwrap_or(&[]);
            self.name = String::from_utf8_lossy(name).to_string();
            self.localplayer = self
                .mem
                .read::<bool>(self.controller + self.offsets.controller.m_bIsLocalPlayerController);
            self.ping = self
                .mem
                .read::<i32>(self.controller + self.offsets.controller.m_iPing)
                as u32;

            let money_services = self
                .mem
                .read::<usize>(self.controller + self.offsets.controller.m_pInGameMoneyServices);
            if money_services != 0 {
                self.money = self
                    .mem
                    .read::<i32>(money_services + self.offsets.controller.m_iAccount);
            }
        }
    }

    fn update_weapon(&mut self) {
        unsafe {
            let weapon_ptr = self
                .mem
                .read::<usize>(self.pawn + self.offsets.pawn.m_pClippingWeapon);
            if weapon_ptr == 0 {
                return;
            }

            let weapon_ptr2 = self.mem.read::<usize>(weapon_ptr + 0x10);
            let weapon_ptr3 = self.mem.read::<usize>(weapon_ptr2 + 0x20);

            let weapon_bytes = self.mem.read_buffer(weapon_ptr3, 32);
            self.weapon = String::from_utf8_lossy(&weapon_bytes).to_string();
            self.clean_weapon = self.weapon.replace("weapon_", "");
        }
    }
    fn update_skeleton(&mut self, raw_pawn: &[u8]) {
        self.bone_list.clear();

        let game_scene = self.read_u64(raw_pawn, self.offsets.pawn.m_pGameSceneNode);
        if game_scene == 0 {
            return;
        }

        unsafe {
            let bone_array = self
                .mem
                .read::<usize>(game_scene as usize + (self.offsets.bone.m_modelState + 0x80));
            if bone_array == 0 {
                return;
            }

            let raw_bones = self.mem.read_buffer(bone_array, 30 * 0x20);
            for i in 0..30 {
                let x = f32::from_le_bytes(raw_bones[i * 0x20..i * 0x20 + 4].try_into().unwrap());
                let y =
                    f32::from_le_bytes(raw_bones[i * 0x20 + 4..i * 0x20 + 8].try_into().unwrap());
                let z =
                    f32::from_le_bytes(raw_bones[i * 0x20 + 8..i * 0x20 + 12].try_into().unwrap());
                self.bone_list.push(Vec3::new(x, y, z));
            }
        }
    }

    fn read_i32(&self, buffer: &[u8], offset: usize) -> i32 {
        i32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap_or([0; 4]))
    }

    fn read_f32(&self, buffer: &[u8], offset: usize) -> f32 {
        f32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap_or([0; 4]))
    }

    fn read_bool(&self, buffer: &[u8], offset: usize) -> bool {
        buffer[offset] != 0
    }

    fn read_u64(&self, buffer: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(buffer[offset..offset + 8].try_into().unwrap_or([0; 8]))
    }
}

