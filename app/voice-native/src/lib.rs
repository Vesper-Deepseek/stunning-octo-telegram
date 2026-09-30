use std::ffi::{CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use jni_sys::{jboolean, jclass, jstring, JNIEnv};

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

    // whisper.cpp / ggml allocate large buffers and raise C++ exceptions
    // (std::bad_alloc on low-memory devices) or abort on corrupt weights.
    // A Rust panic unwinding across an `extern "C"` boundary is itself UB
    // and immediately aborts the process (SIGABRT force-close), so every
    // native call must be wrapped in catch_unwind and degraded to a null
    // return that Kotlin turns into a recoverable channel error.
    let result = catch_unwind(AssertUnwindSafe(|| {
        loose_ends_core::voice::transcribe_wav(wav, model)
    }));

    match result {
        Ok(Ok(text)) => string_to_jstring(env, &text),
        Ok(Err(err)) => {
            eprintln!("[loose_ends_voice] transcription failed: {err}");
            ptr::null_mut()
        }
        Err(panic) => {
            let detail = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_string());
            eprintln!("[loose_ends_voice] caught native panic, returning null instead of crashing: {detail}");
            ptr::null_mut()
        }
    }
}

/// JNI name-based export for the Whisper bridge.
///
/// The JVM can resolve this method directly from the exported symbol, avoiding
/// manual RegisterNatives calls during library loading.
///
/// # Safety
/// The JVM must pass a valid JNI environment pointer and Java object handles,
/// and the string handles must remain valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn Java_com_looseends_loose_1ends_VoiceNativeBridge_looseEndsTranscribeWav(
    env: JNIEnv,
    class: jclass,
    wav_path: jstring,
    model_path: jstring,
) -> jstring {
    native_loose_ends_transcribe_wav(env, class, wav_path, model_path)
}

/// JNI name-based export for the `VoiceNative` object.
///
/// Without this symbol, any caller that still references `VoiceNative`
/// throws UnsatisfiedLinkError; combined with a missing .so in jniLibs it
/// previously force-closed the app the moment voice recognition started.
///
/// # Safety
/// Same JNI contract as the VoiceNativeBridge export above.
#[no_mangle]
pub unsafe extern "C" fn Java_com_looseends_loose_1ends_VoiceNative_looseEndsTranscribeWav(
    env: JNIEnv,
    class: jclass,
    wav_path: jstring,
    model_path: jstring,
) -> jstring {
    native_loose_ends_transcribe_wav(env, class, wav_path, model_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn voice_native_crate_is_buildable() { assert_eq!(std::mem::size_of::<i32>(), 4); }
}
