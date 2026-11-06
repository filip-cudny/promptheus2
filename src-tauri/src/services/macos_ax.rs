use std::ffi::c_void;
use std::ptr;

use objc2_foundation::NSString;

const AX_VALUE_CF_RANGE: u32 = 4;
const CF_NUMBER_SINT64: isize = 4;

type CFTypeRef = *const c_void;

#[repr(C)]
struct CFRange {
    location: isize,
    length: isize,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub(crate) fn AXUIElementCreateSystemWide() -> CFTypeRef;
    pub(crate) fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(element: CFTypeRef, attribute: &NSString, value: *mut CFTypeRef) -> i32;
    pub(crate) fn AXUIElementCopyParameterizedAttributeValue(
        element: CFTypeRef,
        attribute: &NSString,
        parameter: CFTypeRef,
        result: *mut CFTypeRef,
    ) -> i32;
    pub(crate) fn AXUIElementPerformAction(element: CFTypeRef, action: &NSString) -> i32;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut c_void) -> u8;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: CFTypeRef);
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFEqual(a: CFTypeRef, b: CFTypeRef) -> u8;
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFArrayGetCount(array: CFTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, index: isize) -> CFTypeRef;
    fn CFBooleanGetValue(boolean: CFTypeRef) -> u8;
    fn CFNumberGetValue(number: CFTypeRef, kind: isize, out: *mut c_void) -> u8;
}

pub(crate) struct Cf(pub(crate) CFTypeRef);

unsafe impl Send for Cf {}

impl Cf {
    pub(crate) fn owned(raw: CFTypeRef) -> Option<Self> {
        (!raw.is_null()).then_some(Self(raw))
    }

    fn retained(borrowed: CFTypeRef) -> Option<Self> {
        (!borrowed.is_null()).then(|| Self(unsafe { CFRetain(borrowed) }))
    }

    fn type_id(&self) -> usize {
        unsafe { CFGetTypeID(self.0) }
    }

    pub(crate) fn attribute(&self, name: &str) -> Option<Cf> {
        let mut value = ptr::null();
        let status = unsafe { AXUIElementCopyAttributeValue(self.0, &NSString::from_str(name), &mut value) };
        if status != 0 {
            return None;
        }
        Cf::owned(value)
    }

    pub(crate) fn string(&self) -> Option<String> {
        (self.type_id() == unsafe { CFStringGetTypeID() })
            .then(|| unsafe { &*(self.0 as *const NSString) }.to_string())
    }

    pub(crate) fn string_attribute(&self, name: &str) -> Option<String> {
        self.attribute(name)?.string()
    }

    pub(crate) fn bool_attribute(&self, name: &str) -> Option<bool> {
        let value = self.attribute(name)?;
        (value.type_id() == unsafe { CFBooleanGetTypeID() }).then(|| unsafe { CFBooleanGetValue(value.0) } != 0)
    }

    pub(crate) fn number_attribute(&self, name: &str) -> Option<i64> {
        let value = self.attribute(name)?;
        let mut number = 0i64;
        let ok = unsafe { CFNumberGetValue(value.0, CF_NUMBER_SINT64, ptr::from_mut(&mut number).cast()) };
        (ok != 0).then_some(number)
    }

    pub(crate) fn range_attribute(&self, name: &str) -> Option<(isize, isize)> {
        let value = self.attribute(name)?;
        let mut range = CFRange { location: 0, length: 0 };
        let ok = unsafe { AXValueGetValue(value.0, AX_VALUE_CF_RANGE, ptr::from_mut(&mut range).cast()) };
        (ok != 0).then_some((range.location, range.length))
    }

    pub(crate) fn children(&self) -> Vec<Cf> {
        let Some(array) = self.attribute("AXChildren") else {
            return Vec::new();
        };
        let count = unsafe { CFArrayGetCount(array.0) };
        (0..count)
            .filter_map(|index| Cf::retained(unsafe { CFArrayGetValueAtIndex(array.0, index) }))
            .collect()
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}

impl PartialEq for Cf {
    fn eq(&self, other: &Self) -> bool {
        unsafe { CFEqual(self.0, other.0) != 0 }
    }
}
