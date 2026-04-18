//! Enums whose discriminants are the SDK's FourCC tags. Values taken from
//! the Blackmagic RAW SDK manual (August 2025) §"Basic Types" p.22–23.

/// Decode pipeline backend. CPU is always available on Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Pipeline {
    Cpu = u32::from_be_bytes(*b"cpub"),
    Cuda = u32::from_be_bytes(*b"cuda"),
    Metal = u32::from_be_bytes(*b"metl"),
    OpenCL = u32::from_be_bytes(*b"opcl"),
}

/// Decode scale. Smaller scales produce smaller output buffers and faster
/// decodes (manual p.23, p.77). Some BRAW variants silently clamp
/// `Eighth` to `Quarter`; always trust `ProcessedImage::width()/height()`
/// over [`divisor`](Self::divisor) for actual output dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResolutionScale {
    Full = u32::from_be_bytes(*b"full"),
    Half = u32::from_be_bytes(*b"half"),
    Quarter = u32::from_be_bytes(*b"qrtr"),
    Eighth = u32::from_be_bytes(*b"eith"),
}

impl ResolutionScale {
    /// Divisor the scale *asks* the SDK to apply to each dimension.
    pub fn divisor(self) -> u32 {
        match self {
            Self::Full => 1,
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Eighth => 8,
        }
    }
}

/// Output pixel format. Values are the SDK's FourCC tags (manual p.22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResourceFormat {
    RgbaU8 = u32::from_be_bytes(*b"rgba"),
    BgraU8 = u32::from_be_bytes(*b"bgra"),
    RgbU16 = u32::from_be_bytes(*b"16il"),
    RgbaU16 = u32::from_be_bytes(*b"16al"),
    BgraU16 = u32::from_be_bytes(*b"16la"),
    RgbU16Planar = u32::from_be_bytes(*b"16pl"),
    RgbF32 = u32::from_be_bytes(*b"f32s"),
    RgbaF32 = u32::from_be_bytes(*b"f32l"),
    BgraF32 = u32::from_be_bytes(*b"f32a"),
    RgbF32Planar = u32::from_be_bytes(*b"f32p"),
    RgbF16 = u32::from_be_bytes(*b"f16s"),
    RgbaF16 = u32::from_be_bytes(*b"f16l"),
    BgraF16 = u32::from_be_bytes(*b"f16a"),
    RgbF16Planar = u32::from_be_bytes(*b"f16p"),
}

impl ResourceFormat {
    pub(crate) fn from_raw(v: u32) -> Option<Self> {
        match v {
            v if v == Self::RgbaU8 as u32 => Some(Self::RgbaU8),
            v if v == Self::BgraU8 as u32 => Some(Self::BgraU8),
            v if v == Self::RgbU16 as u32 => Some(Self::RgbU16),
            v if v == Self::RgbaU16 as u32 => Some(Self::RgbaU16),
            v if v == Self::BgraU16 as u32 => Some(Self::BgraU16),
            v if v == Self::RgbU16Planar as u32 => Some(Self::RgbU16Planar),
            v if v == Self::RgbF32 as u32 => Some(Self::RgbF32),
            v if v == Self::RgbaF32 as u32 => Some(Self::RgbaF32),
            v if v == Self::BgraF32 as u32 => Some(Self::BgraF32),
            v if v == Self::RgbF32Planar as u32 => Some(Self::RgbF32Planar),
            v if v == Self::RgbF16 as u32 => Some(Self::RgbF16),
            v if v == Self::RgbaF16 as u32 => Some(Self::RgbaF16),
            v if v == Self::BgraF16 as u32 => Some(Self::BgraF16),
            v if v == Self::RgbF16Planar as u32 => Some(Self::RgbF16Planar),
            _ => None,
        }
    }

    /// Total bytes per pixel (sum across channels). For planar formats
    /// this is the per-pixel total across all planes, not per plane.
    pub(crate) fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::RgbaU8 | Self::BgraU8 => 4,
            Self::RgbU16 | Self::RgbU16Planar | Self::RgbF16 | Self::RgbF16Planar => 6,
            Self::RgbaU16 | Self::BgraU16 | Self::RgbaF16 | Self::BgraF16 => 8,
            Self::RgbF32 | Self::RgbF32Planar => 12,
            Self::RgbaF32 | Self::BgraF32 => 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_FORMATS: &[ResourceFormat] = &[
        ResourceFormat::RgbaU8,
        ResourceFormat::BgraU8,
        ResourceFormat::RgbU16,
        ResourceFormat::RgbaU16,
        ResourceFormat::BgraU16,
        ResourceFormat::RgbU16Planar,
        ResourceFormat::RgbF32,
        ResourceFormat::RgbaF32,
        ResourceFormat::BgraF32,
        ResourceFormat::RgbF32Planar,
        ResourceFormat::RgbF16,
        ResourceFormat::RgbaF16,
        ResourceFormat::BgraF16,
        ResourceFormat::RgbF16Planar,
    ];

    #[test]
    fn resource_format_round_trip() {
        for &fmt in ALL_FORMATS {
            assert_eq!(ResourceFormat::from_raw(fmt as u32), Some(fmt));
        }
    }

    #[test]
    fn resource_format_unknown_is_none() {
        assert_eq!(ResourceFormat::from_raw(0), None);
        assert_eq!(ResourceFormat::from_raw(0xDEAD_BEEF), None);
    }

    #[test]
    fn bytes_per_pixel_covers_all_variants() {
        for &fmt in ALL_FORMATS {
            let bpp = fmt.bytes_per_pixel();
            assert!(matches!(bpp, 4 | 6 | 8 | 12 | 16), "{fmt:?} -> {bpp}");
        }
    }

    #[test]
    fn bytes_per_pixel_known_values() {
        assert_eq!(ResourceFormat::RgbaU8.bytes_per_pixel(), 4);
        assert_eq!(ResourceFormat::RgbU16.bytes_per_pixel(), 6);
        assert_eq!(ResourceFormat::RgbaU16.bytes_per_pixel(), 8);
        assert_eq!(ResourceFormat::RgbF32.bytes_per_pixel(), 12);
        assert_eq!(ResourceFormat::RgbaF32.bytes_per_pixel(), 16);
    }

    #[test]
    fn resolution_scale_divisor() {
        assert_eq!(ResolutionScale::Full.divisor(), 1);
        assert_eq!(ResolutionScale::Half.divisor(), 2);
        assert_eq!(ResolutionScale::Quarter.divisor(), 4);
        assert_eq!(ResolutionScale::Eighth.divisor(), 8);
    }

    #[test]
    fn pipeline_fourcc_values() {
        // Tags documented on manual p.22.
        assert_eq!(Pipeline::Cpu as u32, u32::from_be_bytes(*b"cpub"));
        assert_eq!(Pipeline::Cuda as u32, u32::from_be_bytes(*b"cuda"));
        assert_eq!(Pipeline::Metal as u32, u32::from_be_bytes(*b"metl"));
        assert_eq!(Pipeline::OpenCL as u32, u32::from_be_bytes(*b"opcl"));
    }
}
