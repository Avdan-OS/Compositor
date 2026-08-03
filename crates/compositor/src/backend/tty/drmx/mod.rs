use std::{ops::Deref, os::fd::AsFd, sync::Arc};

use smithay::{
    backend::drm::DrmNode,
    reexports::{
        calloop,
        drm::{self, Device, control::Device as _},
    },
};

pub mod atomic_req;
pub mod format;
pub mod format_modifier;

pub use format::Format;

#[derive(Debug)]
pub struct Card(std::fs::File, DrmNode);

impl AsFd for Card {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for Card {}
impl drm::control::Device for Card {}

const GRAPHICS_CARDS: &str = "/dev/dri/card*";

impl Card {
    fn new(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        Ok(Self(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path.as_ref())?,
            DrmNode::from_path(path)?,
        ))
    }

    pub fn major(&self) -> u32 {
        self.1.major()
    }

    pub fn minor(&self) -> u32 {
        self.1.minor()
    }

    pub fn find_primary() -> anyhow::Result<Self> {
        let overridden = match std::env::var("NADVA_DRM_DEVICE") {
            Ok(path) => Some(std::path::PathBuf::from(path)),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(anyhow::anyhow!(
                    "Non-unicode string passed into `NADVA_DRM_DEVICE`"
                ));
            }
        };

        let is_overridden = overridden.is_some();

        let iter = match overridden {
            Some(overridden) => {
                Box::new(std::iter::once(Ok(overridden))) as Box<dyn Iterator<Item = _>>
            }
            None => Box::new(glob::glob(GRAPHICS_CARDS).unwrap()),
        };

        let mut cards = iter
            .filter_map(|res| res.map_err(anyhow::Error::from).and_then(Card::new).ok())
            .collect::<Vec<_>>();

        cards.retain(|card| {
            match card.resource_handles() {
                Ok(resource_handles) if !resource_handles.crtcs().is_empty()
                // Try to set Atomic and UniversalPlanes (we need this for atomic modesetting, and separate cursor, overlay planes)
                && let Ok(()) = card.set_client_capability(drm::ClientCapability::Atomic, true)
                && let Ok(()) = card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
                // Rule out GPUs without any displays connected to them for now.
                && resource_handles.connectors().iter().any(|&con| card.get_connector(con, true).is_ok_and(|con| con.state() == drm::control::connector::State::Connected)) => true,
                _ => false,
            }
        });

        match cards.len() {
            0 if is_overridden => {
                return Err(anyhow::anyhow!(
                    "device specified by NADVA_DRM_DEVICE is not usable: either no connected connectors, and/or no (atomic + universal plane capability)"
                ));
            }
            0 => return Err(anyhow::anyhow!("No suitable graphics devices found! either no connected connectors, and/or no (atomic + universal plane capability)")),
            1 => return Ok(cards.pop().unwrap()),
            _ => (),
        }

        // Tie-breaking by using the boot VGA device.

        let Some((i, _)) = cards.iter().enumerate().find(|(_, card)| {
            let (major, minor) = (card.1.major(), card.1.minor());
            std::fs::read_to_string(format!("/sys/dev/char/{major}:{minor}/device/boot_vga"))
                .is_ok_and(|s| s.trim() == "1")
        }) else {
            // Or just return the first device...
            return Ok(cards.swap_remove(0));
        };

        Ok(cards.swap_remove(i))
    }
}

pub struct DrmEventNotifier<R> {
    token: Option<calloop::Token>,
    card: Arc<R>,
}

impl<R: Deref<Target = Card>> DrmEventNotifier<R> {
    pub fn new(card: Arc<R>) -> Self {
        Self { token: None, card }
    }
}

impl<R: Deref<Target = Card>> calloop::EventSource for DrmEventNotifier<R> {
    type Event = drm::control::Event;
    type Metadata = ();
    type Ret = ();
    type Error = std::io::Error;

    fn process_events<F>(
        &mut self,
        _: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> Result<calloop::PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        if Some(token) != self.token {
            return Ok(calloop::PostAction::Continue);
        }

        log::trace!("Pooling for DRM events...");
        let events = self.card.receive_events()?;
        events.for_each(|event| callback(event, &mut ()));

        Ok(calloop::PostAction::Continue)
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.token = Some(factory.token());

        // Safety: the FD cannot be closed without removing the DrmDeviceNotifier from the event loop
        unsafe {
            poll.register(
                self.card.as_fd(),
                calloop::Interest::READ,
                calloop::Mode::Level,
                self.token.unwrap(),
            )
        }
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.token = Some(factory.token());
        poll.reregister(
            self.card.as_fd(),
            calloop::Interest::READ,
            calloop::Mode::Level,
            self.token.unwrap(),
        )
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.token = None;
        poll.unregister(self.card.as_fd())
    }
}
