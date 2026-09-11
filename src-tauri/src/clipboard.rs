use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::png::PngEncoder, ImageEncoder, ImageFormat, ImageReader, Limits};
use serde::Serialize;
use std::io::{Cursor, Write};

use crate::storage::{StorageError, StorageResult};

pub(crate) const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const MAX_IMAGE_DIMENSION: u32 = 20_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardImage {
    pub base64: String,
}

#[tauri::command]
pub async fn read_clipboard_image() -> StorageResult<ClipboardImage> {
    // Reading happens only in response to the user's paste action, never on a timer.
    tauri::async_runtime::spawn_blocking(|| {
        let mut clipboard = arboard::Clipboard::new().map_err(|_| {
            StorageError::new("CLIPBOARD_UNAVAILABLE", "暂时无法读取剪贴板，请稍后重试。")
        })?;
        let image = clipboard.get_image().map_err(|error| match error {
            arboard::Error::ContentNotAvailable => StorageError::new(
                "NO_CLIPBOARD_IMAGE",
                "剪贴板里没有图片，请先截图或复制图片后再粘贴。",
            ),
            _ => StorageError::new(
                "CLIPBOARD_UNAVAILABLE",
                "暂时无法读取剪贴板图片，请重新复制后再试。",
            ),
        })?;
        encode_clipboard_rgba(image.width, image.height, &image.bytes)
    })
    .await
    .map_err(|_| StorageError::new("CLIPBOARD_READ_FAILED", "读取剪贴板图片失败，请重试。"))?
}

fn validate_dimensions(width: u32, height: u32) -> StorageResult<()> {
    if width == 0
        || height == 0
        || width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
    {
        return Err(StorageError::new(
            "IMAGE_DIMENSIONS_TOO_LARGE",
            "图片尺寸无效或过大，请使用不超过 4000 万像素、单边不超过 20000 像素的图片。",
        ));
    }
    Ok(())
}

pub(crate) fn validate_image_bytes(bytes: &[u8], format: ImageFormat) -> StorageResult<()> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(StorageError::new(
            "IMAGE_TOO_LARGE",
            "图片不能为空，且每张图片不能超过 20 MB。",
        ));
    }
    let dimensions = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| {
            StorageError::new(
                "INVALID_IMAGE_DATA",
                "图片数据损坏或格式不正确，请重新复制或选择图片。",
            )
        })?;
    validate_dimensions(dimensions.0, dimensions.1)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_PIXELS * 8);
    reader.limits(limits);
    reader.decode().map_err(|_| {
        StorageError::new(
            "INVALID_IMAGE_DATA",
            "图片无法完整解码，请重新复制或选择图片。",
        )
    })?;
    Ok(())
}

pub(crate) fn decode_clipboard_png(encoded: &str) -> StorageResult<Vec<u8>> {
    if encoded.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
        return Err(StorageError::new(
            "IMAGE_TOO_LARGE",
            "每张粘贴图片不能超过 20 MB。",
        ));
    }
    let bytes = STANDARD.decode(encoded).map_err(|_| {
        StorageError::new(
            "INVALID_IMAGE_DATA",
            "粘贴图片的数据不完整，请重新复制后再试。",
        )
    })?;
    validate_image_bytes(&bytes, ImageFormat::Png)?;
    Ok(bytes)
}

fn encode_clipboard_rgba(
    width: usize,
    height: usize,
    bytes: &[u8],
) -> StorageResult<ClipboardImage> {
    let width = u32::try_from(width)
        .map_err(|_| StorageError::new("IMAGE_DIMENSIONS_TOO_LARGE", "剪贴板图片宽度过大。"))?;
    let height = u32::try_from(height)
        .map_err(|_| StorageError::new("IMAGE_DIMENSIONS_TOO_LARGE", "剪贴板图片高度过大。"))?;
    validate_dimensions(width, height)?;
    if bytes.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err(StorageError::new(
            "INVALID_IMAGE_DATA",
            "剪贴板图片的数据不完整，请重新截图。",
        ));
    }
    let mut png = BoundedPngBuffer(Vec::new());
    PngEncoder::new(&mut png)
        .write_image(bytes, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|_| {
            StorageError::new(
                "IMAGE_ENCODING_FAILED",
                "剪贴板图片转换失败或超过 20 MB，请缩小截图范围后重试。",
            )
        })?;
    Ok(ClipboardImage {
        base64: STANDARD.encode(png.0),
    })
}

struct BoundedPngBuffer(Vec<u8>);

impl Write for BoundedPngBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_IMAGE_BYTES {
            return Err(std::io::Error::other("PNG exceeds image size limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_png_preserves_all_pixels_including_alpha() {
        let rgba = [
            0, 0, 0, 0, 10, 20, 30, 255, 250, 100, 60, 127, 255, 255, 255, 255,
        ];
        let encoded = encode_clipboard_rgba(2, 2, &rgba).unwrap();
        let png = decode_clipboard_png(&encoded.base64).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!(decoded.as_raw().as_slice(), rgba);
    }

    #[test]
    fn rejects_malformed_truncated_and_non_png_clipboard_data() {
        assert!(decode_clipboard_png("not base64!").is_err());
        assert!(decode_clipboard_png(&STANDARD.encode(b"not an image")).is_err());
        let encoded = encode_clipboard_rgba(2, 1, &[255; 8]).unwrap();
        let mut bytes = STANDARD.decode(encoded.base64).unwrap();
        bytes.truncate(40);
        assert!(decode_clipboard_png(&STANDARD.encode(bytes)).is_err());
    }

    #[test]
    fn rejects_invalid_dimensions_before_encoding() {
        assert!(encode_clipboard_rgba(0, 1, &[]).is_err());
        assert!(encode_clipboard_rgba(10_000, 10_000, &[]).is_err());
        assert!(encode_clipboard_rgba(2, 1, &[255; 4]).is_err());
        assert!(encode_clipboard_rgba(20_001, 1, &[]).is_err());
    }

    #[test]
    fn bounded_encoder_never_writes_past_limit() {
        let mut output = BoundedPngBuffer(vec![0; MAX_IMAGE_BYTES - 1]);
        assert!(output.write_all(&[1, 2]).is_err());
        assert_eq!(output.0.len(), MAX_IMAGE_BYTES - 1);
    }
}
