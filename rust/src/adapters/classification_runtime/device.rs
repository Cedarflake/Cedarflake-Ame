use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{D3D12CreateDevice, ID3D12Device};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
};

pub(super) struct GpuDevice {
    pub index: i32,
    pub name: String,
    memory: usize,
}

pub(super) fn preferred_gpu() -> Option<GpuDevice> {
    // COM interfaces own their references; no pointer or device outlives this probe.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    let mut preferred: Option<GpuDevice> = None;
    for index in 0..32 {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
            break;
        };
        let Ok(description) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        if description.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        if preferred
            .as_ref()
            .is_some_and(|chosen| chosen.memory >= description.DedicatedVideoMemory)
        {
            continue;
        }
        let mut device: Option<ID3D12Device> = None;
        if unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }.is_err() {
            continue;
        }
        drop(device);
        let length = description
            .Description
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(description.Description.len());
        preferred = Some(GpuDevice {
            index: index as i32,
            name: String::from_utf16_lossy(&description.Description[..length]),
            memory: description.DedicatedVideoMemory,
        });
    }
    preferred
}
