use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr;

use jni_sys::{
    jboolean, jclass, jint, jsize, jstring, JNIEnv, JNINativeMethod, JavaVM, JNI_OK,
    JNI_VERSION_1_6,
};

fn jstring_to_optional_string(env: JNIEnv, jstr: jstring) -> Option<String> {
    if jstr.is_null() { return None; }
    unsafe {
        let mut is_copy: jboolean = false;
        let utf = ((*env).v1_1.GetStringUTFChars)(env as *mut JNIEnv, jstr, &mut is_copy);
        if utf.is_null() { return None; }
        let result = CStr::from_ptr(utf).to_str().ok().map(String::from);
        ((*env).v1_1.ReleaseStringUTFChars)(env as *mut JNIEnv, jstr, utf);
        result
    }
}

unsafe fn string_to_jstring(env: JNIEnv, value: &str) -> jstring {
    let cstr = CString::new(value).unwrap_or_default();
    ((*env).v1_1.NewStringUTF)(env as *mut JNIEnv, cstr.as_ptr())
}

unsafe extern "C" fn native_loose_ends_transcribe_wav(
    env: JNIEnv,
    _class: jclass,
    wav_path: jstring,
    model_path: jstring,
) -> jstring {
    let wav = jstring_to_optional_string(env, wav_path).unwrap_or_default();
    let model = jstring_to_optional_string(env, model_path).unwrap_or_default();
    if wav.trim().is_empty() || model.trim().is_empty() { return ptr::null_mut(); }
    match loose_ends_core::voice::transcribe_wav(wav, model) {
        Ok(text) => string_to_jstring(env, &text),
        Err(_) => ptr::null_mut(),
    }
}

/// Registers the Whisper JNI bridge in the dedicated voice shared library.
///
/// # Safety
/// The JVM must provide a valid JavaVM pointer during native library loading,
/// and the target Kotlin class must exist with the expected JNI method signature.
#[no_mangle]
pub unsafe extern "C" fn JNI_OnLoad(vm: *mut JavaVM, _reserved: *mut c_void) -> jint {
    if vm.is_null() { return -1; }
    let vm_interface = *vm;
    if vm_interface.is_null() { return -1; }
    let mut env_ptr: *mut c_void = ptr::null_mut();
    let get_env = (*vm_interface).v1_2.GetEnv;
    if get_env(vm, &mut env_ptr as *mut *mut c_void, JNI_VERSION_1_6) != JNI_OK { return -1; }
    let env = env_ptr as JNIEnv;
    if env.is_null() { return -1; }
    let class_name = c"com/looseends/loose_ends/VoiceNativeBridge";
    let class = ((*env).v1_1.FindClass)(env as *mut JNIEnv, class_name.as_ptr());
    if class.is_null() { return -1; }
    let methods = [JNINativeMethod {
        name: c"looseEndsTranscribeWav".as_ptr().cast_mut(),
        signature: c"(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".as_ptr().cast_mut(),
        fnPtr: native_loose_ends_transcribe_wav as *mut c_void,
    }];
    let register = (*env).v1_2.RegisterNatives;
    if register(env as *mut JNIEnv, class, methods.as_ptr(), methods.len() as jsize) < 0 { return -1; }
    JNI_VERSION_1_6
}

#[cfg(test)]
mod tests {
    #[test]
    fn voice_native_crate_is_buildable() { assert_eq!(std::mem::size_of::<i32>(), 4); }
}
