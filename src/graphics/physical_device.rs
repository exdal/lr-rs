use ash::{ext, khr, vk, Entry};
use core::ffi;
use std::error::Error;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window,
};

use super::{CommandType, Surface};

fn type_score(t: vk::PhysicalDeviceType) -> usize {
    match t {
        vk::PhysicalDeviceType::DISCRETE_GPU => 20,
        vk::PhysicalDeviceType::INTEGRATED_GPU => 10,
        vk::PhysicalDeviceType::VIRTUAL_GPU => 5,
        vk::PhysicalDeviceType::CPU => 1,
        _ => 0,
    }
}

fn get_first_queue_index(
    queue_family_properties: &[(usize, vk::QueueFamilyProperties)],
    desired_flags: vk::QueueFlags,
) -> Option<usize> {
    for (i, prop) in queue_family_properties {
        if (prop.queue_flags & desired_flags) == desired_flags {
            return Some(*i);
        }
    }

    None
}

fn get_separate_queue_index(
    queue_family_properties: &[(usize, vk::QueueFamilyProperties)],
    desired_flags: vk::QueueFlags,
    undesired_flags: vk::QueueFlags,
) -> Option<usize> {
    let mut index: Option<usize> = None;

    for (i, prop) in queue_family_properties {
        if ((prop.queue_flags & desired_flags) == desired_flags)
            && (prop.queue_flags & vk::QueueFlags::GRAPHICS).as_raw() == 0
        {
            if (prop.queue_flags & undesired_flags).as_raw() == 0 {
                return Some(*i);
            } else {
                index = Some(*i);
            }
        }
    }

    index
}

pub struct PhysicalDevice {
    entry: Entry,
    pub instance: ash::Instance,
    pub handle: vk::PhysicalDevice,
    pub queue_type_indices: [usize; 3],
    pub properties: vk::PhysicalDeviceProperties,
}

impl PhysicalDevice {
    pub fn new() -> Result<Self, vk::Result> {
        let app_name = unsafe { ffi::CStr::from_bytes_with_nul_unchecked(b"Lorr\0") };
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .engine_name(app_name)
            .api_version(vk::make_api_version(0, 1, 3, 0));

        let instance_extensions = [
            ext::debug_utils::NAME.as_ptr(),
            ext::debug_report::NAME.as_ptr(),
            khr::surface::NAME.as_ptr(),
            #[cfg(target_os = "linux")]
            khr::xlib_surface::NAME.as_ptr(),
            #[cfg(target_os = "windows")]
            khr::win32_surface::NAME.as_ptr(),
            khr::get_physical_device_properties2::NAME.as_ptr(),
        ];

        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .flags(vk::InstanceCreateFlags::default());

        let entry = unsafe { Entry::load().expect("Cannot load Vulkan library") };
        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&instance_info, None)
                .expect("Cannot create Vulkan Instance")
        };
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Cannot get Vulkan Physical Device")
        };

        let mut physical_devices_by_score = physical_devices.iter().enumerate().collect::<Box<_>>();
        physical_devices_by_score.sort_unstable_by(|(_, lhs), (_, rhs)| {
            let lhs_props = unsafe { instance.get_physical_device_properties(**lhs) };
            let rhs_props = unsafe { instance.get_physical_device_properties(**rhs) };

            let lhs_score = type_score(lhs_props.device_type);
            let rhs_score = type_score(rhs_props.device_type);
            lhs_score.cmp(&rhs_score)
        });
        let (idx, _) = physical_devices_by_score[0];

        let handle = physical_devices[idx];
        let properties = unsafe { instance.get_physical_device_properties(handle) };
        let queue_family_properties = unsafe {
            instance
                .get_physical_device_queue_family_properties(handle)
                .into_iter()
                .enumerate()
                .collect::<Box<_>>()
        };

        let mut queue_type_indices: [usize; 3] = [0; 3];
        queue_type_indices[CommandType::Graphics as usize] =
            get_first_queue_index(queue_family_properties.as_ref(), vk::QueueFlags::GRAPHICS)
                .expect("Graphics queue not found");
        queue_type_indices[CommandType::Compute as usize] = get_separate_queue_index(
            queue_family_properties.as_ref(),
            vk::QueueFlags::COMPUTE,
            vk::QueueFlags::TRANSFER,
        )
        .expect("Compute queue not found");
        queue_type_indices[CommandType::Transfer as usize] = get_separate_queue_index(
            queue_family_properties.as_ref(),
            vk::QueueFlags::TRANSFER,
            vk::QueueFlags::COMPUTE,
        )
        .expect("Transfer queue not found");

        Ok(Self {
            entry,
            instance,
            handle,
            queue_type_indices,
            properties,
        })
    }

    pub fn create_device(&self) -> Result<ash::Device, vk::Result> {
        let queue_priorities = [1.0];
        let mut queue_create_infos = Vec::new();
        for queue_family_index in self.queue_type_indices {
            let queue_create_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&queue_priorities);

            queue_create_infos.push(queue_create_info);
        }

        let extensions = [khr::swapchain::NAME.as_ptr()];

        let mut vk13_features = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);
        let mut vk12_features = vk::PhysicalDeviceVulkan12Features::default()
            .descriptor_indexing(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .shader_storage_buffer_array_non_uniform_indexing(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .descriptor_binding_variable_descriptor_count(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_partially_bound(true)
            .runtime_descriptor_array(true)
            .timeline_semaphore(true)
            .buffer_device_address(true)
            .host_query_reset(true);
        let mut vk11_features = vk::PhysicalDeviceVulkan11Features::default()
            .variable_pointers(true)
            .variable_pointers_storage_buffer(true);
        let vk10_features = vk::PhysicalDeviceFeatures::default().shader_int64(true);
        let mut device_features = vk::PhysicalDeviceFeatures2::default()
            .features(vk10_features)
            .push_next(&mut vk11_features)
            .push_next(&mut vk12_features)
            .push_next(&mut vk13_features)
            .features(vk10_features);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(queue_create_infos.as_ref())
            .enabled_extension_names(&extensions)
            .push_next(&mut device_features);

        let device: ash::Device = unsafe {
            self.instance
                .create_device(self.handle, &device_create_info, None)
                .expect("Failed to create device")
        };

        Ok(device)
    }

    pub fn create_surface(&self, window: &window::Window) -> Result<Surface, Box<dyn Error>> {
        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
            .expect("Failed to create surface")
        };

        let surface_loader = khr::surface::Instance::new(&self.entry, &self.instance);
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(self.handle, surface)
                .expect("Failed to get physical device surface formats")
        };
        let surface_capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(self.handle, surface)
                .expect("Failed to get physical device surface capabilities")
        };
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(self.handle, surface)
                .expect("Failed to get physical device present modes")
        };

        Ok(Surface {
            capabilities: surface_capabilities,
            formats: surface_formats,
            present_modes,
            handle: surface,
        })
    }
}
