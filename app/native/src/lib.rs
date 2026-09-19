#![allow(unused_imports)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

use chrono::NaiveDate;
use loose_ends_core::{
    models::{self, Direction as CoreDirection, FieldConfidence, Provenance},
    planner::{PlanAction, PlannerConfig},
    store::{NewCommitment, Store},
};

#[cfg(feature = "neural")]
use loose_ends_core::neural::{NeuralExtractor, ProvenancePath};

pub mod api;
#[cfg(feature = "frb")]
mod frb_generated;

pub struct StoreHandle {
    store: Store,
    #[cfg(feature = "neural")]
    extractor: NeuralExtractor,
}

// ===========================================================================
// JNI bridge for Android (feature-gated)
// ===========================================================================

#[cfg(feature = "jni")]
pub mod jni_sys {
    pub use jni_sys::*;
}

#[cfg(feature = "jni")]
mod jni_bridge {
    use super::*;
    use crate::jni_sys::{
        jboolean, jclass, jint, jlong, jsize, jstring, JNIEnv, JNIInvokeInterface_,
        JNINativeMethod, JavaVM, JNI_OK, JNI_VERSION_1_6,
    };

    unsafe extern "C" fn native_loose_ends_open(
        env: JNIEnv,
        _class: jclass,
        path: jstring,
    ) -> jlong {
        let path_str = jstring_to_optional_string(env, path);
        match path_str {
            Some(p) if !p.is_empty() => {
                let cstr = CString::new(p).unwrap_or_default();
                super::loose_ends_open(cstr.as_ptr()) as jlong
            }
            _ => 0,
        }
    }

    #[cfg(feature = "neural")]
    unsafe extern "C" fn native_loose_ends_extract_text(
        env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        text: jstring,
        model_path: jstring,
        year: jint,
        month: jint,
        day: jint,
    ) -> jstring {
        let text_str = jstring_to_optional_string(env, text).unwrap_or_default();
        let model_str = jstring_to_optional_string(env, model_path).unwrap_or_default();
        let text_c = CString::new(text_str).unwrap_or_default();
        let model_c = CString::new(model_str).unwrap_or_default();
        let out = super::loose_ends_extract_text(
            store_ptr as *mut _,
            text_c.as_ptr(),
            model_c.as_ptr(),
            year,
            month as u32,
            day as u32,
        );
        let result = cstr_to_owned(out).unwrap_or_default();
        if !out.is_null() {
            let _ = CString::from_raw(out);
        }
        string_to_jstring(env, &result)
    }

    unsafe extern "C" fn native_loose_ends_ingest_rules(
        env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        text: jstring,
        year: jint,
        month: jint,
        day: jint,
    ) -> jstring {
        let text_str = jstring_to_optional_string(env, text).unwrap_or_default();
        let ct = CString::new(text_str).unwrap_or_default();
        let out = super::loose_ends_ingest_rules(
            store_ptr as *mut _,
            ct.as_ptr(),
            year,
            month as u32,
            day as u32,
        );
        let result = cstr_to_owned(out);
        let _ = CString::from_raw(out);
        string_to_jstring(env, &result.unwrap_or_default())
    }

    unsafe extern "C" fn native_loose_ends_confirm_draft(
        env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        draft_id: jlong,
        description: jstring,
        direction: jstring,
        expected_date: jstring,
        party: jstring,
    ) -> jlong {
        let desc = jstring_to_optional_string(env, description);
        let dir = jstring_to_optional_string(env, direction);
        let date = jstring_to_optional_string(env, expected_date);
        let party = jstring_to_optional_string(env, party);

        let desc_c = desc
            .as_ref()
            .map(|s| CString::new(s.as_str()).unwrap_or_default().into_raw());
        let dir_c = dir
            .as_ref()
            .map(|s| CString::new(s.as_str()).unwrap_or_default().into_raw());
        let date_c = date
            .as_ref()
            .map(|s| CString::new(s.as_str()).unwrap_or_default().into_raw());
        let party_c = party
            .as_ref()
            .map(|s| CString::new(s.as_str()).unwrap_or_default().into_raw());

        let result = super::loose_ends_confirm_draft(
            store_ptr as *mut _,
            draft_id,
            desc_c.map(|p| p as *const c_char).unwrap_or(ptr::null()),
            dir_c.map(|p| p as *const c_char).unwrap_or(ptr::null()),
            date_c.map(|p| p as *const c_char).unwrap_or(ptr::null()),
            party_c.map(|p| p as *const c_char).unwrap_or(ptr::null()),
        );

        if let Some(p) = desc_c {
            unsafe { drop(CString::from_raw(p)) };
        }
        if let Some(p) = dir_c {
            unsafe { drop(CString::from_raw(p)) };
        }
        if let Some(p) = date_c {
            unsafe { drop(CString::from_raw(p)) };
        }
        if let Some(p) = party_c {
            unsafe { drop(CString::from_raw(p)) };
        }

        result
    }

    unsafe extern "C" fn native_loose_ends_list_drafts(
        _env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
    ) -> jstring {
        let out = super::loose_ends_list_drafts(store_ptr as *mut _);
        let result = cstr_to_owned(out).unwrap_or_default();
        let _ = CString::from_raw(out);
        string_to_jstring(_env, &result)
    }

    unsafe extern "C" fn native_loose_ends_create_commitment(
        env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        description: jstring,
        direction: jstring,
        expected_date: jstring,
        party: jstring,
    ) -> jlong {
        let desc = jstring_to_optional_string(env, description).unwrap_or_default();
        let dir = jstring_to_optional_string(env, direction).unwrap_or_default();
        let date = jstring_to_optional_string(env, expected_date).unwrap_or_default();
        let party = jstring_to_optional_string(env, party);
        let desc_c = CString::new(desc).unwrap_or_default();
        let dir_c = CString::new(dir).unwrap_or_default();
        let date_c = CString::new(date).unwrap_or_default();
        let party_c = party.map(|v| CString::new(v).unwrap_or_default());

        super::loose_ends_create_commitment(
            store_ptr as *mut _,
            desc_c.as_ptr(),
            dir_c.as_ptr(),
            date_c.as_ptr(),
            party_c.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null()),
        ) as jlong
    }

    unsafe extern "C" fn native_loose_ends_resolve_commitment(
        env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        commitment_id: jlong,
        note: jstring,
    ) -> jboolean {
        let note = jstring_to_optional_string(env, note);
        let note_c = note.map(|v| CString::new(v).unwrap_or_default());
        let result = super::loose_ends_resolve_commitment(
            store_ptr as *mut _,
            commitment_id,
            note_c.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null()),
        );
        result == 0
    }

    unsafe extern "C" fn native_loose_ends_snooze_commitment(
        _env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        commitment_id: jlong,
    ) -> jboolean {
        let result = super::loose_ends_snooze_commitment(store_ptr as *mut _, commitment_id);
        result == 0
    }

    #[cfg(feature = "voice")]
    unsafe extern "C" fn native_loose_ends_transcribe_wav(
        env: JNIEnv,
        _class: jclass,
        wav_path: jstring,
        model_path: jstring,
    ) -> jstring {
        let wav = jstring_to_optional_string(env, wav_path).unwrap_or_default();
        let model = jstring_to_optional_string(env, model_path).unwrap_or_default();
        let wav_c = CString::new(wav).unwrap_or_default();
        let model_c = CString::new(model).unwrap_or_default();
        let out = super::loose_ends_transcribe_wav(wav_c.as_ptr(), model_c.as_ptr());
        let result = cstr_to_owned(out).unwrap_or_default();
        if !out.is_null() {
            let _ = CString::from_raw(out);
        }
        string_to_jstring(env, &result)
    }

    unsafe extern "C" fn native_loose_ends_list_open(
        env: JNIEnv,
        _class: jclass,
        store_ptr: jlong,
        direction: jstring,
        year: jint,
        month: jint,
        day: jint,
    ) -> jstring {
        let dir_str = jstring_to_optional_string(env, direction).unwrap_or_default();
        let ct = CString::new(dir_str).unwrap_or_default();
        let out = super::loose_ends_list_open(
            store_ptr as *mut _,
            ct.as_ptr(),
            year,
            month as u32,
            day as u32,
        );
        let result = cstr_to_owned(out);
        let _ = CString::from_raw(out);
        string_to_jstring(env, &result.unwrap_or_default())
    }

    /// Registers the native bridge methods with the Android JVM and returns the JNI version.
    ///
    /// # Safety
    /// The JVM must pass a valid pointer to a JNI `JavaVM` handle and reserved argument
    /// according to the JNI invocation contract. This function is called by the JVM during
    /// native library loading.
    #[no_mangle]
    pub unsafe extern "C" fn JNI_OnLoad(vm: *mut JavaVM, _reserved: *mut c_void) -> jint {
        if vm.is_null() {
            return -1;
        }
        let mut env_ptr: *mut c_void = ptr::null_mut();
        let vm_interface = *vm;
        if vm_interface.is_null() {
            return -1;
        }
        let get_env = (*vm_interface).v1_2.GetEnv;
        if get_env(vm, &mut env_ptr as *mut *mut c_void, JNI_VERSION_1_6) != JNI_OK {
            return -1;
        }
        let env: JNIEnv = env_ptr as JNIEnv;
        if env.is_null() {
            return -1;
        }
        let class_name = CString::new("com/looseends/loose_ends/NativeBridge").unwrap();
        let find_class = (*env).v1_1.FindClass;
        let cls = find_class(env as *mut JNIEnv, class_name.as_ptr());
        if cls.is_null() {
            return -1;
        }

        let methods = [
            JNINativeMethod {
                name: c"looseEndsOpen".as_ptr().cast_mut(),
                signature: c"(Ljava/lang/String;)J".as_ptr().cast_mut(),
                fnPtr: native_loose_ends_open as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsIngestRules".as_ptr().cast_mut(),
                signature: c"(JLjava/lang/String;III)Ljava/lang/String;"
                    .as_ptr()
                    .cast_mut(),
                fnPtr: native_loose_ends_ingest_rules as *mut c_void,
            },
            #[cfg(feature = "neural")]
            JNINativeMethod {
                name: c"looseEndsExtractText".as_ptr().cast_mut(),
                signature: c"(JLjava/lang/String;Ljava/lang/String;III)Ljava/lang/String;"
                    .as_ptr()
                    .cast_mut(),
                fnPtr: native_loose_ends_extract_text as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsConfirmDraft".as_ptr().cast_mut(),
                signature:
                    c"(JJLjava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)J"
                        .as_ptr()
                        .cast_mut(),
                fnPtr: native_loose_ends_confirm_draft as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsListOpen".as_ptr().cast_mut(),
                signature: c"(JLjava/lang/String;III)Ljava/lang/String;"
                    .as_ptr()
                    .cast_mut(),
                fnPtr: native_loose_ends_list_open as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsListDrafts".as_ptr().cast_mut(),
                signature: c"(J)Ljava/lang/String;".as_ptr().cast_mut(),
                fnPtr: native_loose_ends_list_drafts as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsCreateCommitment".as_ptr().cast_mut(),
                signature:
                    c"(JLjava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)J"
                        .as_ptr()
                        .cast_mut(),
                fnPtr: native_loose_ends_create_commitment as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsResolveCommitment".as_ptr().cast_mut(),
                signature: c"(JJLjava/lang/String;)Z".as_ptr().cast_mut(),
                fnPtr: native_loose_ends_resolve_commitment as *mut c_void,
            },
            JNINativeMethod {
                name: c"looseEndsSnoozeCommitment".as_ptr().cast_mut(),
                signature: c"(JJ)Z".as_ptr().cast_mut(),
                fnPtr: native_loose_ends_snooze_commitment as *mut c_void,
            },
            #[cfg(feature = "voice")]
            JNINativeMethod {
                name: c"looseEndsTranscribeWav".as_ptr().cast_mut(),
                signature: c"(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
                    .as_ptr()
                    .cast_mut(),
                fnPtr: native_loose_ends_transcribe_wav as *mut c_void,
            },
        ];

        let register = (*env).v1_2.RegisterNatives;
        let result = register(
            env as *mut JNIEnv,
            cls,
            methods.as_ptr(),
            methods.len() as jsize,
        );

        if result < 0 {
            return -1;
        }

        JNI_VERSION_1_6
    }

    unsafe fn jstring_to_optional_string(env: JNIEnv, jstr: jstring) -> Option<String> {
        if jstr.is_null() {
            return None;
        }
        let mut is_copy: jboolean = false;
        let get_utf_chars = (*env).v1_1.GetStringUTFChars;
        let utf = get_utf_chars(env as *mut JNIEnv, jstr, &mut is_copy);
        if utf.is_null() {
            return None;
        }
        let cstr = CStr::from_ptr(utf);
        let result = cstr.to_str().ok().map(String::from);
        let release_utf_chars = (*env).v1_1.ReleaseStringUTFChars;
        release_utf_chars(env as *mut JNIEnv, jstr, utf);
        result
    }

    unsafe fn string_to_jstring(env: JNIEnv, s: &str) -> jstring {
        let cstr = CString::new(s).unwrap_or_default();
        let new_string_utf = (*env).v1_1.NewStringUTF;
        new_string_utf(env as *mut JNIEnv, cstr.as_ptr())
    }
}

// ===========================================================================
// Shared helpers
// ===========================================================================

fn cstr_to_owned(s: *const c_char) -> Option<String> {
    if s.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(s) }.to_str().ok().map(String::from)
}

// ===========================================================================
// C ABI wrappers (for direct use / Linux desktop)
// ===========================================================================

/// Opens a persistent store at the UTF-8 filesystem path provided by the caller.
///
/// # Safety
/// `path` must be non-null and point to a valid NUL-terminated C string that remains
/// readable for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_open(path: *const c_char) -> *mut StoreHandle {
    let cstr = unsafe { CStr::from_ptr(path) };
    let path_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    match Store::open(path_str) {
        Ok(store) => {
            let handle = StoreHandle {
                store,
                #[cfg(feature = "neural")]
                extractor: NeuralExtractor::default(),
            };
            Box::into_raw(Box::new(handle))
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Creates an in-memory store and returns an opaque handle owned by the caller.
///
/// # Safety
/// The returned handle must later be passed exactly once to `loose_ends_close` and
/// must not be used after it has been closed.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_open_in_memory() -> *mut StoreHandle {
    match Store::open_in_memory() {
        Ok(store) => {
            let handle = StoreHandle {
                store,
                #[cfg(feature = "neural")]
                extractor: NeuralExtractor::default(),
            };
            Box::into_raw(Box::new(handle))
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Closes a store handle previously returned by one of the open functions.
///
/// # Safety
/// `handle` must be null or a valid, live `StoreHandle` pointer returned by this
/// library and must not have been closed already.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_close(handle: *mut StoreHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

/// Frees a string returned by this C API.
///
/// # Safety
/// `s` must be null or a pointer returned by a string-returning function in this
/// library that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Ingests rule text into the store and returns a newly allocated JSON report.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`. `text` must be null or point to a
/// valid NUL-terminated C string for the duration of this call. The date components
/// must form a valid calendar date.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_ingest_rules(
    handle: *mut StoreHandle,
    text: *const c_char,
    today_year: i32,
    today_month: u32,
    today_day: u32,
) -> *mut c_char {
    let handle = unsafe { &*handle };
    let text = match cstr_to_owned(text) {
        Some(t) => t,
        None => return ptr::null_mut(),
    };
    let today = match NaiveDate::from_ymd_opt(today_year, today_month, today_day) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    let report = match handle.store.ingest_text_rules(&text, today) {
        Ok(r) => r,
        Err(_) => return ptr::null_mut(),
    };

    let drafts = match handle.store.list_drafts() {
        Ok(d) => d,
        Err(_) => return ptr::null_mut(),
    };

    let json = serde_json::json!({
        "entry_source_id": report.entry_source_id,
        "drafts": drafts.into_iter().map(|d| {
            let confidence: models::Confidence = serde_json::from_str(&d.confidence_json)
                .unwrap_or(models::Confidence { party: None, date: None, overall: None });
            serde_json::json!({
                "id": d.id,
                "description": d.description,
                "direction": format!("{:?}", d.direction).to_lowercase(),
                "expected_date": d.expected_date,
                "party": d.party_guess,
                "party_confidence": match confidence.party {
                    Some(FieldConfidence::High) => "high",
                    _ => "low",
                },
                "date_confidence": match confidence.date {
                    Some(FieldConfidence::High) => "high",
                    _ => "low",
                },
                "overall_confidence": match confidence.overall {
                    Some(FieldConfidence::High) => "high",
                    _ => "low",
                },
                "source_provenance": d.source_provenance.as_str(),
            })
        }).collect::<Vec<_>>()
    });

    match CString::new(json.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Lists all pending drafts, including provenance and confidence metadata.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle` pointer returned by this library.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_list_drafts(handle: *mut StoreHandle) -> *mut c_char {
    let handle = unsafe { &*handle };
    let drafts = match handle.store.list_drafts() {
        Ok(drafts) => drafts,
        Err(_) => return ptr::null_mut(),
    };
    let json = drafts
        .into_iter()
        .map(|d| {
            let confidence: models::Confidence =
                serde_json::from_str(&d.confidence_json).unwrap_or_default();
            serde_json::json!({
                "id": d.id,
                "description": d.description,
                "direction": match d.direction {
                    models::ExtractDirection::UserOwes => "user_owes",
                    models::ExtractDirection::OwedToUser => "owed_to_user",
                    models::ExtractDirection::Unclear => "unclear",
                },
                "expected_date": d.expected_date,
                "party": d.party_guess,
                "party_confidence": match confidence.party {
                    Some(FieldConfidence::High) => "high",
                    _ => "low",
                },
                "date_confidence": match confidence.date {
                    Some(FieldConfidence::High) => "high",
                    _ => "low",
                },
                "overall_confidence": match confidence.overall {
                    Some(FieldConfidence::High) => "high",
                    _ => "low",
                },
                "source_provenance": d.source_provenance.as_str(),
                "created_at": d.created_at,
                "entry_source_id": d.entry_source_id,
            })
        })
        .collect::<Vec<_>>();

    match CString::new(serde_json::to_string(&json).unwrap_or_default()) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Confirms a draft in the store, optionally overriding its fields.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`. Each override pointer may be null;
/// otherwise it must point to a valid NUL-terminated C string for the duration of
/// this call.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_confirm_draft(
    handle: *mut StoreHandle,
    draft_id: i64,
    description_override: *const c_char,
    direction_override: *const c_char,
    date_override: *const c_char,
    party_override: *const c_char,
) -> i64 {
    let handle = unsafe { &*handle };
    let desc = cstr_to_owned(description_override);
    let dir_str = cstr_to_owned(direction_override);
    let date = cstr_to_owned(date_override);
    let party = cstr_to_owned(party_override);

    let dir = dir_str
        .as_deref()
        .map(|s| match s {
            "user_owes" => CoreDirection::UserOwes,
            "owed_to_user" => CoreDirection::OwedToUser,
            _ => CoreDirection::UserOwes,
        })
        .unwrap_or(CoreDirection::UserOwes);

    match handle.store.confirm_draft(
        draft_id,
        desc.as_deref(),
        Some(dir),
        Some(date.as_deref()),
        party.as_deref(),
    ) {
        Ok(Some(id)) => id,
        _ => 0,
    }
}

/// Resolves a commitment and records an optional resolution note.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`. `note` may be null.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_resolve_commitment(
    handle: *mut StoreHandle,
    commitment_id: i64,
    note: *const c_char,
) -> i32 {
    let handle = unsafe { &*handle };
    let note = cstr_to_owned(note);
    match handle
        .store
        .resolve_commitment(commitment_id, note.as_deref())
    {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Snoozes a commitment so the planner will not surface it as open.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_snooze_commitment(
    handle: *mut StoreHandle,
    commitment_id: i64,
) -> i32 {
    let handle = unsafe { &*handle };
    match handle.store.snooze_commitment(commitment_id) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Transcribes a local WAV file with a local Whisper model.

#[cfg(feature = "voice")]
#[no_mangle]
pub unsafe extern "C" fn loose_ends_transcribe_wav(
    wav_path: *const c_char,
    model_path: *const c_char,
) -> *mut c_char {
    let wav = match cstr_to_owned(wav_path) {
        Some(v) if !v.trim().is_empty() => v,
        _ => return ptr::null_mut(),
    };
    let model = match cstr_to_owned(model_path) {
        Some(v) if !v.trim().is_empty() => v,
        _ => return ptr::null_mut(),
    };

    match loose_ends_core::voice::transcribe_wav(wav, model) {
        Ok(text) => match CString::new(text) {
            Ok(value) => value.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Lists open commitments for the requested direction and returns JSON text.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`, and `direction` must point to a
/// valid NUL-terminated C string for the duration of this call. The date components
/// must form a valid calendar date.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_list_open(
    handle: *mut StoreHandle,
    direction: *const c_char,
    today_year: i32,
    today_month: u32,
    today_day: u32,
) -> *mut c_char {
    let handle = unsafe { &*handle };
    let dir_str = match cstr_to_owned(direction) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    let dir = match dir_str.as_str() {
        "user_owes" => CoreDirection::UserOwes,
        _ => CoreDirection::OwedToUser,
    };
    let today = match NaiveDate::from_ymd_opt(today_year, today_month, today_day) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    let cfg = PlannerConfig::default();
    match handle.store.view(dir, today, &cfg) {
        Ok(items) => {
            let json: Vec<serde_json::Value> = items
                .into_iter()
                .map(|(c, a)| {
                    serde_json::json!({
                        "id": c.id,
                        "description": c.description,
                        "direction": format!("{:?}", c.direction).to_lowercase(),
                        "expected_date": c.expected_date,
                        "party": match dir {
                            CoreDirection::UserOwes => c.owed_to,
                            CoreDirection::OwedToUser => c.owed_by,
                        },
                        "aging_action": match a {
                            PlanAction::SurfaceNow => "surface",
                            PlanAction::Snooze { .. } => "snooze",
                            PlanAction::EscalateReminder => "escalate",
                            PlanAction::Archive => "archive",
                        },
                        "created_at": c.created_at,
                    })
                })
                .collect();
            match CString::new(serde_json::to_string(&json).unwrap_or_default()) {
                Ok(s) => s.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Extracts text into persisted drafts, preferring the on-device neural model when a
/// local GGUF model path is supplied and falling back to deterministic rules.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`. `text` and `model_path` must
/// be null or valid NUL-terminated C strings for the duration of the call. The
/// date components must form a valid calendar date.
#[cfg(feature = "neural")]
#[no_mangle]
pub unsafe extern "C" fn loose_ends_extract_text(
    handle: *mut StoreHandle,
    text: *const c_char,
    model_path: *const c_char,
    today_year: i32,
    today_month: u32,
    today_day: u32,
) -> *mut c_char {
    let handle = unsafe { &*handle };
    let text = match cstr_to_owned(text) {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ptr::null_mut(),
    };
    let today = match NaiveDate::from_ymd_opt(today_year, today_month, today_day) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let model_path = cstr_to_owned(model_path).unwrap_or_default();
    let (candidates, provenance_path) =
        if !model_path.trim().is_empty() && std::path::Path::new(&model_path).is_file() {
            handle
                .extractor
                .extract_with_model_path(&text, today, &model_path)
        } else {
            (
                loose_ends_core::rules::extract_rules(&text, today)
                    .into_iter()
                    .map(|r| loose_ends_core::neural::CrossChecked {
                        description: r.description,
                        party: r.party_guess,
                        direction_symbolic: r.direction,
                        expected_date: r.expected_date.map(|d| d.to_string()),
                        confidence: r.confidence,
                    })
                    .collect(),
                ProvenancePath::RuleFallbackFailure,
            )
        };

    let src = match handle.store.add_entry_source(models::RawInputType::Text) {
        Ok(id) => id,
        Err(_) => return ptr::null_mut(),
    };
    let provenance = match provenance_path {
        ProvenancePath::Model => Provenance::ModelExtracted,
        ProvenancePath::RuleFallbackTimeout
        | ProvenancePath::RuleFallbackFailure
        | ProvenancePath::RuleFallbackBreakerOpen => Provenance::RuleExtracted,
    };

    let mut drafts = Vec::new();
    for candidate in candidates {
        let id = match handle.store.add_draft(
            &candidate.description,
            candidate.direction_symbolic,
            candidate.expected_date.as_deref(),
            candidate.party.as_deref(),
            provenance,
            &candidate.confidence,
            Some(src),
        ) {
            Ok(id) => id,
            Err(_) => return ptr::null_mut(),
        };
        drafts.push(serde_json::json!({
            "id": id,
            "description": candidate.description,
            "direction": match candidate.direction_symbolic {
                models::ExtractDirection::UserOwes => "user_owes",
                models::ExtractDirection::OwedToUser => "owed_to_user",
                models::ExtractDirection::Unclear => "unclear",
            },
            "expected_date": candidate.expected_date,
            "party": candidate.party,
            "party_confidence": match candidate.confidence.party {
                Some(FieldConfidence::High) => "high",
                _ => "low",
            },
            "date_confidence": match candidate.confidence.date {
                Some(FieldConfidence::High) => "high",
                _ => "low",
            },
            "overall_confidence": match candidate.confidence.overall {
                Some(FieldConfidence::High) => "high",
                _ => "low",
            },
            "source_provenance": provenance.as_str(),
        }));
    }
    match CString::new(serde_json::to_string(&drafts).unwrap_or_default()) {
        Ok(value) => value.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Creates a commitment from C-string inputs and returns its database identifier.
///
/// # Safety
/// `handle` must be a valid, live `StoreHandle`. Each pointer argument may be null
/// where the API permits it; otherwise it must point to a valid NUL-terminated C
/// string for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn loose_ends_create_commitment(
    handle: *mut StoreHandle,
    description: *const c_char,
    direction: *const c_char,
    expected_date: *const c_char,
    party: *const c_char,
) -> i64 {
    let handle = unsafe { &*handle };
    let desc = match cstr_to_owned(description) {
        Some(s) => s,
        None => return 0,
    };
    let dir_str = cstr_to_owned(direction);
    let dir = match dir_str.as_deref() {
        Some("user_owes") => CoreDirection::UserOwes,
        _ => CoreDirection::OwedToUser,
    };
    let date = cstr_to_owned(expected_date);
    let party = cstr_to_owned(party);

    let new = match dir {
        CoreDirection::UserOwes => NewCommitment {
            description: &desc,
            direction: dir,
            expected_date: date.as_deref(),
            owed_by_party: Some("user"),
            owed_to_party: party.as_deref(),
            provenance: Provenance::Manual,
            confidence: Default::default(),
        },
        CoreDirection::OwedToUser => NewCommitment {
            description: &desc,
            direction: dir,
            expected_date: date.as_deref(),
            owed_by_party: party.as_deref(),
            owed_to_party: Some("user"),
            provenance: Provenance::Manual,
            confidence: Default::default(),
        },
    };
    handle.store.create_commitment(new).unwrap_or_default()
}
