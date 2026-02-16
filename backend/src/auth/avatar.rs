pub fn detect_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }

    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("image/jpeg");
    }

    if bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47 {
        return Some("image/png");
    }

    if bytes.len() >= 12
        && bytes[0] == 0x52
        && bytes[1] == 0x49
        && bytes[2] == 0x46
        && bytes[3] == 0x46
        && bytes[8] == 0x57
        && bytes[9] == 0x45
        && bytes[10] == 0x42
        && bytes[11] == 0x50
    {
        return Some("image/webp");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jpeg() {
        let bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_content_type(&bytes), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_png() {
        let bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_content_type(&bytes), Some("image/png"));
    }

    #[test]
    fn test_detect_webp() {
        let bytes = [
            0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
        ];
        assert_eq!(detect_content_type(&bytes), Some("image/webp"));
    }

    #[test]
    fn test_detect_unknown() {
        let bytes = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_content_type(&bytes), None);
    }

    #[test]
    fn test_detect_too_short() {
        let bytes = [0xFF, 0xD8];
        assert_eq!(detect_content_type(&bytes), None);
    }
}
