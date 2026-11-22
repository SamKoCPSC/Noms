use web_sys::{Document, HtmlElement, ScrollBehavior, ScrollToOptions, Window};

/// Reveal focused fields once while the shared popup owns result placement.
/// Scrolling is limited to a focus/keyboard transition, never ordinary typing.
#[derive(Default)]
pub(super) struct AutocompleteViewport {
    field: Option<HtmlElement>,
    scroller: Option<HtmlElement>,
    positioned: bool,
    viewport_height: f64,
}

impl AutocompleteViewport {
    pub(super) fn update(
        &mut self,
        window: &Window,
        document: &Document,
        root: &HtmlElement,
        keyboard_open: bool,
        visual: Option<(f64, f64)>,
    ) {
        use wasm_bindgen::JsCast;
        let field = document.active_element().filter(|element| {
            keyboard_open
                && element
                    .matches(".ingredient-autocomplete > input[role=combobox]")
                    .unwrap_or(false)
        });
        let field = field.and_then(|field| field.dyn_into::<HtmlElement>().ok());
        let (Some(field), Some((height, top))) = (field, visual) else {
            self.clear();
            let _ = root.remove_attribute("data-mobile-autocomplete");
            return;
        };
        let changed = self.field.as_ref() != Some(&field);
        if changed {
            self.clear();
            self.field = Some(field.clone());
        }
        let may_position = changed || height < self.viewport_height;
        self.viewport_height = height;
        let _ = root.set_attribute("data-mobile-autocomplete", "open");
        if self.scroller.is_none() {
            self.scroller = scroll_parent(window, &field);
            if let Some(scroller) = &self.scroller {
                let _ = scroller.set_attribute("data-autocomplete-scroller", "true");
            }
        }
        let header_bottom = document
            .query_selector(".navbar")
            .ok()
            .flatten()
            .map(|header| header.get_bounding_client_rect())
            .filter(|rect| rect.top() < top + 100.0 && rect.bottom() > top)
            .map_or(top, |rect| rect.bottom());
        let target_top = header_bottom.max(top) + 12.0;
        let rect = field.get_bounding_client_rect();
        let desired_room = (top + height - target_top - rect.height() - 12.0).clamp(0.0, 192.0);
        let available = top + height - rect.bottom() - 12.0;
        if may_position && !self.positioned && (rect.top() < target_top || available < desired_room)
        {
            let distance = rect.top() - target_top;
            if let Some(scroller) = &self.scroller {
                scroller.set_scroll_top(scroller.scroll_top() + distance.round() as i32);
            } else {
                let options = ScrollToOptions::new();
                options.set_top(distance);
                options.set_behavior(ScrollBehavior::Instant);
                window.scroll_by_with_scroll_to_options(&options);
            }
            self.positioned = true;
        }
    }

    fn clear(&mut self) {
        self.field = None;
        if let Some(scroller) = self.scroller.take() {
            let _ = scroller.remove_attribute("data-autocomplete-scroller");
        }
        self.positioned = false;
        self.viewport_height = 0.0;
    }
}

impl Drop for AutocompleteViewport {
    fn drop(&mut self) {
        self.clear();
    }
}

fn scroll_parent(window: &Window, field: &HtmlElement) -> Option<HtmlElement> {
    use wasm_bindgen::JsCast;
    let mut parent = field.parent_element();
    while let Some(element) = parent {
        let overflow = window
            .get_computed_style(&element)
            .ok()
            .flatten()
            .and_then(|style| style.get_property_value("overflow-y").ok());
        if matches!(overflow.as_deref(), Some("auto" | "scroll"))
            && !matches!(element.tag_name().as_str(), "BODY" | "HTML")
        {
            return element.dyn_into::<HtmlElement>().ok();
        }
        parent = element.parent_element();
    }
    None
}
