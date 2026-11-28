//! The demo camera's modules. This is the only sample catalog in the app: a
//! real camera always describes its own modules.

use serde_json::json;

use crate::domain::LookError;
use crate::domain::modules::{
    CatalogRead, ModuleCatalog, ModulePatch, ModuleState, ModulesState, parse_module_catalog,
    parse_modules_state,
};

fn values(labels: &[&str]) -> serde_json::Value {
    labels
        .iter()
        .enumerate()
        .map(|(value, label)| json!({"value": value, "label": label}))
        .collect()
}

fn setting(key: &str, group: Option<&str>, label: &str, labels: &[&str]) -> serde_json::Value {
    json!({"key": key, "group": group, "label": label, "values": values(labels)})
}

fn page(id: &str, title: &str, settings: Vec<serde_json::Value>) -> serde_json::Value {
    json!({"id": id, "title": title, "managed": null, "settings": settings, "options": null})
}

fn with_options(mut module: serde_json::Value, key: &str, ordered: bool) -> serde_json::Value {
    module["options"] = json!({"setting": key, "ordered": ordered});
    module
}

fn managed(id: &str, title: &str, kind: &str) -> serde_json::Value {
    json!({"id": id, "title": title, "managed": kind, "settings": [], "options": null})
}

const ICON: &[&str] = &["Off", "On"];

/// The demo's words in Simplified Chinese. A real camera sends its own
/// translations in all five languages.
const DEMO_ZH: &[(&str, &str)] = &[
    ("Recipes", "配方"),
    ("Film Look", "胶片风格"),
    ("Crop", "裁切"),
    ("Focal Length", "焦距"),
    ("Mode", "模式"),
    ("Off", "关闭"),
    ("On", "开启"),
    ("Frame Only", "仅辅助构图"),
    ("Framing", "画幅"),
    ("Ratio", "画幅比例"),
    ("Mask", "遮罩"),
    ("Dark", "暗"),
    ("Black", "黑色"),
    ("Icon", "图标显示"),
    ("Guides", "辅助线"),
    ("Style", "样式"),
    ("Thirds", "三分线"),
    ("Grid", "网格"),
    ("Spiral", "螺旋"),
    ("Color", "颜色"),
    ("White", "白色"),
    ("Red", "红色"),
    ("Amber", "琥珀色"),
    ("Date Imprint", "加印日期"),
    ("Classic", "经典"),
    ("Compact", "紧凑"),
    ("Border", "边框"),
    ("Thin", "细"),
    ("Wide", "宽"),
    ("Glow", "光晕"),
    ("Bloom", "柔光"),
    ("Halation", "胶片光晕"),
    ("Level", "强度"),
    ("Strength", "强度"),
    ("Low", "弱"),
    ("Medium", "中"),
    ("High", "强"),
    ("Max", "特强"),
    ("Grain", "颗粒感"),
    ("Size", "大小"),
    ("Auto", "自动"),
    ("Small", "小"),
    ("Large", "大"),
];

/// Replaces the demo's display words, and no keys or IDs.
fn translate(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => items.iter_mut().for_each(translate),
        serde_json::Value::Object(fields) => {
            for (key, field) in fields.iter_mut() {
                match (key.as_str(), field.as_str()) {
                    ("title" | "label" | "group", Some(word)) => {
                        if let Some((_, zh)) = DEMO_ZH.iter().find(|(en, _)| *en == word) {
                            *field = json!(zh);
                        }
                    }
                    _ => translate(field),
                }
            }
        }
        _ => {}
    }
}

pub fn demo_catalog(language: &str) -> ModuleCatalog {
    let mut body = json!({
        "schema": 1,
        "catalog": "0123456789abcdef",
        "language": "en",
        "modules": [
            managed("RECIPES", "Recipes", "recipes"),
            managed("FILM", "Film Look", "looks"),
            with_options(page("CROP", "Crop", vec![
                setting("crop.focal", None, "Focal Length", &["Off", "35mm", "50mm", "75mm"]),
                setting("crop.mode", None, "Mode", &["Crop", "Frame Only"]),
            ]), "crop.focal", false),
            with_options(page("FRAMING", "Framing", vec![
                setting("framing.ratio", None, "Ratio",
                    &["Off", "1:1", "4:3", "16:9", "7:6", "XPan", "2:1", "2.39:1", "5:4"]),
                setting("framing.mode", None, "Mode", &["Crop", "Frame Only"]),
                setting("framing.mask", None, "Mask", &["Dark", "Black"]),
                setting("framing.icon", None, "Icon", ICON),
            ]), "framing.ratio", true),
            with_options(page("GUIDES", "Guides", vec![
                setting("guides.style", None, "Style", &["Off", "Thirds", "Grid", "Spiral"]),
                setting("guides.color", None, "Color", &["White", "Red"]),
            ]), "guides.style", true),
            with_options(page("DATE", "Date Imprint", vec![
                setting("date.style", None, "Style", &["Off", "Classic", "Compact"]),
                setting("date.icon", None, "Icon", ICON),
            ]), "date.style", true),
            with_options(page("BORDER", "Border", vec![
                setting("border.style", None, "Style", &["Off", "Thin", "Wide"]),
                setting("border.color", None, "Color", &["White", "Black"]),
            ]), "border.style", false),
            page("GLOW", "Glow", vec![
                setting("glow.bloom", Some("Bloom"), "Level", &["Off", "Low", "High"]),
                setting("glow.bloom.icon", Some("Bloom"), "Icon", ICON),
                setting("glow.halation", Some("Halation"), "Level", &["Off", "Low", "High"]),
                setting("glow.halation.color", Some("Halation"), "Color", &["Red", "Amber"]),
            ]),
            page("GRAIN", "Grain", vec![
                setting("grain.level", None, "Strength", &["Off", "Low", "Medium", "High", "Max"]),
                setting("grain.size", None, "Size", &["Auto", "Small", "Large"]),
                setting("grain.icon", None, "Icon", ICON),
            ]),
            page("BROKEN.GRF", "BROKEN.GRF", Vec::new()),
        ],
    });
    if language == "zh-Hans" {
        body["language"] = json!(language);
        translate(&mut body["modules"]);
    }
    match parse_module_catalog(body.to_string().as_bytes()) {
        Ok(CatalogRead::Ready(catalog)) => catalog,
        _ => ModuleCatalog::default(),
    }
}

fn entry(
    id: &str,
    place: &str,
    orderable: bool,
    values: serde_json::Value,
    hidden: &[i64],
    order: &[i64],
) -> serde_json::Value {
    json!({"id": id, "enabled": true, "status": "running", "place": place,
           "orderable": orderable, "placeable": true, "values": values, "hiddenValues": hidden,
           "valueOrder": order})
}

pub fn demo_state() -> ModulesState {
    let mut failed = entry("BROKEN.GRF", "hidden", false, json!({}), &[], &[]);
    failed["status"] = json!("failed");
    failed["placeable"] = json!(false);
    failed["enabled"] = json!(false);
    let body = json!({
        "revision": 1,
        "toolbarMax": 4,
        "grkitEnabled": true,
        "features": {"looks": true, "recipes": true},
        "modules": [
            entry("RECIPES", "toolbar", false, json!({}), &[], &[]),
            entry("FILM", "toolbar", false, json!({}), &[], &[]),
            entry("CROP", "root", true,
                json!({"crop.focal": 0, "crop.mode": 0}), &[3], &[0, 1, 2, 3]),
            entry("FRAMING", "root", true,
                json!({"framing.ratio": 3, "framing.mode": 0, "framing.mask": 0, "framing.icon": 1}),
                &[7, 8], &[0, 3, 1, 2, 4, 5, 6, 7, 8]),
            entry("GUIDES", "root", true,
                json!({"guides.style": 1, "guides.color": 0}), &[], &[0, 1, 2, 3]),
            entry("BORDER", "root", true,
                json!({"border.style": 0, "border.color": 0}), &[], &[0, 1, 2]),
            entry("GLOW", "root", true, json!({
                "glow.bloom": 1, "glow.bloom.icon": 1,
                "glow.halation": 0, "glow.halation.color": 0
            }), &[], &[]),
            entry("GRAIN", "root", true,
                json!({"grain.level": 2, "grain.size": 0, "grain.icon": 1}), &[], &[]),
            entry("DATE", "hidden", false,
                json!({"date.style": 0, "date.icon": 1}), &[], &[0, 1, 2]),
            failed,
        ],
    });
    parse_modules_state(body.to_string().as_bytes()).unwrap_or_default()
}

fn module_mut<'a>(state: &'a mut ModulesState, id: &str) -> Result<&'a mut ModuleState, LookError> {
    state
        .modules
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or(LookError::NotFound)
}

/// Change one module as the camera would. The answer is the module and the
/// new revision.
pub fn patch(
    state: &mut ModulesState,
    id: &str,
    patch: &ModulePatch,
) -> Result<(ModuleState, u64), LookError> {
    let module = module_mut(state, id)?;
    if let Some(on) = patch.enabled {
        module.enabled = on;
        module.status = if on { "running" } else { "stopped" }.into();
    }
    for (key, value) in &patch.values {
        module.values.insert(key.clone(), *value);
    }
    if let Some(hidden) = &patch.hidden_values {
        module.hidden_values = hidden.clone();
    }
    if let Some(order) = &patch.value_order {
        module.value_order = order.clone();
    }
    let module = module.clone();
    if let (Some(on), Some(features)) = (patch.enabled, &mut state.features) {
        match id {
            "FILM" => features.looks = on,
            "RECIPES" => features.recipes = on,
            _ => {}
        }
    }
    state.revision += 1;
    Ok((module, state.revision))
}

/// The icon tile that the camera shows for a demo module: the module's own
/// tile from the repository, as the camera serves it.
pub fn demo_icon(id: &str) -> Option<&'static [u8]> {
    Some(match id {
        "RECIPES" => include_bytes!("../../../modules/recipes/tiles/0-module.png"),
        "FILM" => include_bytes!("../../../modules/film-look/tiles/0-module.png"),
        "CROP" => include_bytes!("../../../modules/crop/tiles/0-module.png"),
        "FRAMING" => include_bytes!("../../../modules/framing/tiles/0-module.png"),
        "GUIDES" => include_bytes!("../../../modules/guides/tiles/0-module.png"),
        "DATE" => include_bytes!("../../../modules/date/tiles/0-module.png"),
        "BORDER" => include_bytes!("../../../modules/border/tiles/0-module.png"),
        "GLOW" => include_bytes!("../../../modules/glow/tiles/0-module.png"),
        "GRAIN" => include_bytes!("../../../modules/grain/tiles/0-module.png"),
        _ => return None,
    })
}
