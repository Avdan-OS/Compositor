use std::{
    backtrace::Backtrace,
    collections::HashSet,
    os::{fd::AsRawFd, unix::prelude::OwnedFd},
    sync::Arc,
    task::Poll,
};

use smithay::reexports::{
    ash::{
        self,
        vk::{self, Handle},
    },
    drm::buffer::Buffer,
    gbm,
};
use thiserror::Error;

use crate::backend::tty::{
    Backend, BackendBuffer, Card, DrmState, drmx,
    utils::Guard,
    vulkan::utils::{CommandPool, Device, ExportableSemaphore, ImportableFence, Instance},
};

pub mod utils;

pub struct Vulkan {
    device: Arc<Device>,

    #[allow(unused)]
    pool: Arc<CommandPool>,
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        // Ensure device has finished all of the queues before trying to deallocate/destroy anything.
        unsafe {
            let _ = self.device.device_wait_idle();
        }
    }
}
pub struct SkiaSurface {
    surface: skia_safe::Surface,

    #[expect(unused)] // Needs to be dropped after the SkSurface
    target: skia_safe::gpu::BackendRenderTarget,
    ctx: skia_safe::gpu::DirectContext,
}

pub struct VulkanBuffer {
    skia: SkiaSurface,
    image_view: vk::ImageView,
    image: vk::Image,

    memory: vk::DeviceMemory,

    fence: ImportableFence,
    semaphore: ExportableSemaphore,

    #[expect(unused)]
    bo: gbm::BufferObject<()>,

    device: Arc<Device>,
}

#[derive(Debug, Error)]
pub enum VulkanError {
    #[error(transparent)]
    Loading(
        #[from]
        #[backtrace]
        ash::LoadingError,
    ),

    #[error(transparent)]
    Vulkan(
        #[from]
        #[backtrace]
        ash::vk::Result,
    ),

    #[error(transparent)]
    Io(
        #[backtrace]
        #[from]
        std::io::Error,
    ),

    #[error("no supported DMA memory type can be imported into Vulkan")]
    UnsupportedDmaMemory,

    #[error(transparent)]
    Fd(
        #[from]
        #[backtrace]
        gbm::InvalidFdError,
    ),

    #[error("{reason}")]
    Other {
        reason: String,
        backtrace: Backtrace,
    },
}

impl From<String> for VulkanError {
    fn from(reason: String) -> Self {
        Self::Other {
            reason,
            backtrace: Backtrace::capture(),
        }
    }
}

impl Backend for Vulkan {
    const NAME: &str = "Vulkan";
    type Error = VulkanError;

    fn new(card: &Arc<Card>) -> Result<Self, Self::Error> {
        let instance = Instance::load()?;
        let device = Arc::new(instance.device_for(card)?);

        Ok(Self {
            pool: Arc::new(CommandPool::new(device.clone())?),
            device,
        })
    }

    fn modifiers(&self, format: drmx::Format, _: &HashSet<u64>) -> HashSet<u64> {
        let device = &self.device;

        // Get the number of available format modifiers.
        let format_modifier_len = {
            let mut modifiers = vk::DrmFormatModifierPropertiesListEXT::default();
            let mut fmt_props2 = vk::FormatProperties2::default().push_next(&mut modifiers);
            unsafe {
                device.instance.get_physical_device_format_properties2(
                    device.physical,
                    format.into(),
                    &mut fmt_props2,
                )
            };

            modifiers.drm_format_modifier_count as usize
        };
        // Then, load all the format modifiers into a list.
        let modifier_properties = {
            let mut mod_props =
                vec![vk::DrmFormatModifierPropertiesEXT::default(); format_modifier_len];
            let mut modifier_list = vk::DrmFormatModifierPropertiesListEXT::default()
                .drm_format_modifier_properties(&mut mod_props);
            let mut fmt_props2 = vk::FormatProperties2::default().push_next(&mut modifier_list);
            unsafe {
                device.instance.get_physical_device_format_properties2(
                    device.physical,
                    format.into(),
                    &mut fmt_props2,
                )
            };

            mod_props
        };
        // Only select the modifiers for formats with COLOR
        modifier_properties
            .into_iter()
            .filter(|a| {
                a.drm_format_modifier_tiling_features
                    .contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT)
            })
            .map(|a| a.drm_format_modifier)
            .collect::<HashSet<_>>()
    }

    type Buffer = VulkanBuffer;
    fn new_buffer(
        &self,
        drm: &DrmState,
        bo: gbm::BufferObject<()>,
    ) -> Result<Self::Buffer, Self::Error> {
        let format: vk::Format = drmx::Format(bo.format()).into();
        // Create vk::Image
        let image = {
            let plane_layouts = (0..bo.plane_count())
                .map(|i| i as i32)
                .map(|i| {
                    vk::SubresourceLayout::default()
                        .offset(bo.offset(i) as _)
                        .row_pitch(bo.stride_for_plane(i) as _)
                })
                .collect::<Vec<_>>();

            let mut mod_info = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
                .drm_format_modifier(bo.modifier().into())
                .plane_layouts(&plane_layouts);

            let mut ext_info = vk::ExternalMemoryImageCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

            let (width, height) = bo.size();
            let info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
                .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .push_next(&mut ext_info)
                .push_next(&mut mod_info);

            let img = unsafe { self.device.create_image(&info, None)? };

            Guard::new(img, |img| unsafe { self.device.destroy_image(img, None) })
        };

        // Use the external memory via the fd provided by GBM
        let (imported_fd, memory, alloc_size) = {
            // (A) Create a DMA file descriptor (owned by Vulkan)
            let fd = bo.fd()?;
            let fd_props = {
                let mut props = vk::MemoryFdPropertiesKHR::default();
                let ext_mem_device = ash::khr::external_memory_fd::Device::new(
                    &self.device.instance,
                    self.device.as_ref(),
                );
                unsafe {
                    ext_mem_device.get_memory_fd_properties(
                        vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                        fd.as_raw_fd(),
                        &mut props,
                    )?;
                };

                props
            };

            let reqs = unsafe { self.device.get_image_memory_requirements(*image) };

            // Get first mem_type compatible with Image and fd.
            let compatible_mem_types = reqs.memory_type_bits & fd_props.memory_type_bits;
            if compatible_mem_types == 0 {
                return Err(VulkanError::UnsupportedDmaMemory);
            }
            let mem_type_index = compatible_mem_types.trailing_zeros();

            // (A) Vulkan will take ownership over our fd.
            let mut import = vk::ImportMemoryFdInfoKHR::default()
                .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
                .fd(fd.as_raw_fd());

            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(*image);

            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(reqs.size)
                .memory_type_index(mem_type_index)
                .push_next(&mut import)
                .push_next(&mut dedicated);

            let memory = unsafe { self.device.allocate_memory(&info, None)? };

            (
                fd,
                Guard::new(memory, |memory| unsafe {
                    self.device.free_memory(memory, None)
                }),
                reqs.size,
            )
        };

        unsafe { self.device.bind_image_memory(*image, *memory, 0)? };

        // (A) Okay, now everything is good, we can forget the file descriptor here, since Vulkan will close it.
        std::mem::forget(imported_fd);

        let image_view = {
            let info = vk::ImageViewCreateInfo::default()
                .components(vk::ComponentMapping::default())
                .format(format)
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .base_mip_level(0)
                        .level_count(1),
                );

            let image_view = unsafe { self.device.create_image_view(&info, None)? };

            Guard::new(image_view, |view| unsafe {
                self.device.destroy_image_view(view, None)
            })
        };

        let skia = SkiaSurface::new(
            self,
            *image,
            *memory,
            alloc_size,
            drmx::Format(bo.format()),
            {
                let (w, h) = drm.mode.size();
                (w as u32, h as u32)
            },
        )?;

        Ok(VulkanBuffer {
            device: self.device.clone(),
            memory: memory.finish(),
            image: image.finish(),
            image_view: image_view.finish(),
            semaphore: ExportableSemaphore::new(self.device.clone())?,
            fence: ImportableFence::new(self.device.clone())?,
            bo,
            skia,
        })
    }

    fn render<F>(
        &mut self,
        buffer: &mut Self::Buffer,
        frame: usize,
        mut callback: F,
    ) -> Result<std::task::Poll<()>, Self::Error>
    where
        F: for<'a> FnMut(&'a skia_safe::Canvas),
    {
        let fence_signal = unsafe { self.device.get_fence_status(*buffer.fence)? };

        if !fence_signal {
            log::trace!("Pending!");
            return Ok(Poll::Pending);
        }

        unsafe {
            self.device.reset_fences(&[*buffer.fence])?;
        }

        {
            let canvas = buffer.skia.surface.canvas();

            callback(canvas);
        }

        {
            buffer.semaphore = ExportableSemaphore::new(self.device.clone())?;

            let info = unsafe {
                let mut semaphore =
                    skia_safe::gpu::backend_semaphores::make_vk(buffer.semaphore.as_raw() as _);
                let mut info = skia_safe::gpu::FlushInfo::default();
                info.set_signal_semaphores(core::slice::from_mut(&mut semaphore));
                info
            };

            buffer.skia.ctx.flush(&info);
            buffer.skia.ctx.submit(Some(skia_safe::gpu::SyncCpu::Yes));
        }

        // {
        //     unsafe {
        //         self.device.begin_command_buffer(
        //             buffer.cmd,
        //             &vk::CommandBufferBeginInfo::default()
        //                 .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        //         )?
        //     }

        //     let barrier = vk::ImageMemoryBarrier2::default()
        //         .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
        //         .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        //         .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        //         .old_layout(vk::ImageLayout::UNDEFINED)
        //         .new_layout(vk::ImageLayout::GENERAL)
        //         .image(buffer.image)
        //         .subresource_range(
        //             vk::ImageSubresourceRange::default()
        //                 .aspect_mask(vk::ImageAspectFlags::COLOR)
        //                 .level_count(1)
        //                 .layer_count(1),
        //         );

        //     // UNDEFINED -> GENERAL layout
        //     unsafe {
        //         self.device.cmd_pipeline_barrier2(
        //             buffer.cmd,
        //             &vk::DependencyInfo::default()
        //                 .image_memory_barriers(core::slice::from_ref(&barrier)),
        //         )
        //     };

        //     // Fill with some color.
        //     {
        //         let color = COLORS[frame % COLORS.len()];
        //         let attachment = vk::RenderingAttachmentInfo::default()
        //             .image_layout(vk::ImageLayout::GENERAL)
        //             .image_view(buffer.image_view)
        //             // RED
        //             .load_op(vk::AttachmentLoadOp::CLEAR)
        //             .store_op(vk::AttachmentStoreOp::STORE)
        //             .clear_value(vk::ClearValue {
        //                 color: vk::ClearColorValue { float32: color },
        //             });

        //         let (width, height) = buffer.size;
        //         let render = vk::RenderingInfo::default()
        //             .render_area(vk::Rect2D {
        //                 offset: vk::Offset2D { x: 0, y: 0 },
        //                 extent: vk::Extent2D {
        //                     width: width as _,
        //                     height: height as _,
        //                 },
        //             })
        //             .layer_count(1)
        //             .color_attachments(core::slice::from_ref(&attachment));

        //         unsafe {
        //             self.device.cmd_begin_rendering(buffer.cmd, &render);
        //         }
        //         unsafe { self.device.cmd_end_rendering(buffer.cmd) };
        //     }

        //     unsafe { self.device.end_command_buffer(buffer.cmd)? }
        // }

        // unsafe {
        //     self.device.queue_submit(
        //         self.device.queue,
        //         &[vk::SubmitInfo::default()
        //             .command_buffers(&[buffer.cmd])
        //             .signal_semaphores(buffer.semaphore.as_slice())],
        //         buffer.cmd_fence,
        //     )?;
        // }

        Ok(Poll::Ready(()))
    }
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

impl BackendBuffer for VulkanBuffer {
    type Backend = Vulkan;

    fn import_sync(
        &mut self,
        _: &mut Vulkan,
        fd: OwnedFd,
    ) -> Result<(), <Self::Backend as Backend>::Error> {
        self.fence.import(fd)?;
        Ok(())
    }

    fn export_sync(
        &mut self,
        _: &mut Vulkan,
    ) -> Result<Option<OwnedFd>, <Self::Backend as Backend>::Error> {
        self.semaphore.sync_fd().map_err(VulkanError::Vulkan)
    }
}

impl SkiaSurface {
    pub fn new(
        backend: &Vulkan,
        image: vk::Image,
        memory: vk::DeviceMemory,
        alloc_size: u64,
        format: drmx::Format,
        size: (u32, u32),
    ) -> Result<Self, VulkanError> {
        let get_proc = |of| unsafe {
            let fn_ptr = match of {
                skia_safe::gpu::vk::GetProcOf::Instance(instance, name) => backend
                    .device
                    .instance
                    .entry
                    .get_instance_proc_addr(vk::Instance::from_raw(instance as _), name)
                    .map(|ptr| ptr as *const core::ffi::c_void),
                skia_safe::gpu::vk::GetProcOf::Device(device, name) => backend
                    .device
                    .instance
                    .get_device_proc_addr(vk::Device::from_raw(device as _), name)
                    .map(|ptr| ptr as *const core::ffi::c_void),
            };

            fn_ptr.expect("Find Vulkan function ptr for Skia")
        };
        let mut ctx = unsafe {
            let ctx = skia_safe::gpu::vk::BackendContext::new_builder(
                backend.device.instance.handle().as_raw() as _,
                backend.device.physical.as_raw() as _,
                backend.device.handle().as_raw() as _,
                (
                    backend.device.queue.as_raw() as _,
                    backend.device.queue_family as _,
                ),
                &get_proc,
                Some(skia_safe::gpu::vk::Version::new(1, 3, 0)),
            )
            .build();
            skia_safe::gpu::direct_contexts::make_vulkan(&ctx, None)
                .ok_or("Cannot initialize direct Skia-Vulkan context".to_string())?
        };

        let target = unsafe {
            let mut info = skia_safe::gpu::vk::ImageInfo::new(
                image.as_raw() as _,
                skia_safe::gpu::vk::Alloc::from_device_memory(
                    memory.as_raw() as _,
                    0,
                    alloc_size,
                    skia_safe::gpu::vk::AllocFlag::empty(),
                ),
                skia_safe::gpu::vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT,
                skia_safe::gpu::vk::ImageLayout::UNDEFINED,
                std::mem::transmute::<vk::Format, skia_safe::gpu::vk::Format>(format.into()),
                1,
                backend.device.queue_family,
                None,
                Some(skia_safe::gpu::Protected::No),
                None,
            );

            info.image_usage_flags = (vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST)
                .as_raw();

            skia_safe::gpu::backend_render_targets::make_vk((size.0 as _, size.1 as _), &info)
        };
        let surface = skia_safe::gpu::surfaces::wrap_backend_render_target(
            &mut ctx,
            &target,
            skia_safe::gpu::SurfaceOrigin::TopLeft,
            format.try_into().unwrap(),
            None,
            None,
        )
        .expect("SkSurface from Vulkan stuff");

        Ok(Self {
            surface,
            target,
            ctx,
        })
    }
}
