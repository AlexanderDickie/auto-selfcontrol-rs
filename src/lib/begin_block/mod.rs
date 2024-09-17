use std::{fs::File, sync::{mpsc::{self, Receiver, TryRecvError}, Mutex}};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use chrono::NaiveDateTime;
use enigo::{KeyboardControllable, Key};
use fs2::FileExt;
use objc::{runtime::Object, msg_send};
use once_cell::sync::Lazy;
use objc_foundation::INSString;
use {cocoa::base::id, objc::runtime::Sel};
use {cocoa::base::nil, objc::{declare::ClassDecl, runtime::Class, *}};
use std::thread;
use std::time::Duration;
use std::path::Path;

mod selfcontrol_api;
use selfcontrol_api::{start_sc_until, SelfControlError};

use super::{ResultE, Config};

const LOCK_FILE: &str = "/tmp/auto-self-control-rs.lock";

pub fn begin_block_until(config: &Config, block_end: NaiveDateTime) -> ResultE<()> {
    // Don't attempt to start selfcontrol if another auto-self-control-rs process is already running
    let lock_file = File::create(&Path::new(LOCK_FILE))?;
    if lock_file.try_lock_exclusive().is_err() {
        return Err("auto-self-control-rs is already running".into());
    }

    if selfcontrol_api::is_active(&config.paths.self_control)?.is_some() {
        return Ok(());
    }
    
    let (tx_selfcontrol_event, rx_selfcontrol_event) = mpsc::channel();
    let selfcontrol_path = config.paths.self_control.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                let selfcontrol_output = start_sc_until(&selfcontrol_path, block_end).await;
                tx_selfcontrol_event.send(selfcontrol_output).unwrap();
            }
        });
    });

    if let Some(pswd) = config.auto_password_input.get_pswd()? {
        set_input_pswd(pswd)?;
        set_should_input_pswd(true)?;
        wait_for_sc_completion_and_input_password(rx_selfcontrol_event)
    } else {
        wait_for_sc_completion(rx_selfcontrol_event)
    }
}

struct PswdInput {
    pswd: String,
    should_input: bool,
}

static INPUT_PSWD: Lazy<Mutex<PswdInput>> = Lazy::new(|| Mutex::new(PswdInput {
    pswd: String::new(),
    should_input: false,
}));
    
fn set_input_pswd(pswd: String) -> ResultE<()> {
    let mut lock = INPUT_PSWD.lock()?;
    *lock = PswdInput {
        pswd,
        should_input: true,
    };
    Ok(())
}

fn get_input_pswd() -> Option<String> {
    let lock = INPUT_PSWD.lock().unwrap();
    if lock.should_input {
        return Some(lock.pswd.clone());
    }
    None
}

fn set_should_input_pswd(should_input: bool) -> ResultE<()> {
    let mut lock = INPUT_PSWD.lock()?;
    lock.should_input = should_input;
    Ok(())
}

fn wait_for_sc_completion(rx : Receiver<Result<(), SelfControlError>>) -> ResultE<()> {
    loop {
        if let Ok(sc_output) = rx.recv() {
            match sc_output {
                Ok(()) => return Ok(()),
                Err(SelfControlError::UserCancelledHelper) | Err(SelfControlError::NoInputTimeout) => (),
                Err(e) => return Err(e.into()),
            }
       } else {
            return Ok(());
        }
    }
}

fn wait_for_sc_completion_and_input_password(rx_selfcontrol_output: Receiver<Result<(), SelfControlError>>) -> ResultE<()> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let superclass = Class::get("NSObject").ok_or("Failed to get superclass")?;
        let mut decl = ClassDecl::new("RustNotificationDelegate", superclass).ok_or("Failed to create delegate class")?;

        decl.add_method(
            Sel::register("applicationDidActivate:"),
            application_did_activate as extern "C" fn(&Object, Sel, id),
        );
        let delegate_class = decl.register();
        let delegate: *mut Object = msg_send![delegate_class, new];

        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let notification_center: id = msg_send![workspace, notificationCenter];
        let _: () = msg_send![
            notification_center,
            addObserver: delegate
            selector: Sel::register("applicationDidActivate:")
            name: NSString::alloc(nil).init_str("NSWorkspaceDidActivateApplicationNotification")
            object: nil
        ];

        let run_loop: id = msg_send![class!(NSRunLoop), currentRunLoop];
        
        loop {
            match rx_selfcontrol_output.try_recv() {
                Ok(sc_output) => match sc_output {
                    Ok(()) => return Ok(()),
                    Err(SelfControlError::UserCancelledHelper) | Err(SelfControlError::NoInputTimeout) => {
                        set_should_input_pswd(true)?;
                    },
                    Err(e) => return Err(e.into()),
                }
                Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => (),
            };

            let next_date: *mut Object = msg_send![class!(NSDate), dateWithTimeIntervalSinceNow:0.01];
            let _: () = msg_send![run_loop, runUntilDate: next_date];
        }
    }
}

extern "C" fn application_did_activate(_self: &Object, _cmd: Sel, _notification: id) {
    if let Some(pswd) = get_input_pswd() {
        if get_focused_app_name() == "SecurityAgent" {
            let mut e = enigo::Enigo::new();
            thread::sleep(Duration::from_millis(250));
            e.key_sequence(&pswd);
            e.key_down(Key::Return);
            set_should_input_pswd(false).unwrap();
        }
    }
}

fn get_focused_app_name() -> String {
    unsafe {
        let nsworkspace = class!(NSWorkspace);
        let shared_workspace: *mut Object = msg_send![nsworkspace, sharedWorkspace];
        let frontmost_app: *mut Object = msg_send![shared_workspace, frontmostApplication];
        let localized_name: *mut Object = msg_send![frontmost_app, localizedName];

        let ns_string: &objc_foundation::NSString = &*(localized_name as *mut objc_foundation::NSString);
        let app_name = ns_string.as_str().to_owned();
        return app_name;
    }
}

