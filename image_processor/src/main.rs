use anyhow::{Context, bail};
use clap::Parser;
use image::RgbaImage;
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

    if !cli.input.exists() {
        bail!("Input file does not exist: {}", cli.input.display());
    }

    let params = if let Some(params) = cli.params {
        if !params.exists() {
            bail!("Params file does not exist: {}", params.display());
        }
        read_to_string(&params)
            .with_context(|| format!("Failed to read params file: {}", params.display()))?
    } else {
        "{}".to_string()
    };

    // Convert the params string to a CString to pass to the plugin
    let params = CString::new(params).context("Params contain an interior null byte")?;

    // Load the image
    let mut image = image::open(&cli.input)
        .with_context(|| format!("Failed to open image: {}", cli.input.display()))?
        .to_rgba8();

    let (width, height) = image.dimensions();
    let data = image.as_mut_ptr();

    // Load the plugin library
    let plugin_file = plugin_library_path(&cli.plugin_path, &cli.plugin);
    if !plugin_file.exists() {
        bail!("Plugin library does not exist: {}", plugin_file.display());
    }

    // SAFETY: plugin API requires `process_image` to match `ProcessImage`.
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

        process_image(width, height, data, params.as_ptr());
    }

    save_rgba_image(cli.output, width, height, image.into_raw())
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
