//! The name alert: a `UIAlertController` with one text field.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use futures_channel::oneshot;
use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2_foundation::NSString;
use objc2_ui_kit::{
    UIAlertAction, UIAlertActionStyle, UIAlertController, UIAlertControllerStyle, UIApplication,
    UITextAutocorrectionType, UITextField, UITextFieldViewMode, UITextInputTraits,
    UIViewController,
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
    let cancel_reply = reply.clone();
    let cancel = RcBlock::new(move |_: NonNull<UIAlertAction>| {
        if let Some(reply) = cancel_reply.borrow_mut().take() {
            let _ = reply.send(None);
        }
    });
    let shown = Weak::from_retained(&alert);
    let save = RcBlock::new(move |_: NonNull<UIAlertAction>| {
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
    top.presentViewController_animated_completion(&alert, true, None);
}
