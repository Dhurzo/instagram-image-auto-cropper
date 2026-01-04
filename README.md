# instagram-image-auto-cropper

A Rust tool for cropping/resizing images from a folder to Instagram's recommended maximum size.
Supports auto, vertical, and horizontal modes.

## Features

- **Processing modes**: auto, vertical, horizontal
  - `horizontal`: landscape aspect ratio (~1.91:1) → target: 1080 x 566
  - `vertical`: portrait aspect ratio (4:5) → target: 1080 x 1350
  - `auto`: automatically detects orientation (w >= h ⇒ horizontal, else vertical)
- Recursively iterates input folder
- Corrects EXIF orientation if present
- **Center crops** to fit target aspect ratio (preserves the center of the image), then resizes
- Keeps default format; can force JPEG/PNG/WebP output
- No upscaling: images smaller than target size are kept at original dimensions
- Parallel processing with Rayon

## Installation

```bash
git clone <repository-url>
cd ig_image_resizer
cargo build --release
```

## Usage

### Compiled Version (Main Binary)
```bash
./target/release/ig_image_resizer --in-dir ./images --out-dir ./output --mode auto
```

### Script Version (Alternative Binary)
```bash
./target/release/script --in-dir ./images --out-dir ./output --mode auto
```

### Options

- `--in-dir <DIR>`: Input directory (required)
- `--out-dir <DIR>`: Output directory (created if it doesn't exist) (required)
- `--mode <MODE>`: Processing mode [auto, vertical, horizontal] (default: auto)
- `--format <FORMAT>`: Force output format [jpeg, png, webp, keep] (default: keep)
- `--quality <QUALITY>`: JPEG quality (1-100) (default: 100)
- `--threads <THREADS>`: Number of threads for parallel processing (0 = automatic) (default: 0)

### Examples

```bash
# Process images in automatic mode (auto detect if image is vertical or landscape)
./target/release/ig_image_resizer --in-dir ./photos --out-dir ./processed --mode auto

# Force JPEG format with quality 85
./target/release/ig_image_resizer --in-dir ./photos --out-dir ./processed --format jpeg --quality 85

# Process in vertical mode
./target/release/ig_image_resizer --in-dir ./photos --out-dir ./processed --mode vertical

# Parallel processing with 4 threads
./target/release/ig_image_resizer --in-dir ./photos --out-dir ./processed --threads 4

# Force WebP format
./target/release/ig_image_resizer --in-dir ./photos --out-dir ./processed --format webp
```

## Project Structure

- `src/main.rs`: Main binary
- `src/bin/script.rs`: Alternative script-style binary (same functionality)
- `Cargo.toml`: Project configuration and dependencies

## How It Works

1. **Loads** all image files from the input directory recursively
2. **Corrects** EXIF orientation metadata if present
3. **Determines** processing mode (auto/vertical/horizontal)
4. **Center crops** the image to match Instagram's aspect ratio (content from the center is preserved)
5. **Resizes** to Instagram's recommended dimensions (1080px width)
6. **Saves** in the specified format to the output directory

## Dependencies

- `clap`: For command-line argument parsing
- `image`: For image processing
- `walkdir`: For recursive directory iteration
- `kamadak-exif`: For EXIF metadata reading
- `rayon`: For parallel processing

## License

This project is licensed under the MIT License - see the LICENSE file for details.

