use smithay::reexports::gbm;

#[derive(Debug, Clone, Copy)]
pub struct Format(pub gbm::Format);

impl Format {
    pub fn fourcc(&self) -> gbm::Format {
        self.0
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        use gbm::Format as G;

        #[rustfmt::skip]
        const FORMATS: &[gbm::Format] = &[G::C8,G::R8,G::R16,G::Rg88,G::Gr88,G::Rg1616,G::Gr1616,G::Bgr233,G::Rgb332,G::Xrgb4444,G::Xbgr4444,G::Rgbx4444,G::Bgrx4444,G::Argb4444,G::Abgr4444,G::Rgba4444,G::Bgra4444,G::Xrgb1555,G::Xbgr1555,G::Rgbx5551,G::Bgrx5551,G::Argb1555,G::Abgr1555,G::Rgba5551,G::Bgra5551,G::Rgb565,G::Bgr565,G::Rgb888,G::Bgr888,G::Xrgb8888,G::Xbgr8888,G::Rgbx8888,G::Bgrx8888,G::Argb8888,G::Abgr8888,G::Rgba8888,G::Bgra8888,G::Xrgb2101010,G::Xbgr2101010,G::Rgbx1010102,G::Bgrx1010102,G::Argb2101010,G::Abgr2101010,G::Rgba1010102,G::Bgra1010102,G::Yuyv,G::Yvyu,G::Uyvy,G::Vyuy,G::Ayuv,G::Xrgb8888_a8,G::Nv12,G::Nv21,G::Nv16,G::Nv61,G::Nv24,G::Nv42,G::Yuv410,G::Yvu410,G::Yuv411,G::Yvu411,G::Yuv420,G::Yvu420,G::Yuv422,G::Yvu422,G::Yuv444,G::Yvu444,G::Abgr16161616f,G::Argb16161616f,G::Axbxgxrx106106106106,G::Bgr565_a8,G::Bgr888_a8,G::Bgrx8888_a8,G::Big_endian,G::Nv15,G::P010,G::P012,G::P016,G::P210,G::Q401,G::Q410,G::Rgb565_a8,G::Rgb888_a8,G::Rgbx8888_a8,G::Vuy101010,G::Vuy888,G::X0l0,G::X0l2,G::Xbgr16161616f,G::Xbgr8888_a8,G::Xrgb16161616f,G::Xvyu12_16161616,G::Xvyu16161616,G::Xvyu2101010,G::Xyuv8888,G::Y0l0,G::Y0l2,G::Y210,G::Y212,G::Y216,G::Y410,G::Y412,G::Y416,G::Yuv420_10bit,G::Yuv420_8bit];
        FORMATS.iter().copied().map(Self)
    }

    pub fn skia(&self) -> Option<skia_safe::ColorType> {
        TryInto::try_into(*self).ok()
    }

    pub fn rank(&self) -> usize {
        match self.0 {
            gbm::Format::Argb2101010 => 0,
            gbm::Format::Abgr2101010 => 0,
            gbm::Format::Bgra1010102 => 0,
            gbm::Format::Rgba1010102 => 0,
            
            gbm::Format::Bgrx1010102 => 1,
            gbm::Format::Rgbx1010102 => 1,
            gbm::Format::Xbgr2101010 => 1,
            gbm::Format::Xrgb2101010 => 1,

            gbm::Format::Rgba8888 => 2,
            gbm::Format::Argb8888 => 2,
            gbm::Format::Bgra8888 => 2,
            gbm::Format::Bgrx8888 => 2,
            gbm::Format::Abgr8888 => 2,
            gbm::Format::Rgbx8888 => 2,
            gbm::Format::Xbgr8888 => 2,
            gbm::Format::Xrgb8888 => 2,

            gbm::Format::Abgr16161616f => 3,
            gbm::Format::Argb16161616f => 3,
            gbm::Format::Xrgb16161616f => 3,
            gbm::Format::Xbgr16161616f => 3,

            gbm::Format::Abgr1555 => usize::MAX,
            gbm::Format::Abgr4444 => usize::MAX,
            gbm::Format::Argb1555 => usize::MAX,
            gbm::Format::Argb4444 => usize::MAX,
            gbm::Format::Axbxgxrx106106106106 => usize::MAX,
            gbm::Format::Ayuv => usize::MAX,
            gbm::Format::Bgr233 => usize::MAX,
            gbm::Format::Bgr565 => usize::MAX,
            gbm::Format::Bgr565_a8 => usize::MAX,
            gbm::Format::Bgr888 => usize::MAX,
            gbm::Format::Bgr888_a8 => usize::MAX,
            gbm::Format::Bgra4444 => usize::MAX,
            gbm::Format::Bgra5551 => usize::MAX,
            gbm::Format::Bgrx4444 => usize::MAX,
            gbm::Format::Bgrx5551 => usize::MAX,
            gbm::Format::Bgrx8888_a8 => usize::MAX,
            gbm::Format::Big_endian => usize::MAX,
            gbm::Format::C8 => usize::MAX,
            gbm::Format::Gr1616 => usize::MAX,
            gbm::Format::Gr88 => usize::MAX,
            gbm::Format::Nv12 => usize::MAX,
            gbm::Format::Nv15 => usize::MAX,
            gbm::Format::Nv16 => usize::MAX,
            gbm::Format::Nv21 => usize::MAX,
            gbm::Format::Nv24 => usize::MAX,
            gbm::Format::Nv42 => usize::MAX,
            gbm::Format::Nv61 => usize::MAX,
            gbm::Format::P010 => usize::MAX,
            gbm::Format::P012 => usize::MAX,
            gbm::Format::P016 => usize::MAX,
            gbm::Format::P210 => usize::MAX,
            gbm::Format::Q401 => usize::MAX,
            gbm::Format::Q410 => usize::MAX,
            gbm::Format::R16 => usize::MAX,
            gbm::Format::R8 => usize::MAX,
            gbm::Format::Rg1616 => usize::MAX,
            gbm::Format::Rg88 => usize::MAX,
            gbm::Format::Rgb332 => usize::MAX,
            gbm::Format::Rgb565 => usize::MAX,
            gbm::Format::Rgb565_a8 => usize::MAX,
            gbm::Format::Rgb888 => usize::MAX,
            gbm::Format::Rgb888_a8 => usize::MAX,
            gbm::Format::Rgba4444 => usize::MAX,
            gbm::Format::Rgba5551 => usize::MAX,
            gbm::Format::Rgbx4444 => usize::MAX,
            gbm::Format::Rgbx5551 => usize::MAX,
            gbm::Format::Rgbx8888_a8 => usize::MAX,

            gbm::Format::Uyvy => usize::MAX,
            gbm::Format::Vuy101010 => usize::MAX,
            gbm::Format::Vuy888 => usize::MAX,
            gbm::Format::Vyuy => usize::MAX,
            gbm::Format::X0l0 => usize::MAX,
            gbm::Format::X0l2 => usize::MAX,
            gbm::Format::Xbgr1555 => usize::MAX,
            gbm::Format::Xbgr4444 => usize::MAX,
            gbm::Format::Xbgr8888_a8 => usize::MAX,
            gbm::Format::Xrgb1555 => usize::MAX,
            gbm::Format::Xrgb4444 => usize::MAX,
            gbm::Format::Xrgb8888_a8 => usize::MAX,
            gbm::Format::Xvyu12_16161616 => usize::MAX,
            gbm::Format::Xvyu16161616 => usize::MAX,
            gbm::Format::Xvyu2101010 => usize::MAX,
            gbm::Format::Xyuv8888 => usize::MAX,
            gbm::Format::Y0l0 => usize::MAX,
            gbm::Format::Y0l2 => usize::MAX,
            gbm::Format::Y210 => usize::MAX,
            gbm::Format::Y212 => usize::MAX,
            gbm::Format::Y216 => usize::MAX,
            gbm::Format::Y410 => usize::MAX,
            gbm::Format::Y412 => usize::MAX,
            gbm::Format::Y416 => usize::MAX,
            gbm::Format::Yuv410 => usize::MAX,
            gbm::Format::Yuv411 => usize::MAX,
            gbm::Format::Yuv420 => usize::MAX,
            gbm::Format::Yuv420_10bit => usize::MAX,
            gbm::Format::Yuv420_8bit => usize::MAX,
            gbm::Format::Yuv422 => usize::MAX,
            gbm::Format::Yuv444 => usize::MAX,
            gbm::Format::Yuyv => usize::MAX,
            gbm::Format::Yvu410 => usize::MAX,
            gbm::Format::Yvu411 => usize::MAX,
            gbm::Format::Yvu420 => usize::MAX,
            gbm::Format::Yvu422 => usize::MAX,
            gbm::Format::Yvu444 => usize::MAX,
            gbm::Format::Yvyu => usize::MAX,
        }
    }
}

impl TryInto<skia_safe::ColorType> for Format {
    type Error = ();
    fn try_into(self) -> Result<skia_safe::ColorType, ()> {
        use gbm::Format as G;
        use skia_safe::ColorType as S;

        match self.0 {
            // Color index
            G::C8 => Err(()),

            // 8 bpp red
            G::R8 => Ok(S::R8UNorm),

            // 16 bpp red
            G::R16 => Ok(S::R16UNorm),

            // 16 bpp RG
            G::Rg88 => Err(()),
            G::Gr88 => Ok(S::R8G8UNorm),

            // 32 bpp RG
            G::Rg1616 => Err(()),
            G::Gr1616 => Ok(S::R16G16UNorm),

            // 8 bpp RGB
            G::Bgr233 => Err(()),
            G::Rgb332 => Err(()),

            // 16 bpp RGB
            G::Xrgb4444 => Err(()),
            G::Xbgr4444 => Err(()),
            G::Rgbx4444 => Ok(S::ARGB4444),
            G::Bgrx4444 => Err(()),

            G::Argb4444 => Err(()),
            G::Abgr4444 => Err(()),
            G::Rgba4444 => Ok(S::ARGB4444),
            G::Bgra4444 => Err(()),

            G::Xrgb1555 => Err(()),
            G::Xbgr1555 => Err(()),
            G::Rgbx5551 => Err(()),
            G::Bgrx5551 => Err(()),

            G::Argb1555 => Err(()),
            G::Abgr1555 => Err(()),
            G::Rgba5551 => Err(()),
            G::Bgra5551 => Err(()),

            G::Rgb565 => Ok(S::RGB565),
            G::Bgr565 => Err(()),

            // 24 bpp RGB
            G::Rgb888 => Err(()),
            G::Bgr888 => Err(()),

            /* 32 bpp RGB */
            G::Xrgb8888 => Ok(S::BGRA8888),
            G::Xbgr8888 => Ok(S::RGB888x),
            G::Rgbx8888 => Err(()),
            G::Bgrx8888 => Err(()),

            G::Argb8888 => Ok(S::BGRA8888),
            G::Abgr8888 => Ok(S::RGBA8888),
            G::Rgba8888 => Err(()),
            G::Bgra8888 => Err(()),

            G::Xrgb2101010 => Ok(S::BGR101010x),
            G::Xbgr2101010 => Ok(S::RGB101010x),
            G::Rgbx1010102 => Err(()),
            G::Bgrx1010102 => Err(()),

            G::Argb2101010 => Ok(S::BGRA1010102),
            G::Abgr2101010 => Ok(S::RGBA1010102),
            G::Rgba1010102 => Err(()),
            G::Bgra1010102 => Err(()),

            /* packed YCbCr */
            G::Yuyv => Err(()),
            G::Yvyu => Err(()),
            G::Uyvy => Err(()),
            G::Vyuy => Err(()),

            G::Ayuv => Err(()),

            /*
             * 2 plane RGB + A
             * index 0 = RGB plane, same format as the corresponding non _A8 format has
             * index 1 = A plane, [7:0] A
             * index 0 = Y plane, [7:0] Y
             * index 1 = Cr:Cb plane, [15:0] Cr:Cb little endian
             * or
             * index 1 = Cb:Cr plane, [15:0] Cb:Cr little endian
             */
            G::Xrgb8888_a8 => Err(()),
            G::Nv12 => Err(()),
            G::Nv21 => Err(()),
            G::Nv16 => Err(()),
            G::Nv61 => Err(()),
            G::Nv24 => Err(()),
            G::Nv42 => Err(()),

            /*
             * 3 plane YCbCr
             * index 0: Y plane, [7:0] Y
             * index 1: Cb plane, [7:0] Cb
             * index 2: Cr plane, [7:0] Cr
             * or
             * index 1: Cr plane, [7:0] Cr
             * index 2: Cb plane, [7:0] Cb
             */
            G::Yuv410 => Err(()),
            G::Yvu410 => Err(()),
            G::Yuv411 => Err(()),
            G::Yvu411 => Err(()),
            G::Yuv420 => Err(()),
            G::Yvu420 => Err(()),
            G::Yuv422 => Err(()),
            G::Yvu422 => Err(()),
            G::Yuv444 => Err(()),
            G::Yvu444 => Err(()),

            G::Abgr16161616f => Err(()),
            G::Argb16161616f => Err(()),
            G::Axbxgxrx106106106106 => Err(()),
            G::Bgr565_a8 => Err(()),
            G::Bgr888_a8 => Err(()),
            G::Bgrx8888_a8 => Err(()),
            G::Big_endian => Err(()),
            G::Nv15 => Err(()),
            G::P010 => Err(()),
            G::P012 => Err(()),
            G::P016 => Err(()),
            G::P210 => Err(()),
            G::Q401 => Err(()),
            G::Q410 => Err(()),
            G::Rgb565_a8 => Err(()),
            G::Rgb888_a8 => Err(()),
            G::Rgbx8888_a8 => Err(()),
            G::Vuy101010 => Err(()),
            G::Vuy888 => Err(()),
            G::X0l0 => Err(()),
            G::X0l2 => Err(()),
            G::Xbgr16161616f => Err(()),
            G::Xbgr8888_a8 => Err(()),
            G::Xrgb16161616f => Err(()),
            G::Xvyu12_16161616 => Err(()),
            G::Xvyu16161616 => Err(()),
            G::Xvyu2101010 => Err(()),
            G::Xyuv8888 => Err(()),
            G::Y0l0 => Err(()),
            G::Y0l2 => Err(()),
            G::Y210 => Err(()),
            G::Y212 => Err(()),
            G::Y216 => Err(()),
            G::Y410 => Err(()),
            G::Y412 => Err(()),
            G::Y416 => Err(()),
            G::Yuv420_10bit => Err(()),
            G::Yuv420_8bit => Err(()),
        }
    }
}

impl TryFrom<skia_safe::ColorType> for Format {
    type Error = ();
    fn try_from(value: skia_safe::ColorType) -> Result<Self, ()> {
        use gbm::Format as G;
        use skia_safe::ColorType as S;

        match value {
            S::Unknown => Err(()),
            S::Alpha8 => Err(()),
            S::RGB565 => Ok(Format(G::Rgb565)),
            S::ARGB4444 => Ok(Format(G::Rgba4444)),
            S::RGBA8888 => Ok(Format(G::Abgr8888)),
            S::RGB888x => Ok(Format(G::Xbgr8888)),
            S::BGRA8888 => Ok(Format(G::Argb8888)),
            S::RGBA1010102 => Ok(Format(G::Abgr2101010)),
            S::BGRA1010102 => Ok(Format(G::Argb2101010)),
            S::RGB101010x => Ok(Format(G::Xbgr2101010)),
            S::BGR101010x => Ok(Format(G::Xrgb2101010)),
            S::BGR101010xXR => Ok(Format(G::Xrgb2101010)),
            S::BGRA10101010XR => Err(()),
            S::RGBA10x6 => Err(()),
            S::Gray8 => Err(()),
            S::RGBAF16Norm => Err(()),
            S::RGBAF16 => Err(()),
            S::RGBF16F16F16x => Err(()),
            S::RGBAF32 => Err(()),
            S::R8G8UNorm => Ok(Format(G::Gr88)),
            S::A16Float => Err(()),
            S::R16Float => Err(()),
            S::R16G16Float => Err(()),
            S::A16UNorm => Err(()),
            S::R16UNorm => Ok(Format(G::R16)),
            S::R16G16UNorm => Ok(Format(G::Gr1616)),
            S::R16G16B16A16UNorm => Err(()),
            S::SRGBA8888 => Err(()),
            S::R8UNorm => Ok(Format(G::R8)),
        }
    }
}
