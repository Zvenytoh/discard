// Installed before Vencord captures getDisplayMedia; selection arrives after its picker.
(() => {
    if (window.top !== window || location.origin !== "https://discord.com") return;
    const devices = navigator.mediaDevices;
    if (!devices?.getDisplayMedia) return;
    const capture = devices.getDisplayMedia.bind(devices);
    const bridge = window.__discardStream = { profile: null, track: null };
    devices.getDisplayMedia = async (...args) => {
        const options = args[0];
        const picker = window.Vencord?.Settings?.plugins?.WebScreenShare;
        const height = Number(options?.video?.height);
        const framerate = Number(options?.video?.frameRate);
        if (!picker?.enabled || ![480, 720, 1080, 1440, 2160].includes(height) || ![15, 30, 60, 120].includes(framerate)) return capture(...args);
        try { bridge.check?.(true); }
        catch (error) { console.error("Discard stream compatibility:", error); }
        // Replace inherited limits from the previous share, preserving source aspect ratio.
        const video = { ...options.video, height: { ideal: height, max: height }, frameRate: { ideal: framerate, max: framerate } };
        delete video.width;
        delete video.advanced;
        const stream = await capture({ ...options, audio: picker.systemAudio ? false : options.audio, video });
        const track = stream.getVideoTracks()[0];
        if (!track || track.readyState !== "live") return stream;
        const actual = track.getSettings();
        bridge.profile = { height, framerate, width: actual.width, captureHeight: actual.height };
        bridge.track = track;
        const clear = () => {
            if (bridge.track !== track) return;
            bridge.profile = bridge.track = null;
        };
        track.addEventListener("ended", clear, { once: true });
        const stop = track.stop.bind(track);
        track.stop = () => { stop(); clear(); };
        // Plugin integration failure must never discard an authorized browser capture.
        try { bridge.changed?.(); }
        catch (error) { console.error("Discard stream profile:", error); }
        setTimeout(() => {
            if (bridge.track !== track || track.readyState !== "live") return;
            try { bridge.refresh?.(); }
            catch (error) { console.error("Discard stream profile:", error); }
        }, 0);
        return stream;
    };
})();
