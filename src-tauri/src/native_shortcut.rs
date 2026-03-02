use crate::state::SharedAppState;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};

/// Check if the app has Accessibility permission
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    unsafe {
        // kAXTrustedCheckOptionPrompt key
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();

        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        extern "C" {
            fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
        }

        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    true
}

/// Request microphone permission on macOS
/// This triggers the system permission dialog if not already granted
#[cfg(target_os = "macos")]
pub fn request_microphone_permission() {
    use objc::runtime::{Class, Object, BOOL, YES};
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::mpsc;

    log::info!("Requesting microphone permission...");

    // Use a channel to wait for the async callback
    let (tx, rx) = mpsc::channel();

    unsafe {
        // Get AVCaptureDevice class
        let av_capture_device: *const Class = Class::get("AVCaptureDevice").unwrap();

        // AVMediaTypeAudio constant
        let av_media_type_audio: *const Object =
            msg_send![class!(NSString), stringWithUTF8String:b"soun\0".as_ptr()];

        // Check current authorization status first
        // 0 = notDetermined, 1 = restricted, 2 = denied, 3 = authorized
        let status: i64 = msg_send![av_capture_device, authorizationStatusForMediaType: av_media_type_audio];
        log::info!("Current microphone authorization status: {}", status);

        if status == 3 {
            log::info!("Microphone permission already granted");
            return;
        }

        if status == 1 || status == 2 {
            log::warn!("Microphone permission denied or restricted. Please enable in System Settings > Privacy & Security > Microphone");
            return;
        }

        // Status is 0 (notDetermined) - request permission
        // Create a block for the completion handler
        let tx_clone = tx.clone();
        let block = block::ConcreteBlock::new(move |granted: BOOL| {
            let _ = tx_clone.send(granted == YES);
        });
        let block = block.copy();

        // Request access
        let _: () = msg_send![av_capture_device, requestAccessForMediaType: av_media_type_audio completionHandler: &*block];
    }

    // Wait for the permission dialog result (with timeout)
    match rx.recv_timeout(std::time::Duration::from_secs(60)) {
        Ok(granted) => {
            if granted {
                log::info!("Microphone permission granted by user");
            } else {
                log::warn!("Microphone permission denied by user");
            }
        }
        Err(_) => {
            log::warn!("Microphone permission request timed out");
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone_permission() {
    // No-op on non-macOS platforms
}

/// Check microphone permission status without prompting
#[cfg(target_os = "macos")]
pub fn check_microphone_permission() -> bool {
    use objc::runtime::{Class, Object};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let av_capture_device: *const Class = Class::get("AVCaptureDevice").unwrap();
        let av_media_type_audio: *const Object =
            msg_send![class!(NSString), stringWithUTF8String:b"soun\0".as_ptr()];

        let status: i64 = msg_send![av_capture_device, authorizationStatusForMediaType: av_media_type_audio];
        status == 3 // authorized
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone_permission() -> bool {
    true
}

/// Check microphone permission (callable from frontend)
#[tauri::command]
pub fn check_microphone() -> bool {
    let has_permission = check_microphone_permission();
    log::info!("Microphone permission check: {}", has_permission);
    has_permission
}

static RIGHT_ALT_PRESSED: AtomicBool = AtomicBool::new(false);
static LISTENER_PAUSED: AtomicBool = AtomicBool::new(false);

// macOS keycodes for modifier keys
const KEYCODE_RIGHT_OPTION: i64 = 61;
const KEYCODE_LEFT_OPTION: i64 = 58;
const KEYCODE_RIGHT_CMD: i64 = 54;
const KEYCODE_LEFT_CMD: i64 = 55;
const KEYCODE_RIGHT_CTRL: i64 = 62;
const KEYCODE_LEFT_CTRL: i64 = 59;
const KEYCODE_RIGHT_SHIFT: i64 = 60;
const KEYCODE_LEFT_SHIFT: i64 = 56;
const KEYCODE_ESC: i64 = 53;

/// Check if accessibility permission is granted (callable from frontend)
#[tauri::command]
pub fn check_accessibility() -> bool {
    let has_permission = check_accessibility_permission();
    log::info!("Accessibility permission check: {}", has_permission);
    has_permission
}

/// Pause the native listener (for capture mode)
#[tauri::command]
pub fn pause_native_listener() {
    log::info!("Pausing native listener");
    LISTENER_PAUSED.store(true, Ordering::SeqCst);
}

/// Resume the native listener
#[tauri::command]
pub fn resume_native_listener() {
    log::info!("Resuming native listener");
    LISTENER_PAUSED.store(false, Ordering::SeqCst);
}

/// Supported native shortcut triggers (single modifier keys)
#[derive(Debug, Clone, PartialEq)]
pub enum NativeShortcut {
    RightOption,
    LeftOption,
    RightCmd,
    LeftCmd,
    RightCtrl,
    LeftCtrl,
    RightShift,
    LeftShift,
}

impl NativeShortcut {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rightoption" | "right option" | "rightalt" | "right alt" => {
                Some(NativeShortcut::RightOption)
            }
            "leftoption" | "left option" | "leftalt" | "left alt" => {
                Some(NativeShortcut::LeftOption)
            }
            "rightcmd" | "right cmd" | "rightcommand" | "right command" => {
                Some(NativeShortcut::RightCmd)
            }
            "leftcmd" | "left cmd" | "leftcommand" | "left command" => {
                Some(NativeShortcut::LeftCmd)
            }
            "rightctrl" | "right ctrl" | "rightcontrol" | "right control" => {
                Some(NativeShortcut::RightCtrl)
            }
            "leftctrl" | "left ctrl" | "leftcontrol" | "left control" => {
                Some(NativeShortcut::LeftCtrl)
            }
            "rightshift" | "right shift" => Some(NativeShortcut::RightShift),
            "leftshift" | "left shift" => Some(NativeShortcut::LeftShift),
            _ => None,
        }
    }

    fn keycode(&self) -> i64 {
        match self {
            NativeShortcut::RightOption => KEYCODE_RIGHT_OPTION,
            NativeShortcut::LeftOption => KEYCODE_LEFT_OPTION,
            NativeShortcut::RightCmd => KEYCODE_RIGHT_CMD,
            NativeShortcut::LeftCmd => KEYCODE_LEFT_CMD,
            NativeShortcut::RightCtrl => KEYCODE_RIGHT_CTRL,
            NativeShortcut::LeftCtrl => KEYCODE_LEFT_CTRL,
            NativeShortcut::RightShift => KEYCODE_RIGHT_SHIFT,
            NativeShortcut::LeftShift => KEYCODE_LEFT_SHIFT,
        }
    }

    /// Check if this shortcut uses the Option key (which requires checking CGEventFlagAlternate)
    fn uses_option_flag(&self) -> bool {
        matches!(self, NativeShortcut::RightOption | NativeShortcut::LeftOption)
    }

    /// Check if this shortcut uses the Command key (which requires checking CGEventFlagCommand)
    fn uses_command_flag(&self) -> bool {
        matches!(self, NativeShortcut::RightCmd | NativeShortcut::LeftCmd)
    }

    /// Check if this shortcut uses the Control key (which requires checking CGEventFlagControl)
    fn uses_control_flag(&self) -> bool {
        matches!(self, NativeShortcut::RightCtrl | NativeShortcut::LeftCtrl)
    }

    /// Check if this shortcut uses the Shift key (which requires checking CGEventFlagShift)
    fn uses_shift_flag(&self) -> bool {
        matches!(self, NativeShortcut::RightShift | NativeShortcut::LeftShift)
    }
}

/// Check if the shortcut string is a native shortcut (single modifier key)
pub fn is_native_shortcut(shortcut: &str) -> bool {
    NativeShortcut::from_string(shortcut).is_some()
}

/// Start listening for native keyboard events using Core Graphics event tap
pub fn start_native_listener(app: AppHandle, state: SharedAppState) {
    std::thread::spawn(move || {
        log::info!("Starting native keyboard listener (Core Graphics)");

        // Check accessibility permission - this will prompt if not granted
        let has_permission = check_accessibility_permission();
        if !has_permission {
            log::warn!("Accessibility permission not granted. Keyboard shortcuts will not work until permission is granted in System Settings > Privacy & Security > Accessibility");
        } else {
            log::info!("Accessibility permission granted");
        }

        // Create event tap for key events (flags changed handles modifier keys)
        // Use HID location for better keyboard event access
        let event_tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::FlagsChanged, CGEventType::KeyDown],
            {
                let app = app.clone();
                let state = state.clone();

                move |_proxy, event_type, event| {
                    // Debug: log that we received any event
                    log::trace!("Event received: {:?}", event_type);

                    // Check if listener is paused (capture mode)
                    if LISTENER_PAUSED.load(Ordering::SeqCst) {
                        return None;
                    }

                    // Handle ESC key to stop recording (emergency stop - idempotent)
                    if matches!(event_type, CGEventType::KeyDown) {
                        let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);

                        if keycode == KEYCODE_ESC {
                            // Check if currently recording (ESC cancels recording without transcription)
                            let is_recording = {
                                if let Ok(state_guard) = state.lock() {
                                    state_guard.is_recording
                                } else {
                                    false
                                }
                            };

                            if is_recording {
                                log::info!("ESC key pressed - cancelling recording (no transcription)");
                                let state_ref = app.state::<SharedAppState>();
                                // Use cancel_recording_with_app to stop without transcription
                                match crate::state::cancel_recording_with_app(&app, &state_ref) {
                                    Ok(_) => {
                                        log::info!("Recording cancelled via ESC key");
                                    }
                                    Err(e) => {
                                        log::error!("Failed to cancel recording: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    // Handle FlagsChanged event (modifier key press/release)
                    if matches!(event_type, CGEventType::FlagsChanged) {
                        log::info!("FlagsChanged event received!");
                        let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                        let flags = event.get_flags();

                        // Debug: log all modifier key events
                        log::debug!("FlagsChanged: keycode={}, flags={:?}", keycode, flags);

                        // Get current shortcut and push_to_talk mode from state
                        let (shortcut_str, push_to_talk) = {
                            if let Ok(state_guard) = state.lock() {
                                (state_guard.config.shortcut.clone(), state_guard.config.push_to_talk)
                            } else {
                                return None;
                            }
                        };

                        // Check if it's a native shortcut
                        let native_shortcut = match NativeShortcut::from_string(&shortcut_str) {
                            Some(s) => s,
                            None => return None,
                        };

                        log::debug!("Checking keycode {} against expected {}", keycode, native_shortcut.keycode());

                        if keycode == native_shortcut.keycode() {
                            // Check if the modifier key is now pressed by looking at appropriate flag
                            let modifier_pressed = if native_shortcut.uses_option_flag() {
                                flags.contains(CGEventFlags::CGEventFlagAlternate)
                            } else if native_shortcut.uses_command_flag() {
                                flags.contains(CGEventFlags::CGEventFlagCommand)
                            } else if native_shortcut.uses_control_flag() {
                                flags.contains(CGEventFlags::CGEventFlagControl)
                            } else if native_shortcut.uses_shift_flag() {
                                flags.contains(CGEventFlags::CGEventFlagShift)
                            } else {
                                false
                            };

                            log::debug!("Modifier key event: keycode={}, pressed={}", keycode, modifier_pressed);

                            if modifier_pressed && !RIGHT_ALT_PRESSED.load(Ordering::SeqCst) {
                                // Key just pressed
                                RIGHT_ALT_PRESSED.store(true, Ordering::SeqCst);
                                log::info!("Native shortcut pressed (keycode: {}, push_to_talk: {})", keycode, push_to_talk);

                                let state_ref = app.state::<SharedAppState>();
                                if push_to_talk {
                                    // Push-to-talk: start recording on key press
                                    match crate::state::start_recording_with_app(&app, &state_ref) {
                                        Ok(_) => log::info!("Recording started via push-to-talk"),
                                        Err(e) => log::error!("Failed to start recording: {}", e),
                                    }
                                } else {
                                    // Toggle mode: toggle recording on key press
                                    match crate::state::toggle_recording_with_app(&app, &state_ref) {
                                        Ok(snapshot) => {
                                            if snapshot.is_recording {
                                                log::info!("Recording started via native shortcut");
                                            } else {
                                                log::info!("Recording stopped via native shortcut");
                                            }
                                        }
                                        Err(e) => log::error!("Failed to toggle recording: {}", e),
                                    }
                                }
                            } else if !modifier_pressed && RIGHT_ALT_PRESSED.load(Ordering::SeqCst) {
                                // Key released
                                RIGHT_ALT_PRESSED.store(false, Ordering::SeqCst);
                                log::info!("Native shortcut released (push_to_talk: {})", push_to_talk);

                                if push_to_talk {
                                    // Push-to-talk: stop recording on key release
                                    let state_ref = app.state::<SharedAppState>();
                                    match crate::state::stop_recording_with_app(&app, &state_ref) {
                                        Ok(_) => log::info!("Recording stopped via push-to-talk"),
                                        Err(e) => log::error!("Failed to stop recording: {}", e),
                                    }
                                }
                                // Toggle mode: do nothing on key release
                            }
                        }
                    }

                    None // Pass event through unchanged
                }
            },
        );

        match event_tap {
            Ok(tap) => {
                unsafe {
                    let loop_source = tap
                        .mach_port
                        .create_runloop_source(0)
                        .expect("Failed to create run loop source");

                    CFRunLoop::get_current().add_source(&loop_source, kCFRunLoopCommonModes);
                    tap.enable();

                    log::info!("Native keyboard listener started successfully");
                    CFRunLoop::run_current();
                }
            }
            Err(e) => {
                log::error!("Failed to create event tap. Make sure accessibility permissions are granted. Error: {:?}", e);
            }
        }
    });
}

// ============================================================
// Media Key Listener (AirPods/EarPods play/pause button)
// ============================================================

static MEDIA_KEY_ENABLED: AtomicBool = AtomicBool::new(false);

/// Update media key enabled state (called when config changes)
pub fn update_media_key_enabled(enabled: bool) {
    MEDIA_KEY_ENABLED.store(enabled, Ordering::SeqCst);
    log::info!("Media key enabled: {}", enabled);
}

// NX_SYSDEFINED event type (14) is not exposed in the core-graphics crate,
// so we use raw C FFI to create a separate CGEventTap for media key events.
const NX_SYSDEFINED_EVENT_TYPE: u32 = 14;
const NX_KEYTYPE_PLAY: i64 = 16;
const MEDIA_KEY_SUBTYPE: i16 = 8;

// Raw CGEventTap FFI for NX_SYSDEFINED events
mod media_ffi {
    use std::ffi::c_void;

    pub type CGEventTapCallBack = unsafe extern "C" fn(
        proxy: *mut c_void,
        event_type: u32,
        event: *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void;

    extern "C" {
        pub fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> *mut c_void;

        pub fn CGEventTapEnable(tap: *mut c_void, enable: bool);

        pub fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            port: *mut c_void,
            order: i64,
        ) -> *mut c_void;

        pub fn CFRunLoopGetCurrent() -> *mut c_void;

        pub fn CFRunLoopAddSource(
            rl: *mut c_void,
            source: *mut c_void,
            mode: *const c_void,
        );

        pub fn CFRunLoopRun();
    }
}

struct MediaKeyContext {
    app: AppHandle,
    state: SharedAppState,
    tap_port: *mut c_void,
}

// Send + Sync for MediaKeyContext (pointers are only accessed from callback thread)
unsafe impl Send for MediaKeyContext {}
unsafe impl Sync for MediaKeyContext {}

// CGEventTap disabled event types
const CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;
const CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFFFFFF;

/// CGEventTap callback for media key events
unsafe extern "C" fn media_key_tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: *mut c_void,
    user_info: *mut c_void,
) -> *mut c_void {
    // Re-enable tap if it was disabled by the system
    if event_type == CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        log::warn!("Media key event tap was disabled (type={}), re-enabling", event_type);
        let context = &*(user_info as *const MediaKeyContext);
        media_ffi::CGEventTapEnable(context.tap_port, true);
        return event;
    }

    // Only handle NX_SYSDEFINED events
    if event_type != NX_SYSDEFINED_EVENT_TYPE {
        return event;
    }

    log::debug!("NX_SYSDEFINED event received (media_key_enabled={})",
        MEDIA_KEY_ENABLED.load(Ordering::SeqCst));

    // Check if media key handling is enabled
    if !MEDIA_KEY_ENABLED.load(Ordering::SeqCst) {
        return event; // Pass through when disabled
    }

    // Convert CGEvent to NSEvent to access subtype and data1
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    let ns_event: *mut Object = msg_send![class!(NSEvent), eventWithCGEvent: event];
    if ns_event.is_null() {
        log::warn!("Failed to convert CGEvent to NSEvent");
        return event;
    }

    let subtype: i16 = msg_send![ns_event, subtype];
    log::debug!("NX_SYSDEFINED subtype={}", subtype);

    if subtype != MEDIA_KEY_SUBTYPE {
        return event; // Not a media remote event
    }

    let data1: i64 = msg_send![ns_event, data1];
    let key_code = (data1 >> 16) & 0xFFFF;
    let key_flags = data1 & 0xFFFF;
    let key_is_down = ((key_flags & 0xFF00) >> 8) == 0x0A;
    let key_repeat = (key_flags & 0x1) != 0;

    log::debug!(
        "Media key: code={}, down={}, repeat={}, data1=0x{:X}",
        key_code,
        key_is_down,
        key_repeat,
        data1
    );

    if key_code == NX_KEYTYPE_PLAY && key_is_down && !key_repeat {
        log::info!("Play/pause media key pressed - toggling recording");

        let context = &*(user_info as *const MediaKeyContext);
        match crate::state::toggle_recording_with_app(&context.app, &context.state) {
            Ok(snapshot) => {
                if snapshot.is_recording {
                    log::info!("Recording started via media key");
                } else {
                    log::info!("Recording stopped via media key");
                }
            }
            Err(e) => log::error!("Failed to toggle recording via media key: {}", e),
        }

        return std::ptr::null_mut(); // Consume the event (prevent music playback)
    }

    event // Pass through other events
}

/// Start listening for media key events (AirPods/EarPods play/pause button)
#[cfg(target_os = "macos")]
pub fn start_media_key_listener(app: AppHandle, state: SharedAppState) {
    // Read initial config
    if let Ok(state_guard) = state.lock() {
        MEDIA_KEY_ENABLED.store(state_guard.config.media_key_enabled, Ordering::SeqCst);
    }

    std::thread::spawn(move || {
        log::info!("Starting media key listener");

        // First create tap, then create context with tap_port
        unsafe {
            let mask: u64 = 1 << NX_SYSDEFINED_EVENT_TYPE;

            // Create a temporary context to pass to CGEventTapCreate
            // We'll update the tap_port after creation
            let context = Box::new(MediaKeyContext {
                app,
                state,
                tap_port: std::ptr::null_mut(),
            });
            let context_ptr = Box::into_raw(context);

            // Create event tap: HID level, head insert, active (can consume events)
            let tap_port = media_ffi::CGEventTapCreate(
                0, // kCGHIDEventTap
                0, // kCGHeadInsertEventTap
                0, // kCGEventTapOptionDefault (active filter)
                mask,
                media_key_tap_callback,
                context_ptr as *mut c_void,
            );

            if tap_port.is_null() {
                log::error!(
                    "Failed to create media key event tap (check accessibility permissions)"
                );
                let _ = Box::from_raw(context_ptr);
                return;
            }

            // Store tap_port in context for re-enabling
            (*context_ptr).tap_port = tap_port;
            let context_void = context_ptr as *mut c_void;

            // Create run loop source and add to current run loop
            let source = media_ffi::CFMachPortCreateRunLoopSource(
                std::ptr::null(),
                tap_port,
                0,
            );

            if source.is_null() {
                log::error!("Failed to create run loop source for media key tap");
                let _ = Box::from_raw(context_void as *mut MediaKeyContext);
                return;
            }

            let run_loop = media_ffi::CFRunLoopGetCurrent();

            // Use kCFRunLoopCommonModes from core-foundation
            media_ffi::CFRunLoopAddSource(
                run_loop,
                source,
                kCFRunLoopCommonModes as *const _ as *const c_void,
            );
            media_ffi::CGEventTapEnable(tap_port, true);

            log::info!("Media key listener started successfully");
            media_ffi::CFRunLoopRun();
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn start_media_key_listener(_app: AppHandle, _state: SharedAppState) {
    // No-op on non-macOS
}

#[cfg(not(target_os = "macos"))]
pub fn update_media_key_enabled(_enabled: bool) {
    // No-op on non-macOS
}
