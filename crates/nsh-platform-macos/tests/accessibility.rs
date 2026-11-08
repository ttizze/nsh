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
    use std::{
        sync::atomic::{AtomicBool, AtomicU32, Ordering},
        time::{Duration, Instant},
    };

    use nsh_core::{element::Root, prelude::*};
    use nsh_platform_macos::{
        AppOptions, AppView,
        objc2::{
            MainThreadMarker, MainThreadOnly, Message, msg_send, rc::Retained, runtime::AnyObject,
        },
        objc2_app_kit::{
            NSAccessibility, NSApplication, NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask,
        },
        objc2_foundation::{NSDate, NSNumber, NSPoint, NSRect, NSRunLoop, NSSize, NSString},
    };

    /// The test sets it to scroll the scroll area to its end.
    static SCROLLED: AtomicBool = AtomicBool::new(false);

    /// The offset the test scrolls the list of rows to (`u32::MAX`: none).
    static ROWS_AT: AtomicU32 = AtomicU32::new(u32::MAX);

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
            .child(scroll_area())
            .child(rows())
    }

    /// A virtual list of 200 rows, 30 points each, five of them in view.
    #[component]
    fn Rows() -> impl IntoElement {
        let list = element_ref();
        set_interval(Duration::from_millis(20), move || {
            let at = ROWS_AT.swap(u32::MAX, Ordering::SeqCst);
            if at != u32::MAX {
                list.scroll_to(point(px(0.0), px(at as f32)), false);
            }
        });
        virtual_list(200, move |at| {
            text(move || format!("row {}", at.index())).h(px(30.0))
        })
        .estimated_item_size(px(30.0))
        .element_ref(&list)
        .test_id("rows")
        .h(px(150.0))
    }

    /// A scroll view whose content is three times its height.
    #[component]
    fn ScrollArea() -> impl IntoElement {
        let area = element_ref();
        set_interval(Duration::from_millis(20), move || {
            if SCROLLED.swap(false, Ordering::SeqCst) {
                area.scroll_to_end(false);
            }
        });
        scroll()
            .element_ref(&area)
            .test_id("scroll-area")
            .h(px(50.0))
            .child(view().h(px(150.0)))
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

    /// The role and value of the accessibility element `object`.
    fn role_and_value(object: &AnyObject) -> (Option<String>, Option<f64>) {
        // SAFETY: NSAccessibility getters, sent to an element on the main
        // thread.
        let role: Option<Retained<NSString>> = unsafe { msg_send![object, accessibilityRole] };
        let value: Option<Retained<AnyObject>> = unsafe { msg_send![object, accessibilityValue] };
        (
            role.map(|r| r.to_string()),
            value
                .and_then(|v| v.downcast::<NSNumber>().ok())
                .map(|n| n.as_f64()),
        )
    }

    /// The rows the list's accessibility children read, in their order.
    fn rows_read(root: &NSView) -> Vec<u32> {
        let list = view_with_id(root, "rows").expect("the list");
        list.accessibilityChildren()
            .map(|children| {
                children
                    .iter()
                    .filter_map(|c| c.downcast::<NSView>().ok())
                    .filter_map(|c| {
                        let value: Option<Retained<AnyObject>> =
                            // SAFETY: an NSAccessibility getter on the main
                            // thread.
                            unsafe { msg_send![&*c, accessibilityValue] };
                        let text = value?.downcast::<NSString>().ok()?.to_string();
                        text.strip_prefix("row ")?.parse().ok()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Scrolls the list of rows to `offset` and lets it settle.
    fn scroll_rows(offset: u32) {
        ROWS_AT.store(offset, Ordering::SeqCst);
        let soon = NSDate::dateWithTimeIntervalSinceNow(0.3);
        NSRunLoop::currentRunLoop().runUntilDate(&soon);
    }

    pub(super) fn run() {
        let mtm = MainThreadMarker::new().expect("the main thread");
        let _app = NSApplication::sharedApplication(mtm);
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 500.0));
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
                size: (400, 500),
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

        // A scroll view is a scroll area, its scroll bar on its axis telling
        // where it is scrolled to, from 0 at the start to 1 at the end.
        let area = view_with_id(&root, "scroll-area").expect("the scroll area");
        assert_eq!(
            area.accessibilityRole().map(|r| r.to_string()).as_deref(),
            Some("AXScrollArea")
        );
        assert!(area.accessibilityHorizontalScrollBar().is_none());
        let bar = area
            .accessibilityVerticalScrollBar()
            .expect("a vertical bar");
        assert_eq!(
            role_and_value(&bar),
            (Some("AXScrollBar".into()), Some(0.0))
        );
        let children = area.accessibilityChildren().expect("children");
        assert!(
            children
                .iter()
                .any(|c| std::ptr::eq(&*c as *const AnyObject, &*bar as *const AnyObject)),
            "the bar is among the children"
        );
        SCROLLED.store(true, Ordering::SeqCst);
        until("the bar's value at the end", || {
            role_and_value(&bar).1 == Some(1.0)
        });

        // A virtual list scrolled back and forth recycles its cells: they
        // read the rows they show in order, one each, and no row it left
        // behind.
        for offset in [3000, 0, 1500, 600, 2400, 900] {
            scroll_rows(offset);
            let rows = rows_read(&root);
            println!("rows at {offset}: {rows:?}");
            assert!(!rows.is_empty(), "rows at {offset}");
            assert!(
                rows.windows(2).all(|w| w[1] == w[0] + 1),
                "rows at {offset} read in order, one each: {rows:?}"
            );
            let first = offset / 30;
            assert!(
                rows.contains(&first),
                "rows at {offset} include the first row in view, {first}: {rows:?}"
            );
        }

        app.stop();
        window.close();
        println!("accessibility: ok");
    }
}
