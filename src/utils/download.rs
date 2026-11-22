//! Browser downloads shared by recipe and collection export controls.
pub(crate) async fn download_file(
    content: String,
    filename: String,
    mime: &str,
    fetch_url: Option<String>,
) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue};
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, RequestCredentials, Url};

        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or("No browser document is available.")?;
        let body = document
            .body()
            .ok_or("No browser document body is available.")?;
        let blob = if let Some(url) = fetch_url {
            let response = gloo_net::http::Request::get(&url)
                .credentials(RequestCredentials::SameOrigin)
                .send()
                .await
                .map_err(|_| "Could not fetch the export.")?;
            if !response.ok() {
                return Err("The export download is unavailable.".into());
            }
            // Read directly into a browser Blob, avoiding a ZIP copy through WASM memory.
            let response: web_sys::Response = response.into();
            JsFuture::from(response.blob().map_err(|_| "Could not read the export.")?)
                .await
                .map_err(|_| "Could not read the export.")?
                .dyn_into::<Blob>()
                .map_err(|_| "Could not prepare the export file.")?
        } else {
            let options = BlobPropertyBag::new();
            options.set_type(mime);
            let parts = js_sys::Array::of1(&JsValue::from_str(&content));
            Blob::new_with_str_sequence_and_options(&parts, &options)
                .map_err(|_| "Could not prepare the export file.")?
        };
        let url = Url::create_object_url_with_blob(&blob)
            .map_err(|_| "Could not prepare the download URL.")?;
        let result = (|| -> Result<(), JsValue> {
            let link = document
                .create_element("a")?
                .dyn_into::<HtmlAnchorElement>()?;
            link.set_href(&url);
            link.set_download(&filename);
            body.append_child(&link)?;
            link.click();
            link.remove();
            Ok(())
        })();
        if result.is_ok() {
            // Keep the URL alive long enough for the browser to start saving. This task
            // outlives the dialog, so closing it or navigating does not cancel cleanup.
            wasm_bindgen_futures::spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(1000).await;
                let _ = Url::revoke_object_url(&url);
            });
        } else {
            let _ = Url::revoke_object_url(&url);
        }
        result.map_err(|_| "Could not start the file download.".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Browser event handlers run in the WebAssembly client, not during SSR.
        let _ = (content, filename, mime, fetch_url);
        Err("File downloads require the web client.".into())
    }
}
