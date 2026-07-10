use std::os::unix::io::{BorrowedFd, RawFd};

use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_shm::{self, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::{
    Connection, Dispatch, QueueHandle,
};

use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::ImageData;

struct CaptureData {
    width: u32,
    height: u32,
    stride: u32,
    format: wl_shm::Format,
    fd: Option<RawFd>,
    pool: Option<WlShmPool>,
    buffer: Option<WlBuffer>,
    ptr: *mut u8,
    size: usize,
    ready: bool,
    failed: bool,
}

impl CaptureData {
    fn new() -> Self {
        CaptureData {
            width: 0,
            height: 0,
            stride: 0,
            format: wl_shm::Format::Xrgb8888,
            fd: None,
            pool: None,
            buffer: None,
            ptr: std::ptr::null_mut(),
            size: 0,
            ready: false,
            failed: false,
        }
    }

    fn map(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                self.size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.fd.ok_or("no fd")?,
                0,
            );
            if ptr == libc::MAP_FAILED {
                return Err("mmap failed".into());
            }
            self.ptr = ptr as *mut u8;
        }
        Ok(())
    }

    fn unmap(&mut self) {
        if !self.ptr.is_null() && self.size > 0 {
            unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.size); }
            self.ptr = std::ptr::null_mut();
            self.size = 0;
        }
    }

    fn read_rgb(&self) -> Vec<u8> {
        let w = self.width as usize;
        let h = self.height as usize;
        let stride = self.stride as usize;
        let bpp = match self.format {
            wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888 => 4,
            wl_shm::Format::Rgb888 => 3,
            _ => 4,
        };

        let mut rgb = Vec::with_capacity(w * h * 3);
        for row in 0..h {
            let row_off = row * stride;
            for col in 0..w {
                let off = row_off + col * bpp;
                if off + 3 <= self.size {
                    unsafe {
                        let b = *self.ptr.add(off);
                        let g = *self.ptr.add(off + 1);
                        let r = *self.ptr.add(off + 2);
                        rgb.push(r);
                        rgb.push(g);
                        rgb.push(b);
                    }
                }
            }
        }
        rgb
    }
}

impl Drop for CaptureData {
    fn drop(&mut self) {
        self.unmap();
        if let Some(fd) = self.fd {
            unsafe { libc::close(fd); }
        }
    }
}

struct AppState {
    shm: Option<WlShm>,
    manager: Option<ZwlrScreencopyManagerV1>,
    output: Option<WlOutput>,
    capture: Option<CaptureData>,
}

impl Dispatch<WlRegistry, ()> for AppState {
    fn event(
        state: &mut AppState,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<AppState>,
    ) {
        if let wayland_client::protocol::wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_shm" => {
                    let shm = registry.bind::<WlShm, (), AppState>(name, version.min(1), qh, ());
                    state.shm = Some(shm);
                }
                "wl_output" => {
                    let output = registry.bind::<WlOutput, (), AppState>(name, version.min(1), qh, ());
                    state.output = Some(output);
                }
                "zwlr_screencopy_manager_v1" => {
                    let mgr = registry.bind::<ZwlrScreencopyManagerV1, (), AppState>(
                        name, version.min(1), qh, (),
                    );
                    state.manager = Some(mgr);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlShm, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &WlShm,
        _event: <WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<WlOutput, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &WlOutput,
        _event: <WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &WlBuffer,
        _event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<WlShmPool, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &WlShmPool,
        _event: <WlShmPool as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        let capture = _state.capture.as_mut().expect("no capture state");

        match event {
            wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                capture.width = width;
                capture.height = height;
                capture.stride = stride;
                capture.format = format.into_result().unwrap_or(wl_shm::Format::Xrgb8888);
            }
            wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Ready {
                ..
            } => {
                capture.ready = true;
            }
            wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Failed => {
                capture.failed = true;
            }
            _ => {}
        }
    }
}

pub fn capture_full() -> Result<ImageData, Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut state = AppState {
        shm: None,
        manager: None,
        output: None,
        capture: None,
    };

    let _registry = conn.display().get_registry(&qh, ());

    event_queue.roundtrip(&mut state)?;

    if state.manager.is_none() {
        return Err("compositor does not support wlr-screencopy".into());
    }
    if state.output.is_none() {
        return Err("no outputs found".into());
    }
    if state.shm.is_none() {
        return Err("no wl_shm available".into());
    }

    let output = state.output.clone().ok_or("no output")?;
    let manager = state.manager.clone().ok_or("no manager")?;

    state.capture = Some(CaptureData::new());

    let frame = manager.capture_output(0, &output, &qh, ());

    event_queue.roundtrip(&mut state)?;

    {
        let w = state.capture.as_ref().unwrap().width as usize;
        let h = state.capture.as_ref().unwrap().height as usize;
        let stride = state.capture.as_ref().unwrap().stride as usize;
        let buf_size = h * stride;

        let shm = state.shm.clone().ok_or("no shm")?;

        let fd = create_memfd(buf_size)?;

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(fd) },
            buf_size as i32,
            &qh,
            (),
        );
        let buffer = pool.create_buffer(
            0,
            w as i32,
            h as i32,
            stride as i32,
            state.capture.as_ref().unwrap().format,
            &qh,
            (),
        );

        let cap = state.capture.as_mut().unwrap();
        cap.fd = Some(fd);
        cap.pool = Some(pool);
        cap.buffer = Some(buffer);
        cap.size = buf_size;
        cap.map()?;
    }

    {
        let buffer = state.capture.as_ref().unwrap().buffer.clone().ok_or("no buffer")?;
        frame.copy(&buffer);
    }

    let max_tries = 50;
    for _ in 0..max_tries {
        event_queue.roundtrip(&mut state)?;
        let cap = state.capture.as_ref().unwrap();
        if cap.ready || cap.failed {
            break;
        }
    }

    let cap = state.capture.as_ref().unwrap();
    if cap.failed {
        return Err("screencopy failed".into());
    }
    if !cap.ready {
        return Err("screencopy timed out".into());
    }

    let rgb = cap.read_rgb();
    let width = cap.width;
    let height = cap.height;

    drop(state);

    Ok(ImageData {
        data: rgb,
        width,
        height,
    })
}

fn create_memfd(size: usize) -> Result<RawFd, Box<dyn std::error::Error>> {
    unsafe {
        let fd = libc::memfd_create(
            b"tilecap\0".as_ptr() as *const libc::c_char,
            0,
        );
        if fd < 0 {
            return Err("memfd_create failed".into());
        }
        let ret = libc::ftruncate(fd, size as i64);
        if ret < 0 {
            libc::close(fd);
            return Err("ftruncate failed".into());
        }
        Ok(fd)
    }
}
