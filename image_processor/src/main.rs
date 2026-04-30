mod plugin_loader;
mod utils;

use anyhow::{Context, bail};
use clap::Parser;
use image::{ImageError, RgbaImage};
use log::{info, warn};
use plugin_loader::Plugin;
use std::ffi::CString;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use utils::ensure_existing_file;

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    // Validate the input file
    ensure_existing_file(&cli.input, "Input file")?;

    // Read the params file if provided, otherwise use an empty JSON object
    let params = if let Some(params) = cli.params {
        ensure_existing_file(&params, "Params file")?;
        read_to_string(&params)
            .with_context(|| format!("Failed to read params file: {}", params.display()))?
    } else {
        warn!("No --params file provided. The image may remain unchanged.");
        "{}".to_string()
    };

    // Convert the params string to a CString to pass to the plugin
    let params = CString::new(params).context("Params contain an interior null byte")?;

    // Load the image
    info!("Loading image from {}", cli.input.display());
    let mut image = load_rgba_image(&cli.input)?;
    let (width, height) = image.dimensions();
    info!("Loaded image: {}x{} pixels", width, height);

    let expected_len = rgba_buffer_len(width, height)?;
    if image.as_raw().len() != expected_len {
        bail!(
            "Invalid RGBA image buffer length: expected {}, got {}",
            expected_len,
            image.as_raw().len()
        );
    }

    // Load the plugin
    info!(
        "Loading plugin '{}' from {}",
        cli.plugin,
        cli.plugin_path.display()
    );
    let plugin = Plugin::load(&cli.plugin_path, &cli.plugin)?;
    info!("Plugin loaded successfully");

    // SAFETY: plugin API requires `process_image` to match `ProcessImage`.
    // The image buffer and params CString are kept alive and are not moved or
    // freed until `process_image` returns.
    info!("Applying plugin '{}'", cli.plugin);
    unsafe {
        plugin.process_image(width, height, image.as_mut_ptr(), params.as_ptr())?;
    }
    info!("Plugin applied successfully");

    let output = cli.output;
    save_rgba_image(output.clone(), width, height, image.into_raw())?;
    info!("Image saved successfully to {}", output.display());

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

fn save_rgba_image(output: PathBuf, width: u32, height: u32, data: Vec<u8>) -> anyhow::Result<()> {
    let image = RgbaImage::from_raw(width, height, data).context("Invalid RGBA image buffer")?;
    image
        .save(&output)
        .with_context(|| format!("Failed to save image: {}", output.display()))
}
