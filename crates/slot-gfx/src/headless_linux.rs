//! An offscreen GL 3.3 core context on Linux through EGL, for rendering previews and the
//! render tests on a machine with no window system (Mesa's llvmpipe is enough).

use std::ffi::{c_char, c_void, CString};

use libloading::Library;

use crate::surface::{GfxError, Surface, OUT_H, OUT_W};

type Ptr = *mut c_void;

const EGL_NONE: i32 = 0x3038;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_OPENGL_BIT: i32 = 0x0008;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_OPENGL_API: u32 = 0x30A2;
const EGL_CONTEXT_MAJOR_VERSION: i32 = 0x3098;
const EGL_CONTEXT_MINOR_VERSION: i32 = 0x30FB;
const EGL_CONTEXT_OPENGL_PROFILE_MASK: i32 = 0x30FD;
const EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT: i32 = 0x0001;

type GetDisplay = unsafe extern "C" fn(Ptr) -> Ptr;
type Initialize = unsafe extern "C" fn(Ptr, *mut i32, *mut i32) -> u32;
type BindApi = unsafe extern "C" fn(u32) -> u32;
type ChooseConfig = unsafe extern "C" fn(Ptr, *const i32, *mut Ptr, i32, *mut i32) -> u32;
type CreatePbuffer = unsafe extern "C" fn(Ptr, Ptr, *const i32) -> Ptr;
type CreateContext = unsafe extern "C" fn(Ptr, Ptr, Ptr, *const i32) -> Ptr;
type MakeCurrent = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32;
type GetProcAddress = unsafe extern "C" fn(*const c_char) -> *const c_void;

pub struct HeadlessSurface {
    make_current: MakeCurrent,
    get_proc_address: GetProcAddress,
    display: Ptr,
    surface: Ptr,
    context: Ptr,
    _lib: Library,
}

fn sym<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, GfxError> {
    unsafe { lib.get::<T>(name) }
        .map(|s| *s)
        .map_err(|e| GfxError::Context(format!("egl symbol: {e}")))
}

impl HeadlessSurface {
    pub fn new() -> Result<Self, GfxError> {
        if std::env::var_os("EGL_PLATFORM").is_none() {
            std::env::set_var("EGL_PLATFORM", "surfaceless");
        }
        let lib = unsafe { Library::new("libEGL.so.1") }
            .map_err(|e| GfxError::Context(format!("libEGL: {e}")))?;
        let get_display: GetDisplay = sym(&lib, b"eglGetDisplay\0")?;
        let initialize: Initialize = sym(&lib, b"eglInitialize\0")?;
        let bind_api: BindApi = sym(&lib, b"eglBindAPI\0")?;
        let choose_config: ChooseConfig = sym(&lib, b"eglChooseConfig\0")?;
        let create_pbuffer: CreatePbuffer = sym(&lib, b"eglCreatePbufferSurface\0")?;
        let create_context: CreateContext = sym(&lib, b"eglCreateContext\0")?;
        let make_current: MakeCurrent = sym(&lib, b"eglMakeCurrent\0")?;
        let get_proc_address: GetProcAddress = sym(&lib, b"eglGetProcAddress\0")?;
        unsafe {
            let display = get_display(std::ptr::null_mut());
            if display.is_null() {
                return Err(GfxError::Context("no egl display".into()));
            }
            let (mut major, mut minor) = (0, 0);
            if initialize(display, &mut major, &mut minor) == 0 {
                return Err(GfxError::Context("eglInitialize failed".into()));
            }
            if bind_api(EGL_OPENGL_API) == 0 {
                return Err(GfxError::Context("eglBindAPI(GL) failed".into()));
            }
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_NONE,
            ];
            let mut config: Ptr = std::ptr::null_mut();
            let mut n = 0;
            if choose_config(display, attrs.as_ptr(), &mut config, 1, &mut n) == 0 || n == 0 {
                return Err(GfxError::Context("no pbuffer GL config".into()));
            }
            let size = [EGL_WIDTH, OUT_W as i32, EGL_HEIGHT, OUT_H as i32, EGL_NONE];
            let surface = create_pbuffer(display, config, size.as_ptr());
            if surface.is_null() {
                return Err(GfxError::Context("no pbuffer".into()));
            }
            let ctx_attrs = [
                EGL_CONTEXT_MAJOR_VERSION,
                3,
                EGL_CONTEXT_MINOR_VERSION,
                3,
                EGL_CONTEXT_OPENGL_PROFILE_MASK,
                EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT,
                EGL_NONE,
            ];
            let context = create_context(display, config, std::ptr::null_mut(), ctx_attrs.as_ptr());
            if context.is_null() {
                return Err(GfxError::Context("no GL 3.3 core context".into()));
            }
            if make_current(display, surface, surface, context) == 0 {
                return Err(GfxError::Context("eglMakeCurrent failed".into()));
            }
            Ok(HeadlessSurface {
                make_current,
                get_proc_address,
                display,
                surface,
                context,
                _lib: lib,
            })
        }
    }
}

impl Surface for HeadlessSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        match unsafe { (self.make_current)(self.display, self.surface, self.surface, self.context) }
        {
            0 => Err(GfxError::Context("eglMakeCurrent failed".into())),
            _ => Ok(()),
        }
    }

    fn window_size(&self) -> (u32, u32) {
        (OUT_W, OUT_H)
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        unsafe { gl::Flush() };
        Ok(())
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(sym) = CString::new(name) else {
            return std::ptr::null();
        };
        unsafe { (self.get_proc_address)(sym.as_ptr()) }
    }
}
