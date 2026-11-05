// About's decorative mascot uses the approved player and its shared clock.
// Keep the SVG and animation library lazy: they never gate the 3D story.
export function mountLoadingPenguini(host, assets) {
  const root = host.closest("#about-tour");
  const button = root.querySelector("[data-penguini-pause]");
  const motion = matchMedia("(prefers-reduced-motion: reduce)");
  const abort = new AbortController();
  let disposed = false;
  let active = false;
  let paused = false;
  let preparing;
  let activation = 0;
  let svg;
  let player;
  const renderers = {};

  function updateButton() {
    button.hidden = !player || motion.matches;
    button.textContent = paused ? "Resume animation" : "Pause animation";
    button.setAttribute("aria-pressed", String(paused));
  }

  async function prepare() {
    const [response] = await Promise.all([
      fetch(assets.rig, {signal: abort.signal}),
      ...[assets.clips, assets.player, assets.thinking, assets.dance].map(url => import(url)),
    ]);
    if (!response.ok) throw new Error("Unable to load Penguini.");
    const parsed = new DOMParser().parseFromString(await response.text(), "image/svg+xml");
    if (parsed.querySelector("parsererror")) throw new Error("Invalid Penguini SVG.");
    if (disposed || !host.isConnected) return;
    svg = document.importNode(parsed.documentElement, true);
    // The generated still shares the player's rig, so the fallback and animated
    // view reuse one download. Its resting CSS belongs inside the image only;
    // the player supplies inline poses when it joins the page's DOM.
    svg.querySelector("style")?.remove();
    svg.style.visibility = "hidden";
    svg.setAttribute("aria-hidden", "true");
    svg.setAttribute("focusable", "false");
    // Decode before starting the intro, so slow artwork cannot skip its hops.
    await Promise.all([...svg.querySelectorAll("image")].map(async element => {
      const image = new Image();
      image.src = element.getAttribute("href") || element.getAttributeNS("http://www.w3.org/1999/xlink", "href");
      await image.decode();
    }));
    if (disposed || !host.isConnected) return;
    host.replaceChildren(svg);
    if (!motion.matches) {
      for (const [name, renderer] of [["thinking", window.PenguiniThinkingRig], ["dance", window.PenguiniDanceRig]]) {
        try {
          const instance = await renderer.create(svg);
          if (disposed) { instance.destroy(); return; }
          renderers[name] = instance;
        } catch {
          // SVG-only activities remain available without WebGL.
        }
      }
    }
  }

  function begin() {
    player?.destroy();
    player = new window.PenguiniPlayer(svg, {
      idle: window.PenguiniClips.idle,
      animations: Object.values(window.PenguiniClips).filter(clip => clip.id !== "idle" && (!clip.renderer || Object.hasOwn(renderers, clip.renderer))),
      renderers,
      initialAnimation: "excited",
      autoplay: !paused,
    });
    svg.style.visibility = "";
    updateButton();
  }

  function sync() {
    const visible = root.classList.contains("is-flight") && root.dataset.entryState === "loading";
    if (disposed || visible === active) return;
    active = visible;
    const current = ++activation;
    if (!active) { player?.pause(); return; }
    preparing ??= prepare();
    preparing.then(() => {
      if (!disposed && active && current === activation && host.isConnected && svg) begin();
    }).catch(() => {
      // Keep the static image and the reading fallback usable if setup fails.
      preparing = undefined;
      updateButton();
    });
  }

  function toggle() {
    if (!player || motion.matches) return;
    paused = !paused;
    if (paused || !active) player.pause(); else player.play();
    updateButton();
  }

  function dispose() {
    disposed = true;
    abort.abort();
    observer.disconnect();
    button.removeEventListener("click", toggle);
    motion.removeEventListener("change", updateButton);
    host.removeEventListener("noms-penguini-dispose", dispose);
    player?.destroy();
    for (const renderer of Object.values(renderers)) renderer.destroy();
  }

  const observer = new MutationObserver(sync);
  observer.observe(root, {attributes: true, attributeFilter: ["class", "data-entry-state"]});
  button.addEventListener("click", toggle);
  motion.addEventListener("change", updateButton);
  host.addEventListener("noms-penguini-dispose", dispose, {once: true});
  sync();
  return dispose;
}
