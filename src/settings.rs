use crate::win::{self, HKEY};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Fr,
    Es,
    Zh,
    Ru,
    Ja,
    Ko,
    De,
    It,
    Pt,
    Ar,
    Hi,
    Tr,
    Nl,
    Pl,
    Vi,
    Th,
    Id,
}

/// Font family used by GDI+ for a given script. Poppins stays the default for
/// Latin UI text; the other families avoid tofu boxes for non-Latin scripts.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum FontKind {
    Poppins,
    CjkSc,
    CjkJp,
    CjkKr,
    Cyrillic,
    Arabic,
    Indic,
    Thai,
    SystemLatin,
}

pub const LANG_COUNT: usize = 18;

pub const LANGS: [Lang; LANG_COUNT] = [
    Lang::En,
    Lang::Fr,
    Lang::Es,
    Lang::Zh,
    Lang::Ru,
    Lang::Ja,
    Lang::Ko,
    Lang::De,
    Lang::It,
    Lang::Pt,
    Lang::Ar,
    Lang::Hi,
    Lang::Tr,
    Lang::Nl,
    Lang::Pl,
    Lang::Vi,
    Lang::Th,
    Lang::Id,
];

impl Lang {
    pub const fn index(self) -> usize {
        match self {
            Lang::En => 0,
            Lang::Fr => 1,
            Lang::Es => 2,
            Lang::Zh => 3,
            Lang::Ru => 4,
            Lang::Ja => 5,
            Lang::Ko => 6,
            Lang::De => 7,
            Lang::It => 8,
            Lang::Pt => 9,
            Lang::Ar => 10,
            Lang::Hi => 11,
            Lang::Tr => 12,
            Lang::Nl => 13,
            Lang::Pl => 14,
            Lang::Vi => 15,
            Lang::Th => 16,
            Lang::Id => 17,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::Zh => "zh",
            Lang::Ru => "ru",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::De => "de",
            Lang::It => "it",
            Lang::Pt => "pt",
            Lang::Ar => "ar",
            Lang::Hi => "hi",
            Lang::Tr => "tr",
            Lang::Nl => "nl",
            Lang::Pl => "pl",
            Lang::Vi => "vi",
            Lang::Th => "th",
            Lang::Id => "id",
        }
    }

    /// Native label shown in the language dropdown.
    pub const fn native(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Fr => "Français",
            Lang::Es => "Español",
            Lang::Zh => "中文",
            Lang::Ru => "Русский",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::De => "Deutsch",
            Lang::It => "Italiano",
            Lang::Pt => "Português",
            Lang::Ar => "العربية",
            Lang::Hi => "हिन्दी",
            Lang::Tr => "Türkçe",
            Lang::Nl => "Nederlands",
            Lang::Pl => "Polski",
            Lang::Vi => "Tiếng Việt",
            Lang::Th => "ไทย",
            Lang::Id => "Bahasa Indonesia",
        }
    }

    pub const fn font_kind(self) -> FontKind {
        match self {
            Lang::Zh => FontKind::CjkSc,
            Lang::Ja => FontKind::CjkJp,
            Lang::Ko => FontKind::CjkKr,
            Lang::Ru => FontKind::Cyrillic,
            Lang::Ar => FontKind::Arabic,
            Lang::Hi => FontKind::Indic,
            Lang::Th => FontKind::Thai,
            Lang::Vi => FontKind::SystemLatin,
            _ => FontKind::Poppins,
        }
    }

    pub const fn is_rtl(self) -> bool {
        matches!(self, Lang::Ar)
    }

    pub fn from_code(code: &str) -> Option<Self> {
        let trimmed = code.trim_start_matches('"');
        let short = trimmed.get(..2).unwrap_or(trimmed).to_ascii_lowercase();
        match short.as_str() {
            "en" => Some(Lang::En),
            "fr" => Some(Lang::Fr),
            "es" => Some(Lang::Es),
            "zh" => Some(Lang::Zh),
            "ru" => Some(Lang::Ru),
            "ja" => Some(Lang::Ja),
            "ko" => Some(Lang::Ko),
            "de" => Some(Lang::De),
            "it" => Some(Lang::It),
            "pt" => Some(Lang::Pt),
            "ar" => Some(Lang::Ar),
            "hi" => Some(Lang::Hi),
            "tr" => Some(Lang::Tr),
            "nl" => Some(Lang::Nl),
            "pl" => Some(Lang::Pl),
            "vi" => Some(Lang::Vi),
            "th" => Some(Lang::Th),
            "id" => Some(Lang::Id),
            _ => None,
        }
    }
}

pub struct Settings {
    pub machine_id: String,
    pub indicator: bool,
    pub num_indicator: bool,
    pub language: Lang,
    pub sound: bool,
    pub sound_volume: u8,       // 0..100, in 10 % steps
    pub animation: bool,
    pub startup: bool,

    // Display / behaviour
    pub overlay_ms: u32,        // 0 = permanent
    pub panel_opacity: u8,      // 20..100
    pub overlay_opacity: u8,    // 20..100
    pub hotkey_mod: u8,         // 1=Alt, 2=Ctrl, 4=Shift, 8=Win
    pub hotkey_vk: u32,
    pub overlay_x: i32,         // -1 = automatic (bottom-right)
    pub overlay_y: i32,

    // Local-only statistics
    pub caps_toggles: u64,
    pub num_toggles: u64,
    pub caps_on_ms: u64,
    pub num_on_ms: u64,
    pub stats_started: u64,     // UNIX seconds
}

/// Factory profile used for every new machine/profile: Caps + Num + sound +
/// animation on, startup off, English, all counters zero, Ctrl+Shift+I,
/// 0.8 s notifications, 100 % panel/indicator opacity and volume,
/// automatic bottom-right placement.
impl Default for Settings {
    fn default() -> Self {
        Self {
            machine_id: machine_id(),
            indicator: true,
            num_indicator: true,
            language: Lang::En,
            sound: true,
            sound_volume: 100,
            animation: true,
            startup: false,
            overlay_ms: 800,
            panel_opacity: 100,
            overlay_opacity: 100,
            hotkey_mod: 2 | 4, // Ctrl + Shift
            hotkey_vk: 0x49,   // I
            overlay_x: -1,
            overlay_y: -1,
            caps_toggles: 0,
            num_toggles: 0,
            caps_on_ms: 0,
            num_on_ms: 0,
            stats_started: 0,
        }
    }
}

/// Stable machine fingerprint: system-volume serial + a hash of the computer
/// name. Never an IP address, and never stored inside the executable.
fn machine_id() -> String {
    unsafe {
        let mut system_dir = [0u16; 260];
        let used = win::GetSystemDirectoryW(system_dir.as_mut_ptr(), system_dir.len() as u32) as usize;
        let root = if used >= 3 {
            [system_dir[0], system_dir[1], system_dir[2], 0]
        } else {
            win::wide("C:\\").try_into().unwrap_or([67, 58, 92, 0])
        };

        let mut volume_name = [0u16; 256];
        let mut serial = 0u32;
        let mut max_component = 0u32;
        let mut flags = 0u32;
        let mut fs_name = [0u16; 256];
        let volume_ok = win::GetVolumeInformationW(
            root.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut serial,
            &mut max_component,
            &mut flags,
            fs_name.as_mut_ptr(),
            fs_name.len() as u32,
        ) != 0;

        let mut computer = [0u16; 256];
        let mut size = computer.len() as u32;
        let name_ok = win::GetComputerNameW(computer.as_mut_ptr(), &mut size) != 0;
        let name = if name_ok {
            let end = (size as usize).min(computer.len());
            String::from_utf16_lossy(&computer[..end])
        } else {
            String::new()
        };
        let mut name_hash = 0x811C_9DC5u32;
        for byte in name.to_ascii_uppercase().bytes() {
            name_hash ^= byte as u32;
            name_hash = name_hash.wrapping_mul(0x0100_0193);
        }
        if !volume_ok {
            serial = name_hash;
        }
        format!("{serial:08X}-{name_hash:08X}")
    }
}

/// Settings live under LOCALAPPDATA (never roaming) and in a directory named
/// after the machine fingerprint, so copying the .exe or a config file to
/// another PC cannot reuse these presets.
fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let mut path = PathBuf::from(base);
    path.push("IndiCaps");
    path.push(machine_id());
    path.push("config.json");
    Some(path)
}

fn find_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let start = text.find(&pattern)? + pattern.len();
    let rest = &text[start..];
    let colon = rest.find(':')? + 1;
    Some(rest[colon..].trim_start())
}

fn read_string(text: &str, key: &str) -> Option<String> {
    let value = find_after(text, key)?;
    let start = value.find('"')? + 1;
    let end = value[start..].find('"')? + start;
    Some(value[start..end].to_string())
}

fn read_bool(text: &str, key: &str, default: bool) -> bool {
    match find_after(text, key) {
        Some(v) if v.starts_with("true") => true,
        Some(v) if v.starts_with("false") => false,
        _ => default,
    }
}

fn read_i64(text: &str, key: &str, default: i64) -> i64 {
    let Some(v) = find_after(text, key) else { return default };
    let number: String = v
        .chars()
        .skip_while(|c| !c.is_ascii_digit() && *c != '-')
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    number.parse().unwrap_or(default)
}

impl Settings {
    /// Explicit entry point for the factory profile.
    pub fn factory() -> Self {
        Self::default()
    }

    pub fn load() -> Self {
        let mut s = Self::factory();
        let Ok(text) = fs::read_to_string(config_path().unwrap_or_default()) else {
            return s;
        };
        // A config copied from another PC is ignored, even if it was dropped
        // into this machine's folder by hand.
        if read_string(&text, "MachineId").as_deref() != Some(s.machine_id.as_str()) {
            return s;
        }
        s.indicator = read_bool(&text, "IndicatorEnabled", s.indicator);
        s.num_indicator = read_bool(&text, "NumIndicatorEnabled", s.num_indicator);
        s.sound = read_bool(&text, "SoundEnabled", s.sound);
        s.sound_volume = read_i64(&text, "SoundVolume", s.sound_volume as i64)
            .clamp(0, 100) as u8;
        s.animation = read_bool(&text, "AnimationEnabled", s.animation);
        s.startup = read_bool(&text, "StartWithWindows", s.startup);

        // Clamp before narrowing: a corrupted JSON value must not wrap into a
        // plausible-looking setting (e.g. 5_000_000_000 -> 705_032_704).
        s.overlay_ms = read_i64(&text, "OverlayDurationMs", s.overlay_ms as i64)
            .clamp(0, u32::MAX as i64) as u32;
        s.panel_opacity = read_i64(&text, "PanelOpacity", s.panel_opacity as i64).clamp(20, 100) as u8;
        s.overlay_opacity = read_i64(&text, "OverlayOpacity", s.overlay_opacity as i64).clamp(20, 100) as u8;
        s.hotkey_mod = read_i64(&text, "HotkeyMod", s.hotkey_mod as i64).clamp(0, 15) as u8;
        s.hotkey_vk = read_i64(&text, "HotkeyVk", s.hotkey_vk as i64).clamp(0, 255) as u32;
        s.overlay_x = read_i64(&text, "OverlayX", s.overlay_x as i64)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        s.overlay_y = read_i64(&text, "OverlayY", s.overlay_y as i64)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        s.caps_toggles = read_i64(&text, "CapsToggles", 0).clamp(0, i64::MAX) as u64;
        s.num_toggles = read_i64(&text, "NumToggles", 0).clamp(0, i64::MAX) as u64;
        s.caps_on_ms = read_i64(&text, "CapsOnMs", 0).clamp(0, i64::MAX) as u64;
        s.num_on_ms = read_i64(&text, "NumOnMs", 0).clamp(0, i64::MAX) as u64;
        s.stats_started = read_i64(&text, "StatsStarted", 0).clamp(0, i64::MAX) as u64;
        if s.stats_started == 0 {
            s.stats_started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
        }

        if let Some(v) = find_after(&text, "Language") {
            if let Some(lang) = Lang::from_code(v) {
                s.language = lang;
            }
        }
        s
    }

    pub fn save(&self) {
        let Some(path) = config_path() else { return };
        let Some(dir) = path.parent() else { return };
        if fs::create_dir_all(dir).is_err() {
            return;
        }
        let json = format!(
            concat!(
                "{{\n",
                "  \"MachineId\": \"{}\",\n",
                "  \"IndicatorEnabled\": {},\n",
                "  \"NumIndicatorEnabled\": {},\n",
                "  \"Language\": \"{}\",\n",
                "  \"SoundEnabled\": {},\n",
                "  \"SoundVolume\": {},\n",
                "  \"AnimationEnabled\": {},\n",
                "  \"StartWithWindows\": {},\n",
                "  \"OverlayDurationMs\": {},\n",
                "  \"PanelOpacity\": {},\n",
                "  \"OverlayOpacity\": {},\n",
                "  \"HotkeyMod\": {},\n",
                "  \"HotkeyVk\": {},\n",
                "  \"OverlayX\": {},\n",
                "  \"OverlayY\": {},\n",
                "  \"CapsToggles\": {},\n",
                "  \"NumToggles\": {},\n",
                "  \"CapsOnMs\": {},\n",
                "  \"NumOnMs\": {},\n",
                "  \"StatsStarted\": {}\n",
                "}}"
            ),
            self.machine_id,
            self.indicator,
            self.num_indicator,
            self.language.code(),
            self.sound,
            self.sound_volume,
            self.animation,
            self.startup,
            self.overlay_ms,
            self.panel_opacity,
            self.overlay_opacity,
            self.hotkey_mod,
            self.hotkey_vk,
            self.overlay_x,
            self.overlay_y,
            self.caps_toggles,
            self.num_toggles,
            self.caps_on_ms,
            self.num_on_ms,
            self.stats_started,
        );
        // Write-then-rename so a crash cannot leave a truncated config behind.
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &json).is_ok()
            && fs::remove_file(&path).is_ok()
            && fs::rename(&tmp, &path).is_ok()
        {
            return;
        }
        let _ = fs::write(&path, &json);
        let _ = fs::remove_file(&tmp);
    }

    pub fn apply_startup(&self) {
        unsafe {
            let subkey = win::wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            let mut hkey: HKEY = core::ptr::null_mut();
            if win::RegOpenKeyExW(
                win::HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                win::KEY_QUERY_VALUE | win::KEY_SET_VALUE,
                &mut hkey,
            ) != 0
            {
                return;
            }
            let name = win::wide("IndiCaps");
            if self.startup {
                if let Ok(exe) = std::env::current_exe() {
                    let value = win::wide(&format!("\"{}\"", exe.to_string_lossy()));
                    win::RegSetValueExW(
                        hkey,
                        name.as_ptr(),
                        0,
                        win::REG_SZ,
                        value.as_ptr() as *const u8,
                        (value.len() * 2) as u32,
                    );
                }
            } else {
                win::RegDeleteValueW(hkey, name.as_ptr());
            }
            win::RegCloseKey(hkey);
        }
    }

    pub fn log_path() -> Option<PathBuf> {
        let mut path = PathBuf::from(std::env::var_os("APPDATA")?);
        path.push("IndiCaps");
        path.push("debug.log");
        Some(path)
    }
}

// Translation rows follow the LANGS order. A compile-time check guarantees
// that every language has an entry for every key.
macro_rules! strings {
    ($(($key:literal, [$($tr:literal),+ $(,)?])),+ $(,)?) => {
        pub fn text(lang: Lang, key: &str) -> &'static str {
            let idx = lang.index();
            match key {
                $(
                    $key => {
                        const ROW: &[&str] = &[$($tr),+];
                        const _: () = assert!(ROW.len() == LANG_COUNT);
                        ROW[idx]
                    }
                )+
                _ => "",
            }
        }
    };
}

strings! {
    // Overlay captions
    ("caps_on", [
        "Caps ON", "Maj ACTIVÉ", "Bloq MAYÚS activado", "大写锁定已开启", "Caps Lock включён",
        "Caps Lock オン", "Caps Lock 켜짐", "Caps Lock EIN", "Bloc Maiusc ON", "Caps Lock ON",
        "الحروف الكبيرة مفعّلة", "कैप्स लॉक चालू", "Caps Lock AÇIK", "Caps Lock AAN", "Caps Lock WŁ.",
        "Caps Lock ĐANG BẬT", "Caps Lock เปิดอยู่", "Caps Lock AKTIF",
    ]),
    ("caps_off", [
        "Caps OFF", "Maj DÉSACTIVÉ", "Bloq MAYÚS desactivado", "大写锁定已关闭", "Caps Lock выключен",
        "Caps Lock オフ", "Caps Lock 꺼짐", "Caps Lock AUS", "Bloc Maiusc OFF", "Caps Lock OFF",
        "الحروف الكبيرة متوقفة", "कैप्स लॉक बंद", "Caps Lock KAPALI", "Caps Lock UIT", "Caps Lock WYŁ.",
        "Caps Lock ĐANG TẮT", "Caps Lock ปิดอยู่", "Caps Lock NONAKTIF",
    ]),
    ("num_on", [
        "Num ON", "Num ACTIVÉ", "Bloq NUM activado", "数字锁定已开启", "Num Lock включён",
        "Num Lock オン", "Num Lock 켜짐", "NumLock EIN", "Bloc Num ON", "Num Lock ON",
        "الحروف الرقمية مفعّلة", "नम लॉक चालू", "Num Lock AÇIK", "Num Lock AAN", "Num Lock WŁ.",
        "Num Lock ĐANG BẬT", "Num Lock เปิดอยู่", "Num Lock AKTIF",
    ]),
    ("num_off", [
        "Num OFF", "Num DÉSACTIVÉ", "Bloq NUM desactivado", "数字锁定已关闭", "Num Lock выключен",
        "Num Lock オフ", "Num Lock 꺼짐", "NumLock AUS", "Bloc Num OFF", "Num Lock OFF",
        "الحروف الرقمية متوقفة", "नम लॉक बंद", "Num Lock KAPALI", "Num Lock UIT", "Num Lock WYŁ.",
        "Num Lock ĐANG TẮT", "Num Lock ปิดอยู่", "Num Lock NONAKTIF",
    ]),

    // Header and tabs
    ("title", [
        "Settings", "Réglages", "Ajustes", "设置", "Настройки", "設定", "설정", "Einstellungen",
        "Impostazioni", "Configurações", "الإعدادات", "सेटिंग्स", "Ayarlar", "Instellingen",
        "Ustawienia", "Cài đặt", "ตั้งค่า", "Pengaturan",
    ]),
    ("tab_settings", [
        "Settings", "Réglages", "Ajustes", "设置", "Настройки", "設定", "설정", "Einstellungen",
        "Impostazioni", "Configurações", "الإعدادات", "सेटिंग्स", "Ayarlar", "Instellingen",
        "Ustawienia", "Cài đặt", "ตั้งค่า", "Pengaturan",
    ]),
    ("tab_stats", [
        "Stats", "Stats", "Estadísticas", "统计", "Статистика", "統計", "통계", "Statistik",
        "Statistiche", "Estatísticas", "الإحصائيات", "आंकड़े", "İstatistikler", "Statistieken",
        "Statystyki", "Thống kê", "สถิติ", "Statistik",
    ]),
    ("tab_hotkeys", [
        "Hotkeys", "Raccourcis", "Atajos", "快捷键", "Горячие клавиши", "ホットキー", "단축키",
        "Tastenkürzel", "Scorciatoie", "Atalhos", "اختصارات", "शॉर्टकट", "Kısayollar", "Sneltoetsen",
        "Skróty", "Phím tắt", "ปุ่มลัด", "Pintasan",
    ]),
    ("tab_display", [
        "Display", "Affichage", "Pantalla", "显示", "Отображение", "表示", "표시", "Anzeige",
        "Visualizzazione", "Exibição", "العرض", "प्रदर्शन", "Görünüm", "Weergave", "Wyświetlanie",
        "Hiển thị", "การแสดงผล", "Tampilan",
    ]),
    ("lang_label", [
        "Language", "Langue", "Idioma", "语言", "Язык", "言語", "언어", "Sprache", "Lingua",
        "Idioma", "اللغة", "भाषा", "Dil", "Taal", "Język", "Ngôn ngữ", "ภาษา", "Bahasa",
    ]),
    ("menu_categories", [
        "Categories", "Catégories", "Categorías", "类别", "Категории", "カテゴリ", "카테고리",
        "Kategorien", "Categorie", "Categorias", "الفئات", "श्रेणियाँ", "Kategoriler", "Categorieën",
        "Kategorie", "Danh mục", "หมวดหมู่", "Kategori",
    ]),

    // Main options
    ("opt_indicator", [
        "Caps indicator", "Indicateur Caps", "Indicador de Bloq MAYÚS", "大写锁定指示",
        "Индикатор Caps Lock", "Caps Lock インジケーター", "Caps Lock 표시", "Caps-Lock-Anzeige",
        "Indicatore Bloc Maiusc", "Indicador Caps Lock", "مؤشر الحروف الكبيرة", "कैप्स लॉक संकेतक",
        "Caps Lock göstergesi", "Caps-Lock-indicator", "Wskaźnik Caps Lock", "Chỉ báo Caps Lock",
        "ตัวบ่งชี้ Caps Lock", "Indikator Caps Lock",
    ]),
    ("opt_num", [
        "Num Lock indicator", "Indicateur Num", "Indicador de Bloq NUM", "数字锁定指示",
        "Индикатор Num Lock", "Num Lock インジケーター", "Num Lock 표시", "Num-Lock-Anzeige",
        "Indicatore Bloc Num", "Indicador Num Lock", "مؤشر الحروف الرقمية", "नम लॉक संकेतक",
        "Num Lock göstergesi", "Num-Lock-indicator", "Wskaźnik Num Lock", "Chỉ báo Num Lock",
        "ตัวบ่งชี้ Num Lock", "Indikator Num Lock",
    ]),
    ("opt_sound", [
        "Sound", "Son", "Sonido", "声音", "Звук", "サウンド", "소리", "Ton", "Suono", "Som",
        "الصوت", "ध्वनि", "Ses", "Geluid", "Dźwięk", "Âm thanh", "เสียง", "Suara",
    ]),
    ("opt_animation", [
        "Fade animation", "Animation", "Animación de fundido", "淡入淡出动画",
        "Анимация появления", "フェードアニメーション", "페이드 애니메이션", "Ein-/Ausblenden",
        "Animazione dissolvenza", "Animação de fade", "حركة التلاشي", "फेड एनीमेशन",
        "Soluklaştırma animasyonu", "Vervaaganimatie", "Animacja zanikania", "Hiệu ứng mờ dần",
        "อนิเมชันการเฟด", "Animasi memudar",
    ]),
    ("opt_startup", [
        "Launch at Windows startup", "Lancer au démarrage", "Iniciar con Windows", "开机启动",
        "Запускать при старте Windows", "Windows 起動時に開始", "Windows 시작 시 실행",
        "Bei Windows-Start ausführen", "Avvia all'avvio di Windows", "Iniciar com o Windows",
        "التشغيل عند بدء Windows", "Windows शुरू होने पर चलाएँ", "Windows ile başlat",
        "Starten bij Windows-opstart", "Uruchamiaj przy starcie Windows", "Khởi động cùng Windows",
        "เริ่มเมื่อเปิด Windows", "Jalankan saat Windows mulai",
    ]),

    // Statistics
    ("stats_title", [
        "Local statistics", "Statistiques locales", "Estadísticas locales", "本地统计",
        "Локальная статистика", "ローカル統計", "로컬 통계", "Lokale Statistik", "Statistiche locali",
        "Estatísticas locais", "إحصائيات محلية", "स्थानीय आंकड़े", "Yerel istatistikler",
        "Lokale statistieken", "Statystyki lokalne", "Thống kê cục bộ", "สถิติในเครื่อง",
        "Statistik lokal",
    ]),
    ("stat_caps", [
        "Caps toggles", "Bascules Caps", "Cambios de Bloq MAYÚS", "大写锁定切换次数",
        "Переключения Caps Lock", "Caps Lock 切り替え回数", "Caps Lock 전환 횟수",
        "Caps-Lock-Umschaltungen", "Attivazioni Bloc Maiusc", "Alternâncias Caps Lock",
        "تبديلات الحروف الكبيرة", "कैप्स लॉक टॉगल", "Caps Lock değiştirme",
        "Caps-Lock-wisselingen", "Przełączenia Caps Lock", "Số lần bật/tắt Caps Lock",
        "จำนวนครั้งที่สลับ Caps Lock", "Jumlah peralihan Caps Lock",
    ]),
    ("stat_num", [
        "Num toggles", "Bascules Num", "Cambios de Bloq NUM", "数字锁定切换次数",
        "Переключения Num Lock", "Num Lock 切り替え回数", "Num Lock 전환 횟수",
        "Num-Lock-Umschaltungen", "Attivazioni Bloc Num", "Alternâncias Num Lock",
        "تبديلات الحروف الرقمية", "नम लॉक टॉगल", "Num Lock değiştirme",
        "Num-Lock-wisselingen", "Przełączenia Num Lock", "Số lần bật/tắt Num Lock",
        "จำนวนครั้งที่สลับ Num Lock", "Jumlah peralihan Num Lock",
    ]),
    ("stat_caps_time", [
        "Time Caps on", "Temps Caps activé", "Tiempo con Bloq MAYÚS", "大写锁定开启时长",
        "Время с Caps Lock", "Caps Lock オン時間", "Caps Lock 켜진 시간", "Zeit mit Caps Lock",
        "Tempo con Bloc Maiusc", "Tempo com Caps Lock", "زمن تفعيل الحروف الكبيرة",
        "कैप्स लॉक चालू समय", "Caps Lock açık kalma süresi", "Tijd met Caps Lock",
        "Czas z Caps Lock", "Thời gian Caps Lock bật", "เวลาที่ Caps Lock เปิด",
        "Waktu Caps Lock aktif",
    ]),
    ("stat_num_time", [
        "Time Num on", "Temps Num activé", "Tiempo con Bloq NUM", "数字锁定开启时长",
        "Время с Num Lock", "Num Lock オン時間", "Num Lock 켜진 시간", "Zeit mit Num Lock",
        "Tempo con Bloc Num", "Tempo com Num Lock", "زمن تفعيل الحروف الرقمية",
        "नम लॉक चालू समय", "Num Lock açık kalma süresi", "Tijd met Num Lock",
        "Czas z Num Lock", "Thời gian Num Lock bật", "เวลาที่ Num Lock เปิด",
        "Waktu Num Lock aktif",
    ]),
    ("stat_since", [
        "Tracking for", "Suivi depuis", "Seguimiento desde", "统计时长", "Ведётся",
        "計測期間", "추적 기간", "Erfassung seit", "Rilevamento da", "Registo desde",
        "مدة التتبع", "ट्रैकिंग समय", "İzleme süresi", "Bijhouden sinds", "Czas śledzenia",
        "Thời gian theo dõi", "ระยะเวลาติดตาม", "Durasi pelacakan",
    ]),
    ("stat_local", [
        "100% local · nothing is sent", "100 % local · aucune donnée envoyée",
        "100 % local · no se envía nada", "100% 本地 · 不发送任何数据",
        "100% локально · данные не отправляются", "100% ローカル · データは送信されません",
        "100% 로컬 · 데이터를 전송하지 않습니다", "100 % lokal · es wird nichts gesendet",
        "100% locale · nessun dato inviato", "100% local · nada é enviado",
        "100% محلي · لا يتم إرسال أي بيانات", "100% स्थानीय · कुछ भी नहीं भेजा जाता",
        "%100 yerel · hiçbir şey gönderilmez", "100% lokaal · er wordt niets verzonden",
        "100% lokalnie · nic nie jest wysyłane", "100% cục bộ · không gửi dữ liệu",
        "100% ในเครื่อง · ไม่มีการส่งข้อมูล", "100% lokal · tidak ada data yang dikirim",
    ]),
    ("stat_reset", [
        "Reset statistics", "Réinitialiser les stats", "Restablecer estadísticas", "重置统计",
        "Сбросить статистику", "統計をリセット", "통계 초기화", "Statistik zurücksetzen",
        "Reimposta statistiche", "Redefinir estatísticas", "إعادة تعيين الإحصائيات",
        "आंकड़े रीसेट करें", "İstatistikleri sıfırla", "Statistieken wissen", "Zresetuj statystyki",
        "Đặt lại thống kê", "รีเซ็ตสถิติ", "Atur ulang statistik",
    ]),
    ("stat_reset_confirm", [
        "Reset all statistics?", "Réinitialiser toutes les stats ?", "¿Restablecer todas las estadísticas?",
        "重置所有统计？", "Сбросить всю статистику?", "すべての統計をリセットしますか？",
        "모든 통계를 초기화할까요?", "Alle Statistiken zurücksetzen?", "Reimpostare tutte le statistiche?",
        "Redefinir todas as estatísticas?", "إعادة تعيين كل الإحصائيات؟", "सभी आंकड़े रीसेट करें?",
        "Tüm istatistikler sıfırlansın mı?", "Alle statistieken wissen?", "Zresetować wszystkie statystyki?",
        "Đặt lại tất cả thống kê?", "รีเซ็ตสถิติทั้งหมดหรือไม่?", "Atur ulang semua statistik?",
    ]),
    ("stat_reset_yes", [
        "Yes", "Oui", "Sí", "是", "Да", "はい", "예", "Ja", "Sì",
        "Sim", "نعم", "हाँ", "Evet", "Ja", "Tak", "Có", "ใช่", "Ya",
    ]),
    ("stat_reset_no", [
        "No", "Non", "No", "否", "Нет", "いいえ", "아니오", "Nein", "No",
        "Não", "لا", "नहीं", "Hayır", "Nee", "Nie", "Không", "ไม่", "Tidak",
    ]),

    // Hotkeys
    ("hotkey_title", [
        "Panel shortcut", "Raccourci du panneau", "Atajo del panel", "面板快捷键",
        "Горячая клавиша панели", "パネルのショートカット", "패널 단축키", "Panel-Tastenkürzel",
        "Scorciatoia del pannello", "Atalho do painel", "اختصار اللوحة", "पैनल शॉर्टकट",
        "Panel kısayolu", "Paneelsneltoets", "Skrót panelu", "Phím tắt bảng điều khiển",
        "ปุ่มลัดแผงควบคุม", "Pintasan panel",
    ]),
    ("hotkey_current", [
        "Current shortcut", "Raccourci actuel", "Atajo actual", "当前快捷键",
        "Текущая клавиша", "現在のショートカット", "현재 단축키", "Aktuelles Tastenkürzel",
        "Scorciatoia attuale", "Atalho atual", "الاختصار الحالي", "वर्तमान शॉर्टकट",
        "Geçerli kısayol", "Huidige sneltoets", "Aktualny skrót", "Phím tắt hiện tại",
        "ปุ่มลัดปัจจุบัน", "Pintasan saat ini",
    ]),
    ("hotkey_change", [
        "Change shortcut", "Modifier le raccourci", "Cambiar atajo", "更改快捷键",
        "Изменить клавишу", "ショートカットを変更", "단축키 변경", "Tastenkürzel ändern",
        "Modifica scorciatoia", "Alterar atalho", "تغيير الاختصار", "शॉर्टकट बदलें",
        "Kısayolu değiştir", "Sneltoets wijzigen", "Zmień skrót", "Đổi phím tắt",
        "เปลี่ยนปุ่มลัด", "Ubah pintasan",
    ]),
    ("hotkey_hint1", [
        "Use Ctrl, Alt, Shift or Win", "Utilisez Ctrl, Alt, Maj ou Win",
        "Usa Ctrl, Alt, Mayús o Win", "使用 Ctrl、Alt、Shift 或 Win",
        "Используйте Ctrl, Alt, Shift или Win", "Ctrl、Alt、Shift、Win を使用",
        "Ctrl, Alt, Shift 또는 Win 사용", "Strg, Alt, Umschalt oder Win verwenden",
        "Usa Ctrl, Alt, Maiusc o Win", "Use Ctrl, Alt, Shift ou Win",
        "استخدم Ctrl أو Alt أو Shift أو Win", "Ctrl, Alt, Shift या Win का उपयोग करें",
        "Ctrl, Alt, Shift veya Win kullanın", "Gebruik Ctrl, Alt, Shift of Win",
        "Użyj Ctrl, Alt, Shift lub Win", "Dùng Ctrl, Alt, Shift hoặc Win",
        "ใช้ Ctrl, Alt, Shift หรือ Win", "Gunakan Ctrl, Alt, Shift atau Win",
    ]),
    ("hotkey_hint2", [
        "plus another key.", "plus une autre touche.", "más otra tecla.", "再加另一个键。",
        "и ещё одну клавишу.", "にもう1つのキーを。", "다른 키 하나를 더.",
        "plus eine weitere Taste.", "più un altro tasto.", "mais outra tecla.",
        "ثم مفتاحًا آخر.", "और एक कुंजी.", "ardından başka bir tuş.",
        "plus één andere toets.", "oraz kolejny klawisz.", "rồi thêm một phím khác.",
        "และอีกหนึ่งปุ่ม.", "ditambah satu tombol lagi.",
    ]),
    ("hotkey_wait", [
        "Press a combination…", "Appuyez sur une combinaison…", "Pulsa una combinación…",
        "请按下组合键…", "Нажмите сочетание…", "組み合わせを押してください…",
        "조합을 누르세요…", "Kombination drücken…", "Premi una combinazione…",
        "Prima uma combinação…", "اضغط على تركيبة…", "एक संयोजन दबाएँ…",
        "Bir birleşim tuşu basın…", "Druk op een combinatie…", "Naciśnij kombinację…",
        "Nhấn một tổ hợp phím…", "กดชุดปุ่ม…", "Tekan kombinasi…",
    ]),
    ("hotkey_cancel", [
        "Esc to cancel", "Échap pour annuler", "Esc para cancelar", "按 Esc 取消",
        "Esc — отмена", "Esc でキャンセル", "Esc로 취소", "Esc zum Abbrechen",
        "Esc per annullare", "Esc para cancelar", "Esc للإلغاء", "रद्द करने के लिए Esc",
        "İptal için Esc", "Esc om te annuleren", "Esc, aby anulować", "Esc để hủy",
        "Esc เพื่อยกเลิก", "Esc untuk membatalkan",
    ]),

    // Display
    ("display_title", [
        "Display options", "Options d'affichage", "Opciones de pantalla", "显示选项",
        "Параметры отображения", "表示オプション", "표시 옵션", "Anzeigeoptionen",
        "Opzioni di visualizzazione", "Opções de exibição", "خيارات العرض", "प्रदर्शन विकल्प",
        "Görüntü seçenekleri", "Weergaveopties", "Opcje wyświetlania", "Tùy chọn hiển thị",
        "ตัวเลือกการแสดงผล", "Opsi tampilan",
    ]),
    ("display_duration", [
        "Notification duration", "Durée d'affichage", "Duración de la notificación", "通知时长",
        "Длительность уведомления", "通知の表示時間", "알림 표시 시간", "Benachrichtigungsdauer",
        "Durata della notifica", "Duração da notificação", "مدة الإشعار", "सूचना की अवधि",
        "Bildirim süresi", "Duur van melding", "Czas wyświetlania powiadomienia",
        "Thời gian thông báo", "ระยะเวลาการแจ้งเตือน", "Durasi notifikasi",
    ]),
    ("duration_1", [
        "0.8s", "0,8 s", "0,8 s", "0,8 秒", "0,8 с", "0.8秒", "0.8초", "0,8 s", "0,8 s",
        "0,8 s", "٠٫٨ ث", "0.8 से", "0,8 sn", "0,8 s", "0,8 s", "0,8 giây", "0.8 วิ", "0,8 dk",
    ]),
    ("duration_2", [
        "1.5s", "1,5 s", "1,5 s", "1,5 秒", "1,5 с", "1.5秒", "1.5초", "1,5 s", "1,5 s",
        "1,5 s", "١٫٥ ث", "1.5 से", "1,5 sn", "1,5 s", "1,5 s", "1,5 giây", "1.5 วิ", "1,5 dk",
    ]),
    ("duration_3", [
        "3s", "3 s", "3 s", "3 秒", "3 с", "3秒", "3초", "3 s", "3 s", "3 s", "٣ ث",
        "3 से", "3 sn", "3 s", "3 s", "3 giây", "3 วิ", "3 dk",
    ]),
    ("duration_4", [
        "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞", "∞",
        "∞", "∞",
    ]),
    ("panel_opacity", [
        "Panel opacity", "Opacité du panneau", "Opacidad del panel", "面板不透明度",
        "Непрозрачность панели", "パネルの不透明度", "패널 불투명도", "Deckkraft des Panels",
        "Opacità del pannello", "Opacidade do painel", "عتامة اللوحة", "पैनल की अपारदर्शिता",
        "Panel opaklığı", "Deckkraft van paneel", "Nieprzezroczystość panelu", "Độ mờ của bảng",
        "ความทึบของแผง", "Opasitas panel",
    ]),
    ("overlay_opacity", [
        "Indicator opacity", "Opacité de l'indicateur", "Opacidad del indicador", "指示器不透明度",
        "Непрозрачность индикатора", "インジケーターの不透明度", "표시기 불투명도",
        "Deckkraft der Anzeige", "Opacità dell'indicatore", "Opacidade do indicador",
        "عتامة المؤشر", "संकेतक की अपारदर्शिता", "Göstergenin opaklığı", "Deckkraft van indicator",
        "Nieprzezroczystość wskaźnika", "Độ mờ của chỉ báo", "ความทึบของตัวบ่งชี้",
        "Opasitas indikator",
    ]),
    ("position_title", [
        "Indicator position", "Position de l'indicateur", "Posición del indicador", "指示器位置",
        "Положение индикатора", "インジケーターの位置", "표시기 위치", "Position der Anzeige",
        "Posizione dell'indicatore", "Posição do indicador", "موضع المؤشر", "संकेतक की स्थिति",
        "Gösterge konumu", "Positie van indicator", "Położenie wskaźnika", "Vị trí chỉ báo",
        "ตำแหน่งตัวบ่งชี้", "Posisi indikator",
    ]),
    ("position_button", [
        "Choose position", "Choisir la position", "Elegir posición", "选择位置",
        "Выбрать положение", "位置を選択", "위치 선택", "Position wählen", "Scegli posizione",
        "Escolher posição", "اختيار الموضع", "स्थिति चुनें", "Konum seç", "Positie kiezen",
        "Wybierz położenie", "Chọn vị trí", "เลือกตำแหน่ง", "Pilih posisi",
    ]),
    ("sound_volume", [
        "Sound volume", "Volume du son", "Volumen del sonido", "音量",
        "Громкость звука", "サウンド音量", "소리 볼륨", "Lautstärke",
        "Volume del suono", "Volume do som", "مستوى الصوت", "ध्वनि की मात्रा",
        "Ses düzeyi", "Geluidsvolume", "Głośność", "Âm lượng",
        "ระดับเสียง", "Volume suara",
    ]),
    ("position_drag", [
        "Drag the indicator, then release", "Glissez l'indicateur puis relâchez",
        "Arrastra el indicador y suelta", "拖动指示器后松开", "Перетащите индикатор и отпустите",
        "インジケーターをドラッグして離す", "표시기를 끌어다 놓으세요",
        "Indikator ziehen und loslassen", "Trascina l'indicatore e rilascia",
        "Arraste o indicador e solte", "اسحب المؤشر ثم أفلت", "संकेतक को खींचें फिर छोड़ें",
        "Göstergeyi sürükleyip bırakın", "Sleep de indicator en laat los",
        "Przeciągnij wskaźnik i puść", "Kéo chỉ báo rồi thả", "ลากตัวบ่งชี้แล้วปล่อย",
        "Geser indikator lalu lepas",
    ]),

    // Common
    ("quit", [
        "Quit IndiCaps", "Quitter IndiCaps", "Salir de IndiCaps", "退出 IndiCaps",
        "Выйти из IndiCaps", "IndiCaps を終了", "IndiCaps 종료", "IndiCaps beenden",
        "Esci da IndiCaps", "Sair do IndiCaps", "إنهاء IndiCaps", "IndiCaps बंद करें",
        "IndiCaps'ten çık", "IndiCaps afsluiten", "Zamknij IndiCaps", "Thoát IndiCaps",
        "ออกจาก IndiCaps", "Keluar dari IndiCaps",
    ]),
    ("menu_open", [
        "Open settings", "Ouvrir les réglages", "Abrir ajustes", "打开设置",
        "Открыть настройки", "設定を開く", "설정 열기", "Einstellungen öffnen",
        "Apri impostazioni", "Abrir configurações", "فتح الإعدادات", "सेटिंग्स खोलें",
        "Ayarları aç", "Instellingen openen", "Otwórz ustawienia", "Mở cài đặt",
        "เปิดการตั้งค่า", "Buka pengaturan",
    ]),
    ("menu_quit", [
        "Quit", "Quitter", "Salir", "退出", "Выход", "終了", "종료", "Beenden", "Esci", "Sair",
        "خروج", "बंद करें", "Çıkış", "Afsluiten", "Zamknij", "Thoát", "ออก", "Keluar",
    ]),

    // Key names used by the shortcut label
    ("key_ctrl", [
        "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Strg", "Ctrl", "Ctrl", "Ctrl",
        "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Ctrl", "Ctrl",
    ]),
    ("key_shift", [
        "Shift", "Maj", "Mayús", "Shift", "Shift", "Shift", "Shift", "Umschalt", "Maiusc", "Shift",
        "Shift", "Shift", "Shift", "Shift", "Shift", "Shift", "Shift", "Shift",
    ]),
}
