//! Derleme Zamanı + Çalışma Zamanı Hash ve Sabit Gizleme (Anti-Static Analysis) Katmanı.
//!
//! Bu modül, derlenen ikili dosyada (`.rdata` / `.text`) açık metin (string literal)
//! veya bilinen sabit sayısal imzalar (offsetler, portlar, şema sınıf/alan adları)
//! bırakmamak için iki mekanizma sunar:
//! 1. **Compile-Time / Runtime 64-bit Keyed FNV-1a (`kh!` & `rt_hash`)**:
//!    Sabit tanımlayıcılar derleme zamanında (`const { ... }`) `u64` özetine dönüştürülür;
//!    çalışma zamanında gelen dinamik veriler (JSON anahtarları, modül/süreç adları,
//!    silah önekleri) `rt_hash` ile özetlenerek `u64 == u64` olarak karşılaştırılır.
//! 2. **Compile-Time XOR Sayısal Sabit Gizleme (`obf_u64!`, `obf_usize!`, `obf_u16!`)**:
//!    Sabit offset ve port değerleri derleme zamanında konum-türevli anahtarla XOR'lanır,
//!    çalışma zamanında `black_box` üzerinden çözülür.

use serde_json::Value;

/// Özel tohumlu 64-bit FNV-1a özeti (hem `const` hem runtime bağlamında çalışır).
pub const FNV_SEED: u64 = 0xA57F_93C1_E842_6D19;
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

#[inline]
pub const fn rt_hash(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut h = FNV_SEED;
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    h
}

/// Konum bilgisinden (`line`, `column`,ek sabit) derleme zamanı 64-bit XOR maskesi üretir.
#[inline]
pub const fn ct_key(line: u32, col: u32, salt: u64) -> u64 {
    let mut x = (line as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((col as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        ^ salt
        ^ 0xD1B5_4A32_D192_ED03;
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Derleme zamanında string sabitini `u64` hash'e çevirir; string `.rdata` bölümüne girmez.
#[macro_export]
macro_rules! kh {
    ($s:expr) => {{
        const H: u64 = $crate::hash::rt_hash($s);
        H
    }};
}

/// Derleme zamanında küçük harfli modül/süreç adını `sysmemkit` 32-bit DJB2 hash'ine çevirir.
#[macro_export]
macro_rules! djb2 {
    ($s:expr) => {{
        const H: u32 = sysmemkit::utils::dbj2_hash($s);
        H
    }};
}

/// Derleme zamanında `u64` sabitini XOR ile maskeler ve çalışma zamanında `black_box` ile çözer.
#[macro_export]
macro_rules! obf_u64 {
    ($val:expr) => {{
        const KEY: u64 = $crate::hash::ct_key(line!(), column!(), 0x517C_C1B7_2722_0A95);
        const ENC: u64 = ($val as u64) ^ KEY;
        std::hint::black_box(ENC) ^ std::hint::black_box(KEY)
    }};
}

/// Derleme zamanında `usize` sabitini XOR ile maskeler ve çalışma zamanında `black_box` ile çözer.
#[macro_export]
macro_rules! obf_usize {
    ($val:expr) => {{
        ($crate::obf_u64!($val as u64)) as usize
    }};
}

/// Derleme zamanında `u16` sabitini XOR ile maskeler ve çalışma zamanında `black_box` ile çözer.
#[macro_export]
macro_rules! obf_u16 {
    ($val:expr) => {{
        ($crate::obf_u64!($val as u64)) as u16
    }};
}

/// JSON nesnesi içinde alan adını string olarak saklamadan, çalışma zamanı hash eşleşmesiyle bulur.
pub fn json_get_by_hash<'a>(value: &'a Value, target_hash: u64) -> Option<&'a Value> {
    let map = value.as_object()?;
    for (k, v) in map {
        if rt_hash(k) == target_hash {
            return Some(v);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_time_and_runtime_hashes_match() {
        let dynamic = String::from("dwLocalPlayerController");
        assert_eq!(kh!("dwLocalPlayerController"), rt_hash(&dynamic));
        assert_ne!(kh!("dwEntityList"), kh!("dwViewMatrix"));
    }

    #[test]
    fn obfuscated_numbers_roundtrip() {
        let a = obf_usize!(0x24B0258);
        let b = obf_u64!(0x2310F10);
        let p = obf_u16!(9222);
        assert_eq!(a, 0x24B0258);
        assert_eq!(b, 0x2310F10);
        assert_eq!(p, 9222);
    }

    #[test]
    fn json_lookup_by_hash_works() {
        let v: Value = serde_json::json!({
            "client.dll": {
                "dwEntityList": 123456
            }
        });
        let client = json_get_by_hash(&v, kh!("client.dll")).expect("client.dll bulunmalı");
        let el = json_get_by_hash(client, kh!("dwEntityList"))
            .and_then(|x| x.as_u64())
            .unwrap();
        assert_eq!(el, 123456);
    }
}
