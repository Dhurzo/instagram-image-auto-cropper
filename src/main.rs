use std::fs;
use std::path::{Path, PathBuf};
use clap::{Parser, ValueEnum};
use walkdir::WalkDir;
use image::{DynamicImage, GenericImageView, ImageOutputFormat, imageops};
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
        .filter(|e| {
            if let Some(ext) = e.path().extension().and_then(|s| s.to_str()) {
                let ext_l = ext.to_lowercase();
                matches!(ext_l.as_str(), "jpg" | "jpeg" | "png" | "webp" | "tiff" | "bmp")
            } else { false }
        })
        .collect::<Vec<_>>();

    println!("Procesando {} archivos...", walker.len());

    walker.par_iter().for_each(|entry| {
        if let Err(e) = process_file(entry.path(), &args) {
            eprintln!("Error procesando {}: {}", entry.path().display(), e);
        }
    });

    println!("Listo. Archivos procesados en: {}", args.out_dir.display());
    Ok(())
}

fn process_file(path: &Path, args: &Args) -> anyhow::Result<()> {
    let img_orig = image::open(path)?;

    let img = match fix_orientation(path, &img_orig) {
        Ok(i) => i,
        Err(_) => img_orig,
    };

    let (w, h) = img.dimensions();

    let mode = match args.mode {
        Mode::Auto => {
            if w >= h { Mode::Horizontal } else { Mode::Vertical }
        }
        ref other => other.clone(),
    };

    let (target_w, target_h) = match mode {
        Mode::Horizontal => (INSTAGRAM_WIDTH, INSTAGRAM_HORIZONTAL_HEIGHT),
        Mode::Vertical => (INSTAGRAM_WIDTH, INSTAGRAM_VERTICAL_HEIGHT),
        Mode::Auto => unreachable!(),
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

    let rel = path.strip_prefix(&args.in_dir).unwrap_or(path);
    let mut out_path = args.out_dir.join(rel);
    if let Some(p) = out_path.parent() {
        fs::create_dir_all(p)?;
    }

    let out_format = args.format.to_lowercase();
    let ext = if out_format == "jpeg" || out_format == "jpg" {
        out_path.set_extension("jpg");
        "jpg"
    } else if out_format == "png" {
        out_path.set_extension("png");
        "png"
    } else if out_format == "webp" {
        out_path.set_extension("webp");
        "webp"
    } else {
        if let Some(e) = path.extension().and_then(|s| s.to_str()) {
            out_path.set_extension(e);
            e
        } else {
            out_path.set_extension("jpg");
            "jpg"
        }
    };

    let (final_w, final_h) = final_img.dimensions();

    if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        let rgb = match final_img {
            DynamicImage::ImageRgba8(img) => DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(img).to_rgb8()),
            DynamicImage::ImageRgba16(img) => DynamicImage::ImageRgb16(image::DynamicImage::ImageRgba16(img).to_rgb16()),
            other => other.to_rgb8().into(),
        };
        let mut out_file = fs::File::create(&out_path)?;
        rgb.write_to(&mut out_file, ImageOutputFormat::Jpeg(args.quality))?;
    } else if ext.eq_ignore_ascii_case("png") {
        let mut out_file = fs::File::create(&out_path)?;
        final_img.write_to(&mut out_file, ImageOutputFormat::Png)?;
    } else if ext.eq_ignore_ascii_case("webp") {
        let mut out_file = fs::File::create(&out_path)?;
        final_img.write_to(&mut out_file, ImageOutputFormat::WebP)?;
    } else {
        let mut out_file = fs::File::create(&out_path)?;
        final_img.write_to(&mut out_file, ImageOutputFormat::Png)?;
    }

    println!("OK: {} -> {} ({}x{})", path.display(), out_path.display(), final_w, final_h);
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
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path)?;
    let mut bufreader = BufReader::new(&file);
    let exifreader = Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader);
    if let Ok(exif) = exif {
        if let Some(field) = exif.get_field(Tag::Orientation, exif::In::PRIMARY) {
            if let exif::Value::Short(ref vec) = field.value {
                if let Some(&orient) = vec.get(0) {
                    match orient {
                        1 => Ok(img.clone()), // Horizontal (normal)
                        2 => Ok(img.fliph()),
                        3 => Ok(img.rotate180()),
                        4 => Ok(img.flipv()),
                        5 => Ok(img.rotate90().fliph()),
                        6 => Ok(img.rotate90()),
                        7 => Ok(img.rotate270().fliph()),
                        8 => Ok(img.rotate270()),
                        _ => Ok(img.clone()),
                    }
                } else { Ok(img.clone()) }
            } else { Ok(img.clone()) }
        } else { Ok(img.clone()) }
    } else {
        Ok(img.clone())
    }
}

