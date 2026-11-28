//! The platform's own loader for `native(ACTIVITY_COMPONENT)`: a
//! `UIActivityIndicatorView`, as the system and the camera app show one.
//! Props `light` and `dark`: its color in each appearance, as `#RRGGBB`.

use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use nsh_core::JsonValue;
use nsh_platform_ios::{ComponentContext, Components, NativeComponent};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_ui_kit::{
    UIActivityIndicatorView, UIActivityIndicatorViewStyle, UIColor, UITraitCollection,
    UIUserInterfaceStyle, UIView,
};

use crate::ui::ACTIVITY_COMPONENT;

struct Activity {
    view: Retained<UIActivityIndicatorView>,
    light: Option<Retained<UIColor>>,
    dark: Option<Retained<UIColor>>,
}

/// `#RRGGBB` as an opaque color.
fn color(value: Option<&JsonValue>) -> Option<Retained<UIColor>> {
    let hex = value?.as_str()?.strip_prefix('#')?;
    let rgb = u32::from_str_radix(hex.get(..6)?, 16).ok()?;
    let part = |shift: u32| f64::from((rgb >> shift) & 0xff) / 255.0;
    Some(UIColor::colorWithRed_green_blue_alpha(
        part(16),
        part(8),
        part(0),
        1.0,
    ))
}

impl Activity {
    fn new(mtm: MainThreadMarker) -> Self {
        let view = UIActivityIndicatorView::initWithActivityIndicatorStyle(
            mtm.alloc(),
            UIActivityIndicatorViewStyle::Medium,
        );
        view.setHidesWhenStopped(false);
        view.startAnimating();
        Self {
            view,
            light: None,
            dark: None,
        }
    }

    /// One color that follows the appearance, as the theme's tokens do.
    fn apply_color(&self) {
        let (Some(light), Some(dark)) = (self.light.clone(), self.dark.clone()) else {
            return;
        };
        let provider = RcBlock::new(move |traits: NonNull<UITraitCollection>| {
            // SAFETY: UIKit passes a valid trait collection for the call.
            let style = unsafe { traits.as_ref().userInterfaceStyle() };
            let chosen = if style == UIUserInterfaceStyle::Dark {
                &dark
            } else {
                &light
            };
            // The block owns both colors, so the pointer outlives the call.
            NonNull::from(&**chosen)
        });
        // SAFETY: the provider returns a color the block keeps alive.
        let dynamic = unsafe { UIColor::colorWithDynamicProvider(&provider) };
        // SAFETY: a valid color for the indicator.
        unsafe { self.view.setColor(Some(&dynamic)) };
    }
}

impl NativeComponent for Activity {
    fn view(&self) -> &UIView {
        &self.view
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        match name {
            "light" => self.light = color(value),
            "dark" => self.dark = color(value),
            _ => return,
        }
        self.apply_color();
    }
}

impl Drop for Activity {
    fn drop(&mut self) {
        self.view.stopAnimating();
    }
}

pub fn register(components: &Components) {
    components.register(
        ACTIVITY_COMPONENT,
        Rc::new(|_: ComponentContext| {
            let mtm = MainThreadMarker::new().expect("components are made on the main thread");
            Box::new(Activity::new(mtm)) as Box<dyn NativeComponent>
        }),
    );
}
