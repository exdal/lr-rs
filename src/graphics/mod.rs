macro_rules! define_from {
    ($from:ty, $to:ty) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                value.handle
            }
        }

        impl From<&$from> for $to {
            fn from(value: &$from) -> Self {
                value.handle
            }
        }
    };
}

macro_rules! define_from_tupl {
    ($from:ty, $to:ty, $handle_idx:tt) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                value.$handle_idx
            }
        }

        impl From<&$from> for $to {
            fn from(value: &$from) -> Self {
                value.$handle_idx
            }
        }
    };
}

mod command;
mod device;
mod gpu_resource;
mod physical_device;
mod swapchain;

pub use command::*;
pub use device::*;
pub use gpu_resource::*;
pub use physical_device::*;
pub use swapchain::*;
