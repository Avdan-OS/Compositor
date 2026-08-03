use std::{collections::HashSet, ffi::CStr, ops::Deref, os::fd::{AsRawFd, FromRawFd, OwnedFd}, sync::Arc};

use smithay::reexports::ash::{self, vk::{self, Handle}};

use crate::backend::tty::{Card, drmx, vulkan::VulkanError};

#[cfg(debug_assertions)]
const REQUIRED_INSTANCE_EXTENSIONS: &[&CStr] = &[ash::ext::debug_utils::NAME];
#[cfg(debug_assertions)]
const REQUIRED_INSTANCE_LAYERS: &[&CStr] = &[c"VK_LAYER_KHRONOS_validation"];

const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[
    vk::EXT_PHYSICAL_DEVICE_DRM_NAME,
    ash::khr::external_memory_fd::NAME,
    vk::EXT_EXTERNAL_MEMORY_DMA_BUF_NAME,
    vk::EXT_IMAGE_DRM_FORMAT_MODIFIER_NAME,
    ash::khr::external_fence_fd::NAME,
    ash::khr::external_semaphore_fd::NAME,
];

fn supports<'a, E>(
    supported: impl IntoIterator<Item = Result<&'a CStr, E>>,
    required: &'a [&'a CStr],
) -> Result<Vec<*const i8>, Vec<&'a CStr>> {
    let supported = supported
        .into_iter()
        .filter_map(Result::ok)
        .collect::<HashSet<_>>();

    let required = required.iter().copied().collect::<HashSet<_>>();

    let remaining = required.difference(&supported).copied().collect::<Vec<_>>();

    if remaining.is_empty() {
        return Ok(required.iter().copied().map(CStr::as_ptr).collect());
    }

    Err(remaining)
}

const APP_NAME: &CStr = c"Nadva";
const APP_VERSION: u32 = {
    let Ok(major) = u32::from_str_radix(env!("CARGO_PKG_VERSION_MAJOR"), 10) else {
        panic!("Major version not set!")
    };
    let Ok(minor) = u32::from_str_radix(env!("CARGO_PKG_VERSION_MINOR"), 10) else {
        panic!("Major version not set!")
    };
    let Ok(patch) = u32::from_str_radix(env!("CARGO_PKG_VERSION_PATCH"), 10) else {
        panic!("Major version not set!")
    };

    vk::make_api_version(0, major, minor, patch)
};

pub const MAX_VULKAN_VERSION: u32 = vk::make_api_version(0, 1, 3, 0);
const MINIMUM_VULKAN_VERSION: u32 = vk::make_api_version(0, 1, 1, 0);

pub struct Instance {
    // We're putting this in here for this to be destroyed before instance.
    #[cfg(debug_assertions)]
    #[cfg_attr(debug_assertions, expect(unused))]
    debug: DebugUtils,

    instance: RawInstance,

    // We're putting this in here for this to be dropped last.
    pub entry: ash::Entry,
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance.0
    }
}

struct RawInstance(ash::Instance);

impl Drop for RawInstance {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_instance(None);
        }
    }
}

#[cfg(debug_assertions)]
struct DebugUtils {
    instance: ash::ext::debug_utils::Instance,
    messenger: ash::vk::DebugUtilsMessengerEXT,
}

#[cfg(debug_assertions)]
impl Drop for DebugUtils {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .destroy_debug_utils_messenger(self.messenger, None);
        }
    }
}


impl Instance {
    pub fn load() -> Result<Self, VulkanError> {
        // SAFETY: We will not use functions on this instance after destroying it.
        let entry = unsafe { ash::Entry::load()? };

        let (raw_version, version) = unsafe {
            entry
                .try_enumerate_instance_version()?
                .map(|v| {
                    (
                        v,
                        (
                            vk::api_version_major(v),
                            vk::api_version_minor(v),
                            vk::api_version_patch(v),
                            vk::api_version_variant(v),
                        ),
                    )
                })
                .unwrap_or((0, (1, 0, 0, 0)))
        };

        log::trace!("Vulkan Version: {version:?}");

        if raw_version < MINIMUM_VULKAN_VERSION {
            return Err(
                format!("Your driver's Vulkan version ({0}.{1}.{2}) is below the 1.1.x minimum.",
                version.0,
                version.1,
                version.2
            ).into());
        }

        #[cfg(debug_assertions)]
        let (layers, extensions, mut debug_create_info) = {
            let layers = {
                let supported_layers = unsafe { entry.enumerate_instance_layer_properties()? };
                supports(
                    supported_layers.iter().map(|a| a.layer_name_as_c_str()),
                    REQUIRED_INSTANCE_LAYERS,
                )
                .map_err(|_| {
                        VulkanError::from("Missing layer: VK_LAYER_KHRONOS_validation".to_string())
                })?
            };
            let extensions = {
                let supported_extensions =
                    unsafe { entry.enumerate_instance_extension_properties(None)? };
                supports(
                    supported_extensions
                        .iter()
                        .map(|a| a.extension_name_as_c_str()),
                    REQUIRED_INSTANCE_EXTENSIONS,
                )
                .map_err(|_| {
                    VulkanError::from("Missing layer: VK_EXT_debug_utils".to_string())
                })?
            };

            let debug_create_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity({
                    use vk::DebugUtilsMessageSeverityFlagsEXT as Flags;
                    Flags::ERROR | Flags::WARNING | Flags::INFO | Flags::VERBOSE
                })
                .message_type({
                    use vk::DebugUtilsMessageTypeFlagsEXT as Flags;
                    Flags::GENERAL | Flags::VALIDATION | Flags::PERFORMANCE
                })
                .pfn_user_callback(Some(callback));

            (layers, extensions, debug_create_info)
        };

        #[cfg(not(debug_assertions))]
        let [layers, extensions]: [Vec<*const i8>; _] = const { [Vec::new(), Vec::new()] };

        let app_info = vk::ApplicationInfo::default()
            .application_name(APP_NAME)
            .application_version(APP_VERSION)
            .api_version(MAX_VULKAN_VERSION);

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers);

        #[cfg(debug_assertions)]
        let create_info = create_info.push_next(&mut debug_create_info);

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        #[cfg(debug_assertions)]
        let (debug_utils, debug_messenger) = {
            let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let debug_messenger =
                unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None)? };

            (debug_utils, debug_messenger)
        };

        Ok(Self {
            #[cfg(debug_assertions)]
            debug: DebugUtils {
                instance: debug_utils,
                messenger: debug_messenger,
            },
            instance: RawInstance(instance),
            entry,
        })
    }

    pub fn device_for(self, card: &Card) -> Result<Device, VulkanError> {
        let instance = self;
        let devices = unsafe { instance.enumerate_physical_devices()? };

        let (api_version, physical_device) = devices
            .into_iter()
            .filter(|&device| {
                let Ok(extensions) =
                    (unsafe { instance.enumerate_device_extension_properties(device) })
                else {
                    return false;
                };

                extensions.iter().any(|ext| {
                    ext.extension_name_as_c_str()
                        .is_ok_and(|ext| ext == vk::EXT_PHYSICAL_DEVICE_DRM_NAME)
                })
            })
            .find_map(|device| {
                let mut drm_props = vk::PhysicalDeviceDrmPropertiesEXT::default();
                let mut props = vk::PhysicalDeviceProperties2::default().push_next(&mut drm_props);

                // SAFETY: We checked that this physical device has the DRM extension.
                unsafe { instance.get_physical_device_properties2(device, &mut props) };

                let api_version = props.properties.api_version;
                let found =  drm_props.has_primary == vk::TRUE
                    && drm_props.primary_major == i64::from(card.major())
                    && drm_props.primary_minor == i64::from(card.minor());
            
                found.then_some((api_version, device))
            })
            .ok_or_else(||  VulkanError::from("your graphics device does not support Vulkan, or you are using a multi-GPU setup which is not supported".to_string()))?;

        if api_version < MAX_VULKAN_VERSION {
            log::warn!(
                "graphics device only supports Vulkan {}.{}, but 1.3 is required",
                vk::api_version_major(api_version),
                vk::api_version_minor(api_version),
            );
        }

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let Some(graphics_family_idx) = queue_families
            .iter()
            .position(|queue| queue.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        else {
            return Err( VulkanError::from(
                "Cannot find GRAPHICS queue family on the (physical) graphics device.".to_string()
            ));
        };

        let exts = unsafe { instance.enumerate_device_extension_properties(physical_device)? };
        let exts = match supports(
            exts.iter().map(|a| a.extension_name_as_c_str()),
            REQUIRED_DEVICE_EXTENSIONS,
        ) {
            Ok(exts) => exts,
            Err(missing) => {
                return Err( VulkanError::from(
                    format!("your graphics device is missing the following Vulkan device extensions: {missing:?}")
                ));
            }
        };

        // Check for the fancy Vulkan 1.2, 1.3 features.
        {
            let mut check12 = vk::PhysicalDeviceVulkan12Features::default();
            let mut check13 = vk::PhysicalDeviceVulkan13Features::default();
            let mut feats2 = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut check12)
                .push_next(&mut check13);

            unsafe { instance.get_physical_device_features2(physical_device, &mut feats2) };
            if check12.timeline_semaphore != vk::TRUE
                || check13.synchronization2 != vk::TRUE
                || check13.dynamic_rendering != vk::TRUE
            {
                return Err( VulkanError::from(
                    "device lacks required Vulkan 1.2 and/or 1.3 features: timeline_semaphore, synchronization2, dynamic_rendering".to_string()
                ));
            }
        }

        let features = vk::PhysicalDeviceFeatures::default();

        let mut vk12 = vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);

        let mut vk13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);

        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(graphics_family_idx as u32)
            .queue_priorities(&[1.0]);

        let info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&exts)
            .enabled_features(&features)
            .push_next(&mut vk12)
            .push_next(&mut vk13);

        let device = unsafe { instance.create_device(physical_device, &info, None)? };

        let queue = unsafe { device.get_device_queue(graphics_family_idx as u32, 0) };

        let external_semaphore_api = ash::khr::external_semaphore_fd::Device::new(
            &instance, 
            &device
        );

        let external_fence_api = ash::khr::external_fence_fd::Device::new(
            &instance, 
            &device
        );

        Ok(Device {
            device: RawDevice(device),
            physical: physical_device,
            queue,
            queue_family: graphics_family_idx as u32,
            instance,
            external_semaphore_api,
            external_fence_api
        })
    }
}

pub struct Device {
    device: RawDevice,
    pub queue: vk::Queue,
    pub physical: vk::PhysicalDevice,
    pub queue_family: u32,

    pub instance: Instance,

    pub external_semaphore_api: ash::khr::external_semaphore_fd::Device,
    pub external_fence_api: ash::khr::external_fence_fd::Device,
}

pub struct RawDevice(ash::Device);

impl Drop for RawDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.device_wait_idle();
            self.0.destroy_device(None);
        }
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device.0
    }
}

pub struct ExportableSemaphore {
    device: Arc<Device>, 
    semaphore: vk::Semaphore,
    skia: skia_safe::gpu::ganesh::BackendSemaphore,
}

impl Drop for ExportableSemaphore {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        unsafe { 
            self.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

impl Deref for ExportableSemaphore {
    type Target = vk::Semaphore;

    fn deref(&self) -> &Self::Target {
        &self.semaphore
    }
}

impl ExportableSemaphore {
    pub fn new(device: Arc<Device>) -> ash::prelude::VkResult<Self> {
        let semaphore = {
            let mut export = vk::ExportSemaphoreCreateInfo::default()
                .handle_types(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
            let info = vk::SemaphoreCreateInfo::default().push_next(&mut export);

            (unsafe { device.create_semaphore(&info, None) })?
        };

        // SAFETY: This semaphore will outlive the GrBackendSemaphore.
        let skia = unsafe { skia_safe::gpu::ganesh::vk::backend_semaphores::make_vk(semaphore.as_raw() as *mut _) };

        Ok(Self { semaphore, device, skia })
    }

    pub fn skia(&mut self) -> &mut [skia_safe::gpu::ganesh::BackendSemaphore] {
        core::slice::from_mut(&mut self.skia)
    }

    pub fn as_slice(&self) -> &[vk::Semaphore] {
        core::slice::from_ref(&self.semaphore)
    }

    pub fn sync_fd(&self) -> ash::prelude::VkResult<Option<OwnedFd>> {
        let sync_fd = {
            let info = vk::SemaphoreGetFdInfoKHR::default()
                .semaphore(self.semaphore)
                .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);

            unsafe {
                self.device.external_semaphore_api.get_semaphore_fd(&info)?
            }
        };

        // Okay, the Vulkan spec is a bit unclear about what to do here...
        // See https://github.com/KhronosGroup/Vulkan-Docs/issues/2452
        //
        // So, we'll just do another [Option] just in case some drivers do whatever.
        if sync_fd == -1 {
            return Ok(None); 
        }

        unsafe {
            // SAFETY: Vulkan transfers ownership of this file descriptor to us,
            //         and we have checked that this is indeed a valid file descriptor.
            Ok(Some(OwnedFd::from_raw_fd(sync_fd)))
        }
    }
}

pub struct ImportableFence {
    device: Arc<Device>,
    fence: vk::Fence
}

impl Drop for ImportableFence {
    fn drop(&mut self) {
        unsafe { self.device.destroy_fence(self.fence, None); }
    }
}


impl Deref for ImportableFence {
    type Target = vk::Fence;

    fn deref(&self) -> &Self::Target {
        &self.fence
    }
}


impl ImportableFence {
    pub fn new(device: Arc<Device>) -> ash::prelude::VkResult<Self> {
        let fence = {
            let info = vk::FenceCreateInfo::default()
                .flags(vk::FenceCreateFlags::SIGNALED);

            unsafe { device.create_fence(&info, None)? }
        };


        Ok(Self {
            device,
            fence,
        })
    }

    pub fn import(&mut self, fd: OwnedFd) -> ash::prelude::VkResult<()> {
            let raw_fd = fd.as_raw_fd();
            let info = vk::ImportFenceFdInfoKHR::default()
                .fence(self.fence)
                .fd(raw_fd)
                .handle_type(vk::ExternalFenceHandleTypeFlags::SYNC_FD)
                .flags(vk::FenceImportFlags::TEMPORARY);

            unsafe {
                // self.device.reset_fences(self.as_slice())?;
                self.device.external_fence_api.import_fence_fd(&info)?;
            }

            // Only transfer ownership of the fd to Vulkan if the call is successful.
            std::mem::forget(fd);

            Ok(())
    } 

    pub fn as_slice(&self) -> &[vk::Fence] {
        core::slice::from_ref(&self.fence)
    }
}

pub struct CommandPool {
    device: Arc<Device>,
    pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(device: Arc<Device>) -> ash::prelude::VkResult<Self> {
        let info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(device.queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        Ok(CommandPool {
            pool: unsafe { device.create_command_pool(&info, None) }?,
            device,
        })
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.pool, None);
        }
    }
}
impl std::ops::Deref for CommandPool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}


#[cfg(debug_assertions)]
unsafe extern "system" fn callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _: *mut core::ffi::c_void,
) -> u32 {
    use std::borrow::Cow;
    use vk::DebugUtilsMessageSeverityFlagsEXT as Severity;
    use vk::DebugUtilsMessageTypeFlagsEXT as Type;

    let Some(callback_data) = (unsafe { p_callback_data.as_ref() }) else {
        return vk::FALSE;
    };

    // SAFETY: Spec guarantees this to be a valid (non-null) UTF-8 string (null terminated).
    let message = unsafe { CStr::from_ptr(callback_data.p_message).to_string_lossy() };

    let message_id = if callback_data.p_message_id_name.is_null() {
        Cow::Borrowed("")
    } else {
        unsafe { CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
    };

    let mut kind = String::new();
    if message_types.is_empty() {
        kind.push_str(" UNKNOWN");
    }
    if message_types.contains(Type::GENERAL) {
        kind.push_str(" GENERAL");
    }
    if message_types.contains(Type::PERFORMANCE) {
        kind.push_str(" PERFORMANCE");
    }
    if message_types.contains(Type::VALIDATION) {
        kind.push_str(" VALIDATION");
    }

    let kind = &kind[1..];

    match message_severity {
        Severity::ERROR => log::error!("[{kind}] [{message_id}]: {message}"),
        Severity::WARNING => log::warn!("[{kind}] [{message_id}]: {message}"),
        Severity::INFO => log::info!("[{kind}] [{message_id}]: {message}"),
        Severity::VERBOSE => log::trace!("[{kind}] [{message_id}]: {message}"),
        _ => log::debug!("[{kind}] [{message_id}]: {message}"),
    }

    vk::FALSE
}

impl From<drmx::Format> for vk::Format {
    fn from(value: drmx::Format) -> Self {
        use smithay::reexports::gbm::Format as G;
        use vk::Format as V;
        match value.0 {
            G::Xrgb8888 => V::B8G8R8A8_UNORM,
            G::Argb8888 => V::B8G8R8A8_UNORM,
            G::Xbgr8888 => V::R8G8B8A8_UNORM,
            G::Abgr8888 => V::R8G8B8A8_UNORM,

            // HDR
            G::Xbgr2101010 => V::A2B10G10R10_UNORM_PACK32,
            G::Abgr2101010 => V::A2B10G10R10_UNORM_PACK32,
            G::Abgr16161616f => V::R16G16B16A16_SFLOAT,
            
            G::Xrgb2101010 => V::A2R10G10B10_UNORM_PACK32,
            G::Argb2101010 => V::A2R10G10B10_UNORM_PACK32,
            G::Argb16161616f => V::R16G16B16A16_SFLOAT,
            
            G::Rgb565 => V::R5G6B5_UNORM_PACK16,
            _ => panic!("unsupported color format"),
        }
    }
}