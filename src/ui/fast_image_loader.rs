use std::path::Path;
use anyhow::Result;
use image::DynamicImage;

/// Fast image loader that uses optimized decoders for specific formats
pub struct FastImageLoader;

impl FastImageLoader {
    /// Load an image using the fastest available decoder for the format
    pub fn load_image(file_path: &Path) -> Result<DynamicImage> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("jpg") | Some("jpeg") => Self::load_jpeg_fast(file_path),
            _ => Self::load_generic(file_path),
        }
    }

    /// Fast JPEG loading using jpeg-decoder directly
    fn load_jpeg_fast(file_path: &Path) -> Result<DynamicImage> {
        use std::fs::File;
        use std::io::BufReader;
        
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        
        let mut decoder = jpeg_decoder::Decoder::new(&mut reader);
        let pixels = decoder.decode()?;
        let info = decoder.info().ok_or_else(|| anyhow::anyhow!("Failed to get JPEG info"))?;
        
        let dynamic_image = match info.pixel_format {
            jpeg_decoder::PixelFormat::L8 => {
                DynamicImage::ImageLuma8(
                    image::ImageBuffer::from_raw(info.width as u32, info.height as u32, pixels)
                        .ok_or_else(|| anyhow::anyhow!("Failed to create grayscale image buffer"))?
                )
            },
            jpeg_decoder::PixelFormat::RGB24 => {
                DynamicImage::ImageRgb8(
                    image::ImageBuffer::from_raw(info.width as u32, info.height as u32, pixels)
                        .ok_or_else(|| anyhow::anyhow!("Failed to create RGB image buffer"))?
                )
            },
            _ => {
                // Fallback to loader for non JPEG formats
                return Self::load_generic(file_path);
            }
        };
        
        Ok(dynamic_image)
    }

    /// Generic image loading using the image crate (fallback)
    fn load_generic(file_path: &Path) -> Result<DynamicImage> {
        let img = image::open(file_path)?;
        Ok(img)
    }

    /// Load image with automatic resizing to target dimensions for faster processing
    pub fn load_image_resized(file_path: &Path, target_width: u32, target_height: u32) -> Result<DynamicImage> {
        //if let Ok(metadata) = std::fs::metadata(file_path) {
            //let file_size_mb = metadata.len() / (1024 * 1024);
            // Skip files larger than 50MB
            //if file_size_mb > 50 {
            //    return Err(anyhow::anyhow!("Image file too large: {}MB", file_size_mb));
            //}
        //}
        
        let img = Self::load_image(file_path)?;
        
        // Calculate optimal resize dimensions while maintaining aspect ratio
        let (orig_width, orig_height) = (img.width(), img.height());
        let scale_x = target_width as f32 / orig_width as f32;
        let scale_y = target_height as f32 / orig_height as f32;
        let scale = scale_x.min(scale_y).min(1.0); // Don't upscale
        
        if scale < 1.0 {
            let new_width = (orig_width as f32 * scale) as u32;
            let new_height = (orig_height as f32 * scale) as u32;
            
            // Use fast resize filter for preview images
            Ok(img.resize(new_width, new_height, image::imageops::FilterType::Triangle))
        } else {
            Ok(img)
        }
    }
    
    /// Get estimated terminal display size (in pixels) for optimal resizing
    pub fn get_terminal_display_size(terminal_width: u16, terminal_height: u16) -> (u32, u32) {
        // Use better calculations for reference viewing 
        let pixel_width = (terminal_width as u32).saturating_mul(8);  
        let pixel_height = (terminal_height as u32).saturating_mul(16); 
        
        (pixel_width.min(800), pixel_height.min(600))  
    }
}
