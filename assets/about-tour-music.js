// The element stays mounted as the camera changes mode. Leaving About stops playback.
class TourSoundtrack extends HTMLElement {
  connectedCallback() {
    this.events = new AbortController();
    const { signal } = this.events;
    queueMicrotask(() => {
      if (signal.aborted) return;
      const tour = this.closest('.about-tour');
      const button = this.querySelector('.tour-music-play');
      const status = this.querySelector('.tour-music-status');
      const tracks = Array.from(this.querySelectorAll('[data-audio]'));
      const audio = this.querySelector('audio');
      this.audio = audio;

      let remaining = [];
      let current = null;
      let active = false;
      let attempt = 0;
      let waitingForGesture = false;
      const failed = new Set();

      function render(message) {
        button.textContent = active ? 'Pause music' : 'Play music';
        status.textContent = message;
      }

      render('');

      function play() {
        waitingForGesture = false;
        const request = ++attempt;
        active = true;
        render(current.textContent);
        audio.play().catch(error => {
          if (signal.aborted || request !== attempt || error.name === 'AbortError') return;
          // File errors also dispatch an error event, which advances the playlist.
          if (audio.error) return;
          active = false;
          waitingForGesture = error.name === 'NotAllowedError';
          render('Playback paused. Press Play music to resume.');
        });
      }

      function next() {
        remaining = remaining.filter(track => !failed.has(track));
        if (!remaining.length) remaining = tracks.filter(track => !failed.has(track));
        if (!remaining.length) {
          active = false;
          current = null;
          render('Music is unavailable. Press Play music to retry.');
          return;
        }
        // Draw without replacement. Across rounds, avoid the song that just ended.
        const choices = remaining.length > 1 ? remaining.filter(track => track !== current) : remaining;
        current = choices[Math.floor(Math.random() * choices.length)];
        remaining.splice(remaining.indexOf(current), 1);
        audio.src = current.dataset.audio;
        play();
      }

      function startWhenReady() {
        if (tour.dataset.entryState === 'ready') {
          ready.disconnect();
          next();
        }
      }
      const ready = new MutationObserver(startWhenReady);
      ready.observe(tour, { attributes: true, attributeFilter: ['data-entry-state'] });
      signal.addEventListener('abort', () => ready.disconnect(), { once: true });
      startWhenReady();

      // Browsers can require user activation before the first audible play.
      function retryOnInteraction(event) {
        if (!waitingForGesture || !event.isTrusted || button.contains(event.target)) return;
        if (event.type === 'keydown' && (event.key === 'Escape' || event.key === 'Tab' ||
            event.altKey || event.ctrlKey || event.metaKey || event.repeat)) return;
        play();
      }
      tour.addEventListener('click', retryOnInteraction, { capture: true, signal });
      tour.addEventListener('keydown', retryOnInteraction, { capture: true, signal });

      button.addEventListener('click', () => {
        ready.disconnect();
        waitingForGesture = false;
        if (active) {
          active = false;
          ++attempt;
          audio.pause();
          render('Paused · ' + current.textContent);
        } else if (current && !audio.error) {
          play();
        } else {
          failed.clear();
          next();
        }
      }, { signal });
      audio.addEventListener('ended', () => { if (active) next(); }, { signal });
      audio.addEventListener('error', () => {
        if (!current || !audio.error) return;
        failed.add(current);
        if (active) next();
      }, { signal });
    });
  }

  disconnectedCallback() {
    this.events.abort();
    if (this.audio) {
      this.audio.pause();
      this.audio.removeAttribute('src');
      this.audio.load();
      this.audio = null;
    }
  }
}

if (!customElements.get('noms-tour-soundtrack')) {
  customElements.define('noms-tour-soundtrack', TourSoundtrack);
}
