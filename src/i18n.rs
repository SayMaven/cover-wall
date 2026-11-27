//! Internationalization (English, Indonesian, Simplified Chinese) with system locale detection.

use serde::{Deserialize, Serialize};

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    En,
    Id,
    Zh,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::En, Self::Id, Self::Zh];

    pub const fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Id => "Bahasa Indonesia",
            Self::Zh => "简体中文",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Id => "ID",
            Self::Zh => "中文",
        }
    }

    /// Detect the OS UI language, defaulting to English.
    pub fn detect_system() -> Self {
        let Some(locale) = sys_locale::get_locale() else {
            return Self::En;
        };
        let lower = locale.to_ascii_lowercase();
        if lower.starts_with("id") {
            Self::Id
        } else if lower.starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }
}

/// Localized UI strings for the active language.
#[derive(Debug, Clone, Copy)]
pub struct Strings {
    pub app_title: &'static str,
    pub tab_studio: &'static str,
    pub tab_extractor: &'static str,
    pub status_exporting: &'static str,
    pub status_extracting: &'static str,
    pub status_loading: &'static str,
    pub dismiss: &'static str,

    // Extractor view
    pub extract_heading: &'static str,
    pub extract_subtitle: &'static str,
    pub music_folder: &'static str,
    pub output_covers_folder: &'static str,
    pub browse: &'static str,
    pub start_extraction: &'static str,
    pub extracting: &'static str,
    pub cancel: &'static str,
    pub open_in_studio: &'static str,
    pub extracted_label: &'static str,
    pub missing_label: &'static str,
    pub extract_empty_hint: &'static str,
    pub err_pick_source: &'static str,

    // Studio view
    pub covers_source: &'static str,
    pub load: &'static str,
    pub all: &'static str,
    pub none: &'static str,
    pub shuffle: &'static str,
    pub new_seed: &'static str,
    pub seed: &'static str,
    pub resolution: &'static str,
    pub layout_model: &'static str,
    pub parameters: &'static str,
    pub columns: &'static str,
    pub gap_px: &'static str,
    pub angle_deg: &'static str,
    pub radius_px: &'static str,
    pub shadow: &'static str,
    pub background: &'static str,
    pub bg_solid: &'static str,
    pub bg_blurred: &'static str,
    pub bg_transparent: &'static str,
    pub tint_opacity: &'static str,
    pub export_section: &'static str,
    pub prefix: &'static str,
    pub export_4k_png: &'static str,
    pub exporting: &'static str,
    pub export_both: &'static str,
    pub hide_cover_library: &'static str,
    pub show_cover_library: &'static str,
    pub cover_strip_hint: &'static str,
    pub loading_covers: &'static str,
    pub no_covers_loaded: &'static str,
    pub no_covers_hint: &'static str,
    pub browse_covers_folder: &'static str,
    pub go_to_extract: &'static str,
    pub live_preview: &'static str,
    pub select_one_to_preview: &'static str,
    pub err_select_cover_export: &'static str,
}

impl Language {
    pub const fn strings(self) -> Strings {
        match self {
            Self::En => Strings {
                app_title: "Cover Wall Studio",
                tab_studio: "Wallpaper Studio",
                tab_extractor: "Extract Covers",
                status_exporting: "Exporting 4K...",
                status_extracting: "Extracting FLACs...",
                status_loading: "Loading covers...",
                dismiss: "Dismiss",

                extract_heading: "Extract Album Covers from FLACs",
                extract_subtitle: "Scans FLAC tags and embedded artwork directly in Rust without FFmpeg.",
                music_folder: "Music Folder (FLAC)",
                output_covers_folder: "Output Covers Folder",
                browse: "Browse...",
                start_extraction: "Start Extraction",
                extracting: "Extracting...",
                cancel: "Cancel",
                open_in_studio: "Open Output in Wallpaper Studio",
                extracted_label: "Extracted",
                missing_label: "Missing",
                extract_empty_hint: "Select a music folder above to scan for unique album covers.",
                err_pick_source: "Please select a source music folder first.",

                covers_source: "COVERS SOURCE",
                load: "Load",
                all: "All",
                none: "None",
                shuffle: "Shuffle",
                new_seed: "New Seed",
                seed: "Seed:",
                resolution: "RESOLUTION",
                layout_model: "LAYOUT MODEL",
                parameters: "PARAMETERS",
                columns: "Columns",
                gap_px: "Gap (px)",
                angle_deg: "Angle (°)",
                radius_px: "Radius (px)",
                shadow: "Shadow",
                background: "BACKGROUND",
                bg_solid: "Solid",
                bg_blurred: "Blurred",
                bg_transparent: "Transparent",
                tint_opacity: "Tint Opacity",
                export_section: "EXPORT",
                prefix: "Prefix:",
                export_4k_png: "Export 4K PNG",
                exporting: "Exporting...",
                export_both: "Export Both 16:9 & 9:16",
                hide_cover_library: "Hide Cover Library",
                show_cover_library: "Show Cover Library",
                cover_strip_hint: "Click any cover to include or exclude it from the wallpaper.",
                loading_covers: "Loading covers in parallel...",
                no_covers_loaded: "No Covers Loaded",
                no_covers_hint: "Select an existing covers folder or extract artwork from your FLAC library.",
                browse_covers_folder: "Browse Covers Folder...",
                go_to_extract: "Go to Extract Covers",
                live_preview: "Live Preview",
                select_one_to_preview: "Select at least one cover below to preview.",
                err_select_cover_export: "Select at least one cover before exporting.",
            },
            Self::Id => Strings {
                app_title: "Cover Wall Studio",
                tab_studio: "Studio Wallpaper",
                tab_extractor: "Ekstrak Cover",
                status_exporting: "Mengekspor 4K...",
                status_extracting: "Mengekstrak FLAC...",
                status_loading: "Memuat cover...",
                dismiss: "Tutup",

                extract_heading: "Ekstrak Cover Album dari File FLAC",
                extract_subtitle: "Memindai tag FLAC dan artwork bawaan langsung di Rust tanpa FFmpeg.",
                music_folder: "Folder Musik (FLAC)",
                output_covers_folder: "Folder Output Cover",
                browse: "Pilih...",
                start_extraction: "Mulai Ekstraksi",
                extracting: "Mengekstrak...",
                cancel: "Batal",
                open_in_studio: "Buka Output di Studio Wallpaper",
                extracted_label: "Terekstrak",
                missing_label: "Tidak Ada",
                extract_empty_hint: "Pilih folder musik di atas untuk memindai cover album unik.",
                err_pick_source: "Silakan pilih folder sumber musik terlebih dahulu.",

                covers_source: "SUMBER COVER",
                load: "Muat",
                all: "Semua",
                none: "Kosongkan",
                shuffle: "Acak",
                new_seed: "Seed Baru",
                seed: "Seed:",
                resolution: "RESOLUSI",
                layout_model: "MODEL LAYOUT",
                parameters: "PARAMETER",
                columns: "Kolom",
                gap_px: "Celah (px)",
                angle_deg: "Sudut (°)",
                radius_px: "Radius (px)",
                shadow: "Bayangan",
                background: "LATAR BELAKANG",
                bg_solid: "Solid",
                bg_blurred: "Blur",
                bg_transparent: "Transparan",
                tint_opacity: "Opasitas Tint",
                export_section: "EKSPOR",
                prefix: "Awalan:",
                export_4k_png: "Ekspor 4K PNG",
                exporting: "Mengekspor...",
                export_both: "Ekspor 16:9 & 9:16 Sekaligus",
                hide_cover_library: "Sembunyikan Galeri Cover",
                show_cover_library: "Tampilkan Galeri Cover",
                cover_strip_hint: "Klik cover mana saja untuk menyertakan atau mengecualikannya dari wallpaper.",
                loading_covers: "Memuat cover secara paralel...",
                no_covers_loaded: "Belum Ada Cover Dimuat",
                no_covers_hint: "Pilih folder cover yang sudah ada atau ekstrak artwork dari koleksi FLAC Anda.",
                browse_covers_folder: "Pilih Folder Cover...",
                go_to_extract: "Buka Ekstrak Cover",
                live_preview: "Pratinjau Langsung",
                select_one_to_preview: "Pilih minimal satu cover di bawah untuk melihat pratinjau.",
                err_select_cover_export: "Pilih minimal satu cover sebelum mengekspor.",
            },
            Self::Zh => Strings {
                app_title: "Cover Wall Studio",
                tab_studio: "壁纸工作室",
                tab_extractor: "提取封面",
                status_exporting: "正在导出 4K...",
                status_extracting: "正在提取 FLAC...",
                status_loading: "正在加载封面...",
                dismiss: "关闭",

                extract_heading: "从 FLAC 文件提取专辑封面",
                extract_subtitle: "使用纯 Rust 直接读取 FLAC 标签与内嵌封面，无需 FFmpeg。",
                music_folder: "音乐目录 (FLAC)",
                output_covers_folder: "封面输出目录",
                browse: "浏览...",
                start_extraction: "开始提取",
                extracting: "提取中...",
                cancel: "取消",
                open_in_studio: "在壁纸工作室中打开",
                extracted_label: "已提取",
                missing_label: "缺失",
                extract_empty_hint: "请在上方选择音乐目录以扫描唯一专辑封面。",
                err_pick_source: "请先选择音乐源目录。",

                covers_source: "封面来源",
                load: "加载",
                all: "全选",
                none: "清空",
                shuffle: "随机",
                new_seed: "新种子",
                seed: "种子:",
                resolution: "分辨率",
                layout_model: "排版模式",
                parameters: "参数设置",
                columns: "列数",
                gap_px: "间距 (px)",
                angle_deg: "角度 (°)",
                radius_px: "圆角 (px)",
                shadow: "阴影",
                background: "背景设置",
                bg_solid: "纯色",
                bg_blurred: "模糊",
                bg_transparent: "透明",
                tint_opacity: "遮罩透明度",
                export_section: "导出",
                prefix: "前缀:",
                export_4k_png: "导出 4K PNG",
                exporting: "导出中...",
                export_both: "同时导出 16:9 与 9:16",
                hide_cover_library: "隐藏封面库",
                show_cover_library: "显示封面库",
                cover_strip_hint: "点击任意封面即可将其包含或排除在壁纸之外。",
                loading_covers: "正在并行加载封面...",
                no_covers_loaded: "尚未加载封面",
                no_covers_hint: "请选择已有封面目录，或从 FLAC 音乐库提取封面。",
                browse_covers_folder: "选择封面目录...",
                go_to_extract: "前往提取封面",
                live_preview: "实时预览",
                select_one_to_preview: "请在下方至少选择一个封面以进行预览。",
                err_select_cover_export: "导出前请至少选择一个封面。",
            },
        }
    }
}
