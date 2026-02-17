fn generate_icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size / 2) as f32;
    let radius = center - 1.0;

    // Blue circle
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                rgba[idx] = 100;
                rgba[idx + 1] = 180;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            }
        }
    }

    // Hour hand (vertical, pointing up)
    let hand_len = (radius * 0.5) as u32;
    for dy in 0..hand_len {
        let y = (center as u32).saturating_sub(dy);
        let x = center as u32;
        if y < size && x < size {
            let idx = ((y * size + x) * 4) as usize;
            rgba[idx] = 255;
            rgba[idx + 1] = 255;
            rgba[idx + 2] = 255;
            rgba[idx + 3] = 255;
        }
    }

    // Minute hand (horizontal, pointing right)
    let hand_len = (radius * 0.7) as u32;
    for dx in 0..hand_len {
        let y = center as u32;
        let x = (center as u32) + dx;
        if y < size && x < size {
            let idx = ((y * size + x) * 4) as usize;
            rgba[idx] = 255;
            rgba[idx + 1] = 255;
            rgba[idx + 2] = 255;
            rgba[idx + 3] = 255;
        }
    }

    rgba
}

/// Write a valid ICO file containing multiple sizes.
fn write_ico(path: &std::path::Path, sizes: &[u32]) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;

    // ICO header: reserved(2) + type(2, 1=icon) + count(2)
    let count = sizes.len() as u16;
    file.write_all(&[0, 0])?; // reserved
    file.write_all(&1u16.to_le_bytes())?; // type = icon
    file.write_all(&count.to_le_bytes())?; // image count

    // Calculate offsets: header(6) + directory(16 * count) + image data
    let dir_size = 16 * sizes.len();
    let header_size = 6 + dir_size;

    // Pre-compute image data sizes and offsets
    struct ImageInfo {
        width: u32,
        bmp_size: u32,
        offset: u32,
    }

    let mut images: Vec<ImageInfo> = Vec::new();
    let mut current_offset = header_size as u32;
    for &size in sizes {
        // BMP header (40) + pixel data (BGRA, 4 bytes per pixel) + AND mask
        let row_bytes = size * 4;
        let and_row = size.div_ceil(32) * 4; // AND mask row (padded to 4 bytes)
        let bmp_size = 40 + row_bytes * size + and_row * size;
        images.push(ImageInfo {
            width: size,
            bmp_size,
            offset: current_offset,
        });
        current_offset += bmp_size;
    }

    // Write directory entries
    for img in &images {
        let w = if img.width >= 256 {
            0u8
        } else {
            img.width as u8
        };
        let h = w;
        file.write_all(&[w, h, 0, 0])?; // width, height, palette, reserved
        file.write_all(&1u16.to_le_bytes())?; // planes
        file.write_all(&32u16.to_le_bytes())?; // bits per pixel
        file.write_all(&img.bmp_size.to_le_bytes())?; // image size
        file.write_all(&img.offset.to_le_bytes())?; // offset
    }

    // Write image data (BITMAPINFOHEADER + pixels + AND mask)
    for img in &images {
        let size = img.width;
        let rgba = generate_icon_rgba(size);

        // BITMAPINFOHEADER (40 bytes)
        let and_row = size.div_ceil(32) * 4;
        let bih_height = (size * 2) as i32; // doubled for XOR+AND in ICO
        file.write_all(&40u32.to_le_bytes())?; // biSize
        file.write_all(&(size as i32).to_le_bytes())?; // biWidth
        file.write_all(&bih_height.to_le_bytes())?; // biHeight (XOR+AND)
        file.write_all(&1u16.to_le_bytes())?; // biPlanes
        file.write_all(&32u16.to_le_bytes())?; // biBitCount
        file.write_all(&0u32.to_le_bytes())?; // biCompression
        let pixel_data_size = size * size * 4 + and_row * size;
        file.write_all(&pixel_data_size.to_le_bytes())?; // biSizeImage
        file.write_all(&0u32.to_le_bytes())?; // biXPelsPerMeter
        file.write_all(&0u32.to_le_bytes())?; // biYPelsPerMeter
        file.write_all(&0u32.to_le_bytes())?; // biClrUsed
        file.write_all(&0u32.to_le_bytes())?; // biClrImportant

        // Pixel data: BGRA, bottom-up
        for y in (0..size).rev() {
            for x in 0..size {
                let idx = ((y * size + x) * 4) as usize;
                let r = rgba[idx];
                let g = rgba[idx + 1];
                let b = rgba[idx + 2];
                let a = rgba[idx + 3];
                file.write_all(&[b, g, r, a])?; // BGRA
            }
        }

        // AND mask (all zeros = fully opaque, alpha channel handles transparency)
        let and_row_bytes = vec![0u8; and_row as usize];
        for _ in 0..size {
            file.write_all(&and_row_bytes)?;
        }
    }

    Ok(())
}

fn main() {
    // Only re-run when build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");

    // Only run resource embedding on Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ico_path = std::path::Path::new(&out_dir).join("clockor.ico");

    // Generate ICO with 4 sizes
    write_ico(&ico_path, &[16, 32, 48, 256]).expect("Failed to write ICO file");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().unwrap());
    res.set("ProductName", "ClockOR");
    res.set("FileDescription", "Fullscreen game clock overlay");

    // Embed Windows manifest for DPI awareness and visual styles
    res.set_manifest(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    </windowsSettings>
  </application>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>"#,
    );

    res.compile().expect("Failed to compile Windows resources");
}
