# Noms village preview download

Download this branch using **Code → Download ZIP**, extract the entire ZIP, and open **OPEN-VILLAGE.html** in Chrome or Edge. Keep the `assets` folder alongside the HTML. No Python, build, server, or internet connection is needed after downloading.

This folder includes all twelve full-resolution models. Desktop is recommended. Use **Play tour**, **Overview**, or the stop selector.

The preview is a layout blockout: terrain and connecting paths are provisional, and the models retain their existing lighting. This branch only distributes the review preview and does not change the deployed About page.

Source and Blender world: [PR #89](https://github.com/SamKoCPSC/Noms/pull/89), commit `c945f9fbafc503702f47d6038eb5eba87354eb19`.

The archive avoids the chat attachment download service. The large model data is split into local JavaScript files so no single file exceeds GitHub's file-size limit. Each decoded model matches its production source byte-for-byte; see `sources.json`.

Verification: the browser loaded all twelve models from this package over localhost with no external resource requests. Direct file navigation is blocked by the managed verification browser, so that launch mode could not be verified here.
