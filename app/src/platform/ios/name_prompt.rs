//! The name alert: a `UIAlertController` with one text field.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use futures_channel::oneshot;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadMarker, msg_send};
use objc2_foundation::{
    NSComparisonResult, NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue,
    NSString,
};
use objc2_ui_kit::{
    UIAlertAction, UIAlertActionStyle, UIAlertController, UIAlertControllerStyle, UIApplication,
    UIContentSizeCategoryAccessibilityMedium, UIContentSizeCategoryCompareToCategory,
    UITextAutocorrectionType, UITextField, UITextFieldTextDidChangeNotification,
    UITextFieldViewMode, UITextInputTraits, UITraitEnvironment, UIViewController,
};

use crate::platform::NamePrompt;

pub async fn ask(prompt: NamePrompt) -> Option<String> {
    let (reply, answer) = oneshot::channel();
    DispatchQueue::main().exec_async(move || present(prompt, reply));
    answer.await.ok().flatten()
}

/// The controller on top of the key window.
#[allow(deprecated)]
fn top_controller(mtm: MainThreadMarker) -> Option<Retained<UIViewController>> {
    let mut top = UIApplication::sharedApplication(mtm)
        .keyWindow()?
        .rootViewController()?;
    while let Some(next) = top.presentedViewController() {
        top = next;
    }
    Some(top)
}

/// The keyboard covers the alert's buttons at the largest text sizes.
fn limit_text_size(alert: &UIAlertController) {
    // SAFETY: UIKit's constants and traits, used on the main thread. The app
    // needs iOS 17, which has `traitOverrides`.
    unsafe {
        let limit = UIContentSizeCategoryAccessibilityMedium;
        let size = alert.traitCollection().preferredContentSizeCategory();
        if UIContentSizeCategoryCompareToCategory(&size, limit) == NSComparisonResult::Descending {
            let overrides: *mut AnyObject = msg_send![alert, traitOverrides];
            let _: () = msg_send![overrides, setPreferredContentSizeCategory: limit];
        }
    }
}

fn present(prompt: NamePrompt, reply: oneshot::Sender<Option<String>>) {
    let Some(mtm) = MainThreadMarker::new() else {
        let _ = reply.send(None);
        return;
    };
    let Some(top) = top_controller(mtm) else {
        let _ = reply.send(None);
        return;
    };
    let alert = UIAlertController::alertControllerWithTitle_message_preferredStyle(
        Some(&NSString::from_str(&prompt.title)),
        Some(&NSString::from_str(&prompt.message)),
        UIAlertControllerStyle::Alert,
        mtm,
    );
    let value = NSString::from_str(&prompt.value);
    let configure = RcBlock::new(move |field: NonNull<UITextField>| {
        // SAFETY: UIKit passes the alert's live text field.
        let field = unsafe { field.as_ref() };
        field.setText(Some(&value));
        field.setClearButtonMode(UITextFieldViewMode::WhileEditing);
        field.setAutocorrectionType(UITextAutocorrectionType::No);
    });
    alert.addTextFieldWithConfigurationHandler(Some(&configure));

    let reply = Rc::new(RefCell::new(Some(reply)));
    // The observer that checks the name as it is typed; removed with the
    // alert's answer.
    let observer: Rc<RefCell<Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>>> =
        Rc::new(RefCell::new(None));
    let stop = {
        let observer = observer.clone();
        move || {
            if let Some(token) = observer.borrow_mut().take() {
                // SAFETY: the token came from this center.
                unsafe { NSNotificationCenter::defaultCenter().removeObserver(token.as_ref()) };
            }
        }
    };
    let cancel_reply = reply.clone();
    let cancel_stop = stop.clone();
    let cancel = RcBlock::new(move |_: NonNull<UIAlertAction>| {
        cancel_stop();
        if let Some(reply) = cancel_reply.borrow_mut().take() {
            let _ = reply.send(None);
        }
    });
    let shown = Weak::from_retained(&alert);
    let save = RcBlock::new(move |_: NonNull<UIAlertAction>| {
        stop();
        let text = shown
            .load()
            .and_then(|alert| alert.textFields())
            .and_then(|fields| fields.firstObject())
            .and_then(|field| field.text())
            .map(|text| text.to_string());
        if let Some(reply) = reply.borrow_mut().take() {
            let _ = reply.send(text);
        }
    });
    alert.addAction(&UIAlertAction::actionWithTitle_style_handler(
        Some(&NSString::from_str(&prompt.cancel)),
        UIAlertActionStyle::Cancel,
        Some(&cancel),
        mtm,
    ));
    let save = UIAlertAction::actionWithTitle_style_handler(
        Some(&NSString::from_str(&prompt.save)),
        UIAlertActionStyle::Default,
        Some(&save),
        mtm,
    );
    alert.addAction(&save);
    alert.setPreferredAction(Some(&save));
    if let Some(field) = alert.textFields().and_then(|fields| fields.firstObject()) {
        let original = prompt.value.trim().to_owned();
        let (max, allow_empty) = (prompt.max, prompt.allow_empty);
        let save = save.clone();
        let typed = Weak::from_retained(&field);
        let check = move || {
            let Some(field) = typed.load() else {
                return;
            };
            let mut text = field.text().map(|t| t.to_string()).unwrap_or_default();
            // The camera menu holds `max` characters: longer input stops.
            if text.chars().count() > max {
                text = text.chars().take(max).collect();
                field.setText(Some(&NSString::from_str(&text)));
            }
            let name = text.trim();
            save.setEnabled(valid_name(name, max, allow_empty) && name != original);
        };
        check();
        let block = RcBlock::new(move |_: NonNull<NSNotification>| check());
        // SAFETY: the main queue runs the block on the main thread, where
        // the alert lives; the field is the notification's object.
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(UITextFieldTextDidChangeNotification),
                Some(&field),
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
        *observer.borrow_mut() = Some(token);
    }
    limit_text_size(&alert);
    top.presentViewController_animated_completion(&alert, true, None);
}

/// The camera menu shows printable ASCII only (`domain::looks::valid_name`).
fn valid_name(name: &str, max: usize, allow_empty: bool) -> bool {
    if name.is_empty() {
        return allow_empty;
    }
    name.len() <= max && name.bytes().all(|b| (0x20..=0x7e).contains(&b))
}
