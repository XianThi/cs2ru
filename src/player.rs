use crate::SysMemKit;
use crate::offset::OffsetsConfig;
use crate::vec3::Vec3;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Player<'a> {
    index: usize,
    entity_list: usize,
    list_entry: usize,
    mem: Arc<SysMemKit>,
    offsets: &'a OffsetsConfig,
    clientbase: usize,
    controller: usize,
    pawn: usize,
    pawn_handle: usize,
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
        mem: Arc<SysMemKit>,
        offsets: &'a OffsetsConfig,
        clientbase: usize,
    ) -> Self {
        let pawn_read_size = Self::calc_max_pawn_offset(offsets);
        let slow_update_interval = Duration::from_millis(500);

        Self {
            index,
            entity_list: 0,
            list_entry,
            mem,
            offsets,
            clientbase,
            controller: 0,
            pawn: 0,
            pawn_handle: 0,
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
            // İlk güncellemede controller ve silah bilgisinin hemen çekilmesi için geçmiş zaman
            last_slow_update: Instant::now() - slow_update_interval,
            slow_update_interval,
            pawn_read_size,
        }
    }

    pub fn reset_context(&mut self, entity_list: usize, list_entry: usize, mem: Arc<SysMemKit>) {
        if self.list_entry != list_entry || self.entity_list != entity_list {
            self.entity_list = entity_list;
            self.list_entry = list_entry;
            self.controller = 0;
            self.pawn = 0;
            self.pawn_handle = 0;
        }
        self.mem = mem;
    }

    fn calc_max_pawn_offset(offsets: &OffsetsConfig) -> usize {
        let offsets_list = [
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
        *offsets_list.iter().max().unwrap_or(&0) + 32
    }

    pub fn update(&mut self, force_slow: bool) -> bool {
        if !self.get_controller() {
            return false;
        }
        if !self.get_pawn() {
            return false;
        }
        if !self.update_pawn() {
            return false;
        }
        let now = Instant::now();
        if force_slow
            || self.name.is_empty()
            || self.weapon.is_empty()
            || now.duration_since(self.last_slow_update) > self.slow_update_interval
        {
            self.update_controller();
            self.update_weapon();
            self.last_slow_update = now;
        }
        true
    }

    fn get_controller(&mut self) -> bool {
        unsafe {
            let stride = crate::obf_usize!(0x70);
            let controller: usize = self
                .mem
                .read::<usize>(self.list_entry + ((self.index + 1) * stride));
            if controller == 0 {
                self.controller = 0;
                self.pawn = 0;
                self.pawn_handle = 0;
                self.alive = false;
                return false;
            }
            if controller != self.controller {
                self.controller = controller;
                self.pawn = 0;
                self.pawn_handle = 0;
                self.name.clear();
                self.weapon.clear();
                self.clean_weapon.clear();
            }
            true
        }
    }

    fn get_pawn(&mut self) -> bool {
        unsafe {
            let handle_mask = crate::obf_usize!(0x7FFF);
            let entry_mask = crate::obf_usize!(0x1FF);
            let stride = crate::obf_usize!(0x70);
            let entity_pawn_handle = self
                .mem
                .read::<usize>(self.controller + self.offsets.controller.m_hPawn);
            if entity_pawn_handle == 0 || (entity_pawn_handle & handle_mask) == handle_mask {
                self.pawn = 0;
                self.pawn_handle = 0;
                self.alive = false;
                return false;
            }

            if entity_pawn_handle == self.pawn_handle && self.pawn != 0 {
                return true;
            }

            let entity_list = if self.entity_list != 0 {
                self.entity_list
            } else {
                self.mem
                    .read::<usize>(self.clientbase + self.offsets.entityList as usize)
            };
            if entity_list == 0 {
                self.pawn = 0;
                return false;
            }

            let list_entry = self
                .mem
                .read::<usize>(entity_list + crate::obf_usize!(0x10) + 0x8 * ((entity_pawn_handle & handle_mask) >> 9));
            if list_entry == 0 {
                self.pawn = 0;
                return false;
            }

            let pawn = self
                .mem
                .read::<usize>(list_entry + stride * (entity_pawn_handle & entry_mask));
            if pawn != 0 {
                self.pawn = pawn;
                self.pawn_handle = entity_pawn_handle;
                self.weapon.clear();
                self.clean_weapon.clear();
                true
            } else {
                self.pawn = 0;
                false
            }
        }
    }

    fn update_pawn(&mut self) -> bool {
        unsafe {
            let raw = self.mem.read_buffer(self.pawn, self.pawn_read_size);
            if raw.is_empty() {
                return false;
            }

            self.health = self.read_i32(&raw, self.offsets.pawn.m_iHealth);
            self.alive = self.health > 0 && self.health <= 1000;
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
            self.flashed = self.read_f32(&raw, self.offsets.pawn.m_flFlashOverlayAlpha) > 0.5;
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
            let name_ptr = self.controller + self.offsets.controller.m_iszPlayerName;
            let name_bytes = self.mem.read_buffer(name_ptr, 32);
            let name = name_bytes.split(|&b| b == 0).next().unwrap_or(&[]);
            self.name = String::from_utf8_lossy(name).trim().to_string();
            self.localplayer = self
                .mem
                .read::<bool>(self.controller + self.offsets.controller.m_bIsLocalPlayerController);
            self.ping = self
                .mem
                .read::<i32>(self.controller + self.offsets.controller.m_iPing)
                .max(0) as u32;

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
                self.weapon.clear();
                self.clean_weapon.clear();
                return;
            }

            let weapon_ptr2 = self.mem.read::<usize>(weapon_ptr + 0x10);
            if weapon_ptr2 == 0 {
                return;
            }
            let weapon_ptr3 = self.mem.read::<usize>(weapon_ptr2 + 0x20);
            if weapon_ptr3 == 0 {
                return;
            }

            let weapon_bytes = self.mem.read_buffer(weapon_ptr3, 32);
            let raw_name = weapon_bytes.split(|&b| b == 0).next().unwrap_or(&[]);
            self.weapon = String::from_utf8_lossy(raw_name).trim().to_string();
            self.clean_weapon = self.weapon.replace(obfstr::obfstr!("weapon_"), "");
        }
    }

    fn update_skeleton(&mut self, raw_pawn: &[u8]) {
        self.bone_list.clear();

        let game_scene = self.read_u64(raw_pawn, self.offsets.pawn.m_pGameSceneNode);
        if game_scene == 0 {
            return;
        }

        unsafe {
            let bone_array_off = crate::obf_usize!(0x80);
            let bone_array = self
                .mem
                .read::<usize>(game_scene as usize + (self.offsets.bone.m_modelState + bone_array_off));
            if bone_array == 0 {
                return;
            }

            const BONE_COUNT: usize = 30;
            const BONE_STRIDE: usize = 0x20;
            let raw_bones = self.mem.read_buffer(bone_array, BONE_COUNT * BONE_STRIDE);
            if raw_bones.len() < BONE_COUNT * BONE_STRIDE {
                return;
            }
            for i in 0..BONE_COUNT {
                let base = i * BONE_STRIDE;
                let x = f32::from_le_bytes(raw_bones[base..base + 4].try_into().unwrap_or([0; 4]));
                let y =
                    f32::from_le_bytes(raw_bones[base + 4..base + 8].try_into().unwrap_or([0; 4]));
                let z =
                    f32::from_le_bytes(raw_bones[base + 8..base + 12].try_into().unwrap_or([0; 4]));
                self.bone_list.push(Vec3::new(x, y, z));
            }
        }
    }

    fn read_i32(&self, buffer: &[u8], offset: usize) -> i32 {
        buffer
            .get(offset..offset + 4)
            .and_then(|s| s.try_into().ok())
            .map(i32::from_le_bytes)
            .unwrap_or(0)
    }

    fn read_f32(&self, buffer: &[u8], offset: usize) -> f32 {
        buffer
            .get(offset..offset + 4)
            .and_then(|s| s.try_into().ok())
            .map(f32::from_le_bytes)
            .unwrap_or(0.0)
    }

    fn read_bool(&self, buffer: &[u8], offset: usize) -> bool {
        buffer.get(offset).copied().unwrap_or(0) != 0
    }

    fn read_u64(&self, buffer: &[u8], offset: usize) -> u64 {
        buffer
            .get(offset..offset + 8)
            .and_then(|s| s.try_into().ok())
            .map(u64::from_le_bytes)
            .unwrap_or(0)
    }
}

