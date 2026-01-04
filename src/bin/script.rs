use std::fs;
use std::path::{Path, PathBuf};
use clap::{Parser, ValueEnum};
use walkdir::WalkDir;
use image::{DynamicImage, GenericImageView, imageops};
use exif::{Reader, Tag};
use rayon::prelude::*;

const INSTAGRAM_WIDTH: u32 = 1080;
const INSTAGRAM_HORIZONTAL_HEIGHT: u32 = 566;
const INSTAGRAM_VERTICAL_HEIGHT: u32 = 1350;

#[derive(Parser, Debug)]
#[command(author, version, about = "Crop/resize images for Instagram (auto/vertical/horizontal)")]
struct Args {
    #[arg(short, long)]
    in_dir: PathBuf,

    #[arg(short, long)]
    out_dir: PathBuf,

    #[arg(short, long, value_enum, default_value_t = Mode::Auto)]
    mode: Mode,

    #[arg(long, default_value = "keep")]
    format: String,

    #[arg(long, default_value_t = 100)]
    quality: u8,

    #[arg(long, default_value_t = 0)]
    threads: usize,
}

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    Auto,
    Vertical,
    Horizontal,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if !args.in_dir.exists() {
        anyhow::bail!("El directorio de entrada no existe: {}", args.in_dir.display());
    }
    if !args.in_dir.is_dir() {
        anyhow::bail!("La ruta de entrada no es un directorio: {}", args.in_dir.display());
    }

    if args.threads > 0 {
        rayon::ThreadPoolBuilder::new().num_threads(args.threads).build_global()?;
    }

    fs::create_dir_all(&args.out_dir)?;

    let walker = WalkDir::new(&args.in_dir).into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect::<Vec<_>>();

    walker.into_par_iter().for_each(|entry| {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if matches!(ext.to_str(), Some("jpg" | "jpeg" | "png" | "webp" | "bmp" | "tiff")) {
                if let Err(e) = process_image(path, &args) {
                    eprintln!("Error procesando {}: {}", path.display(), e);
                }
            }
        }
    });

    Ok(())
}

fn process_image(path: &Path, args: &Args) -> anyhow::Result<()> {
    let img_orig = image::open(path)?;

    let img = match fix_orientation(path, &img_orig) {
        Ok(i) => i,
        Err(_) => img_orig,
    };

    let (target_w, target_h) = match args.mode {
        Mode::Auto => {
            if img.width() > img.height() {
                (INSTAGRAM_WIDTH, INSTAGRAM_HORIZONTAL_HEIGHT)
            } else {
                (INSTAGRAM_WIDTH, INSTAGRAM_VERTICAL_HEIGHT)
            }
        }
        Mode::Horizontal => (INSTAGRAM_WIDTH, INSTAGRAM_HORIZONTAL_HEIGHT),
        Mode::Vertical => (INSTAGRAM_WIDTH, INSTAGRAM_VERTICAL_HEIGHT),
    };

    let target_aspect = target_w as f32 / target_h as f32;

    let img_cropped = crop_to_aspect_center(&img, target_aspect);
    let (cw, ch) = img_cropped.dimensions();

    let (final_w, final_h) = if cw < target_w || ch < target_h {
        (cw, ch)
    } else {
        (target_w, target_h)
    };

    let final_img = img_cropped.resize_exact(final_w, final_h, imageops::FilterType::Lanczos3);

    let mut out_path = args.out_dir.clone();
    out_path.push(path.file_name().unwrap());

    match args.format.as_str() {
        "jpeg" | "jpg" => {
            out_path.set_extension("jpg");
            final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::Jpeg)?;
        }
        "png" => {
            out_path.set_extension("png");
            final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::Png)?;
        }
        "webp" => {
            out_path.set_extension("webp");
            final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::WebP)?;
        }
        "keep" => {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                match ext.to_lowercase().as_str() {
                    "jpg" | "jpeg" => {
                        final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::Jpeg)?;
                    }
                    "png" => {
                        final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::Png)?;
                    }
                    "webp" => {
                        final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::WebP)?;
                    }
                    _ => {
                        out_path.set_extension("jpg");
                        final_img.write_to(&mut fs::File::create(&out_path)?, image::ImageFormat::Jpeg)?;
                    }
                }
            }
        }
        _ => {
            anyhow::bail!("Formato no soportado: {}", args.format);
        }
    }

    Ok(())
}

fn crop_to_aspect_center(img: &DynamicImage, target_aspect: f32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let img_aspect = w as f32 / h as f32;

    if (img_aspect - target_aspect).abs() < 1e-6 {
        return img.clone();
    }

    if img_aspect > target_aspect {
        let new_w = (target_aspect * h as f32).round() as u32;
        let x0 = (w - new_w) / 2;
        img.crop_imm(x0, 0, new_w, h)
    } else {
        let new_h = (w as f32 / target_aspect).round() as u32;
        let y0 = (h - new_h) / 2;
        img.crop_imm(0, y0, w, new_h)
    }
}

fn fix_orientation(path: &Path, img: &DynamicImage) -> anyhow::Result<DynamicImage> {
    let file = fs::File::open(path)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;

    if let Some(orientation) = exif.get_field(Tag::Orientation, exif::In::PRIMARY) {
        if let Some(orientation_value) = orientation.value.get_uint(0) {
            match orientation_value {
                1 => return Ok(img.clone()), // Normal
                _ => {}
            }
        }
    }

    Ok(img.clone())
}
