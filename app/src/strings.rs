//! UI text in the five languages of the site and the camera, chosen from the
//! system language. The text is `locales/app.yml`, which the app's root
//! carries as nsh translations (`nsh_core::i18n`): nsh's generated roots
//! compile it in, and [`translations`] gives it to the roots nsh does not
//! generate.

use nsh_core::{i18n::Translations, prelude::*};

/// The language tag for a system locale: one of the five the app and the
/// camera serve. An explicit script wins over the region, so `zh-Hans-HK` stays
/// Simplified.
pub fn language_tag(locale: &str) -> &'static str {
    let tag = locale.replace('_', "-").to_ascii_lowercase();
    let mut parts = tag.split('-');
    match parts.next().unwrap_or("") {
        "zh" => {
            let subtags: Vec<&str> = parts.collect();
            if subtags.contains(&"hans") {
                "zh-Hans"
            } else if subtags
                .iter()
                .any(|part| matches!(*part, "hant" | "tw" | "hk" | "mo"))
            {
                "zh-Hant"
            } else {
                "zh-Hans"
            }
        }
        "ja" => "ja",
        "ko" => "ko",
        _ => "en",
    }
}

/// The app's current language tag, for UI copy and camera requests.
pub fn app_language() -> &'static str {
    language_tag(&locale().to_string())
}

/// The text of `key` in [`app_language`], with `%{name}` placeholders filled
/// from `args`. On Android, `android.<key>` replaces `key` where the locales
/// have it.
pub fn tr(key: &str, args: &[(&str, &str)]) -> String {
    let translations = global::<Translations>().get();
    let language = app_language();
    let android_key = cfg!(target_os = "android").then(|| format!("android.{key}"));
    let key = match &android_key {
        Some(android_key) if translations.lookup(language, android_key).is_some() => android_key,
        _ => key,
    };
    let args: Vec<(&str, String)> = args
        .iter()
        .map(|&(name, value)| (name, value.to_owned()))
        .collect();
    translations.translate(language, key, &args)
}

/// `locales/app.yml` as nsh's generated roots compile it in (`build.rs`
/// writes it with nsh's reader), for the roots nsh does not generate: the
/// desktop `cargo run` and the tests.
pub fn translations() -> Translations {
    Translations::from_json(TRANSLATIONS_JSON).expect("locales/app.yml")
}

const TRANSLATIONS_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/translations.json"));

/// Format bytes as a short size label for errors (`12 MB`).
pub fn format_bytes(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if bytes >= MB {
        format!("{} MB", (bytes + MB / 2) / MB)
    } else if bytes >= 1024 {
        format!("{} KB", (bytes + 512) / 1024)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use nsh_core::platform::{Locale, Os};
    use nsh_engine::{EngineConfig, Root};
    use nsh_testing::Headless;

    use super::*;

    /// The tags of `locales/app.yml`: those `nsh.json` lists.
    const LANGUAGES: [&str; 5] = ["en", "zh-Hans", "zh-Hant", "ja", "ko"];
    /// The Simplified Chinese text counts the total, the English the rest.
    const OWN_ZH_HANS_PLACEHOLDER: &str = "looks.use_more";

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split("%{")
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn every_key_has_five_languages_and_the_english_placeholders() {
        // `locales/app.yml` as nsh reads it: keys and text by language.
        let locales: BTreeMap<String, BTreeMap<String, String>> =
            serde_json::from_str(TRANSLATIONS_JSON).unwrap();
        let mut languages = LANGUAGES.to_vec();
        languages.sort();
        assert_eq!(locales.keys().collect::<Vec<_>>(), languages);
        let english = &locales["en"];
        assert!(english.len() > 600);
        for language in LANGUAGES {
            let texts = &locales[language];
            assert_eq!(
                texts.keys().collect::<Vec<_>>(),
                english.keys().collect::<Vec<_>>(),
                "{language}"
            );
            for (key, text) in texts {
                if key == OWN_ZH_HANS_PLACEHOLDER && language == "zh-Hans" {
                    continue;
                }
                assert_eq!(
                    placeholders(text),
                    placeholders(&english[key]),
                    "{key}: {language}"
                );
            }
        }
    }

    #[test]
    fn system_locales_map_to_the_five_tags() {
        for (locale, tag) in [
            ("en-US", "en"),
            ("fr-FR", "en"),
            ("zh-Hans-CN", "zh-Hans"),
            ("zh-CN", "zh-Hans"),
            ("zh-Hans-HK", "zh-Hans"),
            ("zh-Hant-HK", "zh-Hant"),
            ("zh_TW", "zh-Hant"),
            ("zh-HK", "zh-Hant"),
            ("zh-MO", "zh-Hant"),
            ("ja-JP", "ja"),
            ("ko-KR", "ko"),
        ] {
            assert_eq!(language_tag(locale), tag, "{locale}");
        }
    }

    #[component]
    fn Texts() -> impl IntoElement {
        view()
            .child(text(tr("tab.camera", &[])))
            .child(text(tr("update.title", &[("model", "GR IV")])))
    }

    fn shown(locale: &str) -> Headless {
        let mut config = EngineConfig::new(Root::of::<Texts>().translations(translations()));
        config.environment.os = Os::Web;
        config.environment.locale = Locale::parse(locale).unwrap();
        let mut headless = Headless::start(config).unwrap();
        headless.first_render().unwrap();
        headless
    }

    #[test]
    fn the_root_translations_show_in_the_system_language() {
        for (locale, camera, update) in [
            ("en-US", "Camera", "Updating GR IV"),
            ("zh-Hans-CN", "相机", "正在更新 GR IV"),
            ("zh-HK", "相機", "正在更新 GR IV"),
            ("ja-JP", "カメラ", "GR IV をアップデート中"),
            ("fr-FR", "Camera", "Updating GR IV"),
        ] {
            let headless = shown(locale);
            assert!(headless.shows(camera), "{locale}: {camera}");
            assert!(headless.shows(update), "{locale}: {update}");
        }
    }
}
