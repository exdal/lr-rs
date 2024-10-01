use ash::{ext::debug_utils, vk, Entry};
use core::ffi;
use std::{error::Error, fmt::Debug};
use winit::{raw_window_handle::HasDisplayHandle, window};

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
            return Some(i.clone());
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
                return Some(i.clone());
            } else {
                index = Some(i.clone());
            }
        }
    }

    index
}

#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum QueueType {
    Graphics = 0,
    Transfer = 1,
    Compute = 2,
}

pub struct PhysicalDevice {
    entry: Entry,
    pub instance: ash::Instance,
    pub handle: vk::PhysicalDevice,
    pub queue_type_indices: [usize; 3],
    pub properties: vk::PhysicalDeviceProperties,
}

impl PhysicalDevice {
    pub unsafe fn new(window: &window::Window) -> Result<Self, Box<dyn Error>> {
        let mut instance_extensions =
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?.to_vec();

        #[cfg(debug_assertions)]
        instance_extensions.push(debug_utils::NAME.as_ptr());

        let app_name = ffi::CStr::from_bytes_with_nul_unchecked(b"Lorr\0");
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .engine_name(app_name)
            .api_version(vk::make_api_version(0, 1, 3, 0));

        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .flags(vk::InstanceCreateFlags::default());

        let entry = Entry::load().expect("Cannot load Vulkan library");
        let instance: ash::Instance = entry
            .create_instance(&instance_info, None)
            .expect("Cannot create Vulkan Instance");
        let physical_devices = instance
            .enumerate_physical_devices()
            .expect("Cannot get Vulkan Physical Device");
        let mut physical_devices_by_score = physical_devices.iter().enumerate().collect::<Box<_>>();
        physical_devices_by_score.sort_unstable_by(|(_, lhs), (_, rhs)| {
            let lhs_props = instance.get_physical_device_properties(**lhs);
            let rhs_props = instance.get_physical_device_properties(**rhs);

            let lhs_score = type_score(lhs_props.device_type);
            let rhs_score = type_score(rhs_props.device_type);
            lhs_score.cmp(&rhs_score)
        });
        let (idx, _) = physical_devices_by_score[0];

        let handle = physical_devices[idx];
        let properties = instance.get_physical_device_properties(handle);
        let queue_family_properties = instance
            .get_physical_device_queue_family_properties(handle)
            .into_iter()
            .enumerate()
            .collect::<Box<_>>();

        let mut queue_type_indices: [usize; 3] = [0; 3];
        queue_type_indices[QueueType::Graphics as usize] =
            get_first_queue_index(queue_family_properties.as_ref(), vk::QueueFlags::GRAPHICS)
                .expect("Graphics queue not found");
        queue_type_indices[QueueType::Compute as usize]  =
            get_separate_queue_index(&queue_family_properties.as_ref(), vk::QueueFlags::COMPUTE, vk::QueueFlags::TRANSFER)
                .expect("Compute queue not found");
        queue_type_indices[QueueType::Transfer as usize] =
            get_separate_queue_index(queue_family_properties.as_ref(), vk::QueueFlags::TRANSFER, vk::QueueFlags::COMPUTE)
                .expect("Transfer queue not found");


        Ok(Self {
            entry,
            instance,
            handle,
            queue_type_indices,
            properties,
        })
    }

    pub unsafe fn create_device(&self) -> Result<ash::Device, Box<dyn Error>> {
        let queue_priorities = [1.0];
        let mut queue_create_infos = Vec::new();
        for queue_family_index in self.queue_type_indices {
            let queue_create_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&queue_priorities);

            queue_create_infos.push(queue_create_info);
        }

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
        let vk10_features = vk::PhysicalDeviceFeatures::default()
            .shader_int64(true);
        let mut device_features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vk11_features)
            .push_next(&mut vk12_features)
            .push_next(&mut vk13_features)
            .features(vk10_features);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(queue_create_infos.as_ref())
            .enabled_features(&vk10_features)
            .push_next(&mut device_features);

        let device: ash::Device =
            self.instance.create_device(self.handle, &device_create_info, None)
                .expect("Failed to create device");

        Ok(device)
    }
}
