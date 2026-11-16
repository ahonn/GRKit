//! Interprets an XMP tree as Camera Raw develop settings.
//!
//! Top-level `rdf:Description` settings stay on the preset. A nested `crs:Look` keeps its own
//! settings (and optional `crs:Parameters`) without merging into the preset.

use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;

use super::xml::{self, Element};

/// One develop setting: a scalar, a sequence, or a nested map (rare outside Look).
#[derive(Clone, Debug, PartialEq)]
pub enum Setting {
    Text(String),
    Seq(Vec<String>),
    Map(BTreeMap<String, Setting>),
}

/// Nested Camera Raw Look block: its amount, name, and its own develop settings.
#[derive(Clone, Debug, Default)]
pub struct Look {
    pub name: Option<String>,
    /// Plain factor from `crs:Amount` when present.
    pub amount: Option<f64>,
    pub settings: BTreeMap<String, Setting>,
}

impl Look {
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.settings.get(key) {
            Some(Setting::Text(v)) => Some(v.as_str()),
            _ => None,
        }
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        parse_finite(self.text(key)?)
    }

    pub fn seq(&self, key: &str) -> Option<&[String]> {
        match self.settings.get(key) {
            Some(Setting::Seq(v)) => Some(v.as_slice()),
            _ => None,
        }
    }

    pub fn bool_flag(&self, key: &str) -> bool {
        matches!(
            self.text(key).map(|v| v.to_ascii_lowercase()).as_deref(),
            Some("true") | Some("1")
        )
    }
}

/// Parsed Camera Raw settings, an optional Look, and embedded big-table payloads.
#[derive(Clone, Debug, Default)]
pub struct Preset {
    pub name: Option<String>,
    pub settings: BTreeMap<String, Setting>,
    pub look: Option<Look>,
    /// `crs:Table_<fingerprint>` payloads from any scope, still base-85 encoded.
    pub tables: BTreeMap<String, String>,
}

impl Preset {
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.settings.get(key) {
            Some(Setting::Text(v)) => Some(v.as_str()),
            _ => None,
        }
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        parse_finite(self.text(key)?)
    }

    pub fn seq(&self, key: &str) -> Option<&[String]> {
        match self.settings.get(key) {
            Some(Setting::Seq(v)) => Some(v.as_slice()),
            _ => None,
        }
    }

    pub fn bool_flag(&self, key: &str) -> bool {
        matches!(
            self.text(key).map(|v| v.to_ascii_lowercase()).as_deref(),
            Some("true") | Some("1")
        )
    }
}

/// True when the bytes look like a standalone XMP packet (not an image that embeds one).
///
/// Skips an optional UTF-8 BOM and leading white space; the rest must start with `<`, and the
/// first 64 KiB must contain `x:xmpmeta` or `adobe:ns:meta` (case-sensitive, no copy). Camera Raw
/// `crs:` settings are checked later in `parse`.
pub fn looks_like(bytes: &[u8]) -> bool {
    let mut i = 0;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        i = 3;
    }
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'<') {
        return false;
    }
    let end = bytes.len().min(i + 64 * 1024);
    let window = &bytes[i..end];
    contains_bytes(window, b"x:xmpmeta") || contains_bytes(window, b"adobe:ns:meta")
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Parses a finite f64; rejects NaN and infinities so they do not reach the LUT.
fn parse_finite(text: &str) -> Option<f64> {
    text.parse().ok().filter(|v: &f64| v.is_finite())
}

/// Parses Camera Raw develop settings from an XMP packet.
pub fn parse(bytes: &[u8]) -> Result<Preset> {
    ensure!(
        looks_like(bytes),
        "Expected a Lightroom or Camera Raw XMP preset"
    );
    let text = std::str::from_utf8(bytes).context("Expected UTF-8 XMP")?;
    let root = xml::parse_document(text)?;
    let mut preset = Preset::default();

    // Top-level descriptions only: walk the tree but skip anything under crs:Look.
    collect_top_level(&root, &mut preset)?;
    // Look blocks separately.
    for el in root.walk() {
        if el.name == "crs:Look" {
            ensure!(
                preset.look.is_none(),
                "Expected at most one crs:Look in the preset"
            );
            preset.look = Some(parse_look(el, &mut preset.tables)?);
        }
    }

    if preset.settings.is_empty() && preset.look.is_none() {
        bail!("Expected Camera Raw develop settings (crs:) in the XMP preset");
    }
    if let Some(Setting::Text(name)) = preset.settings.get("Name").cloned() {
        preset.name = Some(name);
    }
    drop_base_profile(&mut preset);
    Ok(preset)
}

/// An Adobe profile (`Adobe Color`, `Adobe Landscape`, ...) is the base rendering of the raw
/// converter, not a part of the look: the file names its table without the data, and its tone
/// curve belongs to that rendering. The preset keeps only whether the profile is monochrome.
fn drop_base_profile(preset: &mut Preset) {
    let base = preset
        .look
        .as_ref()
        .is_some_and(|look| look.name.as_deref().is_some_and(is_base_profile));
    if !base {
        return;
    }
    let look = preset.look.take().unwrap();
    if look.bool_flag("ConvertToGrayscale") && !preset.settings.contains_key("ConvertToGrayscale") {
        preset
            .settings
            .insert("ConvertToGrayscale".into(), Setting::Text("True".into()));
    }
}

fn is_base_profile(name: &str) -> bool {
    name.starts_with("Adobe ")
}

fn collect_top_level(root: &Element, preset: &mut Preset) -> Result<()> {
    // Prefer rdf:Description that are not descendants of crs:Look.
    fn visit(el: &Element, under_look: bool, preset: &mut Preset) -> Result<()> {
        let under_look = under_look || el.name == "crs:Look";
        if el.name == "rdf:Description" && !under_look {
            ingest_description(el, &mut preset.settings, &mut preset.tables)?;
        }
        for child in &el.children {
            if let xml::Node::Element(child) = child {
                visit(child, under_look, preset)?;
            }
        }
        Ok(())
    }
    visit(root, false, preset)
}

fn parse_look(el: &Element, tables: &mut BTreeMap<String, String>) -> Result<Look> {
    let mut look = Look::default();
    // Attributes on crs:Look itself.
    take_crs_attrs(el, &mut look.settings, tables);
    // Nested rdf:Description (common) and crs:Parameters.
    for child in el.walk() {
        if child.name == "rdf:Description" || child.name == "crs:Parameters" {
            ingest_description(child, &mut look.settings, tables)?;
        }
        // Child crs: elements hanging directly under Look.
        if let Some(key) = child.name.strip_prefix("crs:") {
            if key != "Parameters" && key != "Look" {
                put_crs_element(child, key, &mut look.settings, tables)?;
            }
        }
    }
    if let Some(Setting::Text(name)) = look.settings.remove("Name") {
        look.name = Some(name);
    }
    if let Some(Setting::Text(amount)) = look.settings.remove("Amount") {
        look.amount = amount.parse().ok();
    } else if let Some(v) = el.attr("crs:Amount").and_then(|s| s.parse().ok()) {
        look.amount = Some(v);
    }
    if look.name.is_none() {
        if let Some(n) = el.attr("crs:Name") {
            look.name = Some(n.to_owned());
        }
    }
    Ok(look)
}

fn ingest_description(
    el: &Element,
    into: &mut BTreeMap<String, Setting>,
    tables: &mut BTreeMap<String, String>,
) -> Result<()> {
    take_crs_attrs(el, into, tables);
    for child in &el.children {
        let xml::Node::Element(child) = child else {
            continue;
        };
        if let Some(key) = child.name.strip_prefix("crs:") {
            if key == "Look" {
                // Nested Look is handled by the top-level walk.
                continue;
            }
            put_crs_element(child, key, into, tables)?;
        }
    }
    Ok(())
}

fn take_crs_attrs(
    el: &Element,
    into: &mut BTreeMap<String, Setting>,
    tables: &mut BTreeMap<String, String>,
) {
    for (key, value) in &el.attrs {
        if let Some(fp) = key.strip_prefix("crs:Table_") {
            tables.insert(fp.to_owned(), value.clone());
        } else if let Some(rest) = key.strip_prefix("crs:") {
            into.insert(rest.to_owned(), Setting::Text(value.clone()));
        } else if let Some(fp) = key.strip_prefix("Table_") {
            // Some writers omit the crs: prefix on Table_ attributes.
            tables.insert(fp.to_owned(), value.clone());
        }
    }
}

fn put_crs_element(
    el: &Element,
    key: &str,
    into: &mut BTreeMap<String, Setting>,
    tables: &mut BTreeMap<String, String>,
) -> Result<()> {
    if let Some(fp) = key.strip_prefix("Table_") {
        let text = el.text_content();
        tables.insert(fp.to_owned(), text.trim().to_owned());
        return Ok(());
    }
    if el.child_named("rdf:Seq").is_some() || el.child_named("rdf:seq").is_some() {
        let seq = read_seq(el)?;
        into.insert(key.to_owned(), Setting::Seq(seq));
        return Ok(());
    }
    if el.child_named("rdf:Description").is_some() {
        let mut map = BTreeMap::new();
        for desc in el.children_named("rdf:Description") {
            ingest_description(desc, &mut map, tables)?;
        }
        take_crs_attrs(el, &mut map, tables);
        into.insert(key.to_owned(), Setting::Map(map));
        return Ok(());
    }
    let text = el.text_content().trim().to_owned();
    into.insert(key.to_owned(), Setting::Text(text));
    Ok(())
}

fn read_seq(el: &Element) -> Result<Vec<String>> {
    let mut items = Vec::new();
    for seq in el
        .children_named("rdf:Seq")
        .chain(el.children_named("rdf:seq"))
    {
        for li in seq.children_named("rdf:li") {
            items.push(li.text_content().trim().to_owned());
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_settings_do_not_overwrite_preset() {
        let xmp = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:Saturation="+10" crs:ConvertToGrayscale="False">
   <crs:Look>
    <rdf:Description crs:Name="Portrait" crs:Amount="1"
     crs:ConvertToGrayscale="True" crs:ProcessVersion="11.0">
     <crs:ToneCurvePV2012>
      <rdf:Seq><rdf:li>0, 0</rdf:li><rdf:li>255, 255</rdf:li></rdf:Seq>
     </crs:ToneCurvePV2012>
    </rdf:Description>
   </crs:Look>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;
        let preset = parse(xmp.as_bytes()).unwrap();
        assert_eq!(preset.text("Saturation"), Some("+10"));
        assert_eq!(preset.text("ConvertToGrayscale"), Some("False"));
        assert!(preset.seq("ToneCurvePV2012").is_none());
        let look = preset.look.as_ref().unwrap();
        assert_eq!(look.name.as_deref(), Some("Portrait"));
        assert_eq!(look.amount, Some(1.0));
        assert!(look.bool_flag("ConvertToGrayscale"));
        assert_eq!(look.seq("ToneCurvePV2012").unwrap(), ["0, 0", "255, 255"]);
    }
}
