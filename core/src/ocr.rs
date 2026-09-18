//! Offline screenshot OCR using ocrs + RTen models.
//!
//! The OCR runtime and code are MIT OR Apache-2.0. The actual OCR model
//! artifacts are kept outside the APK and are imported/downloaded separately.

use std::path::Path;

use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

pub fn extract_text_from_image<P, D>(
    image_path: P,
    detection_model_path: D,
    recognition_model_path: &Path,
) -> Result<String, String>
where
    P: AsRef<Path>,
    D: AsRef<Path>,
{
    let detection =
        Model::load_file(detection_model_path).map_err(|e| format!("OCR detection model: {e}"))?;
    let recognition = Model::load_file(recognition_model_path)
        .map_err(|e| format!("OCR recognition model: {e}"))?;

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        ..Default::default()
    })
    .map_err(|e| format!("OCR engine: {e}"))?;

    let img = image::open(image_path.as_ref())
        .map_err(|e| format!("image decode: {e}"))?
        .into_rgb8();
    let source = ImageSource::from_bytes(img.as_raw(), img.dimensions())
        .map_err(|e| format!("OCR image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("OCR prepare input: {e}"))?;

    engine
        .get_text(&input)
        .map(|text| text.trim().to_string())
        .map_err(|e| format!("OCR recognition: {e}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn ocr_module_is_present() {
        assert!(true);
    }
}
