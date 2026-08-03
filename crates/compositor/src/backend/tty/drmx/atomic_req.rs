use std::{
    collections::HashMap,
    num::NonZeroU64,
    ops::Deref,
    os::fd::{AsRawFd, OwnedFd},
    sync::Arc,
};

use smithay::reexports::{
    drm::{
        self,
        control::{Device, RawResourceHandle, atomic, property},
    },
    gbm,
};

pub use smithay::reexports::drm::control::AtomicCommitFlags;

use crate::backend::tty::drmx::Card;

#[derive(Debug)]
pub struct AtomicRequestCache {
    card: Arc<gbm::Device<Card>>,
    cache: HashMap<RawResourceHandle, HashMap<String, property::Handle>>,
}

pub struct AtomicRequest<'a> {
    cache: &'a mut AtomicRequestCache,
    cleanup: Vec<Cleanup>,
    request: atomic::AtomicModeReq,
}

impl<'a> Drop for AtomicRequest<'a> {
    fn drop(&mut self) {
        for cleanup in self.cleanup.drain(..) {
            let _ = cleanup.failure(&self.cache.card);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AtomicReqError {
    #[error(transparent)]
    Io(
        #[from]
        #[backtrace]
        std::io::Error,
    ),

    #[error("{0}")]
    Other(String),
}

impl From<String> for AtomicReqError {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

impl<'a> AtomicRequest<'a> {
    pub fn set<H: drm::control::ResourceHandle, P: Property<'a, H>>(
        mut self,
        target: H,
        prop: P,
        value: P::Type,
    ) -> Result<Self, AtomicReqError> {
        let atomic_value = value.to_value(&self.cache.card)?;
        self.request.add_property(
            target,
            self.cache.property_handle(target, prop)?,
            atomic_value,
        );

        self.cleanup.push(value.cleanup(atomic_value));

        Ok(self)
    }

    /// FIXME: Potential race condition where the blob is destroyed before the kernel actually applies
    ///        the commit when called with [drm::control::AtomicCommitFlags::NONBLOCK]
    pub fn commit(mut self, flags: AtomicCommitFlags) -> std::io::Result<()> {
        let request = std::mem::take(&mut self.request);

        let res = self.cache.card.atomic_commit(flags, request);

        let drop_fn = match res {
            Ok(_) => Cleanup::success,
            Err(_) => Cleanup::failure,
        };

        // Destroy any leftover resources here to prevent a resource leaks.
        let cleanups = std::mem::take(&mut self.cleanup);
        for cleanup in cleanups {
            if let Err(e) = drop_fn(cleanup, &self.cache.card) {
                log::warn!("failed to destroy property: {e}")
            }
        }

        res
    }
}

impl AtomicRequestCache {
    pub fn new(card: Arc<gbm::Device<Card>>) -> Self {
        Self {
            card,
            cache: Default::default(),
        }
    }

    pub fn property_handle<'s, H: drm::control::ResourceHandle, P: Property<'s, H>>(
        &mut self,
        target: H,
        _: P,
    ) -> Result<drm::control::property::Handle, AtomicReqError> {
        let prop = if let Some(props) = self.cache.get(&target.into()) {
            if let Some(&prop) = props.get(P::NAME) {
                prop
            } else {
                return Err(AtomicReqError::Other(format!(
                    "{} Property does not exist on this resource!",
                    P::NAME
                )));
            }
        } else {
            let map = self
                .card
                .get_properties(target)?
                .as_hashmap(self.card.deref())?
                .into_iter()
                .map(|(name, info)| (name, info.handle()))
                .collect::<HashMap<_, _>>();

            let prop = map.get(P::NAME).copied().ok_or_else(|| {
                AtomicReqError::Other(format!(
                    "{} Property does not exist on this resource!",
                    P::NAME
                ))
            })?;

            self.cache.insert(target.into(), map);

            prop
        };

        Ok(prop)
    }

    pub fn request(&mut self) -> AtomicRequest<'_> {
        AtomicRequest {
            cache: self,
            cleanup: Vec::new(),
            request: atomic::AtomicModeReq::new(),
        }
    }
}

pub trait AtomicReqType: Sized {
    fn to_value(&self, card: &Card) -> std::io::Result<property::Value<'static>>;
    fn cleanup(self, _value: property::Value<'static>) -> Cleanup {
        Cleanup::Nothing
    }
}

#[derive(Debug, Default)]
pub enum Cleanup {
    #[default]
    Nothing,
    Blob(NonZeroU64),
    Fd(OwnedFd),
}

impl Cleanup {
    pub fn success(self, card: &Card) -> std::io::Result<()> {
        match self {
            Cleanup::Nothing => Ok(()),
            Cleanup::Blob(blob) => card.destroy_property_blob(blob.get()),
            Cleanup::Fd(owned_fd) => {
                // Kernel will close the fd now.
                std::mem::forget(owned_fd);
                Ok(())
            }
        }
    }
    pub fn failure(self, card: &Card) -> std::io::Result<()> {
        match self {
            Cleanup::Nothing => Ok(()),
            Cleanup::Blob(blob) => card.destroy_property_blob(blob.get()),
            Cleanup::Fd(owned_fd) => {
                drop(owned_fd); // Close the file descriptor.
                Ok(())
            }
        }
    }
}

pub trait ToBlob {}

impl<B: ToBlob> AtomicReqType for &B {
    fn to_value(&self, card: &Card) -> std::io::Result<property::Value<'static>> {
        // This needs to be dereferenced, otherwise it will serialize just a pointer to mode!!!
        card.create_property_blob(*self)
    }

    fn cleanup(self, value: property::Value<'static>) -> Cleanup {
        let property::Value::Blob(blob) = value else {
            unreachable!()
        };

        NonZeroU64::new(blob).map_or_default(Cleanup::Blob)
    }
}

impl<B: ToBlob> AtomicReqType for Option<&B> {
    fn to_value(&self, card: &Card) -> std::io::Result<property::Value<'static>> {
        match self {
            Some(b) => b.to_value(card),
            None => Ok(property::Value::Blob(0)),
        }
    }

    fn cleanup(self, value: property::Value<'static>) -> Cleanup {
        self.map_or_default(|blob| blob.cleanup(value))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Fixed16(u64);

impl Fixed16 {
    pub const ZERO: Self = Fixed16(0);
    pub const fn integer(value: u16) -> Self {
        Fixed16((value as u64) << 16)
    }

    pub const fn fraction(f: f32) -> Option<Self> {
        if f < 0.0 || f > u16::MAX as f32 {
            return None;
        }

        let integral = f.trunc() as u16;

        const PRECISION: f32 = 1.0 / 65536.0;
        let fractional = (f.fract() / PRECISION).round().min(65535.) as u16;

        Some(Self(((integral as u64) << 16u64) | (fractional as u64)))
    }
}

impl AtomicReqType for u32 {
    fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
        Ok(property::Value::UnsignedRange(*self as u64))
    }
}

impl AtomicReqType for i32 {
    fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
        Ok(property::Value::SignedRange(*self as i64))
    }
}

impl AtomicReqType for Fixed16 {
    fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
        Ok(property::Value::UnsignedRange(self.0))
    }
}

impl AtomicReqType for bool {
    fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
        Ok(property::Value::Boolean(*self))
    }
}

impl<T> AtomicReqType for *mut T {
    fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
        Ok(property::Value::UnsignedRange(self.addr() as u64))
    }
}

macro_rules! handle_impl {
    ($($type: path => $var: ident);* $(;)?) => {
        $(
            impl AtomicReqType for $type {
                fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
                    Ok(property::Value::$var(Some(*self)))
                }
            }

            impl AtomicReqType for Option<$type> {
                fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
                    Ok(property::Value::$var(*self))
                }
            }
        )*
    };
}

handle_impl! {
    drm::control::crtc::Handle => CRTC;
    drm::control::connector::Handle => Connector;
    drm::control::framebuffer::Handle => Framebuffer;
    drm::control::plane::Handle => Plane;
    drm::control::encoder::Handle => Encoder;
}

pub trait Property<'a, T> {
    const NAME: &'static str;
    type Type: AtomicReqType + 'a;
}

macro_rules! properties {
    (
        $(
            $(#[$($attr: tt)*])*
            $prop: ident: $type: ty => $($target: path),*);* $(;)?
        ) => {
        $(
            $(#[$($attr)*])*
            #[derive(Debug, Clone, Copy)]
            pub struct $prop;

            $(
                impl Property<'_, $target> for $prop {
                    const NAME: &'static str = stringify!($prop);
                    type Type = $type;
                }
            )*

        )*
    };
}

impl AtomicReqType for Option<OwnedFd> {
    fn to_value(&self, _: &Card) -> std::io::Result<property::Value<'static>> {
        match self {
            // We will handle ownership transfer later...
            Some(fd) => Ok(property::Value::SignedRange(fd.as_raw_fd().into())),

            // As per the spec, `-1` means: no "fence" to wait on.
            None => Ok(property::Value::SignedRange(-1)),
        }
    }

    fn cleanup(self, _: property::Value<'static>) -> Cleanup {
        match self {
            None => Cleanup::Nothing,
            Some(fd) => Cleanup::Fd(fd),
        }
    }
}

#[allow(nonstandard_style)]
pub mod props {
    use std::os::fd::{OwnedFd, RawFd};

    use crate::backend::tty::drmx::atomic_req::ToBlob;

    use super::{Fixed16, Property};
    use smithay::reexports::drm;

    /// Default atomic CRTC property to set the mode for a CRTC. A 0 mode implies that the CRTC is entirely disabled
    /// - all connectors must be of and active must be set to disabled, too.
    #[derive(Debug, Clone, Copy)]
    pub struct MODE_ID;

    impl ToBlob for drm::control::Mode {}

    impl<'a> Property<'a, drm::control::crtc::Handle> for MODE_ID {
        const NAME: &'static str = stringify!(MODE_ID);
        type Type = Option<&'a drm::control::Mode>;
    }

    properties! {


        ACTIVE: bool => drm::control::crtc::Handle;


        // PLANE PROPERTIES //

        /// X coordinate offset for the source rectangle within the `drm_framebuffer`, in 16.16 fixed point ([Fixed16]). Must be positive.
        SRC_X: Fixed16 => drm::control::plane::Handle;

        /// Y coordinate offset for the source rectangle within the `drm_framebuffer`, in 16.16 fixed point ([Fixed16]). Must be positive.
        SRC_Y: Fixed16 => drm::control::plane::Handle;

        /// Width for the source rectangle within the `drm_framebuffer`, in 16.16 fixed point ([Fixed16]). [SRC_X] plus [SRC_W] must be within the width of the source framebuffer. Must be positive.
        SRC_W: Fixed16 => drm::control::plane::Handle;

        /// Height for the source rectangle within the `drm_framebuffer`, in 16.16 fixed point ([Fixed16]). [SRC_Y] plus [SRC_H] must be within the height of the source framebuffer.  Must be positive.
        SRC_H: Fixed16 => drm::control::plane::Handle;

        /// X coordinate offset for the destination rectangle. Can be negative.
        CRTC_X: i32 => drm::control::plane::Handle;

        /// Y coordinate offset for the destination rectangle. Can be negative.
        CRTC_Y: i32 => drm::control::plane::Handle;

        /// Width for the destination rectangle. [CRTC_X] plus [CRTC_W] can extend past the currently visible horizontal area of the `drm_crtc`.
        CRTC_W: u32 => drm::control::plane::Handle;

        /// Height for the destination rectangle. [CRTC_Y] plus [CRTC_H] can extend past the currently visible vertical area of the `drm_crtc`.
        CRTC_H: u32 => drm::control::plane::Handle;

        /// Mode object ID of the `drm_framebuffer` this plane should scan out.
        ///
        /// When a KMS client is performing front-buffer rendering, it should set [FB_ID] to the same front-buffer FB on each atomic commit. This implies to the driver that it needs to re-read the same FB again. Otherwise drivers which do not employ continuously repeated scanout cycles might not update the screen.
        FB_ID: Option<drm::control::framebuffer::Handle> => drm::control::plane::Handle;

        /// Use this property to pass a fence that DRM should wait on before proceeding with the Atomic Commit request and show the framebuffer for the plane on the screen.
        /// The fence can be either a normal fence or a merged one, the sync_file framework will handle both cases and use a fence_array if a merged fence is received.
        /// Passing `-1` here means no fences to wait on.
        ///
        /// If the Atomic Commit request has the [drm::control::AtomicCommitFlags::TEST_ONLY] flag it will only check if the Sync File is a valid one.
        IN_FENCE_FD: Option<OwnedFd> => drm::control::plane::Handle;

        /// Use this property to pass a file descriptor pointer to DRM. Once the Atomic Commit request call returns `OUT_FENCE_PTR` will be filled with the file descriptor number of a Sync File.
        /// This Sync File contains the CRTC fence that will be signaled when all framebuffers present on the Atomic Commit * request for that given CRTC are scanned out on the screen.
        ///
        /// The Atomic Commit request fails if a invalid pointer is passed. If the Atomic Commit request fails for any other reason the out fence fd returned will be `-1`.
        ///
        /// On a Atomic Commit with the [drm::control::AtomicCommitFlags::TEST_ONLY] flag the out fence will also be set to `-1`.
        OUT_FENCE_PTR: *mut RawFd => drm::control::crtc::Handle;

        /// CRTC that connector is attached to (atomic)
        CRTC_ID: Option<drm::control::crtc::Handle> => drm::control::plane::Handle, drm::control::connector::Handle;
    }
}

pub use props::*;
