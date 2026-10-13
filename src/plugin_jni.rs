//! Each plugin binary registers only its own bridge, never legacy RustSDK.
use jni::objects::{JClass, JString};
use jni::sys::{JNI_ERR, JNI_VERSION_1_6, jint, jstring};
use jni::{JNIEnv, JavaVM, NativeMethod};
use std::ffi::c_void;

const _: () = {
    let domains = cfg!(feature = "domain-account") as usize
        + cfg!(feature = "domain-schedule") as usize
        + cfg!(feature = "domain-grades") as usize
        + cfg!(feature = "domain-campus-card") as usize
        + cfg!(feature = "storage-migration") as usize;
    assert!(
        domains == 1,
        "A plugin SO must compile exactly one native domain"
    );
};

fn string(env: &mut JNIEnv, value: &JString) -> Result<String, String> {
    env.get_string(value)
        .map(Into::into)
        .map_err(|_| "Invalid string argument".to_string())
}

fn response(env: &mut JNIEnv, value: Result<serde_json::Value, String>) -> jstring {
    let value = value.unwrap_or_else(|error| serde_json::json!({"error": error}));
    match env.new_string(value.to_string()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
    let Ok(mut env) = vm.get_env() else {
        return JNI_ERR;
    };
    macro_rules! register {
        ($class:literal, $(($name:literal, $signature:literal, $function:ident)),+ $(,)?) => {{
            let methods = [$(NativeMethod {
                name: $name.into(), sig: $signature.into(), fn_ptr: $function as *mut c_void,
            }),+];
            if env.register_native_methods($class, &methods).is_err() { return JNI_ERR; }
        }};
    }
    #[cfg(feature = "domain-account")]
    register!(
        "com/ahu/ahutong/sdk/nativebridge/AccountNative",
        (
            "login",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            login
        )
    );
    #[cfg(feature = "domain-schedule")]
    register!(
        "com/ahu/ahutong/sdk/nativebridge/ScheduleNative",
        (
            "getSchedule",
            "(Ljava/lang/String;)Ljava/lang/String;",
            get_schedule
        )
    );
    #[cfg(feature = "domain-grades")]
    register!(
        "com/ahu/ahutong/sdk/nativebridge/GradesNative",
        (
            "getGrade",
            "(Ljava/lang/String;)Ljava/lang/String;",
            get_grade
        ),
        (
            "getExamInfo",
            "(Ljava/lang/String;)Ljava/lang/String;",
            get_exam_info
        )
    );
    #[cfg(feature = "domain-campus-card")]
    register!(
        "com/ahu/ahutong/sdk/nativebridge/CampusCardNative",
        (
            "getBalance",
            "(Ljava/lang/String;)Ljava/lang/String;",
            get_balance
        ),
        (
            "getQrcode",
            "(Ljava/lang/String;)Ljava/lang/String;",
            get_qrcode
        ),
        (
            "refreshToken",
            "(Ljava/lang/String;)Ljava/lang/String;",
            refresh_token
        )
    );
    #[cfg(feature = "storage-migration")]
    register!(
        "com/ahu/ahutong/sdk/nativebridge/StorageNative",
        (
            "readStrings",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            read_strings
        ),
        (
            "listKeys",
            "(Ljava/lang/String;)Ljava/lang/String;",
            list_keys
        )
    );
    JNI_VERSION_1_6
}

#[cfg(feature = "domain-account")]
extern "system" fn login(
    mut env: JNIEnv,
    _class: JClass,
    username: JString,
    password: JString,
    cookies: JString,
) -> jstring {
    let result = (|| {
        let username = string(&mut env, &username)?;
        let password = string(&mut env, &password)?;
        let cookies = string(&mut env, &cookies)?;
        Ok(crate::plugin::login(&username, &password, &cookies))
    })();
    response(&mut env, result)
}

macro_rules! session_method {
    ($feature:literal, $name:ident) => {
        #[cfg(feature = $feature)]
        extern "system" fn $name(mut env: JNIEnv, _class: JClass, cookies: JString) -> jstring {
            let result = string(&mut env, &cookies).map(|cookies| crate::plugin::$name(&cookies));
            response(&mut env, result)
        }
    };
}
session_method!("domain-schedule", get_schedule);
session_method!("domain-grades", get_grade);
session_method!("domain-grades", get_exam_info);
session_method!("domain-campus-card", get_balance);
session_method!("domain-campus-card", get_qrcode);
session_method!("domain-campus-card", refresh_token);

#[cfg(feature = "storage-migration")]
extern "system" fn read_strings(
    mut env: JNIEnv,
    _class: JClass,
    snapshot: JString,
    requests: JString,
) -> jstring {
    let result = (|| {
        let snapshot = string(&mut env, &snapshot)?;
        let requests = string(&mut env, &requests)?;
        Ok(crate::storage_migration::read_strings(&snapshot, &requests))
    })();
    response(&mut env, result)
}

#[cfg(feature = "storage-migration")]
extern "system" fn list_keys(mut env: JNIEnv, _class: JClass, snapshot: JString) -> jstring {
    let result =
        string(&mut env, &snapshot).map(|snapshot| crate::storage_migration::list_keys(&snapshot));
    response(&mut env, result)
}
