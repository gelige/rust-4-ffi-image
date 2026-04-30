use core::ffi::c_char;
use serde_json;
use std::ffi::CStr;
use std::slice::from_raw_parts_mut;

const BYTES_PER_PIXEL: usize = 4;

/// Plugin to mirror an image horizontally and/or vertically.
/// `params` must point to a null-terminated JSON string with
/// `horizontal` and `vertical` boolean fields.
#[unsafe(no_mangle)]
pub extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    if params.is_null() || rgba_data.is_null() {
        return;
    }

    // SAFETY: `params` was checked for null and must point to a
    // null-terminated string for the duration of this call.
    let params_str = unsafe {
        match CStr::from_ptr(params).to_str() {
            Ok(s) => s,
            Err(_) => return,
        }
    };
    let params_json: serde_json::Value = match serde_json::from_str(params_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    let horizontal = params_json["horizontal"].as_bool().unwrap_or(false);
    let vertical = params_json["vertical"].as_bool().unwrap_or(false);

    // no need to mirror
    if !horizontal && !vertical {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let row_size = w * BYTES_PER_PIXEL;
    let len = row_size * h;

    // SAFETY: `rgba_data` was checked for null. The host guarantees that it
    // points to a writable RGBA buffer of `len` bytes for this call.
    let data = unsafe { from_raw_parts_mut(rgba_data, len) };

    if horizontal {
        swap_horizontal(data, w, h, row_size);
    }
    if vertical {
        swap_vertical(data, h, row_size);
    }
}

fn swap_horizontal(data: &mut [u8], width: usize, height: usize, row_size: usize) {
    for y in 0..height {
        let row_start = y * row_size;

        for x in 0..width / 2 {
            let left = row_start + x * BYTES_PER_PIXEL;
            let right = row_start + (width - 1 - x) * BYTES_PER_PIXEL;

            swap_pixels(data, left, right);
        }
    }
}

fn swap_vertical(data: &mut [u8], height: usize, row_size: usize) {
    for y in 0..height / 2 {
        let top = y * row_size;
        let bottom = (height - 1 - y) * row_size;

        let (a, b) = data.split_at_mut(bottom);

        let top_row = &mut a[top..top + row_size];
        let bottom_row = &mut b[0..row_size];

        top_row.swap_with_slice(bottom_row);
    }
}

fn swap_pixels(data: &mut [u8], left: usize, right: usize) {
    let (a, b) = data.split_at_mut(right);

    let left_pixel = &mut a[left..left + BYTES_PER_PIXEL];
    let right_pixel = &mut b[0..BYTES_PER_PIXEL];

    left_pixel.swap_with_slice(right_pixel);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn mirrors_image_horizontally() {
        let mut data = vec![
            1, 11, 21, 31, 2, 12, 22, 32, //
            3, 13, 23, 33, 4, 14, 24, 34,
        ];
        let params = CString::new(r#"{"horizontal":true,"vertical":false}"#).unwrap();

        process_image(2, 2, data.as_mut_ptr(), params.as_ptr());

        assert_eq!(
            data,
            vec![
                2, 12, 22, 32, 1, 11, 21, 31, //
                4, 14, 24, 34, 3, 13, 23, 33,
            ]
        );
    }

    #[test]
    fn mirrors_image_vertically() {
        let mut data = vec![
            1, 11, 21, 31, 2, 12, 22, 32, //
            3, 13, 23, 33, 4, 14, 24, 34,
        ];
        let params = CString::new(r#"{"horizontal":false,"vertical":true}"#).unwrap();

        process_image(2, 2, data.as_mut_ptr(), params.as_ptr());

        assert_eq!(
            data,
            vec![
                3, 13, 23, 33, 4, 14, 24, 34, //
                1, 11, 21, 31, 2, 12, 22, 32,
            ]
        );
    }
}
