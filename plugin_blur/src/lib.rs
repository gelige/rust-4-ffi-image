use core::ffi::c_char;
use serde_json;
use std::ffi::CStr;
use std::slice::from_raw_parts_mut;

const BYTES_PER_PIXEL: usize = 4;

/// Plugin to blur an image with a given radius.
/// `params` must point to a null-terminated JSON string with
/// `radius` and `iterations` fields.
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

    let radius = params_json["radius"].as_u64().unwrap_or(0);
    let iterations = params_json["iterations"].as_u64().unwrap_or(0);

    // no need to blur
    if radius == 0 || iterations == 0 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let row_size = w * BYTES_PER_PIXEL;
    let len = row_size * h;

    // SAFETY: `rgba_data` was checked for null. The host guarantees that it
    // points to a writable RGBA buffer of `len` bytes for this call.
    let data = unsafe { from_raw_parts_mut(rgba_data, len) };

    blur_image(data, w, h, radius, iterations);
}

fn blur_image(data: &mut [u8], width: usize, height: usize, radius: u64, iterations: u64) {
    let radius = radius as isize;

    for _ in 0..iterations {
        let source = data.to_vec();

        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0; BYTES_PER_PIXEL];
                let mut weight_sum = 0.0;

                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;

                        if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                            continue;
                        }

                        let distance = ((dx * dx + dy * dy) as f64).sqrt();

                        if distance > radius as f64 {
                            continue;
                        }

                        let neighbor = pixel_offset(nx as usize, ny as usize, width);

                        for channel in 0..BYTES_PER_PIXEL {
                            sum[channel] += source[neighbor + channel] as f64 * distance;
                        }

                        weight_sum += distance;
                    }
                }

                let target = pixel_offset(x, y, width);

                if weight_sum == 0.0 {
                    data[target..target + BYTES_PER_PIXEL]
                        .copy_from_slice(&source[target..target + BYTES_PER_PIXEL]);
                    continue;
                }

                for channel in 0..BYTES_PER_PIXEL {
                    data[target + channel] = (sum[channel] / weight_sum).round() as u8;
                }
            }
        }
    }
}

fn pixel_offset(x: usize, y: usize, width: usize) -> usize {
    (y * width + x) * BYTES_PER_PIXEL
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn blurs_image_with_one_iteration() {
        let mut data = vec![
            0, 10, 20, 30, //
            40, 50, 60, 70, //
            120, 130, 140, 150, //
            200, 210, 220, 230, //
            240, 250, 100, 110,
        ];
        let params = CString::new(r#"{"radius":1,"iterations":1}"#).unwrap();

        process_image(5, 1, data.as_mut_ptr(), params.as_ptr());

        assert_eq!(
            data,
            vec![
                40, 50, 60, 70, //
                60, 70, 80, 90, //
                120, 130, 140, 150, //
                180, 190, 120, 130, //
                200, 210, 220, 230,
            ]
        );
    }

    #[test]
    fn blurs_image_with_radius_two_and_one_iteration() {
        let mut data = vec![
            0, 10, 20, 30, //
            40, 50, 60, 70, //
            120, 130, 140, 150, //
            200, 210, 220, 230, //
            240, 250, 100, 110,
        ];
        let params = CString::new(r#"{"radius":2,"iterations":1}"#).unwrap();

        process_image(5, 1, data.as_mut_ptr(), params.as_ptr());

        assert_eq!(
            data,
            vec![
                93, 103, 113, 123, //
                130, 140, 150, 160, //
                120, 130, 87, 97, //
                110, 120, 90, 100, //
                147, 157, 167, 177,
            ]
        );
    }

    #[test]
    fn blurs_image_with_two_iterations() {
        let mut data = vec![
            0, 10, 20, 30, //
            40, 50, 60, 70, //
            120, 130, 140, 150, //
            200, 210, 220, 230, //
            240, 250, 100, 110,
        ];
        let params = CString::new(r#"{"radius":1,"iterations":2}"#).unwrap();

        process_image(5, 1, data.as_mut_ptr(), params.as_ptr());

        assert_eq!(
            data,
            vec![
                60, 70, 80, 90, //
                80, 90, 100, 110, //
                120, 130, 100, 110, //
                160, 170, 180, 190, //
                180, 190, 120, 130,
            ]
        );
    }
}
