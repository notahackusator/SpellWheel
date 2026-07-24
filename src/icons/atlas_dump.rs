use std::fs::{read_dir, remove_file, File};
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use image::ColorType::Rgba8;
use image_dds::ddsfile::Dds;
use image_dds::SurfaceRgba8;
use lazy_static::lazy_static;
use crate::icons::AtlasIcon;
use crate::paths;

pub fn delete_previous_dumps() {
    let Ok(read_dir) = read_dir(paths::spellwheel()) else {
        return;
    };
    for file in read_dir {
        let Ok(file) = file else {
            continue;
        };
        let file_name = file.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.ends_with(".png") || name.ends_with(".txt") {
            let _ = remove_file(file.path());
        }
    }
}

pub fn dump_dds(atlas_name: &str, dds: &Dds) {
    let f = File::create(paths::spellwheel().join(atlas_name).with_extension("dds"))
        .expect("Couldn't create icon DDS data dump file");
    let mut f = BufWriter::new(f);

    writeln!(f, "header width={}", dds.header.width).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header height={}", dds.header.height).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header pitch={:?}", dds.header.pitch).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header linear_size={:?}", dds.header.linear_size).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header depth={:?}", dds.header.depth).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header mip_map_count={:?}", dds.header.mip_map_count).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header spf={:?}", dds.header.spf).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header caps={:?}", dds.header.caps).expect("Can't write to icon DDS data dump file");
    writeln!(f, "header caps2={:?}", dds.header.caps2).expect("Can't write to icon DDS data dump file");

    writeln!(f).expect("Can't write to icon DDS data dump file");

    writeln!(f, "header10 ok?={:?}", dds.header10.is_some()).expect("Can't write to icon DDS data dump file");
    if let Some(header10) = &dds.header10 {
        writeln!(f, "header10 dxgi_format={:?}", header10.dxgi_format).expect("Can't write to icon DDS data dump file");
        writeln!(f, "header10 resource_dimension={:?}", header10.resource_dimension).expect("Can't write to icon DDS data dump file");
        writeln!(f, "header10 misc_flag={:?}", header10.misc_flag).expect("Can't write to icon DDS data dump file");
        writeln!(f, "header10 array_size={}", header10.array_size).expect("Can't write to icon DDS data dump file");
        writeln!(f, "header10 alpha_mode={:?}", header10.alpha_mode).expect("Can't write to icon DDS data dump file");
    }

    writeln!(f).expect("Can't write to icon DDS data dump file");

    writeln!(f).expect("Data:");

    f.write_all(&dds.data).expect("Can't write to icon DDS data dump file");
}

pub enum DumpedAtlas<'a> {
    Dxgi(&'a Dds),
    Rgba(&'a SurfaceRgba8<Vec<u8>>),
}

pub fn dump_atlas(atlas_name: &str, dumped_atlas: DumpedAtlas) {
    let icon;

    let (data, width, height) = match dumped_atlas {
        DumpedAtlas::Dxgi(dds) => {
            let surface = match image_dds::Surface::from_dds(&dds) {
                Ok(surface) => surface,
                Err(err) => {
                    tracing::error!("Error dumping atlas {atlas_name}: {err}");
                    return;
                }
            };
            icon = match surface.decode_rgba8() {
                Ok(icon) => icon,
                Err(err) => {
                    tracing::error!("Error dumping atlas {atlas_name}: {err}");
                    return;
                }
            };
            (icon.data.as_slice(), icon.width, icon.height)
        }
        DumpedAtlas::Rgba(rgba) => (rgba.data.as_slice(), rgba.width, rgba.height)
    };

    let mut path = paths::spellwheel().join(atlas_name).with_extension("png");

    let mut i = 1;
    while path.exists() {
        path = paths::spellwheel().join(format!("{atlas_name} ({i})")).with_extension("png");
        i += 1;
    }

    if let Err(err) = image::save_buffer(path, data, width, height, Rgba8) {
        tracing::error!("Error dumping atlas {atlas_name}: {err}");
    }
}

lazy_static!(
    pub static ref ICON_DATA: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
);

pub fn upload_icon_data(item_id: u16, icon: &AtlasIcon) {
    ICON_DATA.lock().unwrap().push(format!("{item_id}: {} @ {:?}", icon.atlas_name, icon.original_rect));
}

pub fn dump_icon_data() {
    let f = File::create(paths::spellwheel().join("atlases.txt")).expect("Couldn't create icon data dump file");
    let mut f = BufWriter::new(f);

    for icon_data in ICON_DATA.lock().unwrap().iter() {
        writeln!(f, "{}", icon_data).expect("Can't write to icon data dump file");
    }
}