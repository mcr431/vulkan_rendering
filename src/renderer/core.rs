use std::sync::{Arc, RwLock};
use std::ops::DerefMut;
use hal::adapter::{MemoryType, PhysicalDevice};
use hal::Instance;
use hal::queue::QueueFamily;

pub(crate) struct RendererCore<B: hal::Backend> {
    instance: B::Instance,
    pub backend: GfxBackend<B>,
    pub device: GfxDevice<B>,
}

impl RendererCore<back::Backend> {
    pub fn new(size: winit::dpi::LogicalSize<f64>, event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        unsafe {
            let window_builder = winit::window::WindowBuilder::new()
                .with_title("sxe")
                .with_inner_size(size);
            let (mut backend, instance) = create_backend(window_builder, event_loop);

            let device = GfxDevice::new(
                backend.adapter.adapter.take().unwrap(),
                backend.surface.read().unwrap().as_ref().unwrap(),
            );

            Self {
                instance,
                backend,
                device,
            }
        }
    }
}

pub(crate) fn run_with_device<T, B: hal::Backend>(core: &Arc<RwLock<RendererCore<B>>>, func: impl FnOnce(&mut B::Device) -> T) -> T {
    let mut device_lock = Arc::clone(&core.write().unwrap().device.device);
    let mut raw_device = device_lock.write().unwrap();
    func(raw_device.deref_mut())
}

impl <B: hal::Backend> Drop for RendererCore<B> {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_surface(self.backend.surface.write().unwrap().take().unwrap())
        }
    }
}

pub(crate) struct GfxAdapter<B: hal::Backend> {
    pub adapter: Option<hal::adapter::Adapter<B>>,
    pub memory_types: Vec<MemoryType>,
    pub limits: hal::Limits,
}

impl <B: hal::Backend> GfxAdapter<B> {
    fn new(adapters: &mut Vec<hal::adapter::Adapter<B>>) -> Self {
        match Self::pick_best_adapter(adapters) {
            Some(adapter) => Self::create_adapter_state(adapter),
            None => panic!("Failed to pick an adapter")
        }
    }

    fn pick_best_adapter(adapters: &mut Vec<hal::adapter::Adapter<B>>) -> Option<hal::adapter::Adapter<B>> {
        if adapters.is_empty() {
            return None;
        }

        // TODO -> smarter adapter selection
        return Some(adapters.remove(0));
    }

    fn create_adapter_state(adapter: hal::adapter::Adapter<B>) -> Self {
        let memory_types = adapter.physical_device.memory_properties().memory_types;
        let limits = adapter.physical_device.limits();

        Self {
            adapter: Some(adapter),
            memory_types,
            limits
        }
    }
}

pub(crate) struct GfxDevice<B: hal::Backend> {
    pub device: Arc<RwLock<B::Device>>,
    pub physical_device: B::PhysicalDevice,
    pub queue_group: hal::queue::QueueGroup<B>,
    pub queue_family_id: Option<hal::queue::family::QueueFamilyId>,
}

impl <B: hal::Backend> GfxDevice<B> {
    unsafe fn new(adapter: hal::adapter::Adapter<B>, surface: &dyn hal::window::Surface<B>) -> Self {
        let family = adapter
            .queue_families
            .iter()
            .find(|family|
                surface.supports_queue_family(family) && family.queue_type().supports_graphics())
            .unwrap();

        #[cfg(not(feature = "vulkan"))]
        let family_id = None;

        #[cfg(feature = "vulkan")]
        let family_id = {
            let queue_family_any = family as &dyn std::any::Any;
            let back_queue_family: &back::QueueFamily = queue_family_any.downcast_ref().unwrap();
            Some(back_queue_family.id())
        };

        let mut gpu = adapter
            .physical_device
            .open(&[(family, &[1.0])], hal::Features::empty())
            .unwrap();

        Self {
            device: Arc::new(RwLock::new(gpu.device)),
            physical_device: adapter.physical_device,
            queue_group: gpu.queue_groups.pop().unwrap(),
            queue_family_id: family_id,
        }
    }
}

pub(crate) struct GfxBackend<B: hal::Backend> {
    pub surface: Arc<RwLock<Option<B::Surface>>>,
    pub adapter: GfxAdapter<B>,

    #[cfg(any(feature = "vulkan", feature = "dx11", feature = "dx12", feature = "metal"))]
    #[allow(dead_code)]
    pub window: winit::window::Window,
}

impl <B: hal::Backend> GfxBackend<B> {
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }
}

#[cfg(not(any(feature="gl", feature="dx12", feature="vulkan", feature="metal")))]
fn create_backend<B: hal::Backend>(window_builder: winit::window::WindowBuilder, event_loop: &winit::event_loop::EventLoop<()>) -> (GfxBackend<back::Backend>, ()) {
    panic!("You must specify one of the valid backends using --features=<backend>, with \"gl\", \"dx12\", \"vulkan\", and \"metal\" being valid backends.");
}

#[cfg(feature="gl")]
fn create_backend(window_builder: winit::window::WindowBuilder, event_loop: &winit::event_loop::EventLoop<()>) -> (GfxBackend<back::Backend>, ()) {
    let (mut adapters, mut surface) = {
        let window = {
            let builder = back::config_context(back::glutin::ContextBuilder::new(), Rgba8Srgb::SELF, None).with_vsync(true);
            back::glutin::GlWindow::new(wb, builder, &events_loop).unwrap()
        };

        let surface = back::Surface::from_window(window);
        let adapters = surface.enumerate_adapters();
        (apaters, surface)
    };

    let backend_state = GfxBackend {
        surface: Arc::new(RwLock::new(Some(surface))),
        adapter: GfxAdapter::new(adapters),
    };

    (backend_state, ())
}

#[cfg(any(feature="dx12", feature="vulkan", feature="metal"))]
fn create_backend(window_builder: winit::window::WindowBuilder, event_loop: &winit::event_loop::EventLoop<()>) -> (GfxBackend<back::Backend>, back::Instance) {
    let window = window_builder
        .build(event_loop)
        .unwrap();

    let instance = back::Instance::create("matthew's spectacular rendering engine", 1).expect("failed to create an instance");
    let surface = unsafe { instance.create_surface(&window).expect("Failed to create a surface") };
    let mut adapters = instance.enumerate_adapters();

    let backend_state = GfxBackend {
        surface: Arc::new(RwLock::new(Some(surface))),
        adapter: GfxAdapter::new(&mut adapters),
        window
    };

    (backend_state, instance)
}
