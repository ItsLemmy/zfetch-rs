pub mod cpu;
pub mod gpu;
pub mod memory;
pub mod storage;
pub mod battery;
pub mod screen;

pub use cpu::cpu;
pub use gpu::{gpu, format_gpu_display, GpuInfo};
pub use memory::memory;
pub use storage::storage;
pub use battery::laptop_battery;
pub use screen::screen;