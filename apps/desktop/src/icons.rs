use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconId {
    Plus,
    Search,
    Info,
    X,
    ChevronDown,
    ChevronRight,
    ArrowLeft,
    Pin,
    Archive,
    Trash,
    Eye,
    Pencil,
    Bold,
    Italic,
    Code,
    List,
    ListOrdered,
    ListChecks,
    Quote,
    Link,
    Heading,
    TriangleAlert,
    PanelLeft,
    FileText,
    Calendar,
    Tag,
}

#[derive(Clone, Copy, Debug)]
pub struct IconAsset {
    pub id: IconId,
    pub file: &'static str,
    pub lucide_name: &'static str,
    pub sha256: &'static str,
    pub feather_derived: bool,
    pub bytes: &'static [u8],
}

pub const ALL_ICONS: &[IconAsset] = &[
    IconAsset {
        id: IconId::Plus,
        file: "plus.svg",
        lucide_name: "plus",
        sha256: "7f6af73bf1ff6c4bca3f18351c8d1bdec6749c0c2530c4de5da85d520c21df17",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/plus.svg"),
    },
    IconAsset {
        id: IconId::Search,
        file: "search.svg",
        lucide_name: "search",
        sha256: "283d371c2e433817bb9c0c8310caa6c77fa4177c0f4f1168d9c83b97af7389dc",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/search.svg"),
    },
    IconAsset {
        id: IconId::Info,
        file: "info.svg",
        lucide_name: "info",
        sha256: "bc977a64eb96f3e9c1041ffd09a3fceb70e3e65c02b571f76064062ed31f3cb9",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/info.svg"),
    },
    IconAsset {
        id: IconId::X,
        file: "x.svg",
        lucide_name: "x",
        sha256: "4a9cdab38fbb96162e7dace28e33f4ca0e49d8963a6162abc3d4691b7d675117",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/x.svg"),
    },
    IconAsset {
        id: IconId::ChevronDown,
        file: "chevron-down.svg",
        lucide_name: "chevron-down",
        sha256: "66ea878e72ed3488bb3b464c39dfdccee8d1f78e560dccea40e5e12da0e87e87",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/chevron-down.svg"),
    },
    IconAsset {
        id: IconId::ChevronRight,
        file: "chevron-right.svg",
        lucide_name: "chevron-right",
        sha256: "2758143d7b2434e4aa7307dfd34405c87909ff4052f21b5f3f40d45224b4f19b",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/chevron-right.svg"),
    },
    IconAsset {
        id: IconId::ArrowLeft,
        file: "arrow-left.svg",
        lucide_name: "arrow-left",
        sha256: "c5f4b88076108eb1a0d95336e1aa7e7bb1419a3a92af96f6247d88cd3456f933",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/arrow-left.svg"),
    },
    IconAsset {
        id: IconId::Pin,
        file: "pin.svg",
        lucide_name: "pin",
        sha256: "55e8e5e04bcb671179fc1dd589abf09cf6a1e71301ff14eb739419cfab6011e7",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/pin.svg"),
    },
    IconAsset {
        id: IconId::Archive,
        file: "archive.svg",
        lucide_name: "archive",
        sha256: "78c732e34e232db63af06a2330b4448045cfdf49b48b59f2d31922df54c7bb17",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/archive.svg"),
    },
    IconAsset {
        id: IconId::Trash,
        file: "trash.svg",
        lucide_name: "trash",
        sha256: "27299f69ad7c6272be64b1b8e2d48cbd6dcf0ef0d4f92827a1affa945c91700e",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/trash.svg"),
    },
    IconAsset {
        id: IconId::Eye,
        file: "eye.svg",
        lucide_name: "eye",
        sha256: "5bf90197dd7629cad64a2e48d1186a71559deb6121207d10e3dc5b19ebaffdcf",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/eye.svg"),
    },
    IconAsset {
        id: IconId::Pencil,
        file: "pencil.svg",
        lucide_name: "pencil",
        sha256: "7e1ca7a6f5c1eb949671df762f2baadd32f5bd841d43153c3a15279af7d78d0c",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/pencil.svg"),
    },
    IconAsset {
        id: IconId::Bold,
        file: "bold.svg",
        lucide_name: "bold",
        sha256: "79217276da6c19176d70060a93c792b2b30aae18a95fb0690c263a71851aed96",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/bold.svg"),
    },
    IconAsset {
        id: IconId::Italic,
        file: "italic.svg",
        lucide_name: "italic",
        sha256: "dcce4f9719febb8b04b030133c1d4ad82f68d13932ac806fbee620422000ac83",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/italic.svg"),
    },
    IconAsset {
        id: IconId::Code,
        file: "code.svg",
        lucide_name: "code",
        sha256: "83235d4a0352d74234f949c9fc23f89c65ff165f1b6d54a90e5f0ea5bfc2c3d7",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/code.svg"),
    },
    IconAsset {
        id: IconId::List,
        file: "list.svg",
        lucide_name: "list",
        sha256: "ff97a7379eb962ddc3863bb59f0f45875ac8da91aac8a38a67361e7118d2d627",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/list.svg"),
    },
    IconAsset {
        id: IconId::ListOrdered,
        file: "list-ordered.svg",
        lucide_name: "list-ordered",
        sha256: "df549e10677b86c9a9f07c0c696b40a4937019212e73bc982a4e8dae7c557d8f",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/list-ordered.svg"),
    },
    IconAsset {
        id: IconId::ListChecks,
        file: "list-checks.svg",
        lucide_name: "list-checks",
        sha256: "08d26ab05aaf724222c4c0cd5a19d5eae753b2a5fff0eb2d81759dde11bac6b6",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/list-checks.svg"),
    },
    IconAsset {
        id: IconId::Quote,
        file: "quote.svg",
        lucide_name: "quote",
        sha256: "3c7a2f8443ae5c88cb3f35fdcc27a037a1af13fe35be28eb8cabded8672a9ac1",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/quote.svg"),
    },
    IconAsset {
        id: IconId::Link,
        file: "link.svg",
        lucide_name: "link",
        sha256: "4f3237183dcaabfac92f4e94c776a9a9e45c3c5e77a921061a6b34b61eb5413f",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/link.svg"),
    },
    IconAsset {
        id: IconId::Heading,
        file: "heading.svg",
        lucide_name: "heading",
        sha256: "496295f2ac974c816a61701b5abb5f15b35657e6b573ae1755ebb27541f1e97c",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/heading.svg"),
    },
    IconAsset {
        id: IconId::TriangleAlert,
        file: "triangle-alert.svg",
        lucide_name: "triangle-alert",
        sha256: "4866f38b8560d410f21e3226413e0b77997b6dfbb6931fadfe0a0d5aef9ffeb4",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/triangle-alert.svg"),
    },
    IconAsset {
        id: IconId::PanelLeft,
        file: "panel-left.svg",
        lucide_name: "panel-left",
        sha256: "029f67b9e3eeb7ddf7c7c499b174c50388725e8ea50d4ea59ed7dbaf8d61cb82",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/panel-left.svg"),
    },
    IconAsset {
        id: IconId::FileText,
        file: "file-text.svg",
        lucide_name: "file-text",
        sha256: "52ca55846c202335fb870c8aa23e9345630ec10489303756d62e750f7109605c",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/file-text.svg"),
    },
    IconAsset {
        id: IconId::Calendar,
        file: "calendar.svg",
        lucide_name: "calendar",
        sha256: "312ac8e715ac2b71bcc942dec2e8de9c1929085474f08885af35b563f6aa7a0b",
        feather_derived: true,
        bytes: include_bytes!("../assets/icons/lucide/calendar.svg"),
    },
    IconAsset {
        id: IconId::Tag,
        file: "tag.svg",
        lucide_name: "tag",
        sha256: "85e11c07b29f5b67410e0f14c84799d9406320cd4575b7c8322f31910e0582cc",
        feather_derived: false,
        bytes: include_bytes!("../assets/icons/lucide/tag.svg"),
    },
];

pub fn icon_asset(id: IconId) -> &'static IconAsset {
    ALL_ICONS
        .iter()
        .find(|a| a.id == id)
        .expect("catalog covers every IconId")
}

pub fn icon_bytes(id: IconId) -> &'static [u8] {
    icon_asset(id).bytes
}

fn icon_image(id: IconId, color: egui::Color32, size: f32) -> egui::Image<'static> {
    let asset = icon_asset(id);
    let hex = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    let svg = String::from_utf8_lossy(asset.bytes)
        .replace("currentColor", &hex)
        .replace("stroke-width=\"2\"", "stroke-width=\"1.75\"");
    let uri = format!(
        "bytes://lucide/{}-{:02x}{:02x}{:02x}.svg",
        asset.lucide_name,
        color.r(),
        color.g(),
        color.b()
    );
    egui::Image::from_bytes(uri, svg.into_bytes()).fit_to_exact_size(egui::vec2(size, size))
}

pub fn icon_button(
    ui: &mut egui::Ui,
    icon: IconId,
    accessible_name: &str,
    tooltip: &str,
) -> egui::Response {
    ensure_image_loaders(ui.ctx());
    let color = ui.visuals().text_color();
    let response = ui.add_sized(
        [38.0, 38.0],
        egui::Button::image(icon_image(icon, color, 17.0)),
    );
    let response = response.on_hover_text(tooltip);
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, accessible_name));
    response
}

fn ensure_image_loaders(ctx: &egui::Context) {
    let id = egui::Id::new("crane-lucide-loaders");
    let done = ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    if !done {
        egui_extras::install_image_loaders(ctx);
        ctx.data_mut(|d| d.insert_temp(id, true));
    }
}
