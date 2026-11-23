//! Generate a local release fixture signed only by the public test key.
//! The runtime and modules are fixtures, not executable camera software.

use std::{collections::BTreeMap, error::Error, fs, path::Path};

use grkit_core::{
    Files, camera, grm, packaging, release,
    release_index::{Index, SiteContents},
    testing,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn read_tree(root: &Path) -> Result<Files> {
    fn visit(root: &Path, directory: &Path, files: &mut Files) -> Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() {
                visit(root, &path, files)?;
            } else if kind.is_file() {
                let relative = path.strip_prefix(root)?.to_str().ok_or("Non-UTF-8 path")?;
                files.insert(relative.replace('\\', "/"), fs::read(&path)?);
            } else {
                return Err(format!("Unsupported source file: {}", path.display()).into());
            }
        }
        Ok(())
    }
    let mut files = Files::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

fn fixture(source: &Path, camera: &'static camera::Camera) -> Result<(Index, Files)> {
    let runtimes = testing::release_runtimes(camera, "1.7.0");
    let modules = testing::official_modules();
    let statement = release::Statement::of_release(&runtimes, &modules, camera)?.to_bytes()?;
    let signature = release::SigningKey::from_hex(testing::RELEASE_SEED)?.sign(&statement);
    release::verify(&statement, &signature, &[testing::release_key()])?;
    if release::verify(&statement, &signature, &release::OFFICIAL_KEYS).is_ok() {
        return Err("The fixture key must not be an official release key".into());
    }
    let mut site = SiteContents::default();
    let mut files = Files::new();
    files.insert(
        format!("release/{}/RELEASE.JSON", camera.id),
        statement.clone(),
    );
    files.insert(
        format!("release/{}/RELEASE.SIG", camera.id),
        signature.clone(),
    );
    site.releases
        .insert(camera.id.to_owned(), (statement, signature));
    for runtime in runtimes {
        let target = runtime.target().to_owned();
        let manifest = testing::manifest(&target, runtime.files());
        files.insert(format!("runtime/{target}/build.json"), manifest.clone());
        for (path, bytes) in runtime.files() {
            files.insert(format!("runtime/{target}/sd/{path}"), bytes.clone());
        }
        let effects = read_tree(&source.join("effects").join(&target))?;
        for (path, bytes) in &effects {
            files.insert(format!("effects/{target}/{path}"), bytes.clone());
        }
        site.manifests.insert(target.clone(), manifest);
        site.runtimes.insert(target.clone(), runtime);
        site.effects.insert(target, effects);
    }
    for bytes in modules {
        let name = format!("{}.GRM", grm::Module::decode(&bytes)?.id);
        files.insert(format!("modules/{name}"), bytes.clone());
        site.modules.insert(name, bytes);
    }
    let mut guides = BTreeMap::new();
    for (path, bytes) in read_tree(&source.join("guides").join(camera.id))? {
        if let Some(language) = path
            .strip_suffix(".pdf")
            .filter(|value| !value.contains('/'))
        {
            files.insert(format!("guides/{}/{path}", camera.id), bytes.clone());
            guides.insert(language.to_owned(), bytes);
        }
    }
    site.guides.insert(camera.id.to_owned(), guides);
    let index = Index::of_site(&site)?;
    // Materialize only files actually covered by the index.
    files.retain(|path, _| index.files.contains_key(path));
    for (path, info) in &index.files {
        let bytes = files.get(path).ok_or("Indexed file is missing")?;
        if bytes.len() as u64 != info.size || packaging::sha256(bytes) != info.sha256 {
            return Err(format!("Indexed file differs: {path}").into());
        }
    }
    files.insert("release/index.json".to_owned(), index.to_bytes()?);
    let public_key: String = testing::release_key()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    files.insert(
        "test-public-key.hex".to_owned(),
        format!("{public_key}\n").into_bytes(),
    );
    Ok((index, files))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut source = None;
    let mut output = None;
    let mut camera_id = "gr3".to_owned();
    while let Some(argument) = args.next() {
        let value = args.next().ok_or("Expected a value after the option")?;
        match argument.as_str() {
            "--source" => source = Some(value),
            "--out" => output = Some(value),
            "--camera" => camera_id = value,
            _ => return Err(format!("Unknown option: {argument}").into()),
        }
    }
    let source =
        source.ok_or("Usage: mock_release --source SITE --out DIRECTORY [--camera gr3]")?;
    let output = output.ok_or("Missing --out")?;
    let destination = Path::new(&output);
    if destination.exists() && fs::read_dir(destination)?.next().is_some() {
        return Err("Output directory must be empty; existing files are never replaced".into());
    }
    let (index, files) = fixture(Path::new(&source), camera::find(&camera_id)?)?;
    for (path, bytes) in files {
        let path = destination.join(path);
        fs::create_dir_all(path.parent().ok_or("Missing parent directory")?)?;
        fs::write(path, bytes)?;
    }
    println!(
        "Generated test-signed GRKit {} for {}: {}",
        index.version,
        camera_id,
        destination.display()
    );
    println!(
        "Fixture binaries must not be installed on a camera. Official camera trust rejects this key."
    );
    Ok(())
}
