use sysmemkit::SysMemKit;

fn main() {
     unsafe {
        let kit = SysMemKit::new("cs2.exe",false).expect("Oyun bulunamadı veya handle çalınamadı!");
        let clientdll = match kit.getmodule("client.dll"){
            Some(dll)=> dll,
             None => {
                println!("client.dll bulunamadı");
                return;
            }
        }; 
        let engine2dll = match kit.getmodule("engine2.dll"){
            Some(dll)=> dll,
             None => {
                println!("engine2.dll bulunamadı");
                return;
            }
        };
        if clientdll != 0 && engine2dll != 0{
            let entitylist:u64 = kit.read(clientdll + 0x24B0258);
            let listentry:u64 = kit.read(entitylist as usize + 0x10);
            println!("entitylist ve list entry -> {:X}, {:X}",entitylist,listentry)
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn distance(&self, other: &Vec3) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }
}

pub const fn dbj2_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = (hash << 5).wrapping_add(hash).wrapping_add(bytes[i] as u32);
        i += 1;
    }
    hash
}