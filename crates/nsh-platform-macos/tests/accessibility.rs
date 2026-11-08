//! What AppKit's accessibility clients read of mounted views: a real
//! `AppView` in a window, its elements walked through `accessibilityChildren`
//! as VoiceOver and UI tests walk them. AppKit views live on the main thread,
//! so this test runs without libtest's harness, on the process's own.

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod macos {
    use std::time::{Duration, Instant};

    use nsh_core::{element::Root, prelude::*};
    use nsh_platform_macos::{
        AppOptions, AppView,
        objc2::{MainThreadMarker, MainThreadOnly, Message, rc::Retained},
        objc2_app_kit::{
            NSAccessibility, NSApplication, NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask,
        },
        objc2_foundation::{NSDate, NSPoint, NSRect, NSRunLoop, NSSize},
    };

    /// A scroll view and a plain view, each holding two texts made bottom
    /// first, as a list's recycled cells can be.
    #[component]
    fn Screen() -> impl IntoElement {
        let two = |second: &'static str, first: &'static str| {
            [
                text("Second").test_id(second).absolute().top(px(40.0)),
                text("First").test_id(first).absolute().top(px(0.0)),
            ]
        };
        view()
            .child(
                scroll()
                    .test_id("scrolled")
                    .h(px(80.0))
                    .children(two("scrolled-second", "scrolled-first")),
            )
            .child(
                view()
                    .test_id("plain")
                    .h(px(80.0))
                    .children(two("plain-second", "plain-first")),
            )
    }

    /// Runs the main run loop until `done` holds, for at most 5 s.
    fn until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            assert!(Instant::now() < deadline, "{what} never happened");
            let soon = NSDate::dateWithTimeIntervalSinceNow(0.02);
            NSRunLoop::currentRunLoop().runUntilDate(&soon);
        }
    }

    /// The view with identifier `id` under `view`, through the view tree
    /// (it need not be an element).
    fn view_with_id(view: &NSView, id: &str) -> Option<Retained<NSView>> {
        if view
            .accessibilityIdentifier()
            .is_some_and(|i| i.to_string() == id)
        {
            return Some(view.retain());
        }
        view.subviews().iter().find_map(|c| view_with_id(&c, id))
    }

    /// The identifiers of the accessibility children of view `id`, in
    /// their order.
    fn children_of(root: &NSView, id: &str) -> Vec<String> {
        let view = view_with_id(root, id).unwrap_or_else(|| panic!("no view {id}"));
        view.accessibilityChildren()
            .map(|children| {
                children
                    .iter()
                    .filter_map(|c| c.downcast::<NSView>().ok())
                    .filter_map(|c| c.accessibilityIdentifier().map(|i| i.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn run() {
        let mtm = MainThreadMarker::new().expect("the main thread");
        let _app = NSApplication::sharedApplication(mtm);
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
        // SAFETY: a plain titled window, released by the test's end.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        let app = AppView::new(
            mtm,
            AppOptions {
                size: (400, 300),
                ..AppOptions::default()
            },
        );
        window.setContentView(Some(&app.view()));
        window.orderFrontRegardless();
        app.load(Root::of::<Screen>().into());
        let root = app.view();

        // A scroll view's content is read in the order it is on screen,
        // not the order its views were made in; elsewhere the tree's order
        // stays.
        until("the first mount", || {
            view_with_id(&root, "plain-first").is_some()
        });
        assert_eq!(
            children_of(&root, "scrolled"),
            ["scrolled-first", "scrolled-second"]
        );
        assert_eq!(children_of(&root, "plain"), ["plain-second", "plain-first"]);

        app.stop();
        window.close();
        println!("accessibility: ok");
    }
}
