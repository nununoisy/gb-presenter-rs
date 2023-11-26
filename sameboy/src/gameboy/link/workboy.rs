use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sameboy_sys::{GB_gameboy_t, time_t, GB_connect_workboy, GB_workboy_is_enabled, GB_workboy_set_key};
use crate::gameboy::link::workboy_key::WorkboyKey;
use super::LinkTarget;
use super::super::Gameboy;

extern fn workboy_set_time_callback(gb: *mut GB_gameboy_t, time: time_t) {
    unsafe {
        Gameboy::wrap(gb).set_workboy_time(UNIX_EPOCH + Duration::from_secs(time as u64));
    }
}

extern fn workboy_get_time_callback(gb: *mut GB_gameboy_t) -> time_t {
    unsafe {
        Gameboy::wrap(gb)
            .workboy_time()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as time_t
    }
}

impl Gameboy {
    pub(crate) unsafe fn connect_workboy_inner(&mut self) {
        GB_connect_workboy(self.as_mut_ptr(), Some(workboy_set_time_callback), Some(workboy_get_time_callback));
    }
}

impl Gameboy {
    pub fn connect_workboy(&mut self) {
        unsafe {
            self.disconnect();
            self.connect_inner(LinkTarget::Workboy);
        }
    }

    pub fn workboy_enabled(&mut self) -> bool {
        unsafe {
            (*(*self.inner()).link_target.lock().unwrap()) == LinkTarget::Workboy && GB_workboy_is_enabled(self.as_mut_ptr())
        }
    }

    pub fn workboy_set_key(&mut self, key: WorkboyKey) {
        if !self.workboy_enabled() {
            return;
        }

        unsafe {
            GB_workboy_set_key(self.as_mut_ptr(), key.scan_code());
        }
    }

    pub fn workboy_time(&self) -> SystemTime {
        unsafe {
            (*self.inner()).workboy_time_base.lock().unwrap().clone() + (*self.inner()).workboy_time_last_set.lock().unwrap().elapsed()
        }
    }

    pub fn set_workboy_time(&mut self, time: SystemTime) {
        unsafe {
            (*(*self.inner_mut()).workboy_time_base.lock().unwrap()) = time;
            (*(*self.inner_mut()).workboy_time_last_set.lock().unwrap()) = Instant::now();
        }
    }
}
