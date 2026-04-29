use anyhow::{Context, bail};
use clap::Parser;
use image::{ImageError, RgbaImage};
use libloading::Library;
use std::ffi::{CString, c_char};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

type ProcessImage = unsafe extern "C" fn(u32, u32, *mut u8, *const c_char);

#[derive(Parser)]
#[command(name = "image-processor", about = "Apply an image processing plugin")]
struct Cli {
    /// Input PNG file
    #[arg(long)]
    input: PathBuf,

    /// Output PNG file
    #[arg(long, default_value = "output.png")]
    output: PathBuf,

    /// Plugin to use (without extension)
    #[arg(long)]
    plugin: String,

    /// Path to the plugin parameters file
    #[arg(long)]
    params: Option<PathBuf>,

    /// Directory with plugin dynamic libraries
    #[arg(long, default_value = "target/debug")]
    plugin_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    ensure_existing_file(&cli.input, "Input file")?;

    let params = if let Some(params) = cli.params {
        ensure_existing_file(&params, "Params file")?;
        read_to_string(&params)
            .with_context(|| format!("Failed to read params file: {}", params.display()))?
    } else {
        "{}".to_string()
    };

    // Convert the params string to a CString to pass to the plugin
    let params = CString::new(params).context("Params contain an interior null byte")?;

    let mut image = load_rgba_image(&cli.input)?;
    let (width, height) = image.dimensions();
    let expected_len = rgba_buffer_len(width, height)?;
    if image.as_raw().len() != expected_len {
        bail!(
            "Invalid RGBA image buffer length: expected {}, got {}",
            expected_len,
            image.as_raw().len()
        );
    }

    // Load the plugin library
    let plugin_file = plugin_library_path(&cli.plugin_path, &cli.plugin);
    ensure_existing_file(&plugin_file, "Plugin library")?;

    // SAFETY: plugin API requires `process_image` to match `ProcessImage`.
    // The image buffer and params CString are kept alive and are not moved or
    // freed until `process_image` returns.
    unsafe {
        let library = Library::new(&plugin_file)
            .with_context(|| format!("Failed to load plugin: {}", plugin_file.display()))?;

        let process_image: libloading::Symbol<ProcessImage> =
            library.get(b"process_image").with_context(|| {
                format!(
                    "Plugin has no process_image symbol: {}",
                    plugin_file.display()
                )
            })?;

        process_image(width, height, image.as_mut_ptr(), params.as_ptr());
    }

    save_rgba_image(cli.output, width, height, image.into_raw())
}

fn ensure_existing_file(path: &Path, label: &str) -> anyhow::Result<()> {
    if !path.exists() {
        bail!("{} does not exist: {}", label, path.display());
    }
    if !path.is_file() {
        bail!("{} is not a regular file: {}", label, path.display());
    }
    Ok(())
}

fn load_rgba_image(input: &Path) -> anyhow::Result<RgbaImage> {
    match image::open(input) {
        Ok(image) => Ok(image.to_rgba8()),
        Err(ImageError::Decoding(err)) => {
            bail!(
                "Failed to decode image {}: invalid format or corrupted data ({})",
                input.display(),
                err
            )
        }
        Err(ImageError::Unsupported(err)) => {
            bail!("Unsupported image format {}: {}", input.display(), err)
        }
        Err(err) => Err(err).with_context(|| format!("Failed to load image: {}", input.display())),
    }
}

fn rgba_buffer_len(width: u32, height: u32) -> anyhow::Result<usize> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .context("Image dimensions are too large")
}

fn plugin_library_path(plugin_path: &Path, plugin: &str) -> PathBuf {
    plugin_path.join(format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        plugin,
        std::env::consts::DLL_SUFFIX
    ))
}

fn save_rgba_image(output: PathBuf, width: u32, height: u32, data: Vec<u8>) -> anyhow::Result<()> {
    let image = RgbaImage::from_raw(width, height, data).context("Invalid RGBA image buffer")?;
    image
        .save(&output)
        .with_context(|| format!("Failed to save image: {}", output.display()))
}
