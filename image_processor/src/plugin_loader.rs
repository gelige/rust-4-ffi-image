use crate::utils::ensure_existing_file;
use anyhow::Context;
use libloading::Library;
use std::ffi::c_char;
use std::path::{Path, PathBuf};

type ProcessImage = unsafe extern "C" fn(u32, u32, *mut u8, *const c_char);

pub struct Plugin {
    library: Library,
    path: PathBuf,
}

impl Plugin {
    pub fn load(plugin_path: &Path, plugin: &str) -> anyhow::Result<Self> {
        let path = plugin_library_path(plugin_path, plugin);
        ensure_existing_file(&path, "Plugin library")?;

        // SAFETY: loading a dynamic library is inherently unsafe because its
        // initialization code may run. The path is user-provided and checked to
        // be an existing regular file before loading.
        let library = unsafe { Library::new(&path) }
            .with_context(|| format!("Failed to load plugin: {}", path.display()))?;

        Ok(Self { library, path })
    }

    pub unsafe fn process_image(
        &self,
        width: u32,
        height: u32,
        rgba_data: *mut u8,
        params: *const c_char,
    ) -> anyhow::Result<()> {
        let process_image: libloading::Symbol<ProcessImage> =
            // SAFETY: the plugin contract requires `process_image` to have the
            // `ProcessImage` ABI and signature.
            unsafe { self.library.get(b"process_image") }.with_context(|| {
                format!("Plugin has no process_image symbol: {}", self.path.display())
            })?;

        // SAFETY: the caller guarantees that `rgba_data` and `params` are valid
        // for the duration of this synchronous plugin call.
        unsafe { process_image(width, height, rgba_data, params) };

        Ok(())
    }
}

fn plugin_library_path(plugin_path: &Path, plugin: &str) -> PathBuf {
    plugin_path.join(format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        plugin,
        std::env::consts::DLL_SUFFIX
    ))
}
